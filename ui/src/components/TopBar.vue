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
  <UI.WpAppBar :title="t('app')" :icon="IconWifi">
    <template #status>
      <UI.WpBadge :tone="liveTone" :label="liveLabel" />
      <p v-if="state.telemetry?.capture === 'error'" class="top__error" :title="state.telemetry?.capture_error ?? ''">
        {{ t("capture") }}: {{ state.telemetry?.capture_error }}
      </p>
    </template>
    <template #actions>
      <UI.WpSwitch v-model="testPicture" :label="t('testImage')" />
      <UI.WpDivider direction="vertical" />
      <UI.WpButton :label="t('restart')" :icon="IconRefresh" size="sm" @press="restartCapture" />
      <UI.WpButton :label="paused ? t('resume') : t('pause')" :icon="paused ? IconPlay : IconPause"
                   size="sm" :variant="paused ? 'primary' : 'secondary'" @press="setPaused(!paused)" />
    </template>
  </UI.WpAppBar>
</template>

<style scoped>
/* Nur der Fehlertext: Kopfzeile, Marke und Aktionen liefert WpAppBar. */
.top__error {
  margin: 0; max-width: 46ch; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  font-family: var(--wp-font-family-mono); font-size: var(--wp-font-size-xs); color: var(--wp-color-status-error);
}
</style>
