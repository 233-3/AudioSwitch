<script setup lang="ts">
import { t } from "../i18n";

defineProps<{
  leftClickSwitch: boolean;
  autostartEnabled: boolean;
  language: "zh" | "en";
  theme: "light" | "dark";
  busy: boolean;
}>();

const emit = defineEmits<{
  toggle: [enabled: boolean];
  toggleAutostart: [enabled: boolean];
  setLanguage: [language: "zh" | "en"];
  setTheme: [theme: "light" | "dark"];
}>();
</script>

<template>
  <div class="page">
    <h1 class="page-title">{{ t("general.title") }}</h1>

    <section class="card">
      <div class="setting-list">
        <div class="toggle-row">
          <div class="toggle-text">
            <h2>{{ t("general.leftClick.title") }}</h2>
            <p>
              {{ t("general.leftClick.desc1") }}<br />
              {{ t("general.leftClick.desc2") }}
            </p>
          </div>

          <button
            class="switch"
            type="button"
            role="switch"
            :aria-label="t('general.leftClick.title')"
            :aria-checked="leftClickSwitch"
            :disabled="busy"
            @click="emit('toggle', !leftClickSwitch)"
          >
            <span class="switch-knob"></span>
          </button>
        </div>

        <div class="toggle-row">
          <div class="toggle-text">
            <h2>{{ t("general.autostart.title") }}</h2>
            <p>
              {{ t("general.autostart.desc1") }}<br />
              {{ t("general.autostart.desc2") }}
            </p>
          </div>

          <button
            class="switch"
            type="button"
            role="switch"
            :aria-label="t('general.autostart.title')"
            :aria-checked="autostartEnabled"
            :disabled="busy"
            @click="emit('toggleAutostart', !autostartEnabled)"
          >
            <span class="switch-knob"></span>
          </button>
        </div>

        <div class="toggle-row">
          <div class="toggle-text">
            <h2>{{ t("general.language.title") }}</h2>
            <p>
              {{ t("general.language.desc1") }}<br />
              {{ t("general.language.desc2") }}
            </p>
          </div>

          <div class="segmented" role="radiogroup" :aria-label="t('general.language.title')">
            <button
              type="button"
              class="segmented-btn"
              role="radio"
              :class="{ 'is-active': language === 'zh' }"
              :aria-checked="language === 'zh'"
              :disabled="busy"
              @click="emit('setLanguage', 'zh')"
            >
              {{ t("general.language.zh") }}
            </button>
            <button
              type="button"
              class="segmented-btn"
              role="radio"
              :class="{ 'is-active': language === 'en' }"
              :aria-checked="language === 'en'"
              :disabled="busy"
              @click="emit('setLanguage', 'en')"
            >
              {{ t("general.language.en") }}
            </button>
          </div>
        </div>

        <div class="toggle-row">
          <div class="toggle-text">
            <h2>{{ t("general.theme.title") }}</h2>
            <p>
              {{ t("general.theme.desc1") }}<br />
              {{ t("general.theme.desc2") }}
            </p>
          </div>

          <div class="segmented" role="radiogroup" :aria-label="t('general.theme.title')">
            <button
              type="button"
              class="segmented-btn"
              role="radio"
              :class="{ 'is-active': theme === 'light' }"
              :aria-checked="theme === 'light'"
              :disabled="busy"
              @click="emit('setTheme', 'light')"
            >
              {{ t("general.theme.light") }}
            </button>
            <button
              type="button"
              class="segmented-btn"
              role="radio"
              :class="{ 'is-active': theme === 'dark' }"
              :aria-checked="theme === 'dark'"
              :disabled="busy"
              @click="emit('setTheme', 'dark')"
            >
              {{ t("general.theme.dark") }}
            </button>
          </div>
        </div>
      </div>

      <div class="behavior-grid">
        <div class="behavior-col">
          <h3>{{ t("general.whenOn") }}</h3>
          <div class="behavior-item">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
              <path d="M6 3.2l11.6 8.2-4.9 1.1 2.7 5.4-2.4 1.2-2.7-5.4-4.3 3.4z" />
            </svg>
            <span class="behavior-text">
              {{ t("general.behavior.leftSwitch") }}<em>{{ t("general.behavior.noWindow") }}</em>
            </span>
          </div>
          <div class="behavior-item">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
              <rect x="3.5" y="4.5" width="17" height="15" rx="2.5" />
              <path d="M7.5 9.5h9M7.5 13.5h5" />
            </svg>
            <span>{{ t("general.behavior.rightMenu") }}</span>
          </div>
        </div>

        <div class="behavior-col">
          <h3>{{ t("general.whenOff") }}</h3>
          <div class="behavior-item">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
              <path d="M6 3.2l11.6 8.2-4.9 1.1 2.7 5.4-2.4 1.2-2.7-5.4-4.3 3.4z" />
            </svg>
            <span class="behavior-text">
              {{ t("general.behavior.openWindow") }}<em>{{ t("general.behavior.noSwitch") }}</em>
            </span>
          </div>
          <div class="behavior-item">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
              <rect x="3.5" y="4.5" width="17" height="15" rx="2.5" />
              <path d="M7.5 9.5h9M7.5 13.5h5" />
            </svg>
            <span>{{ t("general.behavior.rightMenu") }}</span>
          </div>
        </div>
      </div>

      <p class="hint">
        {{ t("general.hint1") }}<br />
        {{ t("general.hint2") }}
      </p>
    </section>
  </div>
</template>
