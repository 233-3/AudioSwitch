<script setup lang="ts">
import { joinRoles, t, type MessageKey } from "../i18n";

defineProps<{
  version: string;
  configPath: string;
  logPath: string;
  watcherActive: boolean;
  switchRoles: string[];
  configStatus: string;
}>();

const limitations: MessageKey[] = [
  "about.limit1",
  "about.limit2",
  "about.limit3",
  "about.limit4",
];
</script>

<template>
  <div class="page">
    <h1 class="page-title">{{ t("about.title") }}</h1>

    <section class="card about-hero">
      <svg class="about-icon" viewBox="0 0 256 256" aria-hidden="true">
        <rect x="0" y="0" width="256" height="256" rx="58" fill="#2563EB" />
        <path
          d="M64 144v-12a64 64 0 0 1 128 0v12"
          stroke="#fff"
          stroke-width="26"
          fill="none"
          stroke-linecap="round"
        />
        <rect x="42" y="126" width="44" height="80" rx="22" fill="#fff" />
        <rect x="170" y="126" width="44" height="80" rx="22" fill="#fff" />
      </svg>

      <div>
        <div class="about-name">Audio Switch</div>
        <div class="about-ver">{{ t("about.version", { version }) }}</div>
        <p class="about-desc">
          {{ t("about.desc") }}
        </p>
      </div>
    </section>

    <section class="card">
      <div class="card-head">
        <h2>{{ t("about.runtime") }}</h2>
        <span v-if="configStatus === 'recovered'" class="badge badge-warn">
          <span class="badge-dot" aria-hidden="true"></span>
          {{ t("about.configRecovered") }}
        </span>
      </div>

      <dl class="info-list">
        <div class="info-row">
          <dt>{{ t("about.configFile") }}</dt>
          <dd :title="configPath">{{ configPath }}</dd>
        </div>
        <div class="info-row">
          <dt>{{ t("about.logFile") }}</dt>
          <dd :title="logPath">{{ logPath }}</dd>
        </div>
        <div class="info-row">
          <dt>{{ t("about.watcher") }}</dt>
          <dd>{{ watcherActive ? t("about.watcherOk") : t("about.watcherDegraded") }}</dd>
        </div>
        <div class="info-row">
          <dt>{{ t("about.switchRoles") }}</dt>
          <dd>{{ switchRoles.length ? joinRoles(switchRoles) : t("about.notInitialized") }}</dd>
        </div>
      </dl>
    </section>

    <section class="card">
      <div class="card-head">
        <h2>{{ t("about.limits") }}</h2>
      </div>
      <ul class="limit-list">
        <li v-for="item in limitations" :key="item">{{ t(item) }}</li>
      </ul>
      <p class="hint">{{ t("about.privacy") }}</p>
    </section>
  </div>
</template>
