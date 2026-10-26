<script setup lang="ts">
/** 设备类型图标：依据 Windows 提供的端点总线类型选择，不按名称猜测。 */
withDefaults(
  defineProps<{
    kind: string;
    size?: number;
    muted?: boolean;
  }>(),
  { size: 22, muted: false },
);
</script>

<template>
  <span class="device-icon" :class="{ 'is-muted': muted }" :style="{ width: `${size + 20}px`, height: `${size + 20}px` }">
    <svg
      :width="size"
      :height="size"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width="1.7"
      stroke-linecap="round"
      stroke-linejoin="round"
      aria-hidden="true"
    >
      <!-- 耳机 -->
      <template v-if="kind === 'headphone'">
        <path d="M4 14v-2a8 8 0 0 1 16 0v2" />
        <rect x="2.2" y="13" width="4.6" height="8" rx="2.3" />
        <rect x="17.2" y="13" width="4.6" height="8" rx="2.3" />
      </template>

      <!-- 蓝牙 -->
      <template v-else-if="kind === 'bluetooth'">
        <path d="M7 7l10 10-5 4V3l5 4L7 17" />
      </template>

      <!-- 显示器 / HDMI -->
      <template v-else-if="kind === 'display'">
        <rect x="2.5" y="4" width="19" height="12.5" rx="1.8" />
        <path d="M8.5 20.5h7M12 16.5v4" />
      </template>

      <!-- 音箱 -->
      <template v-else-if="kind === 'speaker'">
        <rect x="7" y="2.5" width="10" height="19" rx="2" />
        <circle cx="12" cy="14.5" r="2.6" />
        <circle cx="12" cy="7" r="1.1" />
      </template>

      <!-- 虚拟设备 -->
      <template v-else-if="kind === 'virtual'">
        <path d="M3 12h2.5l2-5 3 10 2.5-7 2 4H21" />
      </template>

      <!-- 通用音频 -->
      <template v-else>
        <path d="M4 9.5h4l5-3.5v12l-5-3.5H4z" />
        <path d="M17 9.5a3.5 3.5 0 0 1 0 5" />
      </template>
    </svg>
  </span>
</template>
