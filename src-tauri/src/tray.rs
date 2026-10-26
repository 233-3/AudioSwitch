//! 系统托盘：图标状态机、tooltip、右键菜单与点击事件。
//!
//! 托盘是产品的主要入口，本模块负责：
//! - 四态图标（正常 / 切换中 / 成功 / 失败）与 tooltip 的联动；
//! - 左键行为由配置决定（切换设备 或 打开主窗口），右键菜单始终提供完整操作；
//! - 菜单随设备、配置和默认设备变化实时重建。
//!
//! 图标状态机用「代次（generation）」保证可靠性：每次进入临时状态都会递增代次，
//! 延时恢复任务只有在代次未被更新时才真正回落到正常状态，因此连续快速切换
//! 不会把图标永久停在成功/失败状态。

use std::sync::Arc;
use std::time::Duration;

use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Wry};

use crate::state::{AppRuntime, AppSnapshot};

/// 成功/失败反馈的保持时间。
pub const SUCCESS_HOLD: Duration = Duration::from_millis(1500);
pub const ERROR_HOLD: Duration = Duration::from_millis(3000);

/// Windows 托盘 tooltip 上限为 128 个宽字符，这里留出余量。
const TOOLTIP_MAX_CHARS: usize = 120;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconState {
    Normal,
    Switching,
    Success,
    Error,
}

pub struct TrayRuntime {
    pub tray: TrayIcon<Wry>,
    pub icon_state: IconState,
    pub generation: u64,
    /// 创建托盘时的主显示器缩放比例，用于选择合适尺寸的图标资源
    pub scale: f64,
}

/// 根据 DPI 选择托盘图标像素尺寸（Windows 任务栏图标随缩放变化）。
fn icon_px(scale: f64) -> u32 {
    if scale >= 1.75 {
        32
    } else if scale >= 1.4 {
        24
    } else if scale >= 1.15 {
        20
    } else {
        16
    }
}

fn icon_bytes(state: IconState, px: u32) -> &'static [u8] {
    match (state, px) {
        (IconState::Normal, 16) => include_bytes!("../assets/tray/normal-16.png"),
        (IconState::Normal, 20) => include_bytes!("../assets/tray/normal-20.png"),
        (IconState::Normal, 24) => include_bytes!("../assets/tray/normal-24.png"),
        (IconState::Normal, _) => include_bytes!("../assets/tray/normal-32.png"),
        (IconState::Switching, 16) => include_bytes!("../assets/tray/switching-16.png"),
        (IconState::Switching, 20) => include_bytes!("../assets/tray/switching-20.png"),
        (IconState::Switching, 24) => include_bytes!("../assets/tray/switching-24.png"),
        (IconState::Switching, _) => include_bytes!("../assets/tray/switching-32.png"),
        (IconState::Success, 16) => include_bytes!("../assets/tray/success-16.png"),
        (IconState::Success, 20) => include_bytes!("../assets/tray/success-20.png"),
        (IconState::Success, 24) => include_bytes!("../assets/tray/success-24.png"),
        (IconState::Success, _) => include_bytes!("../assets/tray/success-32.png"),
        (IconState::Error, 16) => include_bytes!("../assets/tray/error-16.png"),
        (IconState::Error, 20) => include_bytes!("../assets/tray/error-20.png"),
        (IconState::Error, 24) => include_bytes!("../assets/tray/error-24.png"),
        (IconState::Error, _) => include_bytes!("../assets/tray/error-32.png"),
    }
}

fn load_icon(state: IconState, scale: f64) -> tauri::Result<Image<'static>> {
    Image::from_bytes(icon_bytes(state, icon_px(scale)))
}

fn truncate_chars(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let kept: String = text.chars().take(max.saturating_sub(1)).collect();
    format!("{kept}…")
}

/// 生成 tooltip。名称全部来自真实枚举数据，不硬编码任何设备名。
fn tooltip_text(snapshot: &AppSnapshot, state: IconState) -> String {
    let current = snapshot
        .current_output
        .as_ref()
        .map(|output| output.name.as_str())
        .unwrap_or_else(|| crate::i18n::pick("未检测到输出设备", "no output device"));

    match state {
        IconState::Normal => truncate_chars(
            &crate::i18n::tray::tooltip_normal(current),
            TOOLTIP_MAX_CHARS,
        ),
        IconState::Switching => crate::i18n::tray::tooltip_switching().to_string(),
        IconState::Success => truncate_chars(
            &crate::i18n::tray::tooltip_success(current),
            TOOLTIP_MAX_CHARS,
        ),
        IconState::Error => crate::i18n::tray::tooltip_error().to_string(),
    }
}

fn build_menu(app: &AppHandle, snapshot: &AppSnapshot) -> tauri::Result<Menu<Wry>> {
    // 第一行始终显示真实的当前输出设备
    let current_label = match &snapshot.current_output {
        Some(output) => crate::i18n::tray::current(&truncate_chars(&output.name, 44)),
        None => crate::i18n::tray::no_output().to_string(),
    };
    let current_item = MenuItem::with_id(app, "menu.current", current_label, false, None::<&str>)?;

    let cycle_item = MenuItem::with_id(
        app,
        "menu.cycle-next",
        crate::i18n::tray::cycle_next(),
        true,
        None::<&str>,
    )?;
    let first_separator = PredefinedMenuItem::separator(app)?;

    // 已配置设备直接平铺：循环设备通常只有 2-4 个，比二级菜单少一次点击
    let section_item = MenuItem::with_id(
        app,
        "menu.devices-header",
        crate::i18n::tray::devices_header(),
        false,
        None::<&str>,
    )?;

    let mut device_items: Vec<MenuItem<Wry>> = Vec::new();
    for item in &snapshot.cycle {
        // 当前默认设备用 ✓ 标记；不可用设备明确标注，避免误认为可以切换
        let prefix = if item.is_default { "✓ " } else { "   " };
        let suffix = if !item.present {
            crate::i18n::tray::suffix_missing()
        } else if !item.available {
            crate::i18n::tray::suffix_unavailable()
        } else if !item.enabled {
            crate::i18n::tray::suffix_disabled()
        } else {
            ""
        };
        let label = format!("{prefix}{}{suffix}", truncate_chars(&item.name, 38));
        let enabled = item.present && item.available && item.enabled;
        device_items.push(MenuItem::with_id(
            app,
            format!("menu.device:{}", item.endpoint_id),
            label,
            enabled,
            None::<&str>,
        )?);
    }

    let empty_item = MenuItem::with_id(
        app,
        "menu.device-none",
        crate::i18n::tray::none_configured(),
        false,
        None::<&str>,
    )?;
    let second_separator = PredefinedMenuItem::separator(app)?;

    let configure_item = MenuItem::with_id(
        app,
        "menu.open-devices",
        crate::i18n::tray::configure(),
        true,
        None::<&str>,
    )?;
    let open_item = MenuItem::with_id(
        app,
        "menu.open-main",
        crate::i18n::tray::open_main(),
        true,
        None::<&str>,
    )?;
    let third_separator = PredefinedMenuItem::separator(app)?;
    let quit_item = MenuItem::with_id(
        app,
        "menu.quit",
        crate::i18n::tray::quit(),
        true,
        None::<&str>,
    )?;

    let mut items: Vec<&dyn tauri::menu::IsMenuItem<Wry>> =
        vec![&current_item, &cycle_item, &first_separator, &section_item];

    if device_items.is_empty() {
        items.push(&empty_item);
    } else {
        for item in &device_items {
            items.push(item);
        }
    }

    items.push(&second_separator);
    items.push(&configure_item);
    items.push(&open_item);
    items.push(&third_separator);
    items.push(&quit_item);

    Menu::with_items(app, &items)
}

/// 创建托盘。应用启动时调用一次。
pub fn install(app: &AppHandle, runtime: &Arc<AppRuntime>) -> tauri::Result<()> {
    let snapshot = runtime.snapshot();
    let scale = app
        .primary_monitor()
        .ok()
        .flatten()
        .map(|monitor| monitor.scale_factor())
        .unwrap_or(1.0);

    let image = load_icon(IconState::Normal, scale)?;
    let menu = build_menu(app, &snapshot)?;

    let tray = TrayIconBuilder::with_id("audio-switch-tray")
        .icon(image)
        .tooltip(tooltip_text(&snapshot, IconState::Normal))
        .menu(&menu)
        // 左键绝不弹出菜单：左键要么切换设备，要么打开主窗口
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|tray, event| {
            handle_tray_event(tray.app_handle(), event);
        })
        .on_menu_event(|app, event| {
            handle_menu_event(app, event.id().0.as_str());
        })
        .build(app)?;

    let mut guard = runtime
        .tray
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    *guard = Some(TrayRuntime {
        tray,
        icon_state: IconState::Normal,
        generation: 0,
        scale,
    });

    Ok(())
}

/// 统一渲染：图标 + tooltip + 菜单，保证三者始终与真实状态一致。
pub fn render(app: &AppHandle, runtime: &Arc<AppRuntime>) {
    let snapshot = runtime.snapshot();

    let (tray, state, scale) = {
        let guard = runtime
            .tray
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        match guard.as_ref() {
            Some(tray) => (tray.tray.clone(), tray.icon_state, tray.scale),
            None => return,
        }
    };

    match load_icon(state, scale) {
        Ok(image) => {
            if let Err(err) = tray.set_icon(Some(image)) {
                log::warn!("更新托盘图标失败：{err}");
            }
        }
        Err(err) => log::error!("加载托盘图标资源失败：{err}"),
    }

    if let Err(err) = tray.set_tooltip(Some(tooltip_text(&snapshot, state))) {
        log::warn!("更新托盘提示文本失败：{err}");
    }

    match build_menu(app, &snapshot) {
        Ok(menu) => {
            if let Err(err) = tray.set_menu(Some(menu)) {
                log::warn!("更新托盘菜单失败：{err}");
            }
        }
        Err(err) => log::warn!("构建托盘菜单失败：{err}"),
    }
}

/// 直接切换到某个图标状态（不自动恢复），用于「切换中」这类持续状态。
pub fn set_state(app: &AppHandle, state: IconState) {
    let runtime = app.state::<Arc<AppRuntime>>().inner().clone();

    {
        let mut guard = runtime
            .tray
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        match guard.as_mut() {
            Some(tray) => {
                tray.generation = tray.generation.wrapping_add(1);
                tray.icon_state = state;
            }
            None => return,
        }
    }

    // 立即生效，不等待切换结果
    render(app, &runtime);
}

/// 显示临时状态，并在 `hold` 之后自动回到正常状态。
pub fn flash(app: &AppHandle, state: IconState, hold: Duration) {
    let runtime = app.state::<Arc<AppRuntime>>().inner().clone();

    let generation = {
        let mut guard = runtime
            .tray
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        match guard.as_mut() {
            Some(tray) => {
                tray.generation = tray.generation.wrapping_add(1);
                tray.icon_state = state;
                tray.generation
            }
            None => return,
        }
    };

    // 立即生效，不等待切换结果
    render(app, &runtime);

    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(hold);
        let should_render = {
            let mut guard = runtime
                .tray
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            match guard.as_mut() {
                Some(tray) if tray.generation == generation => {
                    tray.icon_state = IconState::Normal;
                    true
                }
                // 已有更新的状态，不再覆盖
                _ => false,
            }
        };
        if should_render {
            render(&app, &runtime);
        }
    });
}

/// 判断一个托盘事件是否应当被当作「左键单击托盘图标」。
///
/// Windows 会分别为按下和抬起派发 `Click` 事件，只认「左键 + 抬起」，
/// 保证一次物理单击最多触发一次操作；右键、中键与双击一律不参与。
fn is_left_click_release(event: &TrayIconEvent) -> bool {
    match event {
        TrayIconEvent::Click {
            button,
            button_state,
            ..
        } => is_left_click_release_parts(*button, *button_state),
        // 双击、悬停、移动等事件都不参与切换
        _ => false,
    }
}

fn is_left_click_release_parts(button: MouseButton, state: MouseButtonState) -> bool {
    matches!((button, state), (MouseButton::Left, MouseButtonState::Up))
}

fn handle_tray_event(app: &AppHandle, event: TrayIconEvent) {
    if !is_left_click_release(&event) {
        return;
    }

    let runtime = app.state::<Arc<AppRuntime>>().inner().clone();
    let switch_on_left_click = runtime.with_core(|core| core.config.left_click_switch);

    if switch_on_left_click {
        log::info!("托盘左键单击：切换到下一个设备");
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            crate::switching::cycle_next(&app, crate::switching::SwitchSource::TrayClick).await;
        });
    } else {
        log::info!("托盘左键单击：打开主窗口");
        crate::window::show_main_window(app, None);
    }
}

fn handle_menu_event(app: &AppHandle, id: &str) {
    match id {
        "menu.current" => {}
        "menu.cycle-next" => {
            log::info!("托盘菜单：切换到下一个设备");
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                crate::switching::cycle_next(&app, crate::switching::SwitchSource::TrayMenu).await;
            });
        }
        "menu.open-main" => crate::window::show_main_window(app, None),
        "menu.open-devices" => crate::window::show_main_window(app, Some("devices")),
        "menu.quit" => {
            log::info!("托盘菜单：退出应用");
            crate::shutdown::exit_app(app);
        }
        other => {
            if let Some(endpoint_id) = other.strip_prefix("menu.device:") {
                let endpoint_id = endpoint_id.to_string();
                log::info!("托盘菜单：直接切换到 {endpoint_id}");
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    crate::switching::switch_to(
                        &app,
                        &endpoint_id,
                        crate::switching::SwitchSource::TrayMenu,
                    )
                    .await;
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_left_button_release_triggers_action() {
        assert!(is_left_click_release_parts(
            MouseButton::Left,
            MouseButtonState::Up
        ));
        // 同一次物理单击还会派发按下事件，必须忽略，否则一次点击会切换两次
        assert!(!is_left_click_release_parts(
            MouseButton::Left,
            MouseButtonState::Down
        ));
        // 右键绝不能触发切换逻辑
        assert!(!is_left_click_release_parts(
            MouseButton::Right,
            MouseButtonState::Up
        ));
        assert!(!is_left_click_release_parts(
            MouseButton::Right,
            MouseButtonState::Down
        ));
        assert!(!is_left_click_release_parts(
            MouseButton::Middle,
            MouseButtonState::Up
        ));
    }

    #[test]
    fn tooltip_is_truncated_within_windows_limit() {
        let long_name: String = "超长设备名称".repeat(40);
        let text = truncate_chars(&long_name, TOOLTIP_MAX_CHARS);
        assert_eq!(text.chars().count(), TOOLTIP_MAX_CHARS);
        assert!(text.ends_with('…'));

        let short = truncate_chars("扬声器", TOOLTIP_MAX_CHARS);
        assert_eq!(short, "扬声器");
    }

    #[test]
    fn tray_icon_size_follows_dpi() {
        assert_eq!(icon_px(1.0), 16);
        assert_eq!(icon_px(1.25), 20);
        assert_eq!(icon_px(1.5), 24);
        assert_eq!(icon_px(2.0), 32);
    }
}
