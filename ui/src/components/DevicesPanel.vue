<script setup lang="ts">
// Geräte-Spalte: eine Karte je Gerät, alles direkt bedienbar, dazu die
// Netzwerksuche wie in der alten Oberfläche. Die Reihenfolge folgt der
// Konfiguration, weil sie auch die Reihenfolge der Ausgabe ist.
import { computed, ref } from "vue";
import * as UI from "@weppelmann/components";
import { IconPlus, IconSearch, IconSliders, IconTrash } from "@weppelmann/icons";
import type { DeviceCfg } from "../api";
import { addFound, commit, removeDevice, scanNetwork, state, t, zoneRanges } from "../store";
import DeviceDialog from "./DeviceDialog.vue";

const host = ref("");
const dialogOpen = ref(false);
const editing = ref<DeviceCfg | null>(null);

const devices = computed(() => state.config?.devices ?? []);
const ranges = computed(() => zoneRanges());
const found = computed(() => state.found);
const scanning = computed(() => state.scanning);

const used = (id: string) => ranges.value
  .filter((_, i) => state.config?.zones[i]?.device === id)
  .reduce((sum, r) => sum + r.count, 0);

const status = (id: string) => state.telemetry?.devices.find((d) => d.id === id)?.state ?? "connecting";
const tone = (id: string) => {
  const s = status(id);
  return (s === "ok" ? "ok" : s === "error" ? "error" : "neutral") as "ok" | "error" | "neutral";
};

const fpsOptions = [
  { value: "0", label: t("global") },
  { value: "24", label: "24" },
  { value: "30", label: "30" },
  { value: "60", label: "60" },
  { value: "120", label: "120" },
];

function set<K extends keyof DeviceCfg>(device: DeviceCfg, key: K, value: DeviceCfg[K]) {
  device[key] = value;
  commit();
}

function openDialog(device: DeviceCfg | null) {
  editing.value = device;
  dialogOpen.value = true;
}

function createManual() {
  const address = host.value.trim();
  if (!state.config || !address) return;
  const device: DeviceCfg = {
    id: address, name: address, host: address, led_count: 60, enabled: true,
    protocol: "Ddp", kind: "Wled", port: 0, target: "", token: null,
    brightness: 1, reverse: false, bgr: false, max_fps: 0,
  };
  state.config.devices.push(device);
  host.value = "";
  commit();
  openDialog(device);
}
</script>

<template>
  <UI.WpSidePanel :title="t('devices')" :count="devices.length">
    <template #actions>
      <button type="button" class="icon" :aria-label="t('scan')" :title="t('scan')" @click="scanNetwork">
        <UI.WpIcon :icon="IconSearch" size="sm" />
      </button>
      <button type="button" class="icon" :aria-label="t('addDevice')" :title="t('addDevice')" @click="openDialog(null)">
        <UI.WpIcon :icon="IconPlus" size="sm" />
      </button>
    </template>

    <div v-if="scanning" class="scan"><UI.WpSpinner size="sm" /> <span>{{ t("scanning") }}</span></div>
    <div v-else-if="found.length" class="scan scan--list">
      <p class="scan__title">{{ t("found") }}</p>
      <div v-for="f in found" :key="f.id" class="scan__row">
        <span class="scan__name">{{ f.name }}</span>
        <span class="scan__detail">{{ f.detail }}</span>
        <button type="button" class="icon" :aria-label="t('add')" @click="addFound(f)">
          <UI.WpIcon :icon="IconPlus" size="sm" />
        </button>
      </div>
    </div>

    <div class="devices__list">
      <div v-for="device in devices" :key="device.id" class="card">
        <div class="card__head">
          <button type="button" class="card__name" @click="openDialog(device)">{{ device.name }}</button>
          <UI.WpSwitch :model-value="device.enabled" @update:model-value="set(device, 'enabled', $event)" />
        </div>

        <div class="card__row2">
          <span class="card__host">{{ device.host }}</span>
          <UI.WpBadge :tone="tone(device.id)" :label="t(status(device.id) as 'ok')" />
        </div>

        <div class="card__usage">{{ used(device.id) }} / {{ device.led_count }} {{ t("ledsUsed") }}</div>
        <div class="bar"><span :style="{ width: Math.min(100, (used(device.id) / Math.max(1, device.led_count)) * 100) + '%' }" /></div>

        <div v-if="device.kind === 'Wled'" class="segmented">
          <button type="button" :class="{ 'is-on': device.protocol === 'Ddp' }" @click="set(device, 'protocol', 'Ddp')">DDP 4048</button>
          <button type="button" :class="{ 'is-on': device.protocol === 'Drgb' }" @click="set(device, 'protocol', 'Drgb')">DRGB 21324</button>
        </div>

        <UI.WpSlider :model-value="Math.round(device.brightness * 100)" :min="0" :max="100" :label="t('brightness')" suffix=" %"
                     @update:model-value="set(device, 'brightness', $event / 100)" />
        <UI.WpSelect :model-value="String(device.max_fps)" :options="fpsOptions" :label="t('maxFrameRate')"
                     @update:model-value="set(device, 'max_fps', Number($event))" />
        <UI.WpSwitch :model-value="device.reverse" :label="t('reverse')" @update:model-value="set(device, 'reverse', $event)" />
        <UI.WpSwitch :model-value="device.bgr" :label="t('bgr')" @update:model-value="set(device, 'bgr', $event)" />

        <div class="card__actions">
          <UI.WpButton :label="t('ledsOff')" size="sm" @press="set(device, 'enabled', false)" />
          <UI.WpButton :label="t('edit')" :icon="IconSliders" size="sm" @press="openDialog(device)" />
          <UI.WpButton :label="t('remove')" :icon="IconTrash" size="sm" variant="danger" @press="removeDevice(device.id)" />
        </div>
      </div>

      <UI.WpEmptyState v-if="devices.length === 0" kind="leer" :title="t('noDevices')" :description="t('noDevicesHint')">
        <template #action>
          <UI.WpButton :label="t('scan')" :icon="IconSearch" size="sm" variant="primary" @press="scanNetwork" />
        </template>
      </UI.WpEmptyState>
    </div>

    <div class="devices__add">
      <span class="devices__addlabel">{{ t("addByAddress") }}</span>
      <div class="devices__addrow">
        <UI.WpInput v-model="host" placeholder="192.168.1.42" @keyup.enter="createManual" />
        <UI.WpButton :icon="IconPlus" label="" size="sm" :state="host.trim() ? 'default' : 'disabled'" @press="createManual" />
      </div>
    </div>

  </UI.WpSidePanel>
  <DeviceDialog v-model:open="dialogOpen" :device="editing" />
</template>

<style scoped>
.devices {
  width: 284px; flex: none; display: flex; flex-direction: column;
  background: var(--wp-color-bg-panel);
  border-right: var(--wp-size-border) solid var(--wp-color-border-subtle);
  min-height: 0;
}
.icon { background: none; border: 0; color: var(--wp-color-text-muted); cursor: pointer; padding: 3px; display: inline-flex; }
.icon:hover { color: var(--wp-color-text-default); }
.scan { display: flex; align-items: center; gap: var(--wp-space-2); padding: 0 var(--wp-space-3) var(--wp-space-2); font-size: var(--wp-font-size-xs); color: var(--wp-color-text-muted); }
.scan--list { flex-direction: column; align-items: stretch; gap: 2px; }
.scan__title { margin: 0; font-size: var(--wp-font-size-xs); color: var(--wp-color-text-faint); }
.scan__row { display: grid; grid-template-columns: 1fr auto auto; gap: var(--wp-space-2); align-items: center; padding: 3px 0; border-bottom: var(--wp-size-border) solid var(--wp-color-border-subtle); }
.scan__name { font-size: var(--wp-font-size-sm); }
.scan__detail { font-size: 10px; color: var(--wp-color-text-faint); }
.devices__list { flex: 1; overflow-y: auto; padding: 0 var(--wp-space-2) var(--wp-space-2); display: flex; flex-direction: column; gap: var(--wp-space-2); }
.card { display: flex; flex-direction: column; gap: 6px; padding: var(--wp-space-2); background: var(--wp-color-bg-raised); border: var(--wp-size-border) solid var(--wp-color-border-subtle); }
.card__head { display: flex; align-items: center; justify-content: space-between; gap: var(--wp-space-2); }
.card__name { background: none; border: 0; padding: 0; color: var(--wp-color-text-default); font: inherit; font-size: var(--wp-font-size-sm); font-weight: var(--wp-font-weight-medium); cursor: pointer; text-align: left; }
.card__name:hover { color: var(--wp-color-accent-default); }
.card__row2 { display: flex; align-items: center; justify-content: space-between; gap: var(--wp-space-2); }
.card__host { font-family: var(--wp-font-family-mono); font-size: var(--wp-font-size-xs); color: var(--wp-color-text-faint); }
.card__usage { font-size: var(--wp-font-size-xs); color: var(--wp-color-text-muted); }
.card__actions { display: flex; gap: var(--wp-space-1); flex-wrap: wrap; margin-top: 2px; }
.bar { height: 3px; background: var(--wp-color-bg-input); }
.bar span { display: block; height: 100%; background: var(--wp-color-accent-default); }
.segmented { display: flex; gap: 2px; }
.segmented button { flex: 1; background: var(--wp-color-bg-input); border: var(--wp-size-border) solid var(--wp-color-border-subtle); color: var(--wp-color-text-muted); font: inherit; font-size: var(--wp-font-size-xs); padding: 4px; cursor: pointer; }
.segmented button.is-on { border-color: var(--wp-color-accent-default); color: var(--wp-color-text-default); }
.devices__add { padding: var(--wp-space-2) var(--wp-space-3) var(--wp-space-3); border-top: var(--wp-size-border) solid var(--wp-color-border-subtle); display: flex; flex-direction: column; gap: 4px; }
.devices__addlabel { font-size: var(--wp-font-size-xs); color: var(--wp-color-text-faint); }
.devices__addrow { display: flex; gap: var(--wp-space-1); }
</style>
