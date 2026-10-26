//! 退出流程：停止后台任务、保存配置、刷新日志，然后真正结束进程。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tauri::Manager;

/// 退出标志，供降级轮询线程等后台循环检查。
pub static SHUTTING_DOWN: AtomicBool = AtomicBool::new(false);

/// 是否已进入退出流程。
pub fn is_shutting_down() -> bool {
    SHUTTING_DOWN.load(Ordering::SeqCst)
}

pub fn exit_app(app: &tauri::AppHandle) {
    if SHUTTING_DOWN.swap(true, Ordering::SeqCst) {
        return;
    }

    log::info!("开始退出应用");

    if let Some(state) = app.try_state::<Arc<crate::state::AppRuntime>>() {
        let runtime = state.inner().clone();

        // 1) 保存配置（失败也要继续退出，不能让进程卡在托盘）
        if let Err(err) = runtime.persist_config() {
            log::error!("退出前保存配置失败：{err}");
        }

        // 2) 注销设备通知并等待监听线程退出
        {
            let mut guard = runtime
                .watcher
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if let Some(mut watcher) = guard.take() {
                watcher.stop();
            }
        }

        // 3) 移除托盘图标，避免退出后残留幽灵图标
        {
            let mut guard = runtime
                .tray
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            guard.take();
        }
    }

    log::info!("应用已退出");
    log::logger().flush();
    app.exit(0);
}
