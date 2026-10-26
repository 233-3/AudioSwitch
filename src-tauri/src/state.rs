//! 应用运行时状态与前端快照。
//!
//! 设计原则：
//! - 所有可变业务数据集中在 [`Core`] 的单个 Mutex 中，避免多把锁的加锁顺序问题；
//! - 锁只在内存读写时短暂持有，绝不跨越 COM 调用或磁盘 IO；
//! - 前端拿到的是 [`AppSnapshot`]（只读快照），不直接暴露内部结构。

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Mutex;

use serde::Serialize;

use crate::audio::types::{DefaultEndpoints, EndpointInfo};
use crate::config::{AppConfig, ConfigLoadStatus, ConfigStore};

/// 切换反馈类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeedbackKind {
    Info,
    Success,
    Error,
}

impl FeedbackKind {
    pub fn as_key(self) -> &'static str {
        match self {
            FeedbackKind::Info => "info",
            FeedbackKind::Success => "success",
            FeedbackKind::Error => "error",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Feedback {
    pub kind: &'static str,
    pub message: String,
    pub at: String,
}

impl Feedback {
    pub fn new(kind: FeedbackKind, message: impl Into<String>) -> Self {
        Self {
            kind: kind.as_key(),
            message: message.into(),
            at: chrono::Local::now().format("%H:%M:%S").to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrentOutput {
    pub endpoint_id: String,
    pub name: String,
    pub roles: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceView {
    pub endpoint_id: String,
    pub name: String,
    /// 连接方式副标题（如「USB 有线连接」），未知时为空
    pub subtitle: String,
    /// 列表图标分类：headphone / bluetooth / display / speaker / virtual / audio
    pub icon: &'static str,
    pub device_desc: String,
    pub interface_name: String,
    pub state: &'static str,
    pub state_label: &'static str,
    pub is_available: bool,
    pub is_default: bool,
    pub roles: Vec<&'static str>,
    pub is_configured: bool,
    pub is_enabled: bool,
    pub cycle_position: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CycleItemView {
    pub endpoint_id: String,
    pub name: String,
    pub subtitle: String,
    pub icon: &'static str,
    pub last_known_name: String,
    pub enabled: bool,
    /// 系统当前是否还能枚举到这个端点
    pub present: bool,
    pub available: bool,
    pub state: &'static str,
    pub state_label: &'static str,
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSnapshot {
    pub app_name: String,
    pub version: String,
    pub left_click_switch: bool,
    pub current_output: Option<CurrentOutput>,
    pub devices: Vec<DeviceView>,
    pub cycle: Vec<CycleItemView>,
    pub config_status: &'static str,
    pub config_path: String,
    pub log_path: String,
    pub switching: bool,
    pub device_error: Option<String>,
    pub last_feedback: Option<Feedback>,
    /// 切换角色（本程序会一起修改的音频角色），供界面如实告知用户
    pub switch_roles: Vec<&'static str>,
    /// 设备监听是否工作正常（false 时说明处于降级轮询模式）
    pub watcher_active: bool,
    /// 是否已设置为开机自启（真实状态来自系统注册表）
    pub autostart_enabled: bool,
    /// 界面语言：`zh` / `en`
    pub language: String,
    /// 界面主题：`light` / `dark`
    pub theme: String,
}

/// 受单个锁保护的可变业务状态。
pub struct Core {
    pub config: AppConfig,
    pub endpoints: Vec<EndpointInfo>,
    pub defaults: DefaultEndpoints,
    pub device_error: Option<String>,
    pub feedback: Option<Feedback>,
    pub config_status: ConfigLoadStatus,
    /// 开机自启的真实状态（来自系统注册表，不落盘到 config.json）
    pub autostart_enabled: bool,
}

impl Core {
    /// 当前「常规播放」默认端点 ID。
    pub fn current_default(&self) -> Option<String> {
        self.defaults.playback().cloned()
    }

    pub fn find_endpoint(&self, endpoint_id: &str) -> Option<&EndpointInfo> {
        self.endpoints
            .iter()
            .find(|endpoint| endpoint.id == endpoint_id)
    }

    /// 构造循环算法输入：顺序完全来自用户配置，可用性来自实时枚举结果。
    pub fn cycle_entries(&self) -> Vec<crate::audio::cycle::CycleEntry> {
        self.config
            .devices
            .iter()
            .map(|configured| {
                let live = self.find_endpoint(&configured.endpoint_id);
                let name = live
                    .map(|endpoint| endpoint.display_name())
                    .filter(|name| !name.trim().is_empty())
                    .unwrap_or_else(|| configured.last_known_name.clone());
                let available = live
                    .map(|endpoint| endpoint.state.is_active())
                    .unwrap_or(false);
                crate::audio::cycle::CycleEntry::new(
                    configured.endpoint_id.clone(),
                    name,
                    configured.enabled,
                    available,
                )
            })
            .collect()
    }
}

pub struct AppRuntime {
    pub store: ConfigStore,
    pub core: Mutex<Core>,
    pub switching: AtomicBool,
    /// 切换串行化锁：同一时刻只允许一次切换
    pub switch_lock: tauri::async_runtime::Mutex<()>,
    pub tray: Mutex<Option<crate::tray::TrayRuntime>>,
    /// 设备监听句柄，退出时显式停止并等待线程结束
    pub watcher: Mutex<Option<crate::audio::watcher::EndpointWatcher>>,
    pub watcher_active: AtomicBool,
    pub version: String,
    pub config_path: PathBuf,
    pub log_path: PathBuf,
}

impl AppRuntime {
    /// 加锁访问核心状态（互斥锁中毒时继续使用内部数据，避免程序整体不可用）。
    pub fn with_core<T>(&self, f: impl FnOnce(&mut Core) -> T) -> T {
        match self.core.lock() {
            Ok(mut guard) => f(&mut guard),
            Err(poisoned) => {
                log::error!("核心状态锁已中毒，继续使用内部数据");
                f(&mut poisoned.into_inner())
            }
        }
    }

    /// 把当前配置写入磁盘。失败会更新反馈并记录日志，不静默吞掉。
    pub fn persist_config(&self) -> Result<(), String> {
        let config = self.with_core(|core| core.config.clone());
        match self.store.save(&config) {
            Ok(()) => Ok(()),
            Err(err) => {
                let message = err.to_string();
                log::error!("保存配置失败：{message}");
                self.with_core(|core| {
                    core.feedback = Some(Feedback::new(
                        FeedbackKind::Error,
                        format!("配置保存失败：{message}"),
                    ));
                });
                Err(message)
            }
        }
    }

    pub fn set_feedback(&self, kind: FeedbackKind, message: impl Into<String>) {
        let feedback = Feedback::new(kind, message);
        self.with_core(|core| core.feedback = Some(feedback));
    }

    pub fn snapshot(&self) -> AppSnapshot {
        let switching = self.switching.load(std::sync::atomic::Ordering::SeqCst);
        let watcher_active = self
            .watcher_active
            .load(std::sync::atomic::Ordering::SeqCst);

        let core = match self.core.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };

        build_snapshot(
            &core,
            &self.version,
            &self.config_path,
            &self.log_path,
            switching,
            watcher_active,
        )
    }
}

fn build_snapshot(
    core: &Core,
    version: &str,
    config_path: &std::path::Path,
    log_path: &std::path::Path,
    switching: bool,
    watcher_active: bool,
) -> AppSnapshot {
    let defaults = &core.defaults;

    let devices = core
        .endpoints
        .iter()
        .map(|endpoint| {
            let configured = core.config.find(&endpoint.id);
            DeviceView {
                endpoint_id: endpoint.id.clone(),
                name: endpoint.display_name(),
                subtitle: endpoint.subtitle(),
                icon: endpoint.icon_kind(),
                device_desc: endpoint.device_desc.clone(),
                interface_name: endpoint.interface_name.clone(),
                state: endpoint.state.key(),
                state_label: endpoint.state.label(),
                is_available: endpoint.state.is_active(),
                is_default: defaults.is_default(&endpoint.id),
                roles: defaults.roles_of(&endpoint.id),
                is_configured: configured.is_some(),
                is_enabled: configured.map(|d| d.enabled).unwrap_or(false),
                cycle_position: core
                    .config
                    .devices
                    .iter()
                    .position(|d| d.endpoint_id == endpoint.id)
                    .map(|index| index + 1),
            }
        })
        .collect::<Vec<_>>();

    let cycle = core
        .config
        .devices
        .iter()
        .map(|configured| {
            let live = core
                .endpoints
                .iter()
                .find(|e| e.id == configured.endpoint_id);
            match live {
                Some(endpoint) => CycleItemView {
                    endpoint_id: configured.endpoint_id.clone(),
                    name: endpoint.display_name(),
                    subtitle: endpoint.subtitle(),
                    icon: endpoint.icon_kind(),
                    last_known_name: configured.last_known_name.clone(),
                    enabled: configured.enabled,
                    present: true,
                    available: endpoint.state.is_active(),
                    state: endpoint.state.key(),
                    state_label: endpoint.state.label(),
                    is_default: defaults.is_default(&endpoint.id),
                },
                None => CycleItemView {
                    endpoint_id: configured.endpoint_id.clone(),
                    name: if configured.last_known_name.trim().is_empty() {
                        crate::i18n::unknown_device().to_string()
                    } else {
                        configured.last_known_name.clone()
                    },
                    subtitle: crate::i18n::missing_subtitle().to_string(),
                    icon: "audio",
                    last_known_name: configured.last_known_name.clone(),
                    enabled: configured.enabled,
                    present: false,
                    available: false,
                    state: "missing",
                    state_label: crate::i18n::state_label("missing"),
                    is_default: false,
                },
            }
        })
        .collect::<Vec<_>>();

    let current_output = defaults.playback().map(|id| {
        let name = core
            .endpoints
            .iter()
            .find(|e| &e.id == id)
            .map(|e| e.display_name())
            .unwrap_or_else(|| id.clone());
        CurrentOutput {
            endpoint_id: id.clone(),
            name,
            roles: defaults.roles_of(id),
        }
    });

    AppSnapshot {
        app_name: "Audio Switch".to_string(),
        version: version.to_string(),
        left_click_switch: core.config.left_click_switch,
        current_output,
        devices,
        cycle,
        config_status: core.config_status.as_key(),
        config_path: config_path.display().to_string(),
        log_path: log_path.display().to_string(),
        switching,
        device_error: core.device_error.clone(),
        last_feedback: core.feedback.clone(),
        switch_roles: crate::audio::default_endpoint::SWITCH_ROLES
            .iter()
            .map(|role| role.as_key())
            .collect(),
        watcher_active,
        autostart_enabled: core.autostart_enabled,
        language: core.config.language.clone(),
        theme: core.config.theme.clone(),
    }
}
