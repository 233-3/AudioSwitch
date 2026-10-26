/** 与 Rust 后端 `state.rs` 中的快照结构一一对应。 */

export type DeviceStateKey = "active" | "disabled" | "notPresent" | "unplugged" | "unknown" | "missing";

export interface CurrentOutput {
  endpointId: string;
  name: string;
  roles: string[];
}

export interface DeviceView {
  endpointId: string;
  name: string;
  /** 连接方式副标题（来自 Windows 端点总线类型），未知时为空 */
  subtitle: string;
  /** 图标分类：headphone / bluetooth / display / speaker / virtual / audio */
  icon: string;
  deviceDesc: string;
  interfaceName: string;
  state: DeviceStateKey;
  stateLabel: string;
  isAvailable: boolean;
  isDefault: boolean;
  roles: string[];
  isConfigured: boolean;
  isEnabled: boolean;
  cyclePosition: number | null;
}

export interface CycleItemView {
  endpointId: string;
  name: string;
  subtitle: string;
  icon: string;
  lastKnownName: string;
  enabled: boolean;
  present: boolean;
  available: boolean;
  state: DeviceStateKey;
  stateLabel: string;
  isDefault: boolean;
}

export type NavKey = "general" | "devices" | "order" | "about";

export interface Feedback {
  kind: "info" | "success" | "error";
  message: string;
  at: string;
}

export interface AppSnapshot {
  appName: string;
  version: string;
  leftClickSwitch: boolean;
  currentOutput: CurrentOutput | null;
  devices: DeviceView[];
  cycle: CycleItemView[];
  configStatus: "created" | "loaded" | "recovered";
  configPath: string;
  logPath: string;
  switching: boolean;
  deviceError: string | null;
  lastFeedback: Feedback | null;
  switchRoles: string[];
  watcherActive: boolean;
  /** 是否已设置为开机自启（真实状态来自系统启动项） */
  autostartEnabled: boolean;
  /** 界面语言，与后端 config.json 保持一致 */
  language: "zh" | "en";
  /** 界面主题，与后端 config.json 保持一致 */
  theme: "light" | "dark";
}

export interface SwitchOutcome {
  ok: boolean;
  changed: boolean;
  busy: boolean;
  endpointId: string | null;
  name: string | null;
  kind: "info" | "success" | "error";
  message: string;
}

export interface RuntimeInfo {
  configPath: string;
  logPath: string;
  os: string;
  arch: string;
  watcherActive: boolean;
}
