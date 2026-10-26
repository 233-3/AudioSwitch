//! COM 初始化辅助。
//!
//! COM 的初始化状态是线程级的。音频枚举、默认设备读写都可能在不同线程上被调用
//! （Tauri 命令线程、设备监听线程），因此每次调用都用 [`ComGuard`] 保证当前线程
//! 已完成初始化，并在离开作用域时成对释放。

use windows::Win32::Foundation::RPC_E_CHANGED_MODE;
use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_MULTITHREADED};

use crate::audio::error::{AudioError, AudioResult};

pub struct ComGuard {
    /// 是否由本对象完成了一次成功的 CoInitializeEx（需要配对 CoUninitialize）。
    owns_init: bool,
}

impl ComGuard {
    /// 以多线程单元（MTA）初始化当前线程。
    ///
    /// 若当前线程已被初始化为其它套间模型，则直接复用现有套间而不报错
    /// （音频端点枚举与默认设备设置在 MTA/STA 下均可工作）。
    pub fn mta() -> AudioResult<Self> {
        // 注意：windows-rs 中 CoInitializeEx 返回 HRESULT 而不是 Result
        let hr = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
        if hr.is_ok() {
            return Ok(Self { owns_init: true });
        }
        if hr == RPC_E_CHANGED_MODE {
            // 当前线程已被初始化为其它套间模型，直接复用
            return Ok(Self { owns_init: false });
        }
        Err(AudioError::ComInit(format!("0x{:08X}", hr.0 as u32)))
    }
}

impl Drop for ComGuard {
    fn drop(&mut self) {
        if self.owns_init {
            unsafe { CoUninitialize() };
        }
    }
}
