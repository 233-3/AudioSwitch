/**
 * 界面文案的中英文词表与当前语言状态。
 *
 * 约定：
 * - 中文词表是「基准表」，`MessageKey` 由它推导，英文表必须写满同一批 key，
 *   少写一个 key 会在 `vue-tsc` 阶段直接报错，避免漏翻；
 * - 组件里用 `t("key")`，占位符用 `t("key", { count: 3 })`；
 * - Rust 侧的文案（托盘、提示语、错误）由 `src-tauri/src/i18n.rs` 负责，
 *   两边通过快照里的 `language` 字段保持同步。
 */

import { ref } from "vue";

export type Lang = "zh" | "en";
export type Theme = "light" | "dark";

const zh = {
  "app.loading": "正在读取系统音频设备…",

  "nav.general": "常规设置",
  "nav.devices": "设备管理",
  "nav.order": "切换顺序",
  "nav.about": "关于",
  "nav.current": "当前输出",
  "nav.switching": "正在切换…",

  "status.ready": "就绪",
  "status.paths": "配置文件：{config}\n日志文件：{log}",

  "common.noOutput": "未检测到输出设备",

  "general.title": "常规设置",
  "general.leftClick.title": "左键单击托盘图标切换设备",
  "general.leftClick.desc1": "开启后，单击托盘图标直接切换到下一个音频输出设备。",
  "general.leftClick.desc2": "关闭后，单击托盘图标将打开主界面。",
  "general.autostart.title": "开机自启",
  "general.autostart.desc1": "开启后，登录 Windows 时会自动启动 Audio Switch 并驻留系统托盘。",
  "general.autostart.desc2": "关闭后，需要手动从开始菜单启动。",
  "general.language.title": "界面语言",
  "general.language.desc1": "切换后立即生效，托盘菜单与提示语会一起切换。",
  "general.language.desc2": "选择会保存到本地，重启后保持。",
  "general.language.zh": "中文",
  "general.language.en": "English",
  "general.theme.title": "外观主题",
  "general.theme.desc1": "深色适合夜间或弱光环境，浅色适合白天使用。",
  "general.theme.desc2": "切换后立即生效并保存，重启后保持。",
  "general.theme.light": "浅色",
  "general.theme.dark": "深色",
  "general.whenOn": "开启时",
  "general.whenOff": "关闭时",
  "general.behavior.leftSwitch": "左键单击：切换到下一个设备",
  "general.behavior.noWindow": "不打开窗口",
  "general.behavior.rightMenu": "右键单击：打开托盘菜单",
  "general.behavior.openWindow": "左键单击：打开主界面",
  "general.behavior.noSwitch": "不切换设备",
  "general.hint1": "设置修改后立即生效并保存到本地，无需重启程序。右键菜单不受此开关影响；",
  "general.hint2": "开机自启由系统启动项维护，也可以在「任务管理器 → 启动」中随时关闭。",

  "devices.title": "设备管理",
  "devices.currentHeading": "当前默认输出设备",
  "devices.refresh": "刷新",
  "devices.switching": "切换中…",
  "devices.currentBadge": "当前使用",
  "devices.enumerateFailed": "设备枚举失败：{error}",
  "devices.listHeading": "设备列表（来自系统）",
  "devices.configuredCount": "{count} 个已加入循环",
  "devices.listHint": "这是 Windows 实际枚举到的播放端点；麦克风等输入设备不会出现在这里。",
  "devices.empty": "没有检测到任何音频输出设备。",
  "devices.emptyAvailable": "当前没有可用的输出设备，请检查设备连接。",
  "devices.inCycle": "已在循环 #{index}",
  "devices.addToCycle": "加入循环",
  "devices.switch": "切换",
  "devices.expandUnavailable": "展开不可用的设备（{count}）",
  "devices.collapseUnavailable": "收起不可用的设备（{count}）",
  "devices.disabled": "已禁用",

  "order.title": "切换顺序",
  "order.heading": "已配置设备（按顺序循环）",
  "order.count": "{count} 个",
  "order.empty": "还没有配置设备。到「设备管理」中把常用设备加入循环列表。",
  "order.enabled": "启用中",
  "order.disabled": "已禁用",
  "order.enableHint": "点击启用该设备",
  "order.disableHint": "点击禁用该设备",
  "order.moveUp": "上移",
  "order.moveDown": "下移",
  "order.remove": "从循环列表移除",
  "order.addDevice": "+ 添加设备",
  "order.rulesHeading": "循环规则",
  "order.rule1": "以 Windows 当前实际默认输出设备为基准，不使用内存缓存。",
  "order.rule2": "按用户配置的顺序循环，跳过已禁用与当前不可用的设备。",
  "order.rule3": "当前默认设备不在列表中时，切换到第一个有效设备。",
  "order.rule4": "只剩一个有效设备时不会执行无意义的切换。",
  "order.rule5": "切换后回读系统状态验证，失败不会显示成功。",
  "order.missingSubtitle": "端点已失效，请移除后重新添加",

  "about.title": "关于",
  "about.version": "版本 {version} · Windows 音频输出快速切换工具",
  "about.desc": "常驻系统托盘，左键单击即可在常用播放设备之间循环切换；所有切换都基于真实的 Windows 音频端点，并在切换后回读系统状态确认结果。",
  "about.runtime": "运行信息",
  "about.configRecovered": "配置已恢复默认",
  "about.configFile": "配置文件",
  "about.logFile": "日志文件",
  "about.watcher": "设备监听",
  "about.watcherOk": "系统事件监听正常",
  "about.watcherDegraded": "监听不可用，已降级为定时刷新",
  "about.switchRoles": "切换角色",
  "about.notInitialized": "未初始化",
  "about.limits": "已知限制",
  "about.limit1": "切换的是 Windows 系统默认输出设备；自行指定输出设备的应用（如部分游戏、独占模式播放器）不受影响。",
  "about.limit2": "只切换「系统声音」与「多媒体」两个角色，「通信」角色保持不变，以免改变会议软件的通话输出。",
  "about.limit3": "蓝牙耳机可能同时暴露立体声与免提两个端点，会被当作两个独立设备。",
  "about.limit4": "设备重新插拔后若端点 ID 变化，需要重新选择；程序不会按名称自动绑定新设备。",
  "about.privacy": "程序不联网、不登录、不上传任何数据。",

  "role.console": "系统声音",
  "role.multimedia": "多媒体",
  "role.communications": "通信",
  "role.joiner": "、",

  "note.initFailed": "读取初始状态失败：{error}",
  "note.saveFailed": "设置未保存：{error}",
} as const;

export type MessageKey = keyof typeof zh;

const en: Record<MessageKey, string> = {
  "app.loading": "Reading system audio devices…",

  "nav.general": "General",
  "nav.devices": "Devices",
  "nav.order": "Rotation",
  "nav.about": "About",
  "nav.current": "Now playing",
  "nav.switching": "Switching…",

  "status.ready": "Ready",
  "status.paths": "Config file: {config}\nLog file: {log}",

  "common.noOutput": "No output device detected",

  "general.title": "General",
  "general.leftClick.title": "Left-click tray icon to switch device",
  "general.leftClick.desc1": "When on, clicking the tray icon switches to the next audio output.",
  "general.leftClick.desc2": "When off, clicking the tray icon opens the main window.",
  "general.autostart.title": "Launch at login",
  "general.autostart.desc1": "When on, Audio Switch starts with Windows and stays in the notification area.",
  "general.autostart.desc2": "When off, start it manually from the Start menu.",
  "general.language.title": "Language",
  "general.language.desc1": "Applies immediately, including the tray menu and messages.",
  "general.language.desc2": "The choice is saved and restored after a restart.",
  "general.language.zh": "中文",
  "general.language.en": "English",
  "general.theme.title": "Theme",
  "general.theme.desc1": "Dark suits night use; light suits daytime use.",
  "general.theme.desc2": "Applies immediately, is saved, and persists after a restart.",
  "general.theme.light": "Light",
  "general.theme.dark": "Dark",
  "general.whenOn": "When on",
  "general.whenOff": "When off",
  "general.behavior.leftSwitch": "Left click: switch to the next device",
  "general.behavior.noWindow": "the window stays hidden",
  "general.behavior.rightMenu": "Right click: open the tray menu",
  "general.behavior.openWindow": "Left click: open the main window",
  "general.behavior.noSwitch": "no device switching",
  "general.hint1": "Changes apply immediately and are saved locally — no restart needed. The tray menu is not affected by this switch;",
  "general.hint2": "launch at login is a Windows startup entry and can also be turned off in Task Manager → Startup.",

  "devices.title": "Devices",
  "devices.currentHeading": "Current default output",
  "devices.refresh": "Refresh",
  "devices.switching": "Switching…",
  "devices.currentBadge": "In use",
  "devices.enumerateFailed": "Failed to enumerate devices: {error}",
  "devices.listHeading": "System devices",
  "devices.configuredCount": "{count} in rotation",
  "devices.listHint": "Playback endpoints as enumerated by Windows; microphones and other inputs never appear here.",
  "devices.empty": "No audio output device detected.",
  "devices.emptyAvailable": "No usable output device right now — check the connection.",
  "devices.inCycle": "In rotation #{index}",
  "devices.addToCycle": "Add to rotation",
  "devices.switch": "Switch",
  "devices.expandUnavailable": "Show unavailable devices ({count})",
  "devices.collapseUnavailable": "Hide unavailable devices ({count})",
  "devices.disabled": "Disabled",

  "order.title": "Rotation",
  "order.heading": "Configured devices (cycled in order)",
  "order.count": "{count}",
  "order.empty": "No devices configured yet. Add your usual devices to the rotation in Devices.",
  "order.enabled": "Enabled",
  "order.disabled": "Disabled",
  "order.enableHint": "Click to enable this device",
  "order.disableHint": "Click to disable this device",
  "order.moveUp": "Move up",
  "order.moveDown": "Move down",
  "order.remove": "Remove from rotation",
  "order.addDevice": "+ Add device",
  "order.rulesHeading": "Rotation rules",
  "order.rule1": "Always based on the real current default output reported by Windows — no cached state.",
  "order.rule2": "Follows your configured order, skipping disabled and currently unavailable devices.",
  "order.rule3": "If the current default is not in the list, switches to the first usable device.",
  "order.rule4": "Does nothing when only one usable device is left.",
  "order.rule5": "Reads the system state back after switching; a failure is never reported as success.",
  "order.missingSubtitle": "Endpoint is gone — remove it and add it again",

  "about.title": "About",
  "about.version": "Version {version} · Quick audio output switcher for Windows",
  "about.desc": "Lives in the notification area and cycles between your usual playback devices on a single left click. Every switch goes through the real Windows audio endpoint and is verified by reading the system state back.",
  "about.runtime": "Runtime",
  "about.configRecovered": "Config reset to defaults",
  "about.configFile": "Config file",
  "about.logFile": "Log file",
  "about.watcher": "Device watcher",
  "about.watcherOk": "System notifications active",
  "about.watcherDegraded": "Unavailable — falling back to periodic refresh",
  "about.switchRoles": "Roles switched",
  "about.notInitialized": "Not initialized",
  "about.limits": "Known limitations",
  "about.limit1": "Only the Windows default output is changed; apps that pick their own device (some games, exclusive-mode players) are unaffected.",
  "about.limit2": "Only the “System” and “Multimedia” roles are switched. “Communications” is left untouched so meeting apps keep their call output.",
  "about.limit3": "Bluetooth headsets may expose both a stereo and a hands-free endpoint and will appear as two separate devices.",
  "about.limit4": "If an endpoint ID changes after replugging, select the device again; the app never rebinds devices by name.",
  "about.privacy": "No network access, no sign-in, no data leaves this machine.",

  "role.console": "System",
  "role.multimedia": "Multimedia",
  "role.communications": "Communications",
  "role.joiner": ", ",

  "note.initFailed": "Failed to load initial state: {error}",
  "note.saveFailed": "Not saved: {error}",
};

const messages: Record<Lang, Record<MessageKey, string>> = { zh, en };

/** 当前语言。组件模板里调用 `t()` 时会读它，因此切换语言是响应式的。 */
export const locale = ref<Lang>("zh");
/** 当前主题，仅用于界面状态展示；实际样式由 `<html data-theme>` 驱动。 */
export const currentTheme = ref<Theme>("light");

export function setLocale(next: Lang) {
  locale.value = next;
  document.documentElement.lang = next === "zh" ? "zh-CN" : "en";
}

export function applyTheme(next: Theme) {
  currentTheme.value = next;
  document.documentElement.dataset.theme = next;
}

/** 时间格式跟随语言，避免中文界面里出现英文时间或反之。 */
export function localeTag(): string {
  return locale.value === "zh" ? "zh-CN" : "en-US";
}

export function t(key: MessageKey, vars?: Record<string, string | number>): string {
  const template = messages[locale.value][key] ?? messages.zh[key] ?? key;
  if (!vars) return template;
  return Object.entries(vars).reduce(
    (text, [name, value]) => text.replaceAll(`{${name}}`, String(value)),
    template,
  );
}

/** 音频角色名称，未知角色原样返回（后端新增角色时不至于显示为空）。 */
export function roleLabel(role: string): string {
  const key = `role.${role}` as MessageKey;
  return key in messages.zh ? t(key) : role;
}

/** 用当前语言的分隔符拼接角色列表。 */
export function joinRoles(roles: string[]): string {
  return roles.map(roleLabel).join(t("role.joiner"));
}
