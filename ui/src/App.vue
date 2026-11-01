<script setup lang="ts">
// Rahmen: Kopfzeile mit Bereichen, Inhalt je Bereich. Kein Router — vier
// Ansichten brauchen keine URL-Verwaltung.
import { computed, ref } from "vue";
import * as UI from "@weppelmann/components";
import { IconMonitor, IconSliders } from "@weppelmann/icons";
import { state, t } from "./store";
import OverviewView from "./views/OverviewView.vue";
import ZonesView from "./views/ZonesView.vue";
import DevicesView from "./views/DevicesView.vue";
import SettingsView from "./views/SettingsView.vue";

type Tab = "overview" | "zones" | "devices" | "settings";
const tab = ref<Tab>("overview");

const tabs = computed<UI.Tab[]>(() => [
  { id: "overview", label: t("overview") },
  { id: "zones", label: t("zones"), badge: String(state.config?.zones.length ?? 0) },
  { id: "devices", label: t("devices"), badge: String(state.config?.devices.length ?? 0) },
  { id: "settings", label: t("settings") },
]);

const capture = computed(() => state.telemetry?.capture ?? "idle");
const badgeTone = computed(() => {
  if (capture.value === "running") return "ok" as const;
  if (capture.value === "error") return "error" as const;
  if (capture.value === "starting") return "warn" as const;
  return "neutral" as const;
});
</script>

<template>
  <div class="app">
    <header class="top">
      <div class="top__title">
        <UI.WpIcon :icon="IconMonitor" size="lg" class="top__mark" />
        <div>
          <p class="top__name">{{ t("app") }}</p>
          <p class="top__hint">{{ t("capture") }}: {{ t(capture) }}</p>
        </div>
      </div>
      <UI.WpBadge :tone="badgeTone" :label="t(capture)" />
    </header>

    <UI.WpTabs v-model="tab" :tabs="tabs" class="nav">
      <div v-if="!state.config" class="boot">
        <UI.WpSpinner size="md" />
        <UI.WpText tone="muted">{{ state.error || "…" }}</UI.WpText>
      </div>
      <OverviewView v-else-if="tab === 'overview'" />
      <ZonesView v-else-if="tab === 'zones'" />
      <DevicesView v-else-if="tab === 'devices'" />
      <SettingsView v-else />
    </UI.WpTabs>

    <footer class="foot">
      <UI.WpText variant="caption" tone="faint">
        {{ state.config?.devices.length ?? 0 }} {{ t("devices") }} ·
        {{ state.config?.zones.length ?? 0 }} {{ t("zones") }}
      </UI.WpText>
      <span class="foot__right">
        <UI.WpText v-if="state.error" variant="caption" tone="error">{{ state.error }}</UI.WpText>
        <UI.WpSpinner v-else-if="state.busy" size="sm" />
        <UI.WpIcon :icon="IconSliders" size="xs" />
      </span>
    </footer>
  </div>
</template>

<style scoped>
.app { display: flex; flex-direction: column; height: 100%; }
.top {
  display: flex; align-items: center; justify-content: space-between; gap: var(--wp-space-4);
  padding: var(--wp-space-3) var(--wp-space-4);
  border-bottom: var(--wp-size-border) solid var(--wp-color-border-subtle);
  background: var(--wp-color-bg-panel);
}
.top__title { display: flex; align-items: center; gap: var(--wp-space-2); }
.top__mark { color: var(--wp-color-accent-default); }
.top__name { margin: 0; font-size: var(--wp-font-size-md); font-weight: var(--wp-font-weight-semibold); }
.top__hint { margin: 0; font-size: var(--wp-font-size-xs); color: var(--wp-color-text-faint); }
.nav { flex: 1; min-height: 0; display: flex; flex-direction: column; padding: 0 var(--wp-space-4); }
.nav :deep(.wp-tabs__panel) { flex: 1; min-height: 0; overflow-y: auto; padding: var(--wp-space-3) 0; }
.boot { display: flex; align-items: center; gap: var(--wp-space-2); padding: var(--wp-space-6); }
.foot {
  display: flex; align-items: center; justify-content: space-between;
  padding: var(--wp-space-2) var(--wp-space-4);
  border-top: var(--wp-size-border) solid var(--wp-color-border-subtle);
  color: var(--wp-color-text-faint);
}
.foot__right { display: inline-flex; align-items: center; gap: var(--wp-space-2); }
</style>
