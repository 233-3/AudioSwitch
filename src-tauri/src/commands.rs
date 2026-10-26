//! 前后端命令接口。
//!
//! 约定：
//! - 所有命令返回 `Result<_, String>`，错误信息直接给用户看，且已经写入日志；
//! - 任何修改配置的命令都会立即持久化，并刷新托盘菜单与前端快照；
//! - 涉及 COM 的耗时操作放到阻塞线程，避免阻塞界面。

use std::sync::Arc;

use tauri::{AppHandle, State};

use crate::audio::device;
use crate::refresh;
use crate::state::{AppRuntime, AppSnapshot, FeedbackKind};
use crate::switching::{self, SwitchOutcome, SwitchSource};

type SharedRuntime<'a> = State<'a, Arc<AppRuntime>>;

fn runtime_of(state: &State<'_, Arc<AppRuntime>>) -> Arc<AppRuntime> {
    state.inner().clone()
}

/// 同步配置变更后的界面与托盘。
fn after_config_change(app: &AppHandle, runtime: &Arc<AppRuntime>, message: &str) {
    runtime.set_feedback(FeedbackKind::Success, message);
    crate::tray::render(app, runtime);
    refresh::emit_state(app, runtime);
}

#[tauri::command]
pub async fn get_snapshot(state: SharedRuntime<'_>) -> Result<AppSnapshot, String> {
    Ok(runtime_of(&state).snapshot())
}

/// 重新枚举设备并广播最新状态。
#[tauri::command]
pub async fn refresh_devices(
    app: AppHandle,
    state: SharedRuntime<'_>,
) -> Result<AppSnapshot, String> {
    let runtime = runtime_of(&state);
    let app_for_task = app.clone();
    let runtime_for_task = runtime.clone();

    tauri::async_runtime::spawn_blocking(move || {
        refresh::refresh_and_broadcast(&app_for_task, &runtime_for_task);
    })
    .await
    .map_err(|err| format!("刷新设备失败：{err}"))?;

    let has_error = runtime.with_core(|core| core.device_error.is_some());
    if has_error {
        runtime.set_feedback(FeedbackKind::Error, crate::i18n::refresh_failed());
    } else {
        runtime.set_feedback(FeedbackKind::Info, crate::i18n::devices_refreshed());
    }
    refresh::emit_state(&app, &runtime);
    Ok(runtime.snapshot())
}

/// 切换到下一个设备（界面按钮）。
#[tauri::command]
pub async fn switch_next(
    app: AppHandle,
    state: SharedRuntime<'_>,
) -> Result<SwitchOutcome, String> {
    let _ = runtime_of(&state);
    Ok(switching::cycle_next(&app, SwitchSource::Command).await)
}

/// 直接切换到指定设备。
#[tauri::command]
pub async fn switch_to_device(
    app: AppHandle,
    endpoint_id: String,
    state: SharedRuntime<'_>,
) -> Result<SwitchOutcome, String> {
    if !device::is_valid_endpoint_id(&endpoint_id) {
        return Err(crate::i18n::err_invalid_endpoint().to_string());
    }
    let _ = runtime_of(&state);
    Ok(switching::switch_to(&app, &endpoint_id, SwitchSource::Command).await)
}

/// 把设备加入循环列表。
#[tauri::command]
pub async fn add_device(
    app: AppHandle,
    endpoint_id: String,
    state: SharedRuntime<'_>,
) -> Result<AppSnapshot, String> {
    if !device::is_valid_endpoint_id(&endpoint_id) {
        return Err(crate::i18n::err_invalid_endpoint().to_string());
    }

    let runtime = runtime_of(&state);
    let name = runtime.with_core(|core| {
        core.find_endpoint(&endpoint_id)
            .map(|endpoint| endpoint.display_name())
    });

    let Some(name) = name else {
        return Err(crate::i18n::err_device_not_present().to_string());
    };

    let added = runtime.with_core(|core| core.config.add_device(&endpoint_id, &name));
    if !added {
        return Err(crate::i18n::err_already_in_cycle().to_string());
    }

    runtime.persist_config()?;
    log::info!("已加入循环列表：{name}（{endpoint_id}）");
    after_config_change(&app, &runtime, &crate::i18n::added_to_cycle(&name));
    Ok(runtime.snapshot())
}

/// 从循环列表移除设备（仅用户显式操作时调用）。
#[tauri::command]
pub async fn remove_device(
    app: AppHandle,
    endpoint_id: String,
    state: SharedRuntime<'_>,
) -> Result<AppSnapshot, String> {
    let runtime = runtime_of(&state);
    let removed = runtime.with_core(|core| core.config.remove_device(&endpoint_id));
    if !removed {
        return Err(crate::i18n::err_not_in_cycle().to_string());
    }

    runtime.persist_config()?;
    log::info!("已从循环列表移除：{endpoint_id}");
    after_config_change(&app, &runtime, crate::i18n::removed_from_cycle());
    Ok(runtime.snapshot())
}

/// 启用/禁用某个已配置设备。
#[tauri::command]
pub async fn set_device_enabled(
    app: AppHandle,
    endpoint_id: String,
    enabled: bool,
    state: SharedRuntime<'_>,
) -> Result<AppSnapshot, String> {
    let runtime = runtime_of(&state);
    let changed = runtime.with_core(|core| core.config.set_enabled(&endpoint_id, enabled));
    if !changed {
        return Err(crate::i18n::err_not_in_cycle().to_string());
    }

    runtime.persist_config()?;
    log::info!("设备 {endpoint_id} 启用状态改为 {enabled}");
    let message = if enabled {
        crate::i18n::device_enabled()
    } else {
        crate::i18n::device_disabled()
    };
    after_config_change(&app, &runtime, message);
    Ok(runtime.snapshot())
}

/// 调整循环顺序：`offset` 为 -1 上移、1 下移。
#[tauri::command]
pub async fn move_device(
    app: AppHandle,
    endpoint_id: String,
    offset: i32,
    state: SharedRuntime<'_>,
) -> Result<AppSnapshot, String> {
    if offset != -1 && offset != 1 {
        return Err(crate::i18n::err_bad_offset().to_string());
    }

    let runtime = runtime_of(&state);
    let moved = runtime.with_core(|core| core.config.move_device(&endpoint_id, offset));
    if !moved {
        return Err(crate::i18n::err_boundary().to_string());
    }

    runtime.persist_config()?;
    after_config_change(&app, &runtime, crate::i18n::order_updated());
    Ok(runtime.snapshot())
}

/// 按给定顺序重排循环列表。
#[tauri::command]
pub async fn reorder_devices(
    app: AppHandle,
    endpoint_ids: Vec<String>,
    state: SharedRuntime<'_>,
) -> Result<AppSnapshot, String> {
    let runtime = runtime_of(&state);
    runtime.with_core(|core| core.config.reorder(&endpoint_ids));
    runtime.persist_config()?;
    after_config_change(&app, &runtime, crate::i18n::order_updated());
    Ok(runtime.snapshot())
}

/// 左键单击托盘图标的行为开关。
#[tauri::command]
pub async fn set_left_click_switch(
    app: AppHandle,
    enabled: bool,
    state: SharedRuntime<'_>,
) -> Result<AppSnapshot, String> {
    let runtime = runtime_of(&state);

    let unchanged = runtime.with_core(|core| core.config.left_click_switch == enabled);
    if unchanged {
        return Ok(runtime.snapshot());
    }

    runtime.with_core(|core| core.config.left_click_switch = enabled);
    runtime.persist_config()?;
    log::info!(
        "左键单击行为已更新为：{}",
        if enabled {
            "切换设备"
        } else {
            "打开主界面"
        }
    );

    let message = crate::i18n::left_click_saved(enabled);
    after_config_change(&app, &runtime, message);
    Ok(runtime.snapshot())
}

/// 开机自启开关。
///
/// 该状态由系统注册表维护，不写入 config.json：以 `is_enabled()` 回读结果为准，
/// 避免配置文件与系统真实状态不一致（用户也可能在任务管理器中禁用启动项）。
#[tauri::command]
pub async fn set_autostart(
    app: AppHandle,
    enabled: bool,
    state: SharedRuntime<'_>,
) -> Result<AppSnapshot, String> {
    use tauri_plugin_autostart::ManagerExt;

    let runtime = runtime_of(&state);
    let manager = app.autolaunch();

    let change = if enabled {
        manager.enable()
    } else {
        manager.disable()
    };
    if let Err(err) = change {
        let message = crate::i18n::autostart_failed(&err.to_string());
        log::error!("{message}");
        runtime.set_feedback(FeedbackKind::Error, message.clone());
        refresh::emit_state(&app, &runtime);
        return Err(message);
    }

    // 回读系统真实状态，只有确认生效才更新界面
    let actual = match manager.is_enabled() {
        Ok(value) => value,
        Err(err) => {
            let message = crate::i18n::autostart_verify_failed(&err.to_string());
            log::warn!("{message}");
            enabled
        }
    };

    runtime.with_core(|core| core.autostart_enabled = actual);

    if actual != enabled {
        let message = crate::i18n::autostart_rejected().to_string();
        log::warn!("{message}");
        runtime.set_feedback(FeedbackKind::Error, message.clone());
        refresh::emit_state(&app, &runtime);
        return Err(message);
    }

    let message = crate::i18n::autostart_saved(actual);
    log::info!("{message}");
    after_config_change(&app, &runtime, message);
    Ok(runtime.snapshot())
}

/// 切换界面语言。
///
/// 语言同时影响前端界面与 Rust 侧文案（托盘菜单、tooltip、提示语），
/// 因此改完之后必须重绘托盘并广播一次快照。
#[tauri::command]
pub async fn set_language(
    app: AppHandle,
    language: String,
    state: SharedRuntime<'_>,
) -> Result<AppSnapshot, String> {
    let Some(lang) = crate::i18n::Lang::parse(&language) else {
        return Err(crate::i18n::err_bad_language().to_string());
    };

    let runtime = runtime_of(&state);
    let unchanged = runtime.with_core(|core| core.config.language == lang.as_key());

    // 进程级语言先切换，后续提示才能用新语言生成
    crate::i18n::set_lang(lang);

    if !unchanged {
        runtime.with_core(|core| core.config.language = lang.as_key().to_string());
        runtime.persist_config()?;
    }
    log::info!("界面语言已切换为 {language}");

    after_config_change(&app, &runtime, crate::i18n::language_saved());
    Ok(runtime.snapshot())
}

/// 切换界面主题。主题只影响界面呈现，但仍写入配置以便重启后保持。
#[tauri::command]
pub async fn set_theme(
    app: AppHandle,
    theme: String,
    state: SharedRuntime<'_>,
) -> Result<AppSnapshot, String> {
    let normalized = theme.trim().to_ascii_lowercase();
    if !crate::config::THEMES.contains(&normalized.as_str()) {
        return Err(crate::i18n::err_bad_theme().to_string());
    }

    let runtime = runtime_of(&state);
    let unchanged = runtime.with_core(|core| core.config.theme == normalized);

    if !unchanged {
        runtime.with_core(|core| core.config.theme = normalized.clone());
        runtime.persist_config()?;
    }

    // 同步系统标题栏颜色，避免深色界面配浅色边框
    crate::window::apply_theme(&app, &normalized);
    log::info!("界面主题已切换为 {normalized}");

    let message = crate::i18n::theme_saved(&normalized);
    after_config_change(&app, &runtime, message);
    Ok(runtime.snapshot())
}

/// 打开主窗口，可指定要定位的区域。
#[tauri::command]
pub async fn open_main_window(app: AppHandle, section: Option<String>) -> Result<(), String> {
    crate::window::show_main_window(&app, section.as_deref());
    Ok(())
}

/// 真正退出程序。
#[tauri::command]
pub async fn exit_app(app: AppHandle) -> Result<(), String> {
    crate::shutdown::exit_app(&app);
    Ok(())
}

/// 供界面显示的诊断信息（配置与日志路径由快照提供，这里补充运行环境信息）。
#[tauri::command]
pub async fn get_runtime_info(state: SharedRuntime<'_>) -> Result<serde_json::Value, String> {
    let runtime = runtime_of(&state);
    let (config_path, log_path) = (runtime.config_path.clone(), runtime.log_path.clone());
    Ok(serde_json::json!({
        "configPath": config_path.display().to_string(),
        "logPath": log_path.display().to_string(),
        "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
        "watcherActive": runtime.watcher_active.load(std::sync::atomic::Ordering::SeqCst),
    }))
}
