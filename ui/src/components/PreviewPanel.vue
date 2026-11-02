<script setup lang="ts">
// Mitte: der Bildschirm mit den Zonen. Diese Fläche ist das Herz der App —
// hier wird gezogen, geskaliert und gesehen, was ankommt.
//
// Die Größe der Vorschau wird gerechnet, nicht dem CSS überlassen: der Kasten
// nimmt die größtmögliche Fläche im Seitenverhältnis der Aufnahme ein und wird
// mittig gesetzt. Damit quetscht sich weder Testbild noch Stream, und die Zonen
// liegen immer genau auf dem Bild.
import { computed, onBeforeUnmount, onMounted, reactive, ref, watch } from "vue";
import { commit, preview, previewFps, state, t, zoneRanges } from "../store";
import ZoneEditor from "./ZoneEditor.vue";

const props = defineProps<{ active: number }>();
const emit = defineEmits<{ "update:active": [index: number] }>();

const zones = computed(() => state.config?.zones ?? []);
const ranges = computed(() => zoneRanges());
const colors = computed(() => state.telemetry?.zone_colors ?? []);
const screen = computed(() => state.telemetry?.screen ?? [0, 0]);
const dims = computed(() => state.telemetry?.dims ?? [0, 0]);
const camera = computed(() => state.telemetry?.capture ?? "idle");

// Seitenverhältnis der Aufnahme (Puffer), Vorgabe 16:9.
const aspect = computed(() => {
  const [w, h] = dims.value;
  return w > 0 && h > 0 ? w / h : 16 / 9;
});

const stage = ref<HTMLElement | null>(null);
const box = reactive({ width: 0, height: 0 });

function fit() {
  const element = stage.value;
  if (!element) return;
  const availableWidth = element.clientWidth;
  const availableHeight = element.clientHeight;
  if (availableWidth <= 0 || availableHeight <= 0) return;
  const width = Math.min(availableWidth, availableHeight * aspect.value);
  box.width = Math.max(1, Math.floor(width));
  box.height = Math.max(1, Math.floor(width / aspect.value));
}

let observer: ResizeObserver | null = null;
onMounted(() => {
  observer = new ResizeObserver(fit);
  if (stage.value) observer.observe(stage.value);
  fit();
});
onBeforeUnmount(() => observer?.disconnect());
// Andere Aufnahmebreite → anderes Seitenverhältnis → neu einpassen.
watch(aspect, fit);

</script>

<template>
  <section class="preview">
    <div ref="stage" class="preview__stage">
      <ZoneEditor :zones="zones" :active="props.active" :snap="state.config?.settings.snap ?? true"
                  :colors="colors" :ranges="ranges" :image="preview"
                  :style="{ width: box.width + 'px', height: box.height + 'px' }"
                  @update:active="emit('update:active', $event)" @changed="commit" />
    </div>

    <div class="preview__foot">
      <span class="preview__stat">{{ t("screen") }} {{ screen[0] }}×{{ screen[1] }}</span>
      <span class="preview__stat">{{ t("buffer") }} {{ dims[0] }}×{{ dims[1] }}</span>
      <span class="preview__stat">{{ zones.length }} {{ t("zones") }}</span>
      <span class="preview__stat">{{ box.width }}×{{ box.height }}</span>
      <span class="preview__stat">{{ previewFps }} fps Vorschau</span>
      <span class="preview__spacer" />
      <span class="preview__stat">{{ t("capture") }}: {{ t(camera as "idle") }}</span>
    </div>
  </section>
</template>

<style scoped>
.preview { flex: 1; min-width: 0; display: flex; flex-direction: column; padding: var(--wp-space-3); gap: var(--wp-space-2); min-height: 0; }
/* Die Bühne nimmt den freien Platz; der Kasten darin behält sein Seitenverhältnis
   (Größe kommt aus `fit()`) und wird mittig gesetzt. */
.preview__stage { flex: 1; min-height: 0; min-width: 0; display: flex; align-items: center; justify-content: center; overflow: hidden; }
.preview__foot { display: flex; align-items: center; gap: var(--wp-space-3); padding-top: var(--wp-space-1); border-top: var(--wp-size-border) solid var(--wp-color-border-subtle); }
.preview__stat { font-family: var(--wp-font-family-mono); font-size: var(--wp-font-size-xs); color: var(--wp-color-text-faint); }
.preview__spacer { flex: 1; }
</style>
