//! 进程级界面语言。
//!
//! 语言是进程级状态：托盘菜单、tooltip、以及 Rust 侧产生的提示文案都需要它，
//! 因此集中放在一个原子变量里，由 `set_language` 命令统一修改。
//!
//! 分工：
//! - 前端界面的固定文案由 `src/i18n.ts` 负责；
//! - 本模块只覆盖 **Rust 侧产生**、最终会显示给用户的文本（托盘、状态标签、提示语、错误）。
//!
//! 所有文案都以「中文 + 英文」成对出现，避免漏翻：新增文案时只要两个分支都写满即可。

use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    Zh,
    En,
}

impl Lang {
    pub fn as_key(self) -> &'static str {
        match self {
            Lang::Zh => "zh",
            Lang::En => "en",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "zh" | "zh-cn" | "zh-hans" | "chinese" => Some(Lang::Zh),
            "en" | "en-us" | "en-gb" | "english" => Some(Lang::En),
            _ => None,
        }
    }
}

static CURRENT: AtomicU8 = AtomicU8::new(0);

pub fn set_lang(lang: Lang) {
    let raw = match lang {
        Lang::Zh => 0,
        Lang::En => 1,
    };
    CURRENT.store(raw, Ordering::SeqCst);
}

pub fn lang() -> Lang {
    if CURRENT.load(Ordering::SeqCst) == 1 {
        Lang::En
    } else {
        Lang::Zh
    }
}

pub fn is_en() -> bool {
    matches!(lang(), Lang::En)
}

/// 二选一文案。返回 `&'static str`，调用方可以直接放进结构体字段。
pub fn pick(zh: &'static str, en: &'static str) -> &'static str {
    if is_en() {
        en
    } else {
        zh
    }
}

/// 把占位符 `{name}` 之类的模板填好；两侧模板由调用方保证一致。
fn fill(template: &str, args: &[(&str, &str)]) -> String {
    let mut text = template.to_string();
    for (key, value) in args {
        text = text.replace(&format!("{{{key}}}"), value);
    }
    text
}

fn pair<'a>(zh: &'a str, en: &'a str) -> &'a str {
    if is_en() {
        en
    } else {
        zh
    }
}

// ---------------- 设备状态与连接方式 ----------------

pub fn state_label(key: &str) -> &'static str {
    match key {
        "active" => pick("可用", "Available"),
        "disabled" => pick("已禁用", "Disabled"),
        "notPresent" => pick("未连接", "Not connected"),
        "unplugged" => pick("已拔出", "Unplugged"),
        "missing" => pick("已失效", "Missing"),
        _ => pick("状态未知", "Unknown"),
    }
}

pub fn unknown_device() -> &'static str {
    pick("未知设备", "Unknown device")
}

/// 端点 ID 已失效时的副标题。
pub fn missing_subtitle() -> &'static str {
    pick("端点已失效，请重新选择", "Endpoint is gone — pick it again")
}

/// 总线类型（只在有区分意义时返回）。
pub fn bus_label(enumerator: &str) -> Option<&'static str> {
    match enumerator {
        "usb" => Some(pick("USB", "USB")),
        "bthenum" | "bthhfenum" | "bthledevice" => Some(pick("蓝牙", "Bluetooth")),
        "swd" => Some(pick("虚拟", "Virtual")),
        _ => None,
    }
}

/// 端点形态（`PKEY_AudioEndpoint_FormFactor`）。
pub fn form_label(form_factor: i32) -> Option<&'static str> {
    match form_factor {
        0 => Some(pick("网络音频", "Network audio")),
        1 => Some(pick("扬声器", "Speakers")),
        2 => Some(pick("线性输出", "Line out")),
        3 => Some(pick("耳机", "Headphones")),
        5 => Some(pick("头戴式耳机", "Headset")),
        6 => Some(pick("手持设备", "Handset")),
        8 => Some(pick("S/PDIF 输出", "S/PDIF out")),
        9 => Some(pick("HDMI 输出", "HDMI out")),
        _ => None,
    }
}

/// 总线与形态都不认识时的兜底描述。
pub fn fallback_bus_label(enumerator: &str) -> Option<&'static str> {
    match enumerator {
        "hdaudio" => Some(pick("板载音频", "Onboard audio")),
        "pci" => Some(pick("显卡音频", "GPU audio")),
        _ => None,
    }
}

// ---------------- 托盘 ----------------

pub mod tray {
    use super::*;

    pub fn current(name: &str) -> String {
        fill(
            pair("当前输出：{name}", "Now playing: {name}"),
            &[("name", name)],
        )
    }

    pub fn no_output() -> &'static str {
        pick(
            "当前输出：未检测到输出设备",
            "Now playing: no output device",
        )
    }

    pub fn cycle_next() -> &'static str {
        pick("切换到下一个设备", "Switch to next device")
    }

    pub fn devices_header() -> &'static str {
        pick("已配置设备", "Configured devices")
    }

    pub fn suffix_missing() -> &'static str {
        pick("（已失效）", " (missing)")
    }

    pub fn suffix_unavailable() -> &'static str {
        pick("（不可用）", " (unavailable)")
    }

    pub fn suffix_disabled() -> &'static str {
        pick("（已禁用）", " (disabled)")
    }

    pub fn none_configured() -> &'static str {
        pick("（尚未配置设备）", " (no devices configured)")
    }

    pub fn configure() -> &'static str {
        pick("设备配置与切换顺序", "Devices & rotation order")
    }

    pub fn open_main() -> &'static str {
        pick("打开主界面", "Open main window")
    }

    pub fn quit() -> &'static str {
        pick("退出", "Quit")
    }

    pub fn tooltip_normal(name: &str) -> String {
        fill(
            pair(
                "Audio Switch | 当前输出：{name}",
                "Audio Switch | Now playing: {name}",
            ),
            &[("name", name)],
        )
    }

    pub fn tooltip_switching() -> &'static str {
        pick(
            "Audio Switch | 正在切换音频设备...",
            "Audio Switch | Switching audio device...",
        )
    }

    pub fn tooltip_success(name: &str) -> String {
        fill(
            pair(
                "Audio Switch | 已切换到：{name}",
                "Audio Switch | Switched to: {name}",
            ),
            &[("name", name)],
        )
    }

    pub fn tooltip_error() -> &'static str {
        pick(
            "Audio Switch | 切换失败，请检查设备",
            "Audio Switch | Switch failed — check the device",
        )
    }
}

// ---------------- 切换与命令反馈 ----------------

pub fn switched_to(name: &str) -> String {
    fill(
        pair("已切换到：{name}", "Switched to: {name}"),
        &[("name", name)],
    )
}

pub fn switch_busy() -> &'static str {
    pick(
        "正在切换中，已忽略本次请求",
        "A switch is already in progress — request ignored",
    )
}

pub fn no_available_device() -> &'static str {
    pick(
        "没有可用的已配置设备，请先在「设备管理」中添加并启用设备",
        "No usable device configured. Add and enable one in Devices first.",
    )
}

pub fn single_device(name: &str) -> String {
    fill(
        pair(
            "当前只有「{name}」一个可用设备，已在默认输出上，无需切换",
            "“{name}” is the only usable device and already the default — nothing to switch",
        ),
        &[("name", name)],
    )
}

pub fn switch_task_failed(err: &str) -> String {
    fill(
        pair("切换任务执行失败：{err}", "Switch task failed: {err}"),
        &[("err", err)],
    )
}

pub fn device_missing(endpoint_id: &str) -> String {
    fill(
        pair(
            "设备不存在或已被移除：{id}",
            "Device not found or removed: {id}",
        ),
        &[("id", endpoint_id)],
    )
}

pub fn already_default(name: &str) -> String {
    fill(
        pair(
            "「{name}」已经是当前输出设备，无需切换",
            "“{name}” is already the current output — nothing to switch",
        ),
        &[("name", name)],
    )
}

pub fn device_unavailable(name: &str) -> String {
    fill(
        pair(
            "「{name}」当前不可用（已禁用或未连接），已取消切换",
            "“{name}” is unavailable (disabled or disconnected); switch cancelled",
        ),
        &[("name", name)],
    )
}

pub fn switch_failed(name: &str, err: &str) -> String {
    fill(
        pair(
            "切换到「{name}」失败：{err}",
            "Failed to switch to “{name}”: {err}",
        ),
        &[("name", name), ("err", err)],
    )
}

pub fn refresh_failed() -> &'static str {
    pick(
        "刷新设备失败，请检查 Windows 音频服务",
        "Failed to refresh devices. Check the Windows audio service.",
    )
}

pub fn devices_refreshed() -> &'static str {
    pick("设备列表已刷新", "Device list refreshed")
}

pub fn added_to_cycle(name: &str) -> String {
    fill(
        pair("已加入循环列表：{name}", "Added to rotation: {name}"),
        &[("name", name)],
    )
}

pub fn removed_from_cycle() -> &'static str {
    pick("已从循环列表移除设备", "Device removed from rotation")
}

pub fn device_enabled() -> &'static str {
    pick("已启用该设备", "Device enabled")
}

pub fn device_disabled() -> &'static str {
    pick("已禁用该设备", "Device disabled")
}

pub fn order_updated() -> &'static str {
    pick("循环顺序已更新", "Rotation order updated")
}

pub fn left_click_saved(enabled: bool) -> &'static str {
    if enabled {
        pick(
            "已保存：左键单击切换设备",
            "Saved: left click switches device",
        )
    } else {
        pick(
            "已保存：左键单击打开主界面",
            "Saved: left click opens the main window",
        )
    }
}

pub fn autostart_failed(err: &str) -> String {
    fill(
        pair(
            "设置开机自启失败：{err}",
            "Failed to change launch at login: {err}",
        ),
        &[("err", err)],
    )
}

pub fn autostart_verify_failed(err: &str) -> String {
    fill(
        pair(
            "开机自启已提交，但回读状态失败：{err}",
            "Launch at login applied, but reading it back failed: {err}",
        ),
        &[("err", err)],
    )
}

pub fn autostart_rejected() -> &'static str {
    pick(
        "系统未接受开机自启设置（可能被组策略或安全软件拦截）",
        "Windows did not accept the launch-at-login change (blocked by policy or security software?)",
    )
}

pub fn autostart_saved(enabled: bool) -> &'static str {
    if enabled {
        pick("已开启开机自启", "Launch at login enabled")
    } else {
        pick("已关闭开机自启", "Launch at login disabled")
    }
}

pub fn err_invalid_endpoint() -> &'static str {
    pick("端点 ID 格式非法", "Invalid endpoint ID")
}

pub fn err_device_not_present() -> &'static str {
    pick(
        "该设备当前不在系统中，请刷新设备列表后重试",
        "That device is not currently present. Refresh the list and try again.",
    )
}

pub fn err_already_in_cycle() -> &'static str {
    pick(
        "该设备已在循环列表中",
        "That device is already in the rotation",
    )
}

pub fn err_not_in_cycle() -> &'static str {
    pick("该设备不在循环列表中", "That device is not in the rotation")
}

pub fn err_bad_offset() -> &'static str {
    pick("排序方向非法", "Invalid move direction")
}

pub fn err_boundary() -> &'static str {
    pick(
        "已经到达列表边界，无法继续移动",
        "Already at the edge of the list",
    )
}

pub fn err_bad_language() -> &'static str {
    pick("不支持的语言", "Unsupported language")
}

pub fn err_bad_theme() -> &'static str {
    pick("不支持的主题", "Unsupported theme")
}

pub fn language_saved() -> &'static str {
    pick("界面语言已切换为中文", "Interface language set to English")
}

pub fn theme_saved(theme: &str) -> &'static str {
    if theme == "dark" {
        pick("已切换到深色主题", "Switched to the dark theme")
    } else {
        pick("已切换到浅色主题", "Switched to the light theme")
    }
}

pub fn watcher_degraded() -> &'static str {
    pick(
        "设备变化监听不可用，已切换为定时刷新",
        "Device notifications unavailable — falling back to periodic refresh",
    )
}

// ---------------- 错误描述 ----------------

pub fn com_init(detail: &str) -> String {
    fill(
        pair(
            "初始化 COM 失败：{detail}",
            "Failed to initialize COM: {detail}",
        ),
        &[("detail", detail)],
    )
}

pub fn windows_api(context: &str, detail: &str) -> String {
    fill(
        pair(
            "调用 Windows 音频接口失败（{context}）：{detail}",
            "Windows audio API call failed ({context}): {detail}",
        ),
        &[("context", context), ("detail", detail)],
    )
}

pub fn no_devices() -> &'static str {
    pick(
        "未检测到任何音频输出设备",
        "No audio output device detected",
    )
}

pub fn set_default_failed(detail: &str) -> String {
    fill(
        pair(
            "设置默认输出设备失败：{detail}",
            "Failed to set the default output device: {detail}",
        ),
        &[("detail", detail)],
    )
}

pub fn verification_failed(expected: &str, actual: &str) -> String {
    fill(
        pair(
            "切换后校验失败：期望 {expected}，实际 {actual}",
            "Post-switch verification failed: expected {expected}, got {actual}",
        ),
        &[("expected", expected), ("actual", actual)],
    )
}

pub fn policy_unavailable(detail: &str) -> String {
    fill(
        pair(
            "Windows 音频策略接口不可用：{detail}",
            "Windows audio policy interface unavailable: {detail}",
        ),
        &[("detail", detail)],
    )
}

// ---------------- 底层诊断细节 ----------------

pub fn policy_create_failed(detail: &str, code: u32) -> String {
    let code = format!("{code:08X}");
    fill(
        pair(
            "无法创建默认设备策略对象（{detail}，0x{code}）",
            "Cannot create the default-device policy object ({detail}, 0x{code})",
        ),
        &[("detail", detail), ("code", &code)],
    )
}

pub fn policy_interface_missing(code: u32) -> String {
    let code = format!("{code:08X}");
    fill(
        pair(
            "当前系统未提供 IPolicyConfig 接口（QueryInterface 返回 0x{code}）",
            "This system does not expose IPolicyConfig (QueryInterface returned 0x{code})",
        ),
        &[("code", &code)],
    )
}

pub fn watcher_thread_failed(err: &str) -> String {
    fill(
        pair(
            "创建设备监听线程失败：{err}",
            "Failed to create the device-watcher thread: {err}",
        ),
        &[("err", err)],
    )
}

pub fn watcher_start_timeout() -> &'static str {
    pick(
        "设备监听线程启动超时",
        "Device-watcher thread timed out during startup",
    )
}

pub fn set_default_returned(role: &str, endpoint: &str, code: u32) -> String {
    let code = format!("{code:08X}");
    fill(
        pair(
            "SetDefaultEndpoint({role}, {endpoint}) 返回 0x{code}",
            "SetDefaultEndpoint({role}, {endpoint}) returned 0x{code}",
        ),
        &[("role", role), ("endpoint", endpoint), ("code", &code)],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 语言是进程级全局状态：用互斥锁串行执行，避免并行测试互相覆盖。
    static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn with_lang<T>(lang: Lang, f: impl FnOnce() -> T) -> T {
        let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let previous = self::lang();
        set_lang(lang);
        let out = f();
        set_lang(previous);
        out
    }

    #[test]
    fn state_labels_follow_language() {
        assert_eq!(with_lang(Lang::Zh, || state_label("active")), "可用");
        assert_eq!(with_lang(Lang::En, || state_label("active")), "Available");
        assert_eq!(with_lang(Lang::En, || state_label("missing")), "Missing");
    }

    #[test]
    fn templates_are_filled_in_both_languages() {
        assert_eq!(
            with_lang(Lang::Zh, || switched_to("扬声器")),
            "已切换到：扬声器"
        );
        assert_eq!(
            with_lang(Lang::En, || switched_to("Speakers")),
            "Switched to: Speakers"
        );
        assert_eq!(
            with_lang(Lang::En, || switch_failed("Speakers", "timeout")),
            "Failed to switch to “Speakers”: timeout"
        );
    }

    #[test]
    fn language_parsing_accepts_locale_forms() {
        assert_eq!(Lang::parse("zh-CN"), Some(Lang::Zh));
        assert_eq!(Lang::parse("EN"), Some(Lang::En));
        assert_eq!(Lang::parse("fr"), None);
    }

    #[test]
    fn tray_suffixes_match_config_state() {
        assert_eq!(with_lang(Lang::En, tray::suffix_missing), " (missing)");
        assert_eq!(
            with_lang(Lang::Zh, || tray::current("扬声器")),
            "当前输出：扬声器"
        );
    }
}
