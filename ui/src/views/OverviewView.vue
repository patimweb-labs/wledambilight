<script setup lang="ts">
// Übersicht: was die Pipeline gerade tut. Alles hier ist Projektion von Telemetry.
import { computed } from "vue";
import * as UI from "@weppelmann/components";
import { IconPlay, IconRefresh, IconStop, IconWarning } from "@weppelmann/icons";
import { forgetPermission, restartCapture, setPaused, state, t, commit } from "../store";
import LedStrip from "../components/LedStrip.vue";

const telemetry = computed(() => state.telemetry);
const paused = computed(() => telemetry.value?.paused ?? false);
const testImage = computed({
  get: () => state.config?.settings.test_image ?? false,
  set: (value: boolean) => {
    if (!state.config) return;
    state.config.settings.test_image = value;
    commit();
  },
});

const deviceState = (id: string) => telemetry.value?.devices.find((d) => d.id === id)?.state ?? "connecting";

type BadgeTone = "ok" | "warn" | "error" | "neutral";
const tone = (s: string): BadgeTone => (s === "ok" ? "ok" : s === "error" ? "error" : "neutral");
</script>

<template>
  <div class="view">
    <div class="cards">
      <UI.WpCard title="Aufnahme">
        <div class="metrics">
          <div class="metric"><span class="metric__k">{{ t("fpsIn") }}</span><span class="metric__v">{{ telemetry?.fps_in?.toFixed(1) ?? "0" }}</span></div>
          <div class="metric"><span class="metric__k">{{ t("fpsOut") }}</span><span class="metric__v">{{ telemetry?.fps_out?.toFixed(1) ?? "0" }}</span></div>
          <div class="metric"><span class="metric__k">{{ t("latency") }}</span><span class="metric__v">{{ telemetry ? Math.round(telemetry.sample_us / 1000) : 0 }} ms</span></div>
          <div class="metric"><span class="metric__k">{{ t("screen") }}</span><span class="metric__v">{{ telemetry?.screen?.[0] ?? 0 }}×{{ telemetry?.screen?.[1] ?? 0 }}</span></div>
          <div class="metric"><span class="metric__k">{{ t("buffer") }}</span><span class="metric__v">{{ telemetry?.dims?.[0] ?? 0 }}×{{ telemetry?.dims?.[1] ?? 0 }}</span></div>
          <div class="metric"><span class="metric__k">{{ t("leds") }}</span><span class="metric__v">{{ telemetry?.leds.length ?? 0 }}</span></div>
        </div>
        <UI.WpToast v-if="telemetry?.capture === 'error'" tone="error" :title="t('error')" :description="telemetry.capture_error ?? ''" />
        <UI.WpDivider spaced />
        <UI.WpStack direction="row" gap="sm">
          <UI.WpButton :label="paused ? t('start') : t('stop')" :icon="paused ? IconPlay : IconStop"
                       :variant="paused ? 'primary' : 'secondary'" @press="setPaused(!paused)" />
          <UI.WpButton :label="t('restart')" :icon="IconRefresh" @press="restartCapture" />
          <UI.WpButton :label="t('forget')" @press="forgetPermission" />
        </UI.WpStack>
        <UI.WpDivider spaced />
        <UI.WpSwitch v-model="testImage" :label="t('testImage')" />
      </UI.WpCard>

      <UI.WpCard :title="t('preview')" :subtitle="t('leds')">
        <LedStrip :telemetry="telemetry" />
        <div class="devices">
          <div v-for="device in state.config?.devices ?? []" :key="device.id" class="device">
            <UI.WpIcon :icon="IconWarning" size="xs" v-if="deviceState(device.id) === 'error'" class="device__warn" />
            <span class="device__name">{{ device.name }}</span>
            <UI.WpText variant="mono" tone="faint">{{ device.led_count }}</UI.WpText>
            <UI.WpBadge :tone="tone(deviceState(device.id))" :label="t(deviceState(device.id) as 'ok')" />
          </div>
          <UI.WpEmptyState v-if="(state.config?.devices.length ?? 0) === 0" kind="leer"
                           :title="t('noDevices')" :description="t('noDevicesHint')" />
        </div>
      </UI.WpCard>
    </div>
  </div>
</template>

<style scoped>
.view { display: flex; flex-direction: column; gap: var(--wp-space-4); }
.cards { display: grid; grid-template-columns: 1.1fr 1fr; gap: var(--wp-space-4); align-items: start; }
.metrics { display: grid; grid-template-columns: repeat(auto-fit, minmax(120px, 1fr)); gap: var(--wp-space-3); }
.metric { display: flex; flex-direction: column; gap: 2px; }
.metric__k { font-size: var(--wp-font-size-xs); color: var(--wp-color-text-faint); }
.metric__v { font-family: var(--wp-font-family-mono); font-size: var(--wp-font-size-lg); font-variant-numeric: tabular-nums; }
.devices { display: flex; flex-direction: column; gap: var(--wp-space-1); margin-top: var(--wp-space-3); }
.device { display: grid; grid-template-columns: auto 1fr auto auto; align-items: center; gap: var(--wp-space-2); padding: 4px 0; border-bottom: var(--wp-size-border) solid var(--wp-color-border-subtle); }
.device__name { font-size: var(--wp-font-size-sm); }
.device__warn { color: var(--wp-color-status-warn); }
@media (max-width: 980px) { .cards { grid-template-columns: 1fr; } }
</style>
