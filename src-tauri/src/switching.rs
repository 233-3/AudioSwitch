//! 切换流程编排。
//!
//! 职责边界：
//! - 循环/直达的**决策**来自纯函数模块 [`crate::audio::cycle`]；
//! - **真实读写**来自 [`crate::audio::default_endpoint`]；
//! - 本模块只负责把它们串起来，并驱动托盘反馈与前端事件。
//!
//! 并发控制：同一时刻只允许一次切换。新的请求在锁被占用时立即以「忙碌」返回，
//! 不排队、不重试，避免用户连续点击导致设备状态错乱。

use std::sync::atomic::Ordering;
use std::sync::Arc;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::audio::cycle::{plan_direct, plan_next, CycleEntry, CyclePlan, DirectPlan};
use crate::audio::default_endpoint;
use crate::refresh::{self, EVENT_SWITCH};
use crate::state::{AppRuntime, FeedbackKind};
use crate::tray::{self, IconState, ERROR_HOLD, SUCCESS_HOLD};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwitchSource {
    TrayClick,
    TrayMenu,
    Command,
}

impl SwitchSource {
    pub fn label(self) -> &'static str {
        match self {
            SwitchSource::TrayClick => "托盘左键",
            SwitchSource::TrayMenu => "托盘菜单",
            SwitchSource::Command => "界面命令",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SwitchOutcome {
    pub ok: bool,
    /// 是否真的改变了系统默认设备
    pub changed: bool,
    /// 是否因为已有切换在进行而被忽略
    pub busy: bool,
    pub endpoint_id: Option<String>,
    pub name: Option<String>,
    /// info | success | error
    pub kind: &'static str,
    pub message: String,
}

impl SwitchOutcome {
    fn info(message: impl Into<String>) -> Self {
        Self {
            ok: true,
            changed: false,
            busy: false,
            endpoint_id: None,
            name: None,
            kind: "info",
            message: message.into(),
        }
    }

    fn error(message: impl Into<String>) -> Self {
        Self {
            ok: false,
            changed: false,
            busy: false,
            endpoint_id: None,
            name: None,
            kind: "error",
            message: message.into(),
        }
    }

    fn success(endpoint_id: String, name: String) -> Self {
        Self {
            ok: true,
            changed: true,
            busy: false,
            endpoint_id: Some(endpoint_id),
            name: Some(name.clone()),
            kind: "success",
            message: crate::i18n::switched_to(&name),
        }
    }

    fn busy() -> Self {
        Self {
            ok: false,
            changed: false,
            busy: true,
            endpoint_id: None,
            name: None,
            kind: "info",
            message: crate::i18n::switch_busy().to_string(),
        }
    }

    fn with_target(mut self, endpoint_id: &str, name: &str) -> Self {
        self.endpoint_id = Some(endpoint_id.to_string());
        self.name = Some(name.to_string());
        self
    }
}

fn finish(app: &AppHandle, runtime: &Arc<AppRuntime>, outcome: SwitchOutcome) -> SwitchOutcome {
    let kind = match outcome.kind {
        "success" => FeedbackKind::Success,
        "error" => FeedbackKind::Error,
        _ => FeedbackKind::Info,
    };
    runtime.set_feedback(kind, outcome.message.clone());
    if let Err(err) = app.emit(EVENT_SWITCH, &outcome) {
        log::warn!("向前端推送切换结果失败：{err}");
    }
    outcome
}

/// 切换到下一个设备（托盘左键、托盘菜单「切换到下一个设备」、界面按钮共用）。
pub async fn cycle_next(app: &AppHandle, source: SwitchSource) -> SwitchOutcome {
    let runtime = app.state::<Arc<AppRuntime>>().inner().clone();

    let guard = match runtime.switch_lock.try_lock() {
        Ok(guard) => guard,
        Err(_) => {
            log::warn!("[{}] 上一次切换尚未完成，忽略本次请求", source.label());
            return SwitchOutcome::busy();
        }
    };

    runtime.switching.store(true, Ordering::SeqCst);
    refresh::emit_state(app, &runtime);

    log::info!("[{}] 开始切换到下一个设备", source.label());
    let outcome = run_cycle(app, &runtime).await;

    runtime.switching.store(false, Ordering::SeqCst);
    let outcome = finish(app, &runtime, outcome);
    refresh::emit_state(app, &runtime);
    drop(guard);

    outcome
}

async fn run_cycle(app: &AppHandle, runtime: &Arc<AppRuntime>) -> SwitchOutcome {
    // 1) 读取真实状态并计算目标（COM 调用放到阻塞线程）
    let runtime_task = runtime.clone();
    let plan = match tauri::async_runtime::spawn_blocking(move || {
        refresh::refresh(&runtime_task);
        let entries: Vec<CycleEntry> = runtime_task.with_core(|core| core.cycle_entries());
        let current = runtime_task.with_core(|core| core.current_default());
        plan_next(current.as_deref(), &entries)
    })
    .await
    {
        Ok(plan) => plan,
        Err(err) => {
            let message = crate::i18n::switch_task_failed(&err.to_string());
            log::error!("{message}");
            tray::flash(app, IconState::Error, ERROR_HOLD);
            return SwitchOutcome::error(message);
        }
    };

    match plan {
        CyclePlan::NoAvailableDevice => {
            let message = crate::i18n::no_available_device().to_string();
            log::warn!("{message}");
            // 没有发生真正的切换，用错误图标如实反馈
            tray::flash(app, IconState::Error, ERROR_HOLD);
            refresh::emit_state(app, runtime);
            SwitchOutcome::error(message)
        }
        CyclePlan::SingleDeviceNoop { endpoint_id, name } => {
            let message = crate::i18n::single_device(&name);
            log::info!("{message}");
            tray::set_state(app, IconState::Normal);
            refresh::emit_state(app, runtime);
            SwitchOutcome::info(message).with_target(&endpoint_id, &name)
        }
        CyclePlan::SwitchTo {
            endpoint_id,
            name,
            position,
        } => {
            log::info!(
                "循环切换目标：{name}（可用序列第 {} 位）id={endpoint_id}",
                position + 1
            );
            execute_switch(app, runtime, endpoint_id, name).await
        }
    }
}

/// 直接切换到指定设备。
pub async fn switch_to(app: &AppHandle, endpoint_id: &str, source: SwitchSource) -> SwitchOutcome {
    let runtime = app.state::<Arc<AppRuntime>>().inner().clone();

    let guard = match runtime.switch_lock.try_lock() {
        Ok(guard) => guard,
        Err(_) => {
            log::warn!("[{}] 上一次切换尚未完成，忽略本次请求", source.label());
            return SwitchOutcome::busy();
        }
    };

    runtime.switching.store(true, Ordering::SeqCst);
    refresh::emit_state(app, &runtime);

    log::info!("[{}] 请求直接切换到 {endpoint_id}", source.label());
    let outcome = run_direct(app, &runtime, endpoint_id).await;

    runtime.switching.store(false, Ordering::SeqCst);
    let outcome = finish(app, &runtime, outcome);
    refresh::emit_state(app, &runtime);
    drop(guard);
    outcome
}

async fn run_direct(
    app: &AppHandle,
    runtime: &Arc<AppRuntime>,
    endpoint_id: &str,
) -> SwitchOutcome {
    let target = endpoint_id.to_string();
    let runtime_task = runtime.clone();

    let plan = match tauri::async_runtime::spawn_blocking(move || {
        refresh::refresh(&runtime_task);

        let (entry, current) = runtime_task.with_core(|core| {
            let live = core.find_endpoint(&target);
            let entry = live.map(|endpoint| {
                CycleEntry::new(
                    endpoint.id.clone(),
                    endpoint.display_name(),
                    core.config
                        .find(&endpoint.id)
                        .map(|d| d.enabled)
                        .unwrap_or(true),
                    endpoint.state.is_active(),
                )
            });
            (entry, core.current_default())
        });

        match entry {
            Some(entry) => Ok(plan_direct(&entry, current.as_deref())),
            None => Err(crate::i18n::device_missing(&target)),
        }
    })
    .await
    {
        Ok(Ok(plan)) => plan,
        Ok(Err(message)) => {
            log::warn!("直接切换失败：{message}");
            tray::flash(app, IconState::Error, ERROR_HOLD);
            refresh::emit_state(app, runtime);
            return SwitchOutcome::error(message);
        }
        Err(err) => {
            let message = crate::i18n::switch_task_failed(&err.to_string());
            log::error!("{message}");
            tray::flash(app, IconState::Error, ERROR_HOLD);
            return SwitchOutcome::error(message);
        }
    };

    match plan {
        DirectPlan::AlreadyDefault { endpoint_id, name } => {
            let message = crate::i18n::already_default(&name);
            log::info!("{message}");
            tray::set_state(app, IconState::Normal);
            refresh::emit_state(app, runtime);
            SwitchOutcome::info(message).with_target(&endpoint_id, &name)
        }
        DirectPlan::NotAvailable { name, .. } => {
            let message = crate::i18n::device_unavailable(&name);
            log::warn!("{message}");
            tray::flash(app, IconState::Error, ERROR_HOLD);
            refresh::emit_state(app, runtime);
            SwitchOutcome::error(message)
        }
        DirectPlan::SwitchTo { endpoint_id, name } => {
            log::info!("直接切换目标：{name} id={endpoint_id}");
            execute_switch(app, runtime, endpoint_id, name).await
        }
    }
}

/// 真正执行切换：立即显示「切换中」图标 → 调用系统接口 → 回读验证 → 反馈。
async fn execute_switch(
    app: &AppHandle,
    runtime: &Arc<AppRuntime>,
    endpoint_id: String,
    name: String,
) -> SwitchOutcome {
    // 先切换到「切换中」图标，再执行耗时操作，保证用户立刻看到反馈
    tray::set_state(app, IconState::Switching);
    refresh::emit_state(app, runtime);

    let target = endpoint_id.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        default_endpoint::set_default_and_verify(&target)
    })
    .await;

    match result {
        Ok(Ok(())) => {
            // 验证通过后，用真实状态刷新界面与 tooltip
            let runtime_task = runtime.clone();
            let _ =
                tauri::async_runtime::spawn_blocking(move || refresh::refresh(&runtime_task)).await;

            let actual_name = runtime.with_core(|core| {
                core.defaults
                    .playback()
                    .and_then(|id| {
                        core.find_endpoint(id)
                            .map(|endpoint| endpoint.display_name())
                    })
                    .unwrap_or_else(|| name.clone())
            });

            log::info!("切换成功：{actual_name}（{endpoint_id}）");
            tray::flash(app, IconState::Success, SUCCESS_HOLD);
            refresh::emit_state(app, runtime);
            SwitchOutcome::success(endpoint_id, actual_name)
        }
        Ok(Err(err)) => {
            let message = crate::i18n::switch_failed(&name, &err.to_string());
            log::error!("{message}");

            // 失败后依然刷新真实状态，保证 UI、tooltip 与实际设备一致
            let runtime_task = runtime.clone();
            let _ =
                tauri::async_runtime::spawn_blocking(move || refresh::refresh(&runtime_task)).await;

            tray::flash(app, IconState::Error, ERROR_HOLD);
            refresh::emit_state(app, runtime);
            SwitchOutcome::error(message).with_target(&endpoint_id, &name)
        }
        Err(err) => {
            let message = crate::i18n::switch_task_failed(&err.to_string());
            log::error!("{message}");
            tray::flash(app, IconState::Error, ERROR_HOLD);
            refresh::emit_state(app, runtime);
            SwitchOutcome::error(message).with_target(&endpoint_id, &name)
        }
    }
}
