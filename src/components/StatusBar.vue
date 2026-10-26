<script setup lang="ts">
import { t } from "../i18n";
import type { Feedback } from "../types/audio";

defineProps<{
  version: string;
  feedback: Feedback | null;
  configPath: string;
  logPath: string;
}>();
</script>

<template>
  <footer class="status-bar">
    <span class="status-feedback" :class="feedback ? `save-${feedback.kind}` : ''">
      <template v-if="feedback">
        <span class="status-time">{{ feedback.at }}</span>
        {{ feedback.message }}
      </template>
      <template v-else>{{ t("status.ready") }}</template>
    </span>
    <span
      class="status-right"
      :title="t('status.paths', { config: configPath, log: logPath })"
    >
      v{{ version }}
    </span>
  </footer>
</template>
