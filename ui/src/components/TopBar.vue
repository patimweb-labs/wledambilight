<script setup lang="ts">
// Kopfzeile wie in der alten Oberfläche: Marke links, Zustand daneben,
// Handlungen rechts. Keine Navigation — der Rest des Fensters ist die Ansicht.
import { computed } from "vue";
import * as UI from "@weppelmann/components";
import { IconPause, IconPlay, IconRefresh, IconWifi } from "@weppelmann/icons";
import { commit, restartCapture, setPaused, state, t } from "../store";

const telemetry = computed(() => state.telemetry);
const paused = computed(() => telemetry.value?.paused ?? false);
const capture = computed(() => telemetry.value?.capture ?? "idle");

const liveTone = computed(() => {
  if (paused.value) return "neutral" as const;
  if (capture.value === "running") return "ok" as const;
  if (capture.value === "error") return "error" as const;
  if (capture.value === "starting") return "warn" as const;
  return "neutral" as const;
});

// Testbild liegt wie vorher in der Kopfzeile: ohne Bildschirmfreigabe prüfen,
// ob Zonen, Richtung und Farben stimmen.
const testPicture = computed({
  get: () => state.config?.settings.test_image ?? false,
  set: (value: boolean) => {
    if (!state.config) return;
    state.config.settings.test_image = value;
    commit();
  },
});

const liveLabel = computed(() => {
  if (paused.value) return t("paused");
  if (capture.value === "running") return `${t("live")} · ${(telemetry.value?.fps_in ?? 0).toFixed(0)} fps`;
  return t(capture.value as "idle");
});
</script>

<template>
  <header class="top">
    <div class="top__brand">
      <UI.WpIcon :icon="IconWifi" size="md" class="top__mark" />
      <span class="top__name">{{ t("app") }}</span>
    </div>
    <UI.WpBadge :tone="liveTone" :label="liveLabel" />
    <div class="top__spacer" />
    <UI.WpSwitch v-model="testPicture" :label="t('testImage')" />
    <UI.WpDivider direction="vertical" />
    <UI.WpButton :label="t('restart')" :icon="IconRefresh" size="sm" @press="restartCapture" />
    <UI.WpButton :label="paused ? t('resume') : t('pause')" :icon="paused ? IconPlay : IconPause"
                 size="sm" :variant="paused ? 'primary' : 'secondary'" @press="setPaused(!paused)" />
    <p v-if="state.telemetry?.capture === 'error'" class="top__error" :title="state.telemetry?.capture_error ?? ''">
      {{ t("capture") }}: {{ state.telemetry?.capture_error }}
    </p>
  </header>
</template>

<style scoped>
.top {
  display: flex; align-items: center; gap: var(--wp-space-3);
  padding: var(--wp-space-2) var(--wp-space-3);
  background: var(--wp-color-bg-panel);
  border-bottom: var(--wp-size-border) solid var(--wp-color-border-subtle);
}
.top__brand { display: flex; align-items: center; gap: var(--wp-space-2); }
.top__mark { color: var(--wp-color-accent-default); }
.top__name { font-family: var(--wp-font-family-display); font-size: var(--wp-font-size-md); font-weight: var(--wp-font-weight-semibold); letter-spacing: -0.01em; }
.top__spacer { flex: 1; }
.top__error { margin: 0; font-size: var(--wp-font-size-xs); color: var(--wp-color-status-error); max-width: 46ch; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
</style>
