//! 循环切换算法。
//!
//! 该模块刻意不依赖任何 Windows API，只接收「已整理好的输入」并给出决策，
//! 因此可以用纯 Rust 单元测试覆盖全部边界条件（任务书第六节 1-12 条）。
//!
//! 输入约定：
//! - `current_default` 必须来自**实时读取**的 Windows 默认输出端点，而不是内存里的索引；
//! - `entries` 的顺序就是用户配置的循环顺序；
//! - `available` 表示该端点此刻是否可正常切换（已启用且状态为 Active）。

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CycleEntry {
    pub endpoint_id: String,
    pub name: String,
    /// 用户在配置里是否启用该设备
    pub enabled: bool,
    /// 系统当前是否可用（存在且为 Active 状态）
    pub available: bool,
}

impl CycleEntry {
    pub fn new(
        endpoint_id: impl Into<String>,
        name: impl Into<String>,
        enabled: bool,
        available: bool,
    ) -> Self {
        Self {
            endpoint_id: endpoint_id.into(),
            name: name.into(),
            enabled,
            available,
        }
    }

    fn is_usable(&self) -> bool {
        self.enabled && self.available
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CyclePlan {
    /// 切换到该设备
    SwitchTo {
        endpoint_id: String,
        name: String,
        /// 在可用设备序列中的位置（从 0 开始），仅用于日志
        position: usize,
    },
    /// 只有一个可用设备，且它已经是当前默认设备 —— 不执行无意义切换
    SingleDeviceNoop { endpoint_id: String, name: String },
    /// 没有任何可用的已配置设备
    NoAvailableDevice,
}

/// 计算「切换到下一个设备」的目标。
pub fn plan_next(current_default: Option<&str>, entries: &[CycleEntry]) -> CyclePlan {
    let usable: Vec<&CycleEntry> = entries.iter().filter(|entry| entry.is_usable()).collect();

    if usable.is_empty() {
        return CyclePlan::NoAvailableDevice;
    }

    if usable.len() == 1 {
        let only = usable[0];
        if Some(only.endpoint_id.as_str()) == current_default {
            return CyclePlan::SingleDeviceNoop {
                endpoint_id: only.endpoint_id.clone(),
                name: only.name.clone(),
            };
        }
        return CyclePlan::SwitchTo {
            endpoint_id: only.endpoint_id.clone(),
            name: only.name.clone(),
            position: 0,
        };
    }

    let position = match current_default {
        // 当前默认设备在循环列表中：从它的下一个开始
        Some(current) => usable
            .iter()
            .position(|entry| entry.endpoint_id == current)
            .map(|index| (index + 1) % usable.len())
            // 当前默认设备不在列表中（或已不可用）：从第一个可用设备开始
            .unwrap_or(0),
        None => 0,
    };

    let target = usable[position];
    CyclePlan::SwitchTo {
        endpoint_id: target.endpoint_id.clone(),
        name: target.name.clone(),
        position,
    }
}

/// 直接切换到指定设备的决策结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DirectPlan {
    SwitchTo { endpoint_id: String, name: String },
    AlreadyDefault { endpoint_id: String, name: String },
    NotAvailable { endpoint_id: String, name: String },
}

/// 计算「直接切换到指定设备」是否需要真正调用系统接口。
pub fn plan_direct(target: &CycleEntry, current_default: Option<&str>) -> DirectPlan {
    if !target.is_usable() {
        return DirectPlan::NotAvailable {
            endpoint_id: target.endpoint_id.clone(),
            name: target.name.clone(),
        };
    }

    if Some(target.endpoint_id.as_str()) == current_default {
        return DirectPlan::AlreadyDefault {
            endpoint_id: target.endpoint_id.clone(),
            name: target.name.clone(),
        };
    }

    DirectPlan::SwitchTo {
        endpoint_id: target.endpoint_id.clone(),
        name: target.name.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: &str, enabled: bool, available: bool) -> CycleEntry {
        CycleEntry::new(id, format!("设备{id}"), enabled, available)
    }

    #[test]
    fn cycles_in_configured_order() {
        let entries = vec![
            entry("usb", true, true),
            entry("bt", true, true),
            entry("spk", true, true),
        ];

        assert_eq!(plan_next(Some("usb"), &entries).endpoint_id(), "bt");
        assert_eq!(plan_next(Some("bt"), &entries).endpoint_id(), "spk");
        assert_eq!(
            plan_next(Some("spk"), &entries).endpoint_id(),
            "usb",
            "最后一个应回到第一个"
        );
    }

    #[test]
    fn falls_back_to_first_when_current_not_configured() {
        let entries = vec![entry("usb", true, true), entry("bt", true, true)];
        // HDMI 不在配置中
        assert_eq!(plan_next(Some("hdmi"), &entries).endpoint_id(), "usb");
        assert_eq!(plan_next(None, &entries).endpoint_id(), "usb");
    }

    #[test]
    fn skips_disconnected_devices() {
        let entries = vec![
            entry("usb", true, true),
            entry("bt", true, false), // 断开
            entry("spk", true, true),
        ];
        assert_eq!(plan_next(Some("usb"), &entries).endpoint_id(), "spk");
    }

    #[test]
    fn skips_disabled_devices() {
        let entries = vec![
            entry("usb", true, true),
            entry("bt", false, true), // 用户禁用
            entry("spk", true, true),
        ];
        assert_eq!(plan_next(Some("usb"), &entries).endpoint_id(), "spk");
    }

    #[test]
    fn current_device_disabled_starts_from_first_available() {
        let entries = vec![
            entry("usb", false, true), // 当前默认 但被禁用
            entry("bt", true, true),
        ];
        assert_eq!(plan_next(Some("usb"), &entries).endpoint_id(), "bt");
    }

    #[test]
    fn single_available_device_that_is_current_is_noop() {
        let entries = vec![entry("usb", true, true), entry("bt", true, false)];
        assert!(matches!(
            plan_next(Some("usb"), &entries),
            CyclePlan::SingleDeviceNoop { .. }
        ));
    }

    #[test]
    fn single_available_device_that_is_not_current_still_switches() {
        let entries = vec![entry("usb", true, true), entry("bt", true, false)];
        assert_eq!(plan_next(Some("hdmi"), &entries).endpoint_id(), "usb");
        assert_eq!(plan_next(None, &entries).endpoint_id(), "usb");
    }

    #[test]
    fn no_available_devices() {
        let entries = vec![entry("usb", true, false), entry("bt", false, true)];
        assert_eq!(
            plan_next(Some("usb"), &entries),
            CyclePlan::NoAvailableDevice
        );
        assert_eq!(plan_next(Some("usb"), &[]), CyclePlan::NoAvailableDevice);
    }

    #[test]
    fn wraps_only_over_usable_entries() {
        let entries = vec![
            entry("a", true, true),
            entry("b", true, false),
            entry("c", true, true),
            entry("d", true, false),
        ];
        // 可用序列是 [a, c]，c 的下一个应回到 a
        assert_eq!(plan_next(Some("c"), &entries).endpoint_id(), "a");
    }

    #[test]
    fn direct_switch_to_current_device_is_skipped() {
        let target = entry("usb", true, true);
        assert_eq!(
            plan_direct(&target, Some("usb")),
            DirectPlan::AlreadyDefault {
                endpoint_id: "usb".into(),
                name: "设备usb".into()
            }
        );
    }

    #[test]
    fn direct_switch_to_unavailable_device_is_rejected() {
        let target = entry("usb", true, false);
        assert!(matches!(
            plan_direct(&target, Some("bt")),
            DirectPlan::NotAvailable { .. }
        ));
    }

    #[test]
    fn direct_switch_to_other_device() {
        let target = entry("usb", true, true);
        assert!(matches!(
            plan_direct(&target, Some("bt")),
            DirectPlan::SwitchTo { .. }
        ));
    }

    impl CyclePlan {
        fn endpoint_id(&self) -> &str {
            match self {
                CyclePlan::SwitchTo { endpoint_id, .. } => endpoint_id,
                CyclePlan::SingleDeviceNoop { endpoint_id, .. } => endpoint_id,
                CyclePlan::NoAvailableDevice => panic!("没有可用设备"),
            }
        }
    }
}
