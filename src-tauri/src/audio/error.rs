//! 音频相关错误类型。所有错误都带上可定位的上下文，并按当前界面语言生成提示。
//!
//! `Display` 手动实现（而不是用 `thiserror` 的 `#[error]` 属性），
//! 因为同一条错误需要在中英文之间切换，文本必须在格式化时才确定。

use crate::i18n;

pub type AudioResult<T> = Result<T, AudioError>;

#[derive(Debug)]
pub enum AudioError {
    /// COM 初始化失败
    ComInit(String),
    /// Windows 接口调用失败（带调用点上下文）
    Windows { context: String, detail: String },
    /// 未枚举到任何输出设备
    NoDevices,
    /// 设置默认端点失败
    SetDefaultFailed(String),
    /// 设置后回读校验失败
    VerificationFailed { expected: String, actual: String },
    /// `IPolicyConfig` 不可用
    PolicyConfigUnavailable(String),
    /// 已经是本地化文本的通用错误
    Message(String),
}

impl std::fmt::Display for AudioError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AudioError::ComInit(detail) => f.write_str(&i18n::com_init(detail)),
            AudioError::Windows { context, detail } => {
                f.write_str(&i18n::windows_api(context, detail))
            }
            AudioError::NoDevices => f.write_str(i18n::no_devices()),
            AudioError::SetDefaultFailed(detail) => f.write_str(&i18n::set_default_failed(detail)),
            AudioError::VerificationFailed { expected, actual } => {
                f.write_str(&i18n::verification_failed(expected, actual))
            }
            AudioError::PolicyConfigUnavailable(detail) => {
                f.write_str(&i18n::policy_unavailable(detail))
            }
            AudioError::Message(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for AudioError {}

impl AudioError {
    pub fn windows(context: impl Into<String>, err: impl std::fmt::Display) -> Self {
        AudioError::Windows {
            context: context.into(),
            detail: err.to_string(),
        }
    }
}

impl From<windows::core::Error> for AudioError {
    fn from(err: windows::core::Error) -> Self {
        AudioError::Windows {
            context: i18n::pick("系统调用", "system call").to_string(),
            detail: format!("{} (0x{:08X})", err.message(), err.code().0 as u32),
        }
    }
}
