//! 主窗口生命周期。

use std::sync::Arc;

use tauri::{AppHandle, Emitter, Manager};

use crate::refresh::EVENT_NAVIGATE;
use crate::state::AppRuntime;

/// 显示主窗口并带到前台；可选地通知前端切换到某个区域。
pub fn show_main_window(app: &AppHandle, section: Option<&str>) {
    let Some(window) = app.get_webview_window("main") else {
        log::error!("未找到主窗口，无法显示");
        return;
    };

    if let Err(err) = window.show() {
        log::warn!("显示主窗口失败：{err}");
    }
    if let Err(err) = window.unminimize() {
        log::warn!("取消最小化失败：{err}");
    }
    if let Err(err) = window.set_focus() {
        log::warn!("聚焦主窗口失败：{err}");
    }

    if let Some(section) = section {
        if let Err(err) = app.emit(EVENT_NAVIGATE, section.to_string()) {
            log::warn!("通知界面跳转失败：{err}");
        }
    }

    // 打开界面时同步一次真实设备状态（窗口长时间隐藏期间可能发生过变化）
    if let Some(runtime) = app.try_state::<Arc<AppRuntime>>() {
        let runtime = runtime.inner().clone();
        let app = app.clone();
        std::thread::spawn(move || {
            crate::refresh::refresh_and_broadcast(&app, &runtime);
        });
    }
}

/// 隐藏主窗口（关闭按钮的默认行为）。
pub fn hide_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        if let Err(err) = window.hide() {
            log::warn!("隐藏主窗口失败：{err}");
        }
    }
}

/// 让原生标题栏跟随界面主题，避免深色页面顶着浅色边框。
pub fn apply_theme(app: &AppHandle, theme: &str) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let mode = if theme == "dark" {
        tauri::Theme::Dark
    } else {
        tauri::Theme::Light
    };
    if let Err(err) = window.set_theme(Some(mode)) {
        log::warn!("设置窗口主题失败：{err}");
    }
}
