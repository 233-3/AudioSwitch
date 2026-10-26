//! 真实设备状态的同步。
//!
//! 把 Windows 的真实状态（设备列表 + 各角色默认端点）刷新到内存，并更新配置中的
//! 「最近识别到的名称」。所有需要读取实时状态的流程都必须先调用这里，
//! 避免用内存缓存做出「当前设备是哪个」的判断。

use std::sync::Arc;

use tauri::{AppHandle, Emitter};

use crate::audio::default_endpoint;
use crate::audio::device;
use crate::audio::types::DefaultEndpoints;
use crate::state::AppRuntime;

/// 前端监听的状态变化事件名。
pub const EVENT_STATE: &str = "audio-state-changed";
/// 前端监听的切换结果事件名。
pub const EVENT_SWITCH: &str = "audio-switch-result";
/// 前端监听的界面导航事件名（例如从托盘「设备配置」进入）。
pub const EVENT_NAVIGATE: &str = "ui-navigate";

/// 同步真实设备状态到内存。可能涉及 COM 调用，应在阻塞线程中执行。
///
/// 返回设备名称是否发生变化（发生变化时需要持久化配置）。
pub fn refresh(runtime: &AppRuntime) -> bool {
    let endpoints_result = device::enumerate_endpoints();
    let defaults_result = default_endpoint::read_defaults();

    let mut names_changed = false;

    runtime.with_core(|core| {
        match endpoints_result {
            Ok(endpoints) => {
                core.device_error = None;
                core.endpoints = endpoints;
            }
            Err(err) => {
                let message = err.to_string();
                log::warn!("枚举音频输出设备失败：{message}");
                core.device_error = Some(message);
                core.endpoints.clear();
            }
        }

        match defaults_result {
            Ok(defaults) => core.defaults = defaults,
            Err(err) => {
                log::warn!("读取默认输出设备失败：{err}");
                core.defaults = DefaultEndpoints::default();
            }
        }

        // 地址借用冲突：先把 (id, name) 收集出来再更新配置
        let names: Vec<(String, String)> = core
            .endpoints
            .iter()
            .map(|endpoint| (endpoint.id.clone(), endpoint.display_name()))
            .collect();

        for (id, name) in names {
            if core.config.update_known_name(&id, &name) {
                names_changed = true;
            }
        }
    });

    if names_changed {
        if let Err(err) = runtime.persist_config() {
            log::warn!("同步设备名称后保存配置失败：{err}");
        }
    }

    names_changed
}

/// 把最新快照推送给前端。
pub fn emit_state(app: &AppHandle, runtime: &Arc<AppRuntime>) {
    let snapshot = runtime.snapshot();
    if let Err(err) = app.emit(EVENT_STATE, snapshot) {
        log::warn!("向前端推送状态失败：{err}");
    }
}

/// 刷新并同步托盘与前端（供监听线程、命令与切换流程共用）。
///
/// 该函数内部包含阻塞的 COM 调用，必须在阻塞线程中执行。
pub fn refresh_and_broadcast(app: &AppHandle, runtime: &Arc<AppRuntime>) -> bool {
    let changed = refresh(runtime);
    crate::tray::render(app, runtime);
    emit_state(app, runtime);
    changed
}
