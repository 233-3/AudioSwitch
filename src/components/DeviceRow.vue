<script setup lang="ts">
import DeviceIcon from "./DeviceIcon.vue";

withDefaults(
  defineProps<{
    icon: string;
    title: string;
    subtitle?: string;
    /** 右侧状态徽标文案与色调 */
    badge?: string;
    badgeTone?: "ok" | "muted" | "warn";
    muted?: boolean;
    /** 循环列表中的序号 */
    index?: number | null;
  }>(),
  { subtitle: "", badge: "", badgeTone: "muted", muted: false, index: null },
);
</script>

<template>
  <div class="row" :class="{ 'is-muted': muted }">
    <span v-if="index !== null" class="row-index">{{ index }}</span>
    <DeviceIcon :kind="icon" :muted="muted" />

    <div class="row-text">
      <div class="row-title" :title="title">{{ title }}</div>
      <div v-if="subtitle" class="row-sub" :title="subtitle">{{ subtitle }}</div>
    </div>

    <div class="row-end">
      <span v-if="badge" class="badge" :class="`badge-${badgeTone}`">
        <span class="badge-dot" aria-hidden="true"></span>
        {{ badge }}
      </span>
      <div class="row-actions">
        <slot name="actions"></slot>
      </div>
    </div>
  </div>
</template>
