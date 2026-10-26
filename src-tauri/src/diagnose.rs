//! 命令行诊断模式。
//!
//! 用法（输出可重定向到文件，GUI 子系统同样有效）：
//!
//! ```text
//! audio-switch.exe --diagnose                      仅打印真实设备与默认端点
//! audio-switch.exe --diagnose --switch-next        按配置顺序执行一次循环切换并验证
//! audio-switch.exe --diagnose --switch "<endpointId>"  直接切换到指定端点并验证
//! ```
//!
//! 该模式不创建窗口、不注册托盘，用于排查「设备枚举不到」「切换后没生效」等问题，
//! 也是本项目做真机验证的手段：所有结论都来自 Windows 的真实返回值。

use std::time::Instant;

use crate::audio::cycle::plan_next;
use crate::audio::default_endpoint::{self, SWITCH_ROLES};
use crate::audio::device;
use crate::config::ConfigStore;

/// 与 `tauri.conf.json` 中的 identifier 保持一致。
const APP_IDENTIFIER: &str = "com.audioswitch.desktop";

fn config_dir() -> Option<std::path::PathBuf> {
    std::env::var_os("APPDATA").map(|base| std::path::PathBuf::from(base).join(APP_IDENTIFIER))
}

fn name_of(devices: &[crate::audio::types::EndpointInfo], id: Option<&String>) -> String {
    match id {
        Some(id) => devices
            .iter()
            .find(|endpoint| &endpoint.id == id)
            .map(|endpoint| endpoint.display_name())
            .unwrap_or_else(|| format!("<未在设备列表中：{id}>")),
        None => "<无>".to_string(),
    }
}

pub fn run(args: &[String]) -> i32 {
    println!("=== Audio Switch 诊断 ===");

    let devices = match device::enumerate_endpoints() {
        Ok(devices) => devices,
        Err(err) => {
            println!("设备枚举失败：{err}");
            return 1;
        }
    };

    println!("输出端点数量：{}", devices.len());
    for (index, endpoint) in devices.iter().enumerate() {
        println!(
            "  #{index} [{:?}] {}  id={}",
            endpoint.state,
            endpoint.display_name(),
            endpoint.id
        );
    }

    let defaults = match default_endpoint::read_defaults() {
        Ok(defaults) => defaults,
        Err(err) => {
            println!("读取默认输出设备失败：{err}");
            return 1;
        }
    };

    println!(
        "默认端点（console）        ：{}",
        name_of(&devices, defaults.console.as_ref())
    );
    println!(
        "默认端点（multimedia）     ：{}",
        name_of(&devices, defaults.multimedia.as_ref())
    );
    println!(
        "默认端点（communications） ：{}",
        name_of(&devices, defaults.communications.as_ref())
    );

    if let Some(index) = args.iter().position(|arg| arg == "--switch") {
        let Some(target) = args.get(index + 1) else {
            println!("--switch 需要指定 endpointId");
            return 2;
        };
        return switch_and_report(target);
    }

    if args.iter().any(|arg| arg == "--switch-next") {
        let Some(dir) = config_dir() else {
            println!("无法定位配置目录（APPDATA 缺失）");
            return 2;
        };
        let store = ConfigStore::new(&dir);
        let (config, status) = store.load();
        println!(
            "配置加载状态：{}，已配置设备 {} 个",
            status.as_key(),
            config.devices.len()
        );

        let entries: Vec<_> = config
            .devices
            .iter()
            .map(|configured| {
                let live = devices
                    .iter()
                    .find(|endpoint| endpoint.id == configured.endpoint_id);
                crate::audio::cycle::CycleEntry::new(
                    configured.endpoint_id.clone(),
                    live.map(|e| e.display_name())
                        .unwrap_or_else(|| configured.last_known_name.clone()),
                    configured.enabled,
                    live.map(|e| e.state.is_active()).unwrap_or(false),
                )
            })
            .collect();

        let current = defaults.playback().cloned();
        match plan_next(current.as_deref(), &entries) {
            crate::audio::cycle::CyclePlan::NoAvailableDevice => {
                println!("循环切换结果：没有可用的已配置设备，未执行切换");
                return 3;
            }
            crate::audio::cycle::CyclePlan::SingleDeviceNoop { name, .. } => {
                println!("循环切换结果：只有「{name}」一个可用设备且已是默认，未执行切换");
                return 0;
            }
            crate::audio::cycle::CyclePlan::SwitchTo {
                endpoint_id, name, ..
            } => {
                println!("循环切换目标：{name}");
                return switch_and_report(&endpoint_id);
            }
        }
    }

    0
}

fn switch_and_report(endpoint_id: &str) -> i32 {
    println!("开始切换：{endpoint_id}");
    let started = Instant::now();

    match default_endpoint::set_default_and_verify(endpoint_id) {
        Ok(()) => {
            println!(
                "切换成功：已回读验证 console / multimedia 均指向目标端点（耗时 {} ms）",
                started.elapsed().as_millis()
            );

            // 二次确认：重新完整读取一次
            match default_endpoint::read_defaults() {
                Ok(defaults) => {
                    let ok = SWITCH_ROLES.iter().all(|role| {
                        defaults
                            .get(*role)
                            .map(|id| id == endpoint_id)
                            .unwrap_or(false)
                    });
                    println!("二次确认：{}", if ok { "一致" } else { "不一致" });
                    if ok {
                        0
                    } else {
                        4
                    }
                }
                Err(err) => {
                    println!("二次确认读取失败：{err}");
                    4
                }
            }
        }
        Err(err) => {
            println!(
                "切换失败（耗时 {} ms）：{err}",
                started.elapsed().as_millis()
            );
            1
        }
    }
}
