<script setup lang="ts">
import DeviceRow from "../components/DeviceRow.vue";
import { t, type MessageKey } from "../i18n";
import type { CycleItemView } from "../types/audio";

defineProps<{
  cycle: CycleItemView[];
  busy: boolean;
}>();

const emit = defineEmits<{
  remove: [endpointId: string];
  toggleEnabled: [endpointId: string, enabled: boolean];
  move: [endpointId: string, offset: -1 | 1];
  switchDevice: [endpointId: string];
  addDevice: [];
}>();

const rules: MessageKey[] = [
  "order.rule1",
  "order.rule2",
  "order.rule3",
  "order.rule4",
  "order.rule5",
];
</script>

<template>
  <div class="page">
    <h1 class="page-title">{{ t("order.title") }}</h1>

    <section class="card">
      <div class="card-head">
        <h2>{{ t("order.heading") }}</h2>
        <span class="counter">{{ t("order.count", { count: cycle.length }) }}</span>
      </div>

      <p v-if="cycle.length === 0" class="empty">
        {{ t("order.empty") }}
      </p>

      <div v-else class="row-list">
        <DeviceRow
          v-for="(item, index) in cycle"
          :key="item.endpointId"
          :icon="item.icon"
          :title="item.name"
          :subtitle="item.present ? item.subtitle : t('order.missingSubtitle')"
          :index="index + 1"
          :muted="!item.enabled || !item.available"
        >
          <template #actions>
            <button
              class="chip"
              :class="item.enabled ? 'chip-on' : 'chip-off'"
              :disabled="busy"
              :title="item.enabled ? t('order.disableHint') : t('order.enableHint')"
              @click="emit('toggleEnabled', item.endpointId, !item.enabled)"
            >
              {{ item.enabled ? t("order.enabled") : t("order.disabled") }}
            </button>

            <div class="stepper">
              <button
                class="stepper-btn"
                :disabled="busy || index === 0"
                :aria-label="t('order.moveUp')"
                :title="t('order.moveUp')"
                @click="emit('move', item.endpointId, -1)"
              >
                ⌃
              </button>
              <span class="stepper-index">{{ index + 1 }}</span>
              <button
                class="stepper-btn"
                :disabled="busy || index === cycle.length - 1"
                :aria-label="t('order.moveDown')"
                :title="t('order.moveDown')"
                @click="emit('move', item.endpointId, 1)"
              >
                ⌄
              </button>
            </div>

            <button
              class="icon-btn"
              :disabled="busy"
              :aria-label="t('order.remove')"
              :title="t('order.remove')"
              @click="emit('remove', item.endpointId)"
            >
              ✕
            </button>
          </template>
        </DeviceRow>
      </div>

      <div class="card-foot">
        <button class="btn btn-primary" :disabled="busy" @click="emit('addDevice')">
          {{ t("order.addDevice") }}
        </button>
      </div>
    </section>

    <section class="card">
      <div class="card-head">
        <h2>{{ t("order.rulesHeading") }}</h2>
      </div>
      <ol class="rule-list">
        <li v-for="rule in rules" :key="rule">{{ t(rule) }}</li>
      </ol>
    </section>
  </div>
</template>
