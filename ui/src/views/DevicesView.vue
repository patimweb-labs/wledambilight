<script setup lang="ts">
// Geräte: vorhandene pflegen, neue im Netzwerk finden.
import { computed, ref } from "vue";
import * as UI from "@weppelmann/components";
import { IconPlus, IconSearch, IconTrash } from "@weppelmann/icons";
import type { DeviceCfg, DeviceKind, Protocol } from "../api";
import { DEVICE_KINDS } from "../api";
import { addFound, commit, removeDevice, scanNetwork, state, t } from "../store";

const editing = ref<DeviceCfg | null>(null);
const dialogOpen = ref(false);
const confirmRemove = ref<string | null>(null);
const scanning = computed(() => state.scanning);
const found = computed(() => state.found);

const kindOptions = DEVICE_KINDS.map((k) => ({ value: k.value, label: k.key.toUpperCase() }));
const protocolOptions = [
  { value: "Ddp", label: "DDP (schnell)" },
  { value: "Drgb", label: "DRGB (WLED, 490 LEDs)" },
];

const status = (id: string) => state.telemetry?.devices.find((d) => d.id === id)?.state ?? "connecting";
const statusTone = (id: string) => {
  const s = status(id);
  return (s === "ok" ? "ok" : s === "error" ? "error" : "neutral") as "ok" | "error" | "neutral";
};

function edit(device: DeviceCfg) {
  editing.value = JSON.parse(JSON.stringify(device));
  dialogOpen.value = true;
}

function applyEdit() {
  if (!editing.value || !state.config) return;
  const index = state.config.devices.findIndex((d) => d.id === editing.value!.id);
  if (index >= 0) state.config.devices[index] = { ...editing.value };
  commit();
  dialogOpen.value = false;
}

function numberValue(value: string, apply: (n: number) => void) {
  const parsed = Number(value.replace(",", "."));
  if (Number.isFinite(parsed)) apply(parsed);
}

function addManually() {
  if (!state.config) return;
  const n = state.config.devices.length + 1;
  const device: DeviceCfg = {
    id: `manuell-${Date.now()}`,
    name: `Gerät ${n}`,
    host: "192.168.1.",
    led_count: 60,
    enabled: true,
    protocol: "Ddp",
    kind: "Wled",
    port: 0,
    target: "",
    token: null,
    brightness: 1,
    reverse: false,
    bgr: false,
    max_fps: 0,
  };
  state.config.devices.push(device);
  commit();
  edit(device);
}

function drop(id: string) {
  confirmRemove.value = id;
}
</script>

<template>
  <div class="devices">
    <UI.WpCard :title="t('found')" :subtitle="t('scan')">
      <template #actions>
        <UI.WpButton :label="scanning ? t('scanning') : t('scan')" :icon="IconSearch"
                     :state="scanning ? 'busy' : 'default'" size="sm" @press="scanNetwork" />
      </template>
      <div v-if="found.length" class="found">
        <div v-for="f in found" :key="f.id" class="found__row">
          <span class="found__name">{{ f.name }}</span>
          <span class="found__detail">{{ f.detail }}</span>
          <UI.WpText variant="mono" tone="faint">{{ f.leds }}</UI.WpText>
          <UI.WpButton :label="t('add')" :icon="IconPlus" size="sm" @press="addFound(f)" />
        </div>
      </div>
      <UI.WpEmptyState v-else kind="keine-treffer" :title="t('found')" :description="t('scan')" />
    </UI.WpCard>

    <UI.WpCard :title="t('devices')">
      <template #actions>
        <UI.WpButton :label="t('addDevice')" :icon="IconPlus" size="sm" @press="addManually" />
      </template>
      <div class="list">
        <div v-for="device in state.config?.devices ?? []" :key="device.id" class="row">
          <div class="row__main">
            <UI.WpText>{{ device.name }}</UI.WpText>
            <UI.WpText variant="caption" tone="muted">{{ device.kind }} · {{ device.host }} · {{ device.led_count }} LED</UI.WpText>
          </div>
          <UI.WpBadge :tone="statusTone(device.id)" :label="t(status(device.id) as 'ok')" />
          <UI.WpButton :label="t('edit')" size="sm" @press="edit(device)" />
          <UI.WpButton :label="t('remove')" :icon="IconTrash" size="sm" variant="danger" @press="drop(device.id)" />
        </div>
        <UI.WpEmptyState v-if="(state.config?.devices.length ?? 0) === 0" kind="leer"
                         :title="t('noDevices')" :description="t('noDevicesHint')" />
      </div>
    </UI.WpCard>

    <UI.WpDialog v-model:open="dialogOpen" :title="editing?.name ?? t('edit')" tone="info"
                 :confirm-label="t('save')" :cancel-label="t('close')" @confirm="applyEdit">
      <div v-if="editing" class="form">
        <div class="pair">
          <UI.WpFormField :label="t('name')" v-slot="{ id }">
            <UI.WpInput :id="id" :model-value="editing.name" @update:model-value="editing.name = $event" />
          </UI.WpFormField>
          <UI.WpFormField :label="t('host')" v-slot="{ id }">
            <UI.WpInput :id="id" :model-value="editing.host" @update:model-value="editing.host = $event" />
          </UI.WpFormField>
        </div>
        <div class="pair">
          <UI.WpSelect :model-value="editing.kind" :options="kindOptions" :label="t('kind')"
                       @update:model-value="editing.kind = $event as DeviceKind" />
          <UI.WpSelect v-if="editing.kind === 'Wled'" :model-value="editing.protocol" :options="protocolOptions"
                       :label="t('protocol')" @update:model-value="editing.protocol = $event as Protocol" />
        </div>
        <div class="pair">
          <UI.WpFormField :label="t('ledCount')" v-slot="{ id }">
            <UI.WpInput :id="id" type="number" :model-value="String(editing.led_count)"
                        @update:model-value="numberValue($event, (n) => (editing!.led_count = Math.max(1, Math.round(n))))" />
          </UI.WpFormField>
          <UI.WpFormField :label="t('maxFps')" v-slot="{ id }">
            <UI.WpInput :id="id" type="number" :model-value="String(editing.max_fps)"
                        @update:model-value="numberValue($event, (n) => (editing!.max_fps = Math.max(0, Math.round(n))))" />
          </UI.WpFormField>
        </div>
        <UI.WpSlider :model-value="Math.round(editing.brightness * 100)" :min="0" :max="100" :label="t('brightness')" suffix=" %"
                     @update:model-value="editing.brightness = $event / 100" />
        <UI.WpSwitch :model-value="editing.enabled" :label="t('enabled')" @update:model-value="editing.enabled = $event" />
        <UI.WpSwitch :model-value="editing.reverse" :label="t('reverse')" @update:model-value="editing.reverse = $event" />
        <UI.WpSwitch :model-value="editing.bgr" :label="t('bgr')" @update:model-value="editing.bgr = $event" />
      </div>
    </UI.WpDialog>

    <UI.WpDialog :open="confirmRemove !== null" :title="t('confirmRemove')" :description="t('confirmRemoveText')"
                 tone="warn" :confirm-label="t('remove')" :cancel-label="t('close')"
                 @update:open="confirmRemove = null"
                 @confirm="removeDevice(confirmRemove!); confirmRemove = null" />
  </div>
</template>

<style scoped>
.devices { display: flex; flex-direction: column; gap: var(--wp-space-4); }
.found { display: flex; flex-direction: column; gap: 2px; }
.found__row { display: grid; grid-template-columns: 1fr 1.4fr auto auto; align-items: center; gap: var(--wp-space-2); padding: 4px 0; border-bottom: var(--wp-size-border) solid var(--wp-color-border-subtle); }
.found__name { font-size: var(--wp-font-size-sm); }
.found__detail { font-size: var(--wp-font-size-xs); color: var(--wp-color-text-faint); }
.list { display: flex; flex-direction: column; }
.row { display: grid; grid-template-columns: 1fr auto auto auto; align-items: center; gap: var(--wp-space-2); padding: var(--wp-space-2) 0; border-bottom: var(--wp-size-border) solid var(--wp-color-border-subtle); }
.row__main { display: flex; flex-direction: column; gap: 1px; min-width: 0; }
.form { display: flex; flex-direction: column; gap: var(--wp-space-3); }
.pair { display: grid; grid-template-columns: 1fr 1fr; gap: var(--wp-space-3); }
</style>
