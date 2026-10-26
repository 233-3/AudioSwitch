//! Audio Switch：Windows 音频输出设备快速切换工具（后端入口）。
//!
//! 模块职责：
//! - [`audio`]：Windows Core Audio / MMDevice API 封装（枚举、默认设备、监听、循环算法）
//! - [`config`]：本地配置读写、校验与迁移
//! - [`state`]：运行时状态与前端快照
//! - [`tray`]：托盘图标、菜单、点击事件
//! - [`switching`]：切换流程编排
//! - [`commands`]：前后端命令接口
//! - [`refresh`]：真实状态同步与事件广播
//! - [`window`] / [`shutdown`]：窗口生命周期与退出清理

mod audio;
mod commands;
mod config;
pub mod diagnose;
mod i18n;
mod logging;
mod refresh;
mod shutdown;
mod state;
mod switching;
mod tray;
mod window;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tauri::{Manager, RunEvent, WindowEvent};

use crate::audio::watcher::{EndpointWatcher, WatcherEvent};
use crate::config::ConfigStore;
use crate::state::{AppRuntime, Core};
use crate::switching::SwitchSource;

/// 设备事件去抖窗口：把短时间内的多个系统回调合并成一次刷新。
const DEBOUNCE_WINDOW: Duration = Duration::from_millis(300);
/// 监听不可用时的降级轮询间隔。
const FALLBACK_POLL_INTERVAL: Duration = Duration::from_secs(3);

pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            log::info!("检测到重复启动，已切换到已有实例");
            window::show_main_window(app, None);
        }))
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .setup(|app| {
            let handle = app.handle().clone();

            // ---------- 路径与日志 ----------
            let config_dir = handle
                .path()
                .app_config_dir()
                .map_err(|err| std::io::Error::other(format!("无法获取配置目录：{err}")))?;
            std::fs::create_dir_all(&config_dir)?;

            let log_path = logging::init(&config_dir.join("logs"));
            log::info!("========== Audio Switch 启动 ==========");
            log::info!(
                "版本 {}，配置目录 {}",
                handle.package_info().version,
                config_dir.display()
            );

            // ---------- 配置 ----------
            let store = ConfigStore::new(&config_dir);
            let config_path = store.path().to_path_buf();
            let (config, config_status) = store.load();
            log::info!(
                "配置加载状态：{}，设备数 {}，左键切换：{}，语言：{}，主题：{}",
                config_status.as_key(),
                config.devices.len(),
                config.left_click_switch,
                config.language,
                config.theme
            );

            // ---------- 语言与主题（必须早于托盘与界面） ----------
            i18n::set_lang(config.lang());
            window::apply_theme(&handle, &config.theme);

            // ---------- 运行时状态 ----------
            let runtime = Arc::new(AppRuntime {
                store,
                core: Mutex::new(Core {
                    config,
                    endpoints: Vec::new(),
                    defaults: Default::default(),
                    device_error: None,
                    feedback: None,
                    config_status,
                    autostart_enabled: false,
                }),
                switching: AtomicBool::new(false),
                switch_lock: tauri::async_runtime::Mutex::new(()),
                tray: Mutex::new(None),
                watcher: Mutex::new(None),
                watcher_active: AtomicBool::new(false),
                version: handle.package_info().version.to_string(),
                config_path,
                log_path,
            });
            app.manage(runtime.clone());

            // ---------- 开机自启的真实状态（以系统注册表为准） ----------
            {
                use tauri_plugin_autostart::ManagerExt;
                match handle.autolaunch().is_enabled() {
                    Ok(enabled) => {
                        runtime.with_core(|core| core.autostart_enabled = enabled);
                        log::info!("开机自启状态：{enabled}");
                    }
                    Err(err) => log::warn!("读取开机自启状态失败：{err}"),
                }
            }

            // ---------- 首次同步真实设备状态 ----------
            refresh::refresh(&runtime);

            // ---------- 托盘 ----------
            if let Err(err) = tray::install(&handle, &runtime) {
                log::error!("创建系统托盘失败：{err}");
                return Err(Box::<dyn std::error::Error>::from(format!(
                    "创建系统托盘失败：{err}"
                )));
            }
            tray::render(&handle, &runtime);

            // ---------- 设备监听 ----------
            start_watcher(&handle, &runtime);

            // ---------- 首次启动引导 ----------
            let needs_guide = runtime.with_core(|core| core.config.devices.is_empty());
            if needs_guide {
                log::info!("尚未配置任何设备，显示主窗口引导用户完成设置");
                window::show_main_window(&handle, Some("devices"));
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                // 关闭窗口只隐藏，程序继续驻留托盘
                api.prevent_close();
                window::hide_main_window(window.app_handle());
                log::info!("主窗口已隐藏到托盘");
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_snapshot,
            commands::refresh_devices,
            commands::switch_next,
            commands::switch_to_device,
            commands::add_device,
            commands::remove_device,
            commands::set_device_enabled,
            commands::move_device,
            commands::reorder_devices,
            commands::set_left_click_switch,
            commands::set_autostart,
            commands::set_language,
            commands::set_theme,
            commands::open_main_window,
            commands::exit_app,
            commands::get_runtime_info,
        ])
        .build(tauri::generate_context!())
        .expect("Audio Switch 启动失败");

    app.run(|_app, event| match event {
        RunEvent::ExitRequested { .. } => {
            log::info!("收到退出请求");
        }
        RunEvent::Exit => {
            log::info!("进程即将结束");
            log::logger().flush();
        }
        _ => {}
    });
}

/// 启动设备监听；失败时降级为低频轮询，并在界面中如实标注。
fn start_watcher(app: &tauri::AppHandle, runtime: &Arc<AppRuntime>) {
    let (tx, rx) = std::sync::mpsc::channel::<WatcherEvent>();

    let handler = move |event: WatcherEvent| {
        // 系统回调线程：只投递事件，不做任何阻塞操作
        let _ = tx.send(event);
    };

    match EndpointWatcher::start(handler) {
        Ok(watcher) => {
            {
                let mut guard = runtime
                    .watcher
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                *guard = Some(watcher);
            }
            runtime.watcher_active.store(true, Ordering::SeqCst);

            let app = app.clone();
            let runtime = runtime.clone();
            let spawned = std::thread::Builder::new()
                .name("audio-debounce".to_string())
                .spawn(move || debounce_loop(app, runtime, rx));

            if let Err(err) = spawned {
                log::error!("创建事件去抖线程失败，设备变化可能无法及时刷新：{err}");
            }
        }
        Err(err) => {
            log::error!("设备监听初始化失败，降级为 {FALLBACK_POLL_INTERVAL:?} 轮询：{err}");
            runtime.watcher_active.store(false, Ordering::SeqCst);
            runtime.set_feedback(crate::state::FeedbackKind::Error, i18n::watcher_degraded());
            start_fallback_poll(app.clone(), runtime.clone());
        }
    }
}

/// 合并短时间内的多次设备事件，避免反复枚举造成抖动。
fn debounce_loop(
    app: tauri::AppHandle,
    runtime: Arc<AppRuntime>,
    rx: std::sync::mpsc::Receiver<WatcherEvent>,
) {
    // 通道在监听器释放后关闭，recv 返回 Err 时线程退出
    while let Ok(first) = rx.recv() {
        let mut descriptions = vec![first.describe()];
        let deadline = Instant::now() + DEBOUNCE_WINDOW;
        while let Some(remaining) = deadline.checked_duration_since(Instant::now()) {
            match rx.recv_timeout(remaining) {
                Ok(event) => descriptions.push(event.describe()),
                Err(_) => break,
            }
        }

        if shutdown::is_shutting_down() {
            break;
        }

        log::debug!("收到设备事件：{}", descriptions.join("；"));
        // 只刷新状态，绝不因为监听到变化而自动切换设备
        refresh::refresh_and_broadcast(&app, &runtime);
    }
    log::info!("设备事件线程已退出");
}

/// 监听不可用时的降级方案：定时比对真实状态。
fn start_fallback_poll(app: tauri::AppHandle, runtime: Arc<AppRuntime>) {
    let spawned = std::thread::Builder::new()
        .name("audio-fallback-poll".to_string())
        .spawn(move || {
            while !shutdown::is_shutting_down() {
                std::thread::sleep(FALLBACK_POLL_INTERVAL);
                if shutdown::is_shutting_down() {
                    break;
                }
                refresh::refresh_and_broadcast(&app, &runtime);
            }
            log::info!("降级轮询线程已退出");
        });

    if let Err(err) = spawned {
        log::error!("创建降级轮询线程失败：{err}");
    }
}

/// 供内部代码触发的循环切换（保留入口，便于后续扩展）。
#[allow(dead_code)]
pub(crate) fn cycle_from_internal(app: &tauri::AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        switching::cycle_next(&app, SwitchSource::Command).await;
    });
}
