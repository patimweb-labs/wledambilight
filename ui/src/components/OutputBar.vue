<script setup lang="ts">
// Untere Leiste wie in der alten App: je Gerät ein Streifen mit den Farben, die
// gerade rausgehen, rechts die Messwerte der Pipeline.
import { computed } from "vue";
import { state, t } from "../store";
import LedStrip from "./LedStrip.vue";

const stats = computed(() => state.telemetry);
const devices = computed(() => state.config?.devices ?? []);
const strips = computed(() => stats.value?.device_leds ?? []);
</script>

<template>
  <footer class="out">
    <div class="out__head">
      <span class="out__title">{{ t("output") }}</span>
      <span class="out__spacer" />
      <span class="out__stat">
        capture {{ stats?.dims?.[0] ?? 0 }}×{{ stats?.dims?.[1] ?? 0 }}
        · in {{ (stats?.fps_in ?? 0).toFixed(0) }} fps
        · out {{ (stats?.fps_out ?? 0).toFixed(0) }} fps
        · sampling {{ stats?.sample_us ?? 0 }} µs
      </span>
    </div>
    <div class="out__rows">
      <div v-for="(device, i) in devices" :key="device.id" class="out__row">
        <span class="out__name" :title="device.host">{{ device.name }}</span>
        <LedStrip :colors="strips[i] ?? []" :label="device.name" />
      </div>
    </div>
  </footer>
</template>

<style scoped>
.out {
  flex: none; display: flex; flex-direction: column; gap: var(--wp-space-1);
  padding: var(--wp-space-2) var(--wp-space-3) var(--wp-space-3);
  background: var(--wp-color-bg-panel);
  border-top: var(--wp-size-border) solid var(--wp-color-border-subtle);
}
.out__head { display: flex; align-items: baseline; gap: var(--wp-space-2); }
.out__title { font-family: var(--wp-font-family-display); font-size: var(--wp-font-size-sm); font-weight: var(--wp-font-weight-semibold); }
.out__spacer { flex: 1; }
.out__stat { font-family: var(--wp-font-family-mono); font-size: var(--wp-font-size-xs); color: var(--wp-color-text-faint); }
.out__rows { display: flex; flex-direction: column; gap: 3px; }
.out__row { display: grid; grid-template-columns: 120px 1fr; align-items: center; gap: var(--wp-space-2); }
.out__name { font-size: var(--wp-font-size-xs); color: var(--wp-color-text-muted); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
</style>
