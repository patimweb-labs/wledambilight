<script setup lang="ts">
// Zonen: Liste links, Editor rechts. Der Editor schreibt in dieselben Objekte,
// die gespeichert werden — kein zweiter Entwurf, kein „Übernehmen“-Schritt.
import { computed, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import * as UI from "@weppelmann/components";
import { IconTrash } from "@weppelmann/icons";
import type { Zone } from "../api";
import { commit, removeZone, state, t } from "../store";
import ZoneEditor from "../components/ZoneEditor.vue";

const active = ref(0);

const zones = computed(() => state.config?.zones ?? []);
const devices = computed(() => state.config?.devices ?? []);
const zone = computed(() => zones.value[active.value]);
const colors = computed(() => state.telemetry?.zone_colors ?? []);

const deviceOptions = computed(() =>
  devices.value.map((d) => ({ value: d.id, label: d.name, hint: d.host })),
);

const presets = [
  { id: "top", label: "Oben" },
  { id: "bottom", label: "Unten" },
  { id: "left", label: "Links" },
  { id: "right", label: "Rechts" },
  { id: "full", label: "Vollbild" },
  { id: "custom", label: "Frei" },
];

async function addPreset(preset: string) {
  const device = devices.value[0];
  if (!device || !state.config) return;
  const perZone = Math.max(1, Math.round(device.led_count / (preset === "full" ? 1 : 4)));
  const zone = await invoke<Zone>("zone_preset", { device: device.id, count: perZone, preset });
  state.config.zones.push(zone);
  active.value = state.config.zones.length - 1;
  commit();
}

/** Eine Leiste auf vier Ränder verteilen: dieselbe Anzahl LEDs je Rand. */
async function splitEdges() {
  const device = devices.value[0];
  if (!device || !state.config) return;
  const per = Math.max(1, Math.floor(device.led_count / 4));
  for (const preset of ["top", "right", "bottom", "left"]) {
    const zone = await invoke<Zone>("zone_preset", { device: device.id, count: per, preset });
    state.config.zones.push(zone);
  }
  active.value = state.config.zones.length - 4;
  commit();
}

function withDevice(id: string) {
  if (!zone.value) return;
  zone.value.device = id;
  commit();
}

function numberValue(value: string, apply: (n: number) => void) {
  const parsed = Number(value.replace(",", "."));
  if (Number.isFinite(parsed)) {
    apply(parsed);
    commit();
  }
}

function drop() {
  if (zones.value.length === 0) return;
  removeZone(active.value);
  active.value = Math.max(0, Math.min(active.value, zones.value.length - 1));
}
</script>

<template>
  <div class="zones">
    <div class="zones__list">
      <div class="presets">
        <UI.WpText variant="caption" tone="faint">{{ t("newZone") }}</UI.WpText>
        <div class="presets__row">
          <UI.WpButton v-for="p in presets" :key="p.id" :label="p.label" size="sm"
                       :state="devices.length ? 'default' : 'disabled'" @press="addPreset(p.id)" />
        </div>
        <UI.WpButton label="Leiste auf 4 Ränder" size="sm" :state="devices.length ? 'default' : 'disabled'"
                     @press="splitEdges" />
      </div>
      <div class="list">
        <button v-for="(z, i) in zones" :key="i" type="button" class="list__item"
                :class="{ 'is-active': i === active, 'is-off': !z.enabled }" @click="active = i">
          <span class="list__name">{{ z.name }}</span>
          <span class="list__meta">{{ z.count }} LED · {{ Math.round(z.w * 100) }}×{{ Math.round(z.h * 100) }} %</span>
        </button>
        <UI.WpEmptyState v-if="zones.length === 0" kind="leer" :title="t('noZone')" :description="t('noZoneHint')" />
      </div>
      <UI.WpButton v-if="zone" :label="t('remove')" :icon="IconTrash" size="sm" variant="danger" @press="drop" />
    </div>

    <div class="zones__work">
      <UI.WpCard :title="t('preview')" :subtitle="t('zonesHint')">
        <ZoneEditor :zones="zones" :active="active" :snap="state.config?.settings.snap ?? true"
                    :colors="colors" @update:active="active = $event" @changed="commit" />
      </UI.WpCard>

      <UI.WpCard v-if="zone" :title="zone.name" :subtitle="t('device') + ': ' + zone.device">
        <div class="form">
          <UI.WpFormField :label="t('zoneName')" v-slot="{ id }">
            <UI.WpInput :id="id" :model-value="zone.name" @update:model-value="zone.name = $event; commit()" />
          </UI.WpFormField>
          <UI.WpSelect :model-value="zone.device" :options="deviceOptions" :label="t('device')"
                       @update:model-value="withDevice(String($event))" />
          <div class="pair">
            <UI.WpFormField :label="t('zoneCount')" v-slot="{ id }">
              <UI.WpInput :id="id" type="number" :model-value="String(zone.count)"
                          @update:model-value="numberValue($event, (n) => (zone!.count = Math.max(1, Math.round(n))))" />
            </UI.WpFormField>
            <UI.WpFormField :label="t('offset')" v-slot="{ id }">
              <UI.WpInput :id="id" type="number" :model-value="zone.offset === null ? '' : String(zone.offset)"
                          @update:model-value="zone.offset = $event.trim() === '' ? null : Number($event); commit()" />
            </UI.WpFormField>
          </div>
          <UI.WpSlider :model-value="zone.gain" :min="0.2" :max="2" :step="0.05" :label="t('gain')"
                       @update:model-value="zone.gain = $event; commit()" />
          <UI.WpSwitch :model-value="zone.vertical" :label="t('vertical')"
                       @update:model-value="zone.vertical = $event; commit()" />
          <UI.WpSwitch :model-value="zone.enabled" :label="t('enabled')"
                       @update:model-value="zone.enabled = $event; commit()" />
        </div>
      </UI.WpCard>
    </div>
  </div>
</template>

<style scoped>
.zones { display: grid; grid-template-columns: 260px 1fr; gap: var(--wp-space-4); align-items: start; }
.zones__list { display: flex; flex-direction: column; gap: var(--wp-space-2); }
.zones__work { display: flex; flex-direction: column; gap: var(--wp-space-4); }
.presets { display: flex; flex-direction: column; gap: 4px; }
.presets__row { display: flex; flex-wrap: wrap; gap: 4px; }
.list { display: flex; flex-direction: column; gap: 2px; }
.list__item {
  display: flex; flex-direction: column; gap: 1px; text-align: left;
  background: var(--wp-color-bg-panel); border: var(--wp-size-border) solid var(--wp-color-border-subtle);
  color: var(--wp-color-text-default); font: inherit; padding: 6px var(--wp-space-2); cursor: pointer;
}
.list__item.is-active { border-color: var(--wp-color-accent-default); }
.list__item.is-off { opacity: 0.5; }
.list__name { font-size: var(--wp-font-size-sm); }
.list__meta { font-family: var(--wp-font-family-mono); font-size: var(--wp-font-size-xs); color: var(--wp-color-text-faint); }
.form { display: flex; flex-direction: column; gap: var(--wp-space-3); }
.pair { display: grid; grid-template-columns: 1fr 1fr; gap: var(--wp-space-3); }
@media (max-width: 980px) { .zones { grid-template-columns: 1fr; } }
</style>
