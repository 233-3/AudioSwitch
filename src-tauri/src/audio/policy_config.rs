//! `IPolicyConfig` 的最小可用声明。
//!
//! # 背景与风险评估
//!
//! Windows 至今没有公开「设置系统默认音频输出设备」的官方 API。`IPolicyConfig`
//! （以及 Win7 之前的 `IPolicyConfigVista`）是系统内部接口，未在文档中承诺 ABI 稳定，
//! 但自 Windows 7 起一直存在，也是各类默认设备切换工具的事实标准实现路径。
//!
//! 本模块的处理原则（对应任务书 5.3 的要求）：
//! 1. 不使用第三方 crate 的封装，显式声明 vtable 布局，方法顺序与 Windows SDK 示例
//!    `PolicyConfig.h` 完全一致，便于逐项核对；
//! 2. GUID 硬编码为公开流传的稳定值，并在 [`PolicyConfig::new`] 中通过
//!    `QueryInterface` 实际校验，而不是「能编译就当真」；
//! 3. 只对外暴露 [`PolicyConfig::set_default_endpoint`]，其余方法仅用于占位以保持
//!    vtable 偏移正确，不参与业务逻辑；
//! 4. 调用成功后必须由上层重新读取系统默认设备进行验证（见 `default_endpoint`）。
//!
//! 已知兼容性风险：若未来 Windows 版本移除该 CLSID 或改变 vtable 顺序，`QueryInterface`
//! 会失败或调用返回错误，此时上层会向用户返回明确的失败信息，而不会伪造成功状态。

use std::ffi::c_void;

use windows::core::{IUnknown, Interface, GUID, HRESULT, PCWSTR};
use windows::Win32::Foundation::PROPERTYKEY;
use windows::Win32::Media::Audio::{eCommunications, eConsole, eMultimedia, WAVEFORMATEX};
use windows::Win32::System::Com::StructuredStorage::PROPVARIANT;
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL};

use crate::audio::com::ComGuard;
use crate::audio::error::{AudioError, AudioResult};
use crate::audio::types::Role;

/// `{870AF99C-171D-4F9E-AF0D-E63DF40C2BC9}`，系统内置的默认设备策略实现。
const CLSID_CPOLICYCONFIGCLIENT: GUID = GUID::from_u128(0x870AF99C_171D_4F9E_AF0D_E63DF40C2BC9);

/// `IPolicyConfig` 的 IID。
const IID_IPOLICYCONFIG: GUID = GUID::from_u128(0xF8679F50_850A_41CF_9C72_430F290290C8);

/// `DeviceShareMode`：`{ DWORD dwShareMode; ERole eRole; }`
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
struct DeviceShareMode {
    share_mode: i32,
    role: i32,
}

/// `IPolicyConfig` 的 vtable。
///
/// 顺序严格对应 SDK 示例中的声明顺序：
/// `IUnknown` 3 个方法 + 12 个接口方法 = 15 个函数指针。
/// [`size_check`] 会在编译期校验总量，防止漏写或重排导致偏移错位。
#[repr(C)]
struct IPolicyConfigVtbl {
    query_interface:
        unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> HRESULT,
    add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    release: unsafe extern "system" fn(*mut c_void) -> u32,

    // 1. GetMixFormat
    get_mix_format:
        unsafe extern "system" fn(*mut c_void, PCWSTR, *mut *mut WAVEFORMATEX) -> HRESULT,
    // 2. GetDeviceFormat
    get_device_format:
        unsafe extern "system" fn(*mut c_void, PCWSTR, i32, *mut *mut WAVEFORMATEX) -> HRESULT,
    // 3. ResetDeviceFormat
    reset_device_format: unsafe extern "system" fn(*mut c_void, PCWSTR) -> HRESULT,
    // 4. SetDeviceFormat
    set_device_format: unsafe extern "system" fn(
        *mut c_void,
        PCWSTR,
        *const WAVEFORMATEX,
        *const WAVEFORMATEX,
    ) -> HRESULT,
    // 5. GetProcessingPeriod
    get_processing_period:
        unsafe extern "system" fn(*mut c_void, PCWSTR, i32, *mut i64, *mut i64) -> HRESULT,
    // 6. SetProcessingPeriod
    set_processing_period: unsafe extern "system" fn(*mut c_void, PCWSTR, *const i64) -> HRESULT,
    // 7. GetShareMode
    get_share_mode: unsafe extern "system" fn(*mut c_void, PCWSTR, *mut DeviceShareMode) -> HRESULT,
    // 8. SetShareMode
    set_share_mode:
        unsafe extern "system" fn(*mut c_void, PCWSTR, *const DeviceShareMode) -> HRESULT,
    // 9. GetPropertyValue
    get_property_value: unsafe extern "system" fn(
        *mut c_void,
        PCWSTR,
        *const PROPERTYKEY,
        *mut PROPVARIANT,
    ) -> HRESULT,
    // 10. SetPropertyValue
    set_property_value: unsafe extern "system" fn(
        *mut c_void,
        PCWSTR,
        *const PROPERTYKEY,
        *const PROPVARIANT,
    ) -> HRESULT,
    // 11. SetDefaultEndpoint —— 本模块唯一实际调用的方法
    set_default_endpoint: unsafe extern "system" fn(*mut c_void, PCWSTR, i32) -> HRESULT,
    // 12. SetEndpointVisibility
    set_endpoint_visibility: unsafe extern "system" fn(*mut c_void, PCWSTR, i32) -> HRESULT,
}

/// 编译期校验 vtable 大小：15 个指针。
const _: () = {
    assert!(std::mem::size_of::<IPolicyConfigVtbl>() == 15 * std::mem::size_of::<usize>());
};

/// `IPolicyConfig` 的持有者。
///
/// 通过 `QueryInterface` 拿到接口指针后自行管理引用计数，`Drop` 时先 `Release`
/// 再反初始化 COM。
///
/// 注意字段顺序：`_com` 必须放在最后，Rust 按声明顺序析构字段，
/// 这样才能保证「先释放 COM 对象，再反初始化套间」，否则会踩到已销毁的套间。
pub struct PolicyConfig {
    ptr: *mut c_void,
    vtbl: *const IPolicyConfigVtbl,
    _com: ComGuard,
}

// 该对象只在单个调用栈内使用，不跨线程共享；显式禁止误用。
impl std::fmt::Debug for PolicyConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PolicyConfig")
            .field("ptr", &self.ptr)
            .finish()
    }
}

impl PolicyConfig {
    /// 创建并校验 `IPolicyConfig`。
    ///
    /// 对象会一直持有当前线程的 COM 初始化，直到自身被释放，
    /// 因此调用方可以安全地在创建后继续调用方法。
    pub fn new() -> AudioResult<Self> {
        // 保证当前线程已初始化 COM（COM 初始化是线程级的），并由对象持有
        let com = ComGuard::mta()?;

        let unknown: IUnknown =
            unsafe { CoCreateInstance(&CLSID_CPOLICYCONFIGCLIENT, None, CLSCTX_ALL) }.map_err(
                |err| {
                    AudioError::PolicyConfigUnavailable(crate::i18n::policy_create_failed(
                        &err.message().to_string(),
                        err.code().0 as u32,
                    ))
                },
            )?;

        let raw_unknown = unknown.as_raw();
        let unknown_vtbl = unsafe { *(raw_unknown as *const *const IPolicyConfigVtbl) };
        let mut iface: *mut c_void = std::ptr::null_mut();
        let hr = unsafe {
            ((*unknown_vtbl).query_interface)(raw_unknown, &IID_IPOLICYCONFIG, &mut iface)
        };

        if hr.is_err() || iface.is_null() {
            // unknown 在此处离开作用域时释放
            return Err(AudioError::PolicyConfigUnavailable(
                crate::i18n::policy_interface_missing(hr.0 as u32),
            ));
        }

        // QI 已为 iface 增加了一次引用，这里释放 CoCreateInstance 持有的引用
        drop(unknown);

        let vtbl = unsafe { *(iface as *const *const IPolicyConfigVtbl) };
        Ok(Self {
            ptr: iface,
            vtbl,
            _com: com,
        })
    }

    /// 把指定端点设为某个音频角色的默认设备。
    ///
    /// 注意：返回 `Ok` 仅代表 Windows 接受了这次调用，调用方必须重新读取系统状态验证。
    pub fn set_default_endpoint(&self, endpoint_id: &str, role: Role) -> AudioResult<()> {
        let wide = to_wide(endpoint_id);
        let hr = unsafe {
            ((*self.vtbl).set_default_endpoint)(self.ptr, PCWSTR(wide.as_ptr()), erole_value(role))
        };
        if hr.is_err() {
            return Err(AudioError::SetDefaultFailed(
                crate::i18n::set_default_returned(
                    role.as_key(),
                    &short_endpoint(endpoint_id),
                    hr.0 as u32,
                ),
            ));
        }
        Ok(())
    }
}

impl Drop for PolicyConfig {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe { ((*self.vtbl).release)(self.ptr) };
            self.ptr = std::ptr::null_mut();
        }
    }
}

fn erole_value(role: Role) -> i32 {
    match role {
        Role::Console => eConsole.0,
        Role::Multimedia => eMultimedia.0,
        Role::Communications => eCommunications.0,
    }
}

fn to_wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

fn short_endpoint(endpoint_id: &str) -> String {
    let len = endpoint_id.chars().count();
    if len <= 24 {
        endpoint_id.to_string()
    } else {
        let tail: String = endpoint_id.chars().skip(len - 20).collect();
        format!("…{tail}")
    }
}
