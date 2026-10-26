import { computed, onMounted, onUnmounted, ref } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import { applyTheme, localeTag, setLocale, t } from "../i18n";
import { audioService } from "../services/audioService";
import type { AppSnapshot, Feedback, SwitchOutcome } from "../types/audio";

/**
 * 界面状态与后端同步的唯一入口。
 *
 * - 初始快照通过命令获取；
 * - 后续变化全部由后端事件推送（设备变化、配置变化、切换结果），
 *   因此界面不会与真实设备状态脱节；
 * - 本地只保留「忙碌」「提示信息」这类纯 UI 状态。
 */
export function useAudioDevices() {
  const snapshot = ref<AppSnapshot | null>(null);
  const busy = ref(false);
  const initializing = ref(true);
  const statusMessage = ref<Feedback | null>(null);
  const focusSection = ref<"devices" | "general" | null>(null);

  const unlisteners: UnlistenFn[] = [];

  function applySnapshot(next: AppSnapshot) {
    snapshot.value = next;
    // 语言与主题都属于「后端持久化、前端呈现」的状态，
    // 每次快照到达时对齐，保证托盘与界面永远不会各说各话。
    if (next.language === "zh" || next.language === "en") {
      setLocale(next.language);
    }
    if (next.theme === "light" || next.theme === "dark") {
      applyTheme(next.theme);
    }
  }

  /**
   * 状态栏只显示「切换设备」的结果与失败原因。
   * 设置类操作（开关、语言、主题、排序）成功后界面本身已经变化，
   * 再往状态栏写一条「已保存」属于噪音，因此不再提示；失败仍然要报出来，
   * 否则用户会以为设置生效了。
   */
  function noteSwitch(kind: Feedback["kind"], text: string) {
    statusMessage.value = {
      kind,
      message: text,
      at: new Date().toLocaleTimeString(localeTag(), { hour12: false }),
    };
  }

  function noteError(error: unknown) {
    noteSwitch("error", typeof error === "string" ? error : String(error));
  }

  async function run<T>(task: () => Promise<T>, onDone?: (result: T) => void) {
    if (busy.value) return;
    busy.value = true;
    try {
      const result = await task();
      onDone?.(result);
    } catch (error) {
      noteError(error);
    } finally {
      busy.value = false;
    }
  }

  const cycle = computed(() => snapshot.value?.cycle ?? []);
  const devices = computed(() => snapshot.value?.devices ?? []);
  const unconfiguredDevices = computed(() => devices.value.filter((device) => !device.isConfigured));
  const currentOutputName = computed(() => snapshot.value?.currentOutput?.name ?? t("common.noOutput"));

  const statusLabel = computed(() => {
    if (!snapshot.value) return t("nav.switching");
    if (snapshot.value.switching) return t("nav.switching");
    return t("status.ready");
  });

  const statusTone = computed<"ok" | "busy" | "warn">(() => {
    if (!snapshot.value) return "busy";
    if (snapshot.value.switching) return "busy";
    if (!snapshot.value.watcherActive) return "warn";
    return "ok";
  });

  async function initialize() {
    try {
      applySnapshot(await audioService.getSnapshot());
    } catch (error) {
      noteError(t("note.initFailed", { error: String(error) }));
    } finally {
      initializing.value = false;
    }
  }

  onMounted(async () => {
    await initialize();

    unlisteners.push(
      await listen<AppSnapshot>("audio-state-changed", (event) => {
        applySnapshot(event.payload);
      }),
    );

    unlisteners.push(
      await listen<SwitchOutcome>("audio-switch-result", (event) => {
        const outcome = event.payload;
        // 忙碌提示属于瞬时 UI 反馈，不覆盖后端的正式结论
        if (outcome.busy) return;
        noteSwitch(outcome.kind, outcome.message);
      }),
    );

    unlisteners.push(
      await listen<string>("ui-navigate", (event) => {
        if (event.payload === "devices" || event.payload === "general") {
          focusSection.value = event.payload;
        }
      }),
    );
  });

  onUnmounted(() => {
    unlisteners.forEach((unlisten) => unlisten());
    unlisteners.length = 0;
  });

  const refreshDevices = () => run(() => audioService.refreshDevices(), applySnapshot);

  const addDevice = (endpointId: string) =>
    run(() => audioService.addDevice(endpointId), applySnapshot);

  const removeDevice = (endpointId: string) =>
    run(() => audioService.removeDevice(endpointId), applySnapshot);

  const setDeviceEnabled = (endpointId: string, enabled: boolean) =>
    run(() => audioService.setDeviceEnabled(endpointId, enabled), applySnapshot);

  const moveDevice = (endpointId: string, offset: -1 | 1) =>
    run(() => audioService.moveDevice(endpointId, offset), applySnapshot);

  const switchTo = (endpointId: string) =>
    run(() => audioService.switchToDevice(endpointId), (outcome) => {
      if (!outcome.busy) noteSwitch(outcome.kind, outcome.message);
    });

  const switchNext = () =>
    run(() => audioService.switchNext(), (outcome) => {
      if (!outcome.busy) noteSwitch(outcome.kind, outcome.message);
    });

  /**
   * 左键行为开关：先乐观更新界面，失败时回滚，
   * 并以后端返回的快照为准（后端同时更新了托盘行为）。
   */
  async function setLeftClickSwitch(enabled: boolean) {
    const previous = snapshot.value;
    if (!previous) return;

    snapshot.value = { ...previous, leftClickSwitch: enabled };
    try {
      applySnapshot(await audioService.setLeftClickSwitch(enabled));
    } catch (error) {
      snapshot.value = previous;
      noteError(t("note.saveFailed", { error: typeof error === "string" ? error : String(error) }));
    }
  }

  /**
   * 开机自启：状态以系统启动项为准，因此不做乐观更新，
   * 成功后用后端回读到的真实值刷新界面。
   */
  async function setAutostart(enabled: boolean) {
    if (busy.value) return;
    busy.value = true;
    try {
      applySnapshot(await audioService.setAutostart(enabled));
    } catch (error) {
      noteError(error);
    } finally {
      busy.value = false;
    }
  }

  /**
   * 界面语言：先本地切换，界面立刻变成所选语言（不必等后端往返），
   * 失败时连同 snapshot 一起回滚，避免界面语言与配置文件不一致。
   */
  async function setLanguage(language: "zh" | "en") {
    const previous = snapshot.value;
    if (!previous || busy.value || previous.language === language) return;

    busy.value = true;
    setLocale(language);
    snapshot.value = { ...previous, language };
    try {
      applySnapshot(await audioService.setLanguage(language));
    } catch (error) {
      snapshot.value = previous;
      setLocale(previous.language);
      noteError(t("note.saveFailed", { error: typeof error === "string" ? error : String(error) }));
    } finally {
      busy.value = false;
    }
  }

  /** 主题切换与语言同理：先本地生效，失败回滚。 */
  async function setTheme(theme: "light" | "dark") {
    const previous = snapshot.value;
    if (!previous || busy.value || previous.theme === theme) return;

    busy.value = true;
    applyTheme(theme);
    snapshot.value = { ...previous, theme };
    try {
      applySnapshot(await audioService.setTheme(theme));
    } catch (error) {
      snapshot.value = previous;
      applyTheme(previous.theme);
      noteError(t("note.saveFailed", { error: typeof error === "string" ? error : String(error) }));
    } finally {
      busy.value = false;
    }
  }

  return {
    snapshot,
    busy,
    initializing,
    statusMessage,
    focusSection,
    cycle,
    devices,
    unconfiguredDevices,
    currentOutputName,
    statusLabel,
    statusTone,
    refreshDevices,
    addDevice,
    removeDevice,
    setDeviceEnabled,
    moveDevice,
    switchTo,
    switchNext,
    setLeftClickSwitch,
    setAutostart,
    setLanguage,
    setTheme,
  };
}
