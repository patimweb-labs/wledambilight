<script setup lang="ts">
// Mitte: der Bildschirm mit den Zonen. Diese Fläche ist das Herz der App —
// hier wird gezogen, geskaliert und gesehen, was ankommt.
import { computed } from "vue";
import { commit, state, t, zoneRanges } from "../store";
import ZoneEditor from "./ZoneEditor.vue";

const props = defineProps<{ active: number }>();
const emit = defineEmits<{ "update:active": [index: number] }>();

const zones = computed(() => state.config?.zones ?? []);
const ranges = computed(() => zoneRanges());
const colors = computed(() => state.telemetry?.zone_colors ?? []);
const screen = computed(() => state.telemetry?.screen ?? [0, 0]);
const dims = computed(() => state.telemetry?.dims ?? [0, 0]);
const camera = computed(() => state.telemetry?.capture ?? "idle");
</script>

<template>
  <section class="preview">
    <div class="preview__canvas">
      <ZoneEditor :zones="zones" :active="props.active" :snap="state.config?.settings.snap ?? true"
                  :colors="colors" :ranges="ranges"
                  @update:active="emit('update:active', $event)" @changed="commit" />
    </div>
    <div class="preview__foot">
      <span class="preview__stat">{{ t("screen") }} {{ screen[0] }}×{{ screen[1] }}</span>
      <span class="preview__stat">{{ t("buffer") }} {{ dims[0] }}×{{ dims[1] }}</span>
      <span class="preview__stat">{{ zones.length }} {{ t("zones") }}</span>
      <span class="preview__spacer" />
      <span class="preview__stat">{{ t("capture") }}: {{ t(camera as 'idle') }}</span>
    </div>
  </section>
</template>

<style scoped>
.preview { flex: 1; min-width: 0; display: flex; flex-direction: column; padding: var(--wp-space-3); gap: var(--wp-space-2); min-height: 0; }
.preview__canvas { flex: 1; min-height: 0; display: flex; align-items: stretch; justify-content: center; padding: 0; }
.preview__canvas :deep(.editor) { height: 100%; width: auto; max-width: 100%; max-height: 100%; margin: 0 auto; }
.preview__foot { display: flex; align-items: center; gap: var(--wp-space-3); padding-top: var(--wp-space-1); border-top: var(--wp-size-border) solid var(--wp-color-border-subtle); }
.preview__stat { font-family: var(--wp-font-family-mono); font-size: var(--wp-font-size-xs); color: var(--wp-color-text-faint); }
.preview__spacer { flex: 1; }
</style>
