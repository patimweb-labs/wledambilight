<script setup lang="ts">
// Gerät einrichten: dieselben Felder wie in der alten App, je nach Art.
// WLED kennt ein Protokoll, OpenRGB einen Controller, Nanoleaf einen Token,
// Govee ein Modell. Der Dialog entscheidet nichts über das Gerät — er füllt
// nur die Konfiguration, die der Kern speichert.
import { computed, ref, watch } from "vue";
import * as UI from "@weppelmann/components";
import { IconWifi } from "@weppelmann/icons";
import type { DeviceCfg, DeviceKind, FoundDevice, Protocol } from "../api";
import { DEVICE_KINDS } from "../api";
import { commit, listOpenRgb, pairNanoleaf, probeDevice, state, t } from "../store";

const props = defineProps<{ open: boolean; device: DeviceCfg | null }>();
const emit = defineEmits<{ "update:open": [value: boolean] }>();

const draft = ref<DeviceCfg | null>(null);
const controllers = ref<FoundDevice[]>([]);
const note = ref("");

watch(
  () => [props.open, props.device] as const,
  () => {
    draft.value = props.device ? (JSON.parse(JSON.stringify(props.device)) as DeviceCfg) : null;
    controllers.value = [];
    note.value = "";
  },
  { immediate: true },
);

const kindOptions = DEVICE_KINDS.map((k) => ({ value: k.value, label: k.key.toUpperCase() }));
const protocolOptions = [
  { value: "Ddp", label: "DDP (schnell)" },
  { value: "Drgb", label: "DRGB (WLED, 490 LEDs)" },
];
const controllerOptions = computed(() =>
  controllers.value.map((c) => ({ value: c.target, label: c.name, hint: c.detail })),
);

function num(value: string, apply: (n: number) => void) {
  const parsed = Number(value.replace(",", "."));
  if (Number.isFinite(parsed)) apply(parsed);
}

async function test() {
  if (!draft.value) return;
  const found = await probeDevice(draft.value.host);
  if (found) {
    draft.value.name = found.name;
    draft.value.led_count = found.leds;
    note.value = found.detail;
  }
}

async function pair() {
  if (!draft.value) return;
  const token = await pairNanoleaf(draft.value.host);
  if (token) {
    draft.value.token = token;
    note.value = "Token erhalten.";
  } else {
    note.value = "Kein Token — blinkt der Controller?";
  }
}

async function loadControllers() {
  if (!draft.value) return;
  controllers.value = await listOpenRgb(draft.value.host || "127.0.0.1", draft.value.port);
  note.value = controllers.value.length ? "" : "Kein Controller gefunden.";
}

function apply() {
  if (!draft.value || !state.config) return;
  const index = state.config.devices.findIndex((d) => d.id === draft.value!.id);
  if (index >= 0) state.config.devices[index] = { ...draft.value };
  else state.config.devices.push({ ...draft.value });
  commit();
  emit("update:open", false);
}
</script>

<template>
  <UI.WpDialog :open="props.open" :title="draft?.name ?? t('addDevice')" :confirm-label="t('save')"
               :cancel-label="t('close')" tone="info" @update:open="emit('update:open', $event)" @confirm="apply">
    <div v-if="draft" class="form">
      <div class="pair">
        <UI.WpFormField :label="t('name')" v-slot="{ id }">
          <UI.WpInput :id="id" :model-value="draft.name" @update:model-value="draft.name = $event" />
        </UI.WpFormField>
        <UI.WpSelect :model-value="draft.kind" :options="kindOptions" :label="t('kind')"
                     @update:model-value="draft.kind = $event as DeviceKind" />
      </div>

      <div class="pair">
        <UI.WpFormField :label="t('host')" v-slot="{ id }">
          <UI.WpInput :id="id" :model-value="draft.host" @update:model-value="draft.host = $event" />
        </UI.WpFormField>
        <UI.WpFormField :label="t('ledCount')" v-slot="{ id }">
          <UI.WpInput :id="id" type="number" :model-value="String(draft.led_count)"
                      @update:model-value="num($event, (n) => (draft!.led_count = Math.max(1, Math.round(n))))" />
        </UI.WpFormField>
      </div>

      <template v-if="draft.kind === 'Wled'">
        <UI.WpSelect :model-value="draft.protocol" :options="protocolOptions" :label="t('protocol')"
                     @update:model-value="draft.protocol = $event as Protocol" />
        <UI.WpButton :label="t('probe')" :icon="IconWifi" size="sm" @press="test" />
      </template>

      <template v-else-if="draft.kind === 'OpenRgb'">
        <div class="pair">
          <UI.WpFormField :label="t('port')" v-slot="{ id }">
            <UI.WpInput :id="id" type="number" :model-value="String(draft.port)"
                        @update:model-value="num($event, (n) => (draft!.port = Math.round(n)))" />
          </UI.WpFormField>
          <div class="inline">
            <UI.WpButton :label="t('loadControllers')" size="sm" @press="loadControllers" />
          </div>
        </div>
        <UI.WpSelect v-if="controllerOptions.length" :model-value="draft.target" :options="controllerOptions"
                     :label="t('controller')" @update:model-value="draft.target = String($event)" />
        <UI.WpFormField v-else :label="t('controller')" hint="0 oder leer = erster Controller" v-slot="{ id }">
          <UI.WpInput :id="id" :model-value="draft.target" @update:model-value="draft.target = $event" />
        </UI.WpFormField>
      </template>

      <template v-else-if="draft.kind === 'Nanoleaf'">
        <UI.WpFormField :label="t('token')" hint="Einmal paaren, dann bleibt der Token." v-slot="{ id }">
          <UI.WpInput :id="id" :model-value="draft.token ?? ''" @update:model-value="draft.token = $event || null" />
        </UI.WpFormField>
        <UI.WpButton :label="t('pair')" size="sm" @press="pair" />
      </template>

      <UI.WpFormField v-else-if="draft.kind === 'Govee'" :label="t('model')" hint="SKU, z. B. H6199" v-slot="{ id }">
        <UI.WpInput :id="id" :model-value="draft.target" @update:model-value="draft.target = $event" />
      </UI.WpFormField>

      <UI.WpSlider :model-value="Math.round(draft.brightness * 100)" :min="0" :max="100" :label="t('brightness')" suffix=" %"
                   @update:model-value="draft.brightness = $event / 100" />

      <UI.WpSwitch :model-value="draft.enabled" :label="t('enabled')" @update:model-value="draft.enabled = $event" />
      <UI.WpSwitch :model-value="draft.reverse" :label="t('reverse')" @update:model-value="draft.reverse = $event" />
      <UI.WpSwitch :model-value="draft.bgr" :label="t('bgr')" @update:model-value="draft.bgr = $event" />

      <p v-if="note" class="note">{{ note }}</p>
    </div>
  </UI.WpDialog>
</template>

<style scoped>
.form { display: flex; flex-direction: column; gap: var(--wp-space-2); }
.pair { display: grid; grid-template-columns: 1fr 1fr; gap: var(--wp-space-2); }
.inline { display: flex; align-items: flex-end; }
.note { margin: 0; font-size: var(--wp-font-size-xs); color: var(--wp-color-text-muted); }
</style>
