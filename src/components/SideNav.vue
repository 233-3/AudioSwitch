<script setup lang="ts">
import { t, type MessageKey } from "../i18n";
import type { NavKey } from "../types/audio";

defineProps<{
  active: NavKey;
  currentName: string;
  currentSubtitle: string;
  switching: boolean;
}>();

const emit = defineEmits<{
  select: [key: NavKey];
}>();

const items: { key: NavKey; label: MessageKey }[] = [
  { key: "general", label: "nav.general" },
  { key: "devices", label: "nav.devices" },
  { key: "order", label: "nav.order" },
  { key: "about", label: "nav.about" },
];
</script>

<template>
  <aside class="side-nav">
    <nav class="nav-list">
      <button
        v-for="item in items"
        :key="item.key"
        type="button"
        class="nav-item"
        :class="{ 'is-active': active === item.key }"
        @click="emit('select', item.key)"
      >
        <svg
          class="nav-icon"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="1.7"
          stroke-linecap="round"
          stroke-linejoin="round"
          aria-hidden="true"
        >
          <template v-if="item.key === 'general'">
            <circle cx="12" cy="12" r="3.2" />
            <path
              d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.6 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.6 1.65 1.65 0 0 0 10 3.09V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9c.14.63.7 1.09 1.51 1H21a2 2 0 0 1 0 4h-.09c-.81 0-1.37.46-1.51 1z"
            />
          </template>
          <template v-else-if="item.key === 'devices'">
            <path d="M4 14v-2a8 8 0 0 1 16 0v2" />
            <rect x="2.2" y="13" width="4.6" height="8" rx="2.3" />
            <rect x="17.2" y="13" width="4.6" height="8" rx="2.3" />
          </template>
          <template v-else-if="item.key === 'order'">
            <path d="M4 7h11M4 12h11M4 17h7M19 5v8m0 0 2.2-2.2M19 13l-2.2-2.2" />
          </template>
          <template v-else>
            <circle cx="12" cy="12" r="9" />
            <path d="M12 11v5.5M12 7.6v.2" />
          </template>
        </svg>
        <span>{{ t(item.label) }}</span>
      </button>
    </nav>

    <div class="nav-current" :class="{ 'is-switching': switching }">
      <span class="nav-current-label">{{ switching ? t("nav.switching") : t("nav.current") }}</span>
      <span class="nav-current-name" :title="currentName">{{ currentName }}</span>
      <span v-if="currentSubtitle" class="nav-current-sub">{{ currentSubtitle }}</span>
    </div>
  </aside>
</template>
