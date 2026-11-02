<script setup lang="ts">
// Untere Leiste wie in der alten App: je Gerät ein Streifen mit den Farben, die
// gerade rausgehen, rechts die Messwerte der Pipeline.
//
// Aufbau kommt aus der Bibliothek (WpTelemetryBar, WpColorStrip) — hier stehen
// nur die Geräte und die Zahlen dieser Anwendung.
import { computed } from "vue";
import * as UI from "@weppelmann/components";
import { state, t } from "../store";

const stats = computed(() => state.telemetry);
const devices = computed(() => state.config?.devices ?? []);
const strips = computed(() => stats.value?.device_leds ?? []);
const summary = computed(() => {
  const s = stats.value;
  return `capture ${s?.dims?.[0] ?? 0}×${s?.dims?.[1] ?? 0} · in ${(s?.fps_in ?? 0).toFixed(0)} fps`
    + ` · out ${(s?.fps_out ?? 0).toFixed(0)} fps · sampling ${s?.sample_us ?? 0} µs`;
});
</script>

<template>
  <UI.WpTelemetryBar :title="t('output')" :stats="summary">
    <div v-for="(device, i) in devices" :key="device.id" class="out__row">
      <span class="out__name" :title="device.host">{{ device.name }}</span>
      <UI.WpColorStrip :colors="strips[i] ?? []" :label="device.name" />
    </div>
  </UI.WpTelemetryBar>
</template>

<style scoped>
.out__row { display: grid; grid-template-columns: 120px 1fr; align-items: center; gap: var(--wp-space-2); }
.out__name { font-size: var(--wp-font-size-xs); color: var(--wp-color-text-muted); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
</style>
