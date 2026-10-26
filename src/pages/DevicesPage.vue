<script setup lang="ts">
import { computed, ref } from "vue";

import DeviceIcon from "../components/DeviceIcon.vue";
import DeviceRow from "../components/DeviceRow.vue";
import { t } from "../i18n";
import type { CurrentOutput, DeviceView } from "../types/audio";

const props = defineProps<{
  currentOutput: CurrentOutput | null;
  devices: DeviceView[];
  busy: boolean;
  deviceError: string | null;
  switching: boolean;
}>();

const emit = defineEmits<{
  refresh: [];
  add: [endpointId: string];
  switchDevice: [endpointId: string];
  openOrder: [];
}>();

const showUnavailable = ref(false);

const currentDevice = computed(() => {
  if (!props.currentOutput) return null;
  return props.devices.find((device) => device.endpointId === props.currentOutput?.endpointId) ?? null;
});

const currentIcon = computed(() => currentDevice.value?.icon ?? "audio");
const currentSubtitle = computed(() => currentDevice.value?.subtitle ?? "");

const availableDevices = computed(() => props.devices.filter((device) => device.isAvailable));
const unavailableDevices = computed(() => props.devices.filter((device) => !device.isAvailable));
const configuredCount = computed(() => props.devices.filter((device) => device.isConfigured).length);

function deviceBadge(device: DeviceView): string {
  // 不可用状态（未连接 / 已拔出 / 已禁用）直接用后端给的本地化标签
  if (!device.isAvailable) return device.stateLabel;
  if (device.isConfigured && !device.isEnabled) return t("devices.disabled");
  return device.stateLabel;
}

function badgeTone(device: DeviceView): "ok" | "muted" | "warn" {
  if (!device.isAvailable) return "muted";
  if (device.isConfigured && !device.isEnabled) return "warn";
  return "ok";
}
</script>

<template>
  <div class="page">
    <h1 class="page-title">{{ t("devices.title") }}</h1>

    <div class="section-head">
      <h2>{{ t("devices.currentHeading") }}</h2>
      <button class="btn btn-ghost" :disabled="busy" @click="emit('refresh')">
        <span class="btn-icon" aria-hidden="true">⟳</span>
        {{ switching ? t("devices.switching") : t("devices.refresh") }}
      </button>
    </div>

    <div class="current-card" :class="{ 'is-empty': !currentOutput }">
      <DeviceIcon :kind="currentIcon" :size="26" />
      <div class="current-text">
        <div class="current-name">{{ currentOutput?.name ?? t("common.noOutput") }}</div>
        <div v-if="currentSubtitle" class="current-sub">{{ currentSubtitle }}</div>
      </div>
      <span v-if="currentOutput" class="badge badge-ok">
        <span class="badge-dot" aria-hidden="true"></span>
        {{ t("devices.currentBadge") }}
      </span>
    </div>

    <p v-if="deviceError" class="alert alert-error">
      {{ t("devices.enumerateFailed", { error: deviceError }) }}
    </p>

    <section class="card">
      <div class="card-head">
        <h2>{{ t("devices.listHeading") }}</h2>
        <div class="card-head-side">
          <span class="counter">{{ t("devices.configuredCount", { count: configuredCount }) }}</span>
          <button class="btn btn-ghost" :disabled="busy" @click="emit('refresh')">
            <span class="btn-icon" aria-hidden="true">⟳</span>
            {{ t("devices.refresh") }}
          </button>
        </div>
      </div>

      <p class="card-hint">
        {{ t("devices.listHint") }}
      </p>

      <p v-if="devices.length === 0" class="empty">{{ t("devices.empty") }}</p>

      <template v-else>
        <p v-if="availableDevices.length === 0" class="empty">{{ t("devices.emptyAvailable") }}</p>

        <div v-else class="row-list">
          <DeviceRow
            v-for="device in availableDevices"
            :key="device.endpointId"
            :icon="device.icon"
            :title="device.name"
            :subtitle="device.subtitle"
            :badge="deviceBadge(device)"
            :badge-tone="badgeTone(device)"
          >
            <template #actions>
              <button
                v-if="device.isConfigured"
                class="btn btn-quiet btn-sm"
                :disabled="busy"
                @click="emit('openOrder')"
              >
                {{ t("devices.inCycle", { index: device.cyclePosition ?? 0 }) }}
              </button>
              <button v-else class="btn btn-primary btn-sm" :disabled="busy" @click="emit('add', device.endpointId)">
                {{ t("devices.addToCycle") }}
              </button>
              <button class="btn btn-ghost btn-sm" :disabled="busy" @click="emit('switchDevice', device.endpointId)">
                {{ t("devices.switch") }}
              </button>
            </template>
          </DeviceRow>
        </div>

        <div v-if="unavailableDevices.length > 0" class="unavailable-block">
          <button class="link-btn" type="button" @click="showUnavailable = !showUnavailable">
            {{
              showUnavailable
                ? t("devices.collapseUnavailable", { count: unavailableDevices.length })
                : t("devices.expandUnavailable", { count: unavailableDevices.length })
            }}
          </button>

          <div v-if="showUnavailable" class="row-list is-compact">
            <DeviceRow
              v-for="device in unavailableDevices"
              :key="device.endpointId"
              :icon="device.icon"
              :title="device.name"
              :subtitle="device.subtitle"
              :badge="device.stateLabel"
              badge-tone="muted"
              muted
            >
              <template #actions>
                <span v-if="device.isConfigured" class="badge badge-muted">#{{ device.cyclePosition }}</span>
              </template>
            </DeviceRow>
          </div>
        </div>
      </template>
    </section>
  </div>
</template>
