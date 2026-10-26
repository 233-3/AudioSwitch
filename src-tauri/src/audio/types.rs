//! 与 Windows API 无关的公共数据类型。
//!
//! 这里只放纯数据结构，方便单元测试以及前后端共享。

use serde::{Deserialize, Serialize};

/// Windows 音频端点状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DeviceState {
    /// 正常工作
    Active,
    /// 被用户在系统中禁用
    Disabled,
    /// 未插入（如 HDMI 未接、USB 已拔出但仍留有记录）
    NotPresent,
    /// 未插入（蓝牙/USB 常见）
    Unplugged,
    /// 未知状态
    Unknown,
}

impl DeviceState {
    pub fn is_active(self) -> bool {
        matches!(self, DeviceState::Active)
    }

    /// 前端使用的稳定状态键。
    pub fn key(self) -> &'static str {
        match self {
            DeviceState::Active => "active",
            DeviceState::Disabled => "disabled",
            DeviceState::NotPresent => "notPresent",
            DeviceState::Unplugged => "unplugged",
            DeviceState::Unknown => "unknown",
        }
    }

    /// 面向用户的描述，跟随当前界面语言。
    pub fn label(self) -> &'static str {
        crate::i18n::state_label(self.key())
    }
}

impl std::fmt::Display for DeviceState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

/// 从设备枚举得到的原始端点信息。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EndpointInfo {
    pub id: String,
    pub friendly_name: String,
    pub device_desc: String,
    pub interface_name: String,
    /// 端点所属的总线枚举器名称（USB / HDAUDIO / PCI / BTHENUM …），来自 Windows 属性
    pub enumerator: String,
    /// 端点形态（`PKEY_AudioEndpoint_FormFactor`）：1 扬声器 / 3 耳机 / 8 S-PDIF / 9 HDMI …
    pub form_factor: i32,
    pub state: DeviceState,
}

impl EndpointInfo {
    /// 生成用于界面展示的名称，避免出现空名称。
    pub fn display_name(&self) -> String {
        if !self.friendly_name.trim().is_empty() {
            return self.friendly_name.clone();
        }
        if !self.device_desc.trim().is_empty() && !self.interface_name.trim().is_empty() {
            return format!("{} ({})", self.device_desc, self.interface_name);
        }
        if !self.device_desc.trim().is_empty() {
            return self.device_desc.clone();
        }
        // 兜底：使用端点 ID 尾部，至少保证可区分
        self.id.clone()
    }

    fn enumerator_key(&self) -> String {
        self.enumerator.trim().to_ascii_lowercase()
    }

    /// 总线类型（仅在有区分意义时返回）。
    fn bus_label(&self) -> Option<&'static str> {
        crate::i18n::bus_label(&self.enumerator_key())
    }

    /// 端点形态（`PKEY_AudioEndpoint_FormFactor`），由 Windows 根据端点类型给出。
    fn form_label(&self) -> Option<&'static str> {
        crate::i18n::form_label(self.form_factor)
    }

    /// 副标题：总线 · 形态，例如「USB · 耳机」「HDMI 输出」。未知时为空字符串。
    pub fn subtitle(&self) -> String {
        let parts: Vec<&str> = [self.bus_label(), self.form_label()]
            .into_iter()
            .flatten()
            .collect();
        if !parts.is_empty() {
            return parts.join(" · ");
        }
        crate::i18n::fallback_bus_label(&self.enumerator_key())
            .unwrap_or_default()
            .to_string()
    }

    /// 列表行使用的图标分类。
    pub fn icon_kind(&self) -> &'static str {
        let bus = self.enumerator_key();
        if matches!(bus.as_str(), "bthenum" | "bthhfenum" | "bthledevice") {
            return "bluetooth";
        }

        match self.form_factor {
            9 => "display",
            1 => "speaker",
            3 | 5 => "headphone",
            _ => match bus.as_str() {
                "usb" => "headphone",
                "swd" => "virtual",
                _ => "audio",
            },
        }
    }
}

/// 三个音频角色。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Console,
    Multimedia,
    Communications,
}

impl Role {
    pub const ALL: [Role; 3] = [Role::Console, Role::Multimedia, Role::Communications];

    pub fn as_key(self) -> &'static str {
        match self {
            Role::Console => "console",
            Role::Multimedia => "multimedia",
            Role::Communications => "communications",
        }
    }
}

/// 当前系统默认输出端点（按角色分组）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DefaultEndpoints {
    pub console: Option<String>,
    pub multimedia: Option<String>,
    pub communications: Option<String>,
}

impl DefaultEndpoints {
    pub fn get(&self, role: Role) -> Option<&String> {
        match role {
            Role::Console => self.console.as_ref(),
            Role::Multimedia => self.multimedia.as_ref(),
            Role::Communications => self.communications.as_ref(),
        }
    }

    /// 常规播放使用的默认端点：优先 Console，其次 Multimedia。
    pub fn playback(&self) -> Option<&String> {
        self.console.as_ref().or(self.multimedia.as_ref())
    }

    /// 该端点当前承担的角色列表。
    pub fn roles_of(&self, endpoint_id: &str) -> Vec<&'static str> {
        let mut roles = Vec::new();
        if self.console.as_deref() == Some(endpoint_id) {
            roles.push(Role::Console.as_key());
        }
        if self.multimedia.as_deref() == Some(endpoint_id) {
            roles.push(Role::Multimedia.as_key());
        }
        if self.communications.as_deref() == Some(endpoint_id) {
            roles.push(Role::Communications.as_key());
        }
        roles
    }

    pub fn is_default(&self, endpoint_id: &str) -> bool {
        self.playback().map(|id| id == endpoint_id).unwrap_or(false)
    }
}
