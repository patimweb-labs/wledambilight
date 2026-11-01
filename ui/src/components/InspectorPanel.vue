<script setup lang="ts">
// Rechte Spalte mit den drei Reitern der alten Oberfläche: Zonen, Bild, App.
// Der Aufbau ist bewusst derselbe geblieben — nur die Gestaltung ist neu.
import { computed, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import * as UI from "@weppelmann/components";
import {
  IconArrowDown, IconArrowUp, IconPlus, IconRefresh, IconTrash,
} from "@weppelmann/icons";
import type { ThemeName, Zone } from "../api";
import { commit, forgetPermission, removeZone, restartCapture, state, t, zoneRanges } from "../store";

const props = defineProps<{ active: number }>();
const emit = defineEmits<{ "update:active": [index: number] }>();

type Tab = "zones" | "picture" | "app";
const tab = ref<Tab>("zones");

const tabs = computed<UI.Tab[]>(() => [
  { id: "zones", label: t("zones") },
  { id: "picture", label: t("picture") },
  { id: "app", label: t("appSettings") },
]);

const settings = computed(() => state.config?.settings);
const devices = computed(() => state.config?.devices ?? []);
const zones = computed(() => state.config?.zones ?? []);
const ranges = computed(() => zoneRanges());
const colors = computed(() => state.telemetry?.zone_colors ?? []);
const zone = computed(() => zones.value[props.active]);

const deviceOptions = computed(() => devices.value.map((d) => ({ value: d.id, label: d.name, hint: d.host })));
const themeOptions = [
  { value: "dark", label: t("dark") },
  { value: "light", label: t("light") },
  { value: "mono", label: t("mono") },
];
const languageOptions = [
  { value: "auto", label: t("system") },
  { value: "de", label: "Deutsch" },
  { value: "en", label: "English" },
];
const widthOptions = [{ value: "0", label: t("native") }, ...[160, 240, 320, 480, 640].map((w) => ({ value: String(w), label: `${w} px` }))];

const theme = computed<ThemeName>({
  get: () => {
    const value = settings.value?.theme;
    return value === "light" || value === "mono" ? value : "dark";
  },
  set: (value) => { if (state.config) { state.config.settings.theme = value; commit(); } },
});
const language = computed<string>({
  get: () => settings.value?.language ?? "auto",
  set: (value) => { if (state.config) { state.config.settings.language = value; commit(); } },
});

function numberValue(raw: string, apply: (n: number) => void) {
  const value = Number(raw.replace(",", "."));
  if (Number.isFinite(value)) { apply(value); commit(); }
}

function num(key: "target_fps" | "smoothing_ms" | "spatial" | "slew" | "gamma" | "saturation" | "brightness", value: number) {
  if (!state.config) return;
  state.config.settings[key] = value;
  commit();
}
function flag(key: "test_image" | "snap" | "pause_when_focused", value: boolean) {
  if (!state.config) return;
  state.config.settings[key] = value;
  commit();
}
function setWidth(raw: string) {
  if (!state.config) return;
  state.config.settings.capture_width = Number(raw);
  commit();
}
function setMonitor(raw: string) {
  if (!state.config) return;
  state.config.settings.monitor = Math.max(0, Math.round(Number(raw) || 0));
  commit();
}

async function addZone(preset: string) {
  const device = devices.value[0];
  if (!device || !state.config) return;
  const perZone = Math.max(1, Math.round(device.led_count / (preset === "full" ? 1 : 4)));
  const created = await invoke<Zone>("zone_preset", { device: device.id, count: perZone, preset });
  state.config.zones.push(created);
  emit("update:active", state.config.zones.length - 1);
  commit();
}

async function splitEdges() {
  const device = devices.value[0];
  if (!device || !state.config) return;
  const per = Math.max(1, Math.floor(device.led_count / 4));
  for (const preset of ["top", "right", "bottom", "left"]) {
    state.config.zones.push(await invoke<Zone>("zone_preset", { device: device.id, count: per, preset }));
  }
  emit("update:active", state.config.zones.length - 4);
  commit();
}

async function devicePerEdge() {
  if (!state.config) return;
  let made = 0;
  for (const [index, preset] of ["top", "right", "bottom", "left"].entries()) {
    const device = devices.value[index];
    if (!device) break;
    state.config.zones.push(await invoke<Zone>("zone_preset", { device: device.id, count: device.led_count, preset }));
    made++;
  }
  if (made) emit("update:active", state.config.zones.length - made);
  commit();
}

function move(index: number, step: number) {
  if (!state.config) return;
  const target = index + step;
  if (target < 0 || target >= state.config.zones.length) return;
  const [zone] = state.config.zones.splice(index, 1);
  state.config.zones.splice(target, 0, zone);
  emit("update:active", target);
  commit();
}

function drop(index: number) {
  removeZone(index);
  emit("update:active", Math.max(0, Math.min(index, zones.value.length - 1)));
}

function resetPicture() {
  if (!state.config) return;
  const defaults = { smoothing_ms: 180, spatial: 0.6, slew: 900, gamma: 1, saturation: 1, brightness: 1 };
  Object.assign(state.config.settings, defaults);
  commit();
}
</script>

<template>
  <aside class="inspector">
    <UI.WpTabs v-model="tab" :tabs="tabs" class="inspector__tabs" />
    <div class="inspector__body">
      <!-- ------------------------------------------------------------ Zonen -->
      <template v-if="tab === 'zones'">
        <p class="hint">{{ t("orderHint") }}</p>

        <div class="zonelist">
          <button v-for="(z, i) in zones" :key="i" type="button" class="zonerow"
                  :class="{ 'is-active': i === props.active }" @click="emit('update:active', i)">
            <span class="zonerow__dot" :style="{ background: `rgb(${colors[i]?.join(' ') ?? '60 60 60'})` }" />
            <span class="zonerow__main">
              <span class="zonerow__name">{{ z.name }}</span>
              <span class="zonerow__meta">{{ ranges[i]?.label }} · {{ devices.find((d) => d.id === z.device)?.name ?? z.device }}</span>
            </span>
            <span class="zonerow__actions">
              <button type="button" class="mini" :aria-label="t('up')" @click.stop="move(i, -1)"><UI.WpIcon :icon="IconArrowUp" size="xs" /></button>
              <button type="button" class="mini" :aria-label="t('down')" @click.stop="move(i, 1)"><UI.WpIcon :icon="IconArrowDown" size="xs" /></button>
              <button type="button" class="mini" :aria-label="t('remove')" @click.stop="drop(i)"><UI.WpIcon :icon="IconTrash" size="xs" /></button>
            </span>
          </button>

          <UI.WpEmptyState v-if="zones.length === 0" kind="leer" :title="t('noZone')" :description="t('noZoneHint')" />
        </div>

        <div class="row">
          <UI.WpButton :label="t('zone')" :icon="IconPlus" size="sm" :state="devices.length ? 'default' : 'disabled'" @press="addZone('custom')" />
          <UI.WpButton :label="t('splitEdges')" size="sm" :state="devices.length ? 'default' : 'disabled'" @press="splitEdges" />
          <UI.WpButton :label="t('devicePerEdge')" size="sm" :state="devices.length > 1 ? 'default' : 'disabled'" @press="devicePerEdge" />
        </div>

        <template v-if="zone">
          <UI.WpDivider spaced />
          <p class="section">{{ t("selectedZone") }}</p>

          <UI.WpFormField :label="t('name')" v-slot="{ id }">
            <UI.WpInput :id="id" :model-value="zone.name" @update:model-value="zone.name = $event; commit()" />
          </UI.WpFormField>
          <UI.WpSelect :model-value="zone.device" :options="deviceOptions" :label="t('device')"
                       @update:model-value="zone.device = String($event); commit()" />
          <UI.WpFormField :label="t('ledCount')" v-slot="{ id }">
            <UI.WpInput :id="id" type="number" :model-value="String(zone.count)"
                        @update:model-value="numberValue($event, (n) => (zone!.count = Math.max(1, Math.round(n))))" />
          </UI.WpFormField>
          <UI.WpFormField :label="t('ledBlock')" :hint="t('auto') + ' = ' + ranges[props.active]?.label" v-slot="{ id }">
            <UI.WpInput :id="id" type="number" :model-value="zone.offset === null ? '' : String(zone.offset)"
                        @update:model-value="zone.offset = $event.trim() === '' ? null : Number($event); commit()" />
          </UI.WpFormField>
          <UI.WpSwitch :model-value="!zone.vertical" :label="t('towardsLed0')"
                       @update:model-value="zone.vertical = !$event; commit()" />
          <UI.WpSwitch :model-value="zone.enabled" :label="t('active')"
                       @update:model-value="zone.enabled = $event; commit()" />
          <UI.WpSwitch :model-value="zone.vertical" :label="t('vertical')"
                       @update:model-value="zone.vertical = $event; commit()" />

          <p class="sub">{{ t("sizeInCapture") }}</p>
          <UI.WpSlider :model-value="Math.round(zone.w * 100)" :min="2" :max="100" :label="t('width')" suffix=" %"
                       @update:model-value="zone.w = $event / 100; commit()" />
          <UI.WpSlider :model-value="Math.round(zone.h * 100)" :min="2" :max="100" :label="t('height')" suffix=" %"
                       @update:model-value="zone.h = $event / 100; commit()" />
          <UI.WpSlider :model-value="Math.round(zone.gain * 100)" :min="20" :max="200" :label="t('gain')" suffix=" %"
                       @update:model-value="zone.gain = $event / 100; commit()" />
          <div class="row">
            <UI.WpButton :label="t('fullWidth')" size="sm" @press="zone.w = 1; zone.x = 0; commit()" />
            <UI.WpButton :label="t('fullHeight')" size="sm" @press="zone.h = 1; zone.y = 0; commit()" />
            <UI.WpButton :label="t('toEdge')" size="sm" @press="zone.x = 0; zone.y = 0; zone.w = 1; zone.h = 0.1; commit()" />
          </div>
        </template>
      </template>

      <!-- ------------------------------------------------------------- Bild -->
      <template v-else-if="tab === 'picture' && settings">
        <UI.WpSlider :model-value="Math.round(settings.brightness * 100)" :min="0" :max="200" :label="t('brightness')" suffix=" %"
                     @update:model-value="num('brightness', $event / 100)" />
        <UI.WpSlider :model-value="Math.round(settings.saturation * 100)" :min="0" :max="200" :label="t('saturation')" suffix=" %"
                     @update:model-value="num('saturation', $event / 100)" />

        <p class="section">{{ t("motion") }}</p>
        <UI.WpSlider :model-value="settings.smoothing_ms" :min="0" :max="600" :step="10" :label="t('smoothing')"
                     @update:model-value="num('smoothing_ms', $event)" />
        <UI.WpSlider :model-value="Math.round(settings.spatial * 100)" :min="0" :max="100" :label="t('spatial')" suffix=" %"
                     @update:model-value="num('spatial', $event / 100)" />
        <UI.WpSlider :model-value="settings.slew" :min="0" :max="2000" :step="50" :label="t('slew')"
                     @update:model-value="num('slew', $event)" />
        <UI.WpSlider :model-value="settings.target_fps" :min="10" :max="120" :step="5" :label="t('frameRate')"
                     @update:model-value="num('target_fps', $event)" />
        <UI.WpButton :label="t('resetDefaults')" :icon="IconRefresh" size="sm" @press="resetPicture" />
      </template>

      <!-- -------------------------------------------------------------- App -->
      <template v-else-if="tab === 'app' && settings">
        <p class="section">{{ t("capture") }}</p>
        <UI.WpSwitch :model-value="settings.test_image" :label="t('testPicture')"
                     @update:model-value="flag('test_image', $event)" />
        <UI.WpFormField :label="t('display')" v-slot="{ id }">
          <UI.WpInput :id="id" type="number" :model-value="String(settings.monitor)"
                      @update:model-value="setMonitor($event)" />
        </UI.WpFormField>
        <UI.WpSelect :model-value="String(settings.capture_width)" :options="widthOptions" :label="t('resolution')"
                     @update:model-value="setWidth(String($event))" />
        <div class="row">
          <UI.WpButton :label="t('chooseDisplay')" :icon="IconRefresh" size="sm" @press="restartCapture" />
          <UI.WpButton :label="t('forget')" size="sm" @press="forgetPermission" />
        </div>

        <p class="section">{{ t("editing") }}</p>
        <UI.WpSwitch :model-value="settings.snap" :label="t('snapZones')"
                     @update:model-value="flag('snap', $event)" />
        <UI.WpSwitch :model-value="settings.pause_when_focused" :label="t('pauseWhenFocused')"
                     @update:model-value="flag('pause_when_focused', $event)" />

        <p class="section">{{ t("design") }}</p>
        <UI.WpSelect :model-value="language" :options="languageOptions" :label="t('language')"
                     @update:model-value="language = String($event)" />
        <UI.WpRadioGroup :model-value="theme" name="theme" :label="t('theme')" :options="themeOptions"
                         @update:model-value="theme = $event as ThemeName" />

        <p class="section">{{ t("about") }}</p>
        <p class="hint">WLED Ambilight · GPL-3.0-or-later · Patrick Weppelmann</p>
      </template>
    </div>
  </aside>
</template>

<style scoped>
.inspector {
  width: 320px; flex: none; display: flex; flex-direction: column; min-height: 0;
  background: var(--wp-color-bg-panel);
  border-left: var(--wp-size-border) solid var(--wp-color-border-subtle);
}
.inspector__tabs { padding: 0 var(--wp-space-2); }
.inspector__tabs :deep(.wp-tabs__list) { border-bottom: 0; }
.inspector__tabs :deep(.wp-tabs__panel) { display: none; }
.inspector__body { flex: 1; overflow-y: auto; padding: var(--wp-space-3); display: flex; flex-direction: column; gap: var(--wp-space-2); }
.hint { margin: 0; font-size: var(--wp-font-size-xs); color: var(--wp-color-text-faint); }
.section { margin: var(--wp-space-2) 0 0; font-family: var(--wp-font-family-display); font-size: var(--wp-font-size-sm); font-weight: var(--wp-font-weight-semibold); }
.sub { margin: var(--wp-space-2) 0 0; font-size: var(--wp-font-size-xs); color: var(--wp-color-text-muted); }
.zonelist { display: flex; flex-direction: column; gap: 2px; }
.zonerow {
  display: grid; grid-template-columns: 8px 1fr auto; align-items: center; gap: var(--wp-space-2);
  text-align: left; background: var(--wp-color-bg-raised); border: var(--wp-size-border) solid var(--wp-color-border-subtle);
  color: var(--wp-color-text-default); font: inherit; padding: 5px var(--wp-space-2); cursor: pointer;
}
.zonerow.is-active { border-color: var(--wp-color-accent-default); }
.zonerow__dot { width: 8px; height: 8px; border: var(--wp-size-border) solid var(--wp-color-border-strong); }
.zonerow__main { display: flex; flex-direction: column; gap: 1px; min-width: 0; }
.zonerow__name { font-size: var(--wp-font-size-sm); }
.zonerow__meta { font-family: var(--wp-font-family-mono); font-size: 10px; color: var(--wp-color-text-faint); }
.zonerow__actions { display: flex; gap: 1px; }
.mini { background: none; border: 0; color: var(--wp-color-text-faint); cursor: pointer; padding: 2px; display: inline-flex; }
.mini:hover { color: var(--wp-color-text-default); }
.row { display: flex; gap: var(--wp-space-1); flex-wrap: wrap; margin-top: var(--wp-space-1); }
</style>
