//! 音频设备与默认设备变化监听。
//!
//! 实现方式：`IMMDeviceEnumerator::RegisterEndpointNotificationCallback` 注册一个
//! `IMMNotificationClient`，由 Windows 在设备插拔、状态变化、默认设备切换时回调。
//!
//! 线程模型：
//! - 监听线程以 MTA 初始化 COM，创建枚举器、注册回调，然后阻塞等待退出信号；
//! - 回调由系统在内部线程上触发，回调里只做「投递事件」，不做任何 COM 调用或阻塞操作，
//!   避免在系统回调栈里重入音频接口造成死锁；
//! - 退出时先注销回调，再释放 COM 对象，最后反初始化 COM。

use std::sync::mpsc::{self, Sender};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

use windows::core::{implement, PCWSTR};
use windows::Win32::Foundation::PROPERTYKEY;
use windows::Win32::Media::Audio::{
    EDataFlow, ERole, IMMDeviceEnumerator, IMMNotificationClient, IMMNotificationClient_Impl,
    MMDeviceEnumerator, DEVICE_STATE,
};
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL};

use crate::audio::com::ComGuard;
use crate::audio::error::{AudioError, AudioResult};

/// 监听线程向业务层投递的原始事件。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WatcherEvent {
    DeviceStateChanged(u32),
    DeviceAdded,
    DeviceRemoved,
    DefaultDeviceChanged,
    PropertyChanged,
}

impl WatcherEvent {
    /// 用于日志的简短描述。
    pub fn describe(&self) -> String {
        match self {
            WatcherEvent::DeviceStateChanged(state) => format!("设备状态变化(0x{state:X})"),
            WatcherEvent::DeviceAdded => "设备插入".to_string(),
            WatcherEvent::DeviceRemoved => "设备移除".to_string(),
            WatcherEvent::DefaultDeviceChanged => "默认设备变化".to_string(),
            WatcherEvent::PropertyChanged => "设备属性变化".to_string(),
        }
    }
}

// windows-rs 0.58+ 的 `#[implement]` 用法：属性标在 struct 上，
// 真正的接口实现写在生成的 `NotificationClient_Impl` 类型上（它 Deref 到 NotificationClient）。
#[implement(IMMNotificationClient)]
struct NotificationClient {
    handler: Arc<dyn Fn(WatcherEvent) + Send + Sync>,
}

impl IMMNotificationClient_Impl for NotificationClient_Impl {
    fn OnDeviceStateChanged(
        &self,
        _device_id: &PCWSTR,
        new_state: DEVICE_STATE,
    ) -> windows::core::Result<()> {
        (self.handler)(WatcherEvent::DeviceStateChanged(new_state.0));
        Ok(())
    }

    fn OnDeviceAdded(&self, _device_id: &PCWSTR) -> windows::core::Result<()> {
        (self.handler)(WatcherEvent::DeviceAdded);
        Ok(())
    }

    fn OnDeviceRemoved(&self, _device_id: &PCWSTR) -> windows::core::Result<()> {
        (self.handler)(WatcherEvent::DeviceRemoved);
        Ok(())
    }

    fn OnDefaultDeviceChanged(
        &self,
        _flow: EDataFlow,
        _role: ERole,
        _default_device_id: &PCWSTR,
    ) -> windows::core::Result<()> {
        (self.handler)(WatcherEvent::DefaultDeviceChanged);
        Ok(())
    }

    fn OnPropertyValueChanged(
        &self,
        _device_id: &PCWSTR,
        _key: &PROPERTYKEY,
    ) -> windows::core::Result<()> {
        (self.handler)(WatcherEvent::PropertyChanged);
        Ok(())
    }
}

/// 设备监听器句柄。`Drop` 时会停止监听线程。
pub struct EndpointWatcher {
    stop: Option<Sender<()>>,
    handle: Option<JoinHandle<()>>,
}

impl EndpointWatcher {
    /// 启动监听。`handler` 会在系统回调线程中被调用，必须快速返回。
    pub fn start<F>(handler: F) -> AudioResult<Self>
    where
        F: Fn(WatcherEvent) + Send + Sync + 'static,
    {
        let (stop_tx, stop_rx) = mpsc::channel::<()>();
        let (ready_tx, ready_rx) = mpsc::channel::<Result<(), String>>();
        let handler = Arc::new(handler);

        let handle = std::thread::Builder::new()
            .name("audio-watcher".to_string())
            .spawn(move || {
                let _com = match ComGuard::mta() {
                    Ok(guard) => guard,
                    Err(err) => {
                        let _ = ready_tx.send(Err(format!("监听线程 COM 初始化失败：{err}")));
                        return;
                    }
                };

                let enumerator: IMMDeviceEnumerator =
                    match unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL) } {
                        Ok(enumerator) => enumerator,
                        Err(err) => {
                            let _ = ready_tx.send(Err(format!("创建设备枚举器失败：{err}")));
                            return;
                        }
                    };

                let client: IMMNotificationClient = NotificationClient { handler }.into();

                if let Err(err) =
                    unsafe { enumerator.RegisterEndpointNotificationCallback(&client) }
                {
                    let _ = ready_tx.send(Err(format!("注册设备通知失败：{err}")));
                    return;
                }

                let _ = ready_tx.send(Ok(()));
                log::info!("音频设备监听已启动");

                // 阻塞直到要求停止；系统回调运行在其它线程，不受此处阻塞影响
                let _ = stop_rx.recv();

                unsafe {
                    let _ = enumerator.UnregisterEndpointNotificationCallback(&client);
                }
                drop(client);
                log::info!("音频设备监听已停止");
            })
            .map_err(|err| {
                AudioError::Message(crate::i18n::watcher_thread_failed(&err.to_string()))
            })?;

        match ready_rx.recv_timeout(Duration::from_secs(5)) {
            Ok(Ok(())) => Ok(Self {
                stop: Some(stop_tx),
                handle: Some(handle),
            }),
            Ok(Err(message)) => {
                let _ = stop_tx.send(());
                let _ = handle.join();
                Err(AudioError::Message(message))
            }
            Err(_) => {
                let _ = stop_tx.send(());
                let _ = handle.join();
                Err(AudioError::Message(
                    crate::i18n::watcher_start_timeout().to_string(),
                ))
            }
        }
    }

    /// 停止监听并等待线程退出。重复调用安全。
    pub fn stop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for EndpointWatcher {
    fn drop(&mut self) {
        self.stop();
    }
}
