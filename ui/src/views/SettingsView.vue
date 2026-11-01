<script setup lang="ts">
// Einstellungen: ein Formular über die Settings-Struktur des Kerns.
// Jede Änderung geht entprellt zurück; gespeichert wird, was das Modell sagt.
import { computed } from "vue";
import * as UI from "@weppelmann/components";
import { commit, state, t } from "../store";
import type { ThemeName } from "../api";

const settings = computed(() => state.config?.settings);

const theme = computed<ThemeName>({
  get: () => {
    const value = settings.value?.theme;
    return value === "light" || value === "mono" ? value : "dark";
  },
  set: (value: ThemeName) => {
    if (!state.config) return;
    state.config.settings.theme = value;
    commit();
  },
});

const language = computed<string>({
  get: () => settings.value?.language ?? "auto",
  set: (value: string) => {
    if (!state.config) return;
    state.config.settings.language = value;
    commit();
  },
});

const languageOptions = [
  { value: "auto", label: t("auto") },
  { value: "de", label: "Deutsch" },
  { value: "en", label: "English" },
];
const themeOptions = [
  { value: "dark", label: t("dark") },
  { value: "light", label: t("light") },
  { value: "mono", label: t("mono") },
];
const widthOptions = [160, 240, 320, 480, 640].map((w) => ({ value: String(w), label: `${w} px` }));

function setNumber(key: "target_fps" | "capture_width" | "smoothing_ms" | "spatial" | "slew" | "gamma" | "saturation" | "brightness", value: number) {
  if (!state.config) return;
  state.config.settings[key] = value;
  commit();
}

function setMonitor(value: string) {
  if (!state.config) return;
  state.config.settings.monitor = Math.max(0, Math.round(Number(value) || 0));
  commit();
}

function setWidth(value: string) {
  setNumber("capture_width", Number(value));
}

function setBool(key: "pause_when_focused" | "snap", value: boolean) {
  if (!state.config) return;
  state.config.settings[key] = value;
  commit();
}
</script>

<template>
  <div v-if="settings" class="settings">
    <UI.WpCard :title="t('capture')">
      <div class="form">
        <UI.WpSlider :model-value="settings.target_fps" :min="10" :max="120" :step="5" :label="t('targetFps')"
                     @update:model-value="setNumber('target_fps', $event)" />
        <UI.WpSelect :model-value="String(settings.capture_width)" :options="widthOptions" :label="t('captureWidth')"
                     @update:model-value="setWidth(String($event))" />
        <UI.WpFormField :label="t('monitor')" hint="0 = Hauptbildschirm (Windows/macOS)" v-slot="{ id }">
          <UI.WpInput :id="id" type="number" :model-value="String(settings.monitor)"
                      @update:model-value="setMonitor($event)" />
        </UI.WpFormField>
        <UI.WpSwitch :model-value="settings.test_image" :label="t('testImage')"
                     @update:model-value="settings!.test_image = $event; commit()" />
      </div>
    </UI.WpCard>

    <UI.WpCard title="Bild">
      <div class="form">
        <UI.WpSlider :model-value="settings.smoothing_ms" :min="0" :max="600" :step="10" :label="t('smoothing')"
                     @update:model-value="setNumber('smoothing_ms', $event)" />
        <UI.WpSlider :model-value="Math.round(settings.spatial * 100)" :min="0" :max="100" :label="t('spatial')" suffix=" %"
                     @update:model-value="setNumber('spatial', $event / 100)" />
        <UI.WpSlider :model-value="settings.slew" :min="0" :max="2000" :step="50" :label="t('slew')"
                     @update:model-value="setNumber('slew', $event)" />
        <UI.WpSlider :model-value="Math.round(settings.gamma * 100)" :min="50" :max="250" :label="t('gamma')"
                     @update:model-value="setNumber('gamma', $event / 100)" />
        <UI.WpSlider :model-value="Math.round(settings.saturation * 100)" :min="0" :max="200" :label="t('saturation')" suffix=" %"
                     @update:model-value="setNumber('saturation', $event / 100)" />
        <UI.WpSlider :model-value="Math.round(settings.brightness * 100)" :min="0" :max="200" :label="t('globalBrightness')" suffix=" %"
                     @update:model-value="setNumber('brightness', $event / 100)" />
      </div>
    </UI.WpCard>

    <UI.WpCard title="Verhalten">
      <div class="form">
        <UI.WpSwitch :model-value="settings.pause_when_focused" :label="t('pauseWhenFocused')"
                     @update:model-value="setBool('pause_when_focused', $event)" />
        <UI.WpSwitch :model-value="settings.snap" :label="t('snapZones')"
                     @update:model-value="setBool('snap', $event)" />
      </div>
    </UI.WpCard>

    <UI.WpCard :title="t('settings')">
      <div class="form">
        <UI.WpSelect :model-value="language" :options="languageOptions" :label="t('language')"
                     @update:model-value="language = String($event)" />
        <UI.WpRadioGroup :model-value="theme" name="theme" :label="t('theme')" :options="themeOptions"
                         @update:model-value="theme = $event as ThemeName" />
      </div>
    </UI.WpCard>
  </div>
</template>

<style scoped>
.settings { display: grid; grid-template-columns: repeat(auto-fit, minmax(360px, 1fr)); gap: var(--wp-space-4); align-items: start; }
.form { display: flex; flex-direction: column; gap: var(--wp-space-3); }
</style>
