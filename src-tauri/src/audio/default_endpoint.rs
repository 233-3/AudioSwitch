//! 读取与设置 Windows 默认音频输出设备。
//!
//! 设计要点：
//! - 本模块只负责「真实状态」的读和写，不关心循环算法与 UI；
//! - 写入使用 [`PolicyConfig`]（`IPolicyConfig`），写完后必须调用 [`verify_default`]
//!   重新枚举系统状态确认结果，绝不以「调用返回成功」作为切换成功；
//! - 默认切换 Console + Multimedia 两个角色，Communications 保持不变，
//!   避免影响会议/通话类应用的输出路由（见 README 说明）。

use std::time::{Duration, Instant};

use windows::Win32::Media::Audio::{eCommunications, eConsole, eMultimedia, eRender, ERole};

use crate::audio::com::ComGuard;
use crate::audio::device::{create_enumerator, endpoint_id_of};
use crate::audio::error::{AudioError, AudioResult};
use crate::audio::policy_config::PolicyConfig;
use crate::audio::types::{DefaultEndpoints, Role};

/// 切换默认设备时一起修改的角色。
///
/// 二者对应「系统常规播放」。Communications 角色刻意不修改：Windows 允许通话
/// 使用独立端点，强行同步会改变用户的会议输出路由。
pub const SWITCH_ROLES: [Role; 2] = [Role::Console, Role::Multimedia];

/// 切换后等待系统生效并回读验证的最长时间。
const VERIFY_TIMEOUT: Duration = Duration::from_millis(1500);
const VERIFY_INTERVAL: Duration = Duration::from_millis(40);

fn to_erole(role: Role) -> ERole {
    match role {
        Role::Console => eConsole,
        Role::Multimedia => eMultimedia,
        Role::Communications => eCommunications,
    }
}

/// 读取三个音频角色当前的默认输出端点。
pub fn read_defaults() -> AudioResult<DefaultEndpoints> {
    let _com = ComGuard::mta()?;
    let enumerator = create_enumerator()?;

    let mut defaults = DefaultEndpoints::default();
    for role in Role::ALL {
        // 某个角色可能没有默认端点（例如系统里没有任何输出设备），此时为 None
        let endpoint = unsafe { enumerator.GetDefaultAudioEndpoint(eRender, to_erole(role)) }
            .ok()
            .and_then(|device| unsafe { device.GetId().ok() })
            .map(|pwstr| unsafe { endpoint_id_of(pwstr) })
            .filter(|id| !id.is_empty());

        match role {
            Role::Console => defaults.console = endpoint,
            Role::Multimedia => defaults.multimedia = endpoint,
            Role::Communications => defaults.communications = endpoint,
        }
    }

    if defaults.console.is_none()
        && defaults.multimedia.is_none()
        && defaults.communications.is_none()
    {
        return Err(AudioError::NoDevices);
    }

    Ok(defaults)
}

/// 直接调用系统接口把端点设为指定角色的默认设备。
///
/// 该函数不验证结果，仅表示系统接受了调用。
pub fn apply_default(endpoint_id: &str, roles: &[Role]) -> AudioResult<()> {
    let policy = PolicyConfig::new()?;
    for role in roles {
        policy.set_default_endpoint(endpoint_id, *role)?;
    }
    Ok(())
}

/// 反复回读，直到所有目标角色都指向期望端点，或超时。
pub fn verify_default(endpoint_id: &str, roles: &[Role], timeout: Duration) -> AudioResult<()> {
    let deadline = Instant::now() + timeout;

    loop {
        match read_defaults() {
            Ok(defaults) => {
                let satisfied = roles.iter().all(|role| {
                    defaults
                        .get(*role)
                        .map(|id| id == endpoint_id)
                        .unwrap_or(false)
                });
                if satisfied {
                    return Ok(());
                }
                if Instant::now() >= deadline {
                    let actual = roles
                        .iter()
                        .map(|role| {
                            let value = defaults
                                .get(*role)
                                .cloned()
                                .unwrap_or_else(|| "无".to_string());
                            format!("{} -> {}", role.as_key(), value)
                        })
                        .collect::<Vec<_>>()
                        .join("；");
                    return Err(AudioError::VerificationFailed {
                        expected: endpoint_id.to_string(),
                        actual,
                    });
                }
            }
            Err(err) => {
                if Instant::now() >= deadline {
                    return Err(err);
                }
            }
        }

        std::thread::sleep(VERIFY_INTERVAL);
    }
}

/// 设置默认设备并验证，是全应用唯一的「真正切换」入口。
pub fn set_default_and_verify(endpoint_id: &str) -> AudioResult<()> {
    apply_default(endpoint_id, &SWITCH_ROLES)?;
    verify_default(endpoint_id, &SWITCH_ROLES, VERIFY_TIMEOUT)
}
