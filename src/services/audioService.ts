import { invoke } from "@tauri-apps/api/core";
import type { AppSnapshot, RuntimeInfo, SwitchOutcome } from "../types/audio";

/**
 * 后端命令的薄封装。
 * 所有错误都由 Rust 侧转换为中文字符串，这里只负责透传，不做二次翻译。
 */

export const audioService = {
  getSnapshot(): Promise<AppSnapshot> {
    return invoke<AppSnapshot>("get_snapshot");
  },

  refreshDevices(): Promise<AppSnapshot> {
    return invoke<AppSnapshot>("refresh_devices");
  },

  switchNext(): Promise<SwitchOutcome> {
    return invoke<SwitchOutcome>("switch_next");
  },

  switchToDevice(endpointId: string): Promise<SwitchOutcome> {
    return invoke<SwitchOutcome>("switch_to_device", { endpointId });
  },

  addDevice(endpointId: string): Promise<AppSnapshot> {
    return invoke<AppSnapshot>("add_device", { endpointId });
  },

  removeDevice(endpointId: string): Promise<AppSnapshot> {
    return invoke<AppSnapshot>("remove_device", { endpointId });
  },

  setDeviceEnabled(endpointId: string, enabled: boolean): Promise<AppSnapshot> {
    return invoke<AppSnapshot>("set_device_enabled", { endpointId, enabled });
  },

  moveDevice(endpointId: string, offset: -1 | 1): Promise<AppSnapshot> {
    return invoke<AppSnapshot>("move_device", { endpointId, offset });
  },

  setLeftClickSwitch(enabled: boolean): Promise<AppSnapshot> {
    return invoke<AppSnapshot>("set_left_click_switch", { enabled });
  },

  setAutostart(enabled: boolean): Promise<AppSnapshot> {
    return invoke<AppSnapshot>("set_autostart", { enabled });
  },

  setLanguage(language: "zh" | "en"): Promise<AppSnapshot> {
    return invoke<AppSnapshot>("set_language", { language });
  },

  setTheme(theme: "light" | "dark"): Promise<AppSnapshot> {
    return invoke<AppSnapshot>("set_theme", { theme });
  },

  openMainWindow(section?: string): Promise<void> {
    return invoke<void>("open_main_window", { section: section ?? null });
  },

  exitApp(): Promise<void> {
    return invoke<void>("exit_app");
  },

  getRuntimeInfo(): Promise<RuntimeInfo> {
    return invoke<RuntimeInfo>("get_runtime_info");
  },
};
