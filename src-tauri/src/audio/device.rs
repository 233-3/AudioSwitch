//! 音频输出设备枚举。
//!
//! 只做「读取」：把 Windows 的音频端点映射为 [`EndpointInfo`]。
//! 所有下游逻辑都使用 `endpointId` 定位设备，绝不使用名称或列表下标，
//! 因为同一物理设备可能暴露多个端点，而不同设备的显示名称也可能重复。

use std::ffi::c_void;

use windows::core::PWSTR;
use windows::Win32::Devices::FunctionDiscovery::{
    PKEY_DeviceInterface_FriendlyName, PKEY_Device_DeviceDesc, PKEY_Device_EnumeratorName,
    PKEY_Device_FriendlyName,
};
use windows::Win32::Foundation::PROPERTYKEY;
use windows::Win32::Media::Audio::{
    eRender, IMMDeviceCollection, IMMDeviceEnumerator, MMDeviceEnumerator,
    PKEY_AudioEndpoint_FormFactor, DEVICE_STATE, DEVICE_STATEMASK_ALL, DEVICE_STATE_ACTIVE,
    DEVICE_STATE_DISABLED, DEVICE_STATE_NOTPRESENT, DEVICE_STATE_UNPLUGGED,
};
use windows::Win32::System::Com::StructuredStorage::{PropVariantClear, PROPVARIANT};
use windows::Win32::System::Com::{CoCreateInstance, CoTaskMemFree, CLSCTX_ALL, STGM_READ};
use windows::Win32::System::Variant::{VT_LPWSTR, VT_UI4};
use windows::Win32::UI::Shell::PropertiesSystem::IPropertyStore;

use crate::audio::com::ComGuard;
use crate::audio::error::{AudioError, AudioResult};
use crate::audio::types::{DeviceState, EndpointInfo};

/// 创建设备枚举器。调用方必须保证当前线程已初始化 COM。
pub fn create_enumerator() -> AudioResult<IMMDeviceEnumerator> {
    unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL) }.map_err(|err| {
        AudioError::windows(
            crate::i18n::pick("创建音频设备枚举器", "create device enumerator"),
            err,
        )
    })
}

/// 把 `GetId` 返回的 COM 字符串转为 Rust `String` 并释放系统内存。
///
/// # Safety
/// `pwstr` 必须是系统通过 `CoTaskMemAlloc` 分配、由调用方接管所有权的指针。
pub unsafe fn endpoint_id_of(pwstr: PWSTR) -> String {
    if pwstr.is_null() {
        return String::new();
    }
    let value = pwstr.to_string().unwrap_or_default();
    CoTaskMemFree(Some(pwstr.0 as *const c_void));
    value
}

fn map_state(raw: DEVICE_STATE) -> DeviceState {
    let bits = raw.0;
    if bits & DEVICE_STATE_ACTIVE.0 != 0 {
        DeviceState::Active
    } else if bits & DEVICE_STATE_DISABLED.0 != 0 {
        DeviceState::Disabled
    } else if bits & DEVICE_STATE_NOTPRESENT.0 != 0 {
        DeviceState::NotPresent
    } else if bits & DEVICE_STATE_UNPLUGGED.0 != 0 {
        DeviceState::Unplugged
    } else {
        DeviceState::Unknown
    }
}

/// 读取属性存储中的字符串属性；属性缺失或类型不符时返回 `None`。
///
/// # Safety
/// 需在已初始化 COM 的线程上调用。
unsafe fn read_prop_string(store: &IPropertyStore, key: &PROPERTYKEY) -> Option<String> {
    let mut value: PROPVARIANT = store.GetValue(key as *const PROPERTYKEY).ok()?;

    let text = {
        let inner = &value.Anonymous.Anonymous;
        if inner.vt == VT_LPWSTR {
            let ptr = inner.Anonymous.pwszVal;
            if ptr.is_null() {
                None
            } else {
                ptr.to_string().ok()
            }
        } else {
            None
        }
    };

    // 无论是否取到字符串都要释放 PROPVARIANT 内部资源，避免每次枚举泄漏
    let _ = PropVariantClear(&mut value);

    text.map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

/// 读取属性存储中的 `VT_UI4` 属性（例如端点形态）。
unsafe fn read_prop_u32(store: &IPropertyStore, key: &PROPERTYKEY) -> Option<u32> {
    let mut value: PROPVARIANT = store.GetValue(key as *const PROPERTYKEY).ok()?;
    let result = {
        let inner = &value.Anonymous.Anonymous;
        if inner.vt == VT_UI4 {
            Some(inner.Anonymous.ulVal)
        } else {
            None
        }
    };
    let _ = PropVariantClear(&mut value);
    result
}

unsafe fn read_endpoint(
    device: &windows::Win32::Media::Audio::IMMDevice,
) -> AudioResult<EndpointInfo> {
    let id = endpoint_id_of(device.GetId().map_err(|e| {
        AudioError::windows(crate::i18n::pick("读取端点 ID", "read endpoint ID"), e)
    })?);

    let state = map_state(device.GetState().map_err(|e| {
        AudioError::windows(crate::i18n::pick("读取端点状态", "read endpoint state"), e)
    })?);

    // 属性读取失败（例如设备已被拔出）不应导致整次枚举失败
    let (friendly_name, device_desc, interface_name, enumerator, form_factor) =
        match device.OpenPropertyStore(STGM_READ) {
            Ok(store) => (
                read_prop_string(&store, &PKEY_Device_FriendlyName).unwrap_or_default(),
                read_prop_string(&store, &PKEY_Device_DeviceDesc).unwrap_or_default(),
                read_prop_string(&store, &PKEY_DeviceInterface_FriendlyName).unwrap_or_default(),
                read_prop_string(&store, &PKEY_Device_EnumeratorName).unwrap_or_default(),
                // 10 = UnknownFormFactor
                read_prop_u32(&store, &PKEY_AudioEndpoint_FormFactor).unwrap_or(10) as i32,
            ),
            Err(_) => (
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                10,
            ),
        };

    Ok(EndpointInfo {
        id,
        friendly_name,
        device_desc,
        interface_name,
        enumerator,
        form_factor,
        state,
    })
}

unsafe fn collect(collection: &IMMDeviceCollection) -> AudioResult<Vec<EndpointInfo>> {
    let count = collection.GetCount().map_err(|e| {
        AudioError::windows(crate::i18n::pick("读取端点数量", "read endpoint count"), e)
    })?;

    let mut devices = Vec::with_capacity(count as usize);
    for index in 0..count {
        match collection.Item(index) {
            Ok(device) => match read_endpoint(&device) {
                Ok(info) => devices.push(info),
                Err(err) => log::warn!("跳过无法读取的音频端点 #{index}：{err}"),
            },
            Err(err) => log::warn!("读取音频端点 #{index} 失败：{err}"),
        }
    }
    Ok(devices)
}

/// 枚举所有音频**输出**端点（含已禁用、未连接、已拔出的设备）。
///
/// 输入设备（麦克风）不会出现在结果中，因为枚举使用的数据流方向是 `eRender`。
pub fn enumerate_endpoints() -> AudioResult<Vec<EndpointInfo>> {
    let _com = ComGuard::mta()?;
    let enumerator = create_enumerator()?;

    let mask = DEVICE_STATE(DEVICE_STATEMASK_ALL);
    let collection = unsafe { enumerator.EnumAudioEndpoints(eRender, mask) }.map_err(|err| {
        AudioError::windows(
            crate::i18n::pick("枚举音频输出端点", "enumerate audio output endpoints"),
            err,
        )
    })?;

    let mut devices = unsafe { collect(&collection)? };

    // 同一 endpointId 只保留一条（防御性去重）
    let mut seen = std::collections::HashSet::new();
    devices.retain(|d| !d.id.is_empty() && seen.insert(d.id.clone()));

    // 稳定排序：可用设备优先，其次按显示名称
    devices.sort_by(|a, b| {
        let a_active = a.state.is_active();
        let b_active = b.state.is_active();
        b_active
            .cmp(&a_active)
            .then_with(|| a.display_name().cmp(&b.display_name()))
            .then_with(|| a.id.cmp(&b.id))
    });

    if devices.is_empty() {
        return Err(AudioError::NoDevices);
    }

    Ok(devices)
}

/// 判断端点 ID 是否是合法的 Windows 音频端点 ID。
///
/// 用于拒绝前端传入的任意字符串，避免把非法 ID 直接交给系统接口。
pub fn is_valid_endpoint_id(id: &str) -> bool {
    let trimmed = id.trim();
    !trimmed.is_empty() && trimmed.starts_with('{') && trimmed.ends_with('}') && trimmed.len() < 512
}
