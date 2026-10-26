<script setup lang="ts">
import { computed, ref, watch } from "vue";

import SideNav from "./components/SideNav.vue";
import StatusBar from "./components/StatusBar.vue";
import AboutPage from "./pages/AboutPage.vue";
import DevicesPage from "./pages/DevicesPage.vue";
import GeneralPage from "./pages/GeneralPage.vue";
import OrderPage from "./pages/OrderPage.vue";
import { useAudioDevices } from "./composables/useAudioDevices";
import { t } from "./i18n";
import type { NavKey } from "./types/audio";

const audio = useAudioDevices();
const tab = ref<NavKey>("general");

const snapshot = audio.snapshot;

const currentName = computed(() => snapshot.value?.currentOutput?.name ?? t("common.noOutput"));
const currentSubtitle = computed(() => {
  const id = snapshot.value?.currentOutput?.endpointId;
  if (!id) return "";
  const device = snapshot.value?.devices.find((item) => item.endpointId === id);
  return device?.subtitle ?? "";
});

// 托盘「设备配置与切换顺序」→ 定位到设备管理页
watch(
  () => audio.focusSection.value,
  (section) => {
    if (!section) return;
    tab.value = section === "general" ? "general" : "devices";
    audio.focusSection.value = null;
  },
);

// 首次启动且尚未配置任何设备时，直接落在「设备管理」引导用户添加
const initialTabApplied = ref(false);
watch(
  snapshot,
  (value) => {
    if (!value || initialTabApplied.value) return;
    initialTabApplied.value = true;
    if (value.cycle.length === 0) tab.value = "devices";
  },
  { immediate: true },
);
</script>

<template>
  <div class="app-shell">
    <SideNav
      :active="tab"
      :current-name="currentName"
      :current-subtitle="currentSubtitle"
      :switching="snapshot?.switching ?? false"
      @select="tab = $event"
    />

    <div class="app-main">
      <div v-if="!snapshot" class="app-loading">{{ t("app.loading") }}</div>

      <div v-else class="page-scroll">
        <GeneralPage
          v-if="tab === 'general'"
          :left-click-switch="snapshot.leftClickSwitch"
          :autostart-enabled="snapshot.autostartEnabled"
          :language="snapshot.language"
          :theme="snapshot.theme"
          :busy="audio.busy.value"
          @toggle="audio.setLeftClickSwitch"
          @toggle-autostart="audio.setAutostart"
          @set-language="audio.setLanguage"
          @set-theme="audio.setTheme"
        />

        <DevicesPage
          v-else-if="tab === 'devices'"
          :current-output="snapshot.currentOutput"
          :devices="audio.devices.value"
          :busy="audio.busy.value"
          :device-error="snapshot.deviceError"
          :switching="snapshot.switching"
          @refresh="audio.refreshDevices"
          @add="audio.addDevice"
          @switch-device="audio.switchTo"
          @open-order="tab = 'order'"
        />

        <OrderPage
          v-else-if="tab === 'order'"
          :cycle="audio.cycle.value"
          :busy="audio.busy.value"
          @remove="audio.removeDevice"
          @toggle-enabled="audio.setDeviceEnabled"
          @move="audio.moveDevice"
          @switch-device="audio.switchTo"
          @add-device="tab = 'devices'"
        />

        <AboutPage
          v-else
          :version="snapshot.version"
          :config-path="snapshot.configPath"
          :log-path="snapshot.logPath"
          :watcher-active="snapshot.watcherActive"
          :switch-roles="snapshot.switchRoles"
          :config-status="snapshot.configStatus"
        />
      </div>

      <StatusBar
        v-if="snapshot"
        :version="snapshot.version"
        :feedback="audio.statusMessage.value"
        :config-path="snapshot.configPath"
        :log-path="snapshot.logPath"
      />
    </div>
  </div>
</template>
