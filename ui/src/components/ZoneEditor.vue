<script setup lang="ts">
// Zonen-Editor: ein Rechteck je Zone auf einer Bildschirmfläche.
//
// Die Zonen sind normalisierte Koordinaten (0..1) — dieselben Werte, die der Kern
// speichert. Dadurch stimmt die Vorschau bei jeder Auflösung, und der Editor kennt
// keine Pixel. Die Füllfarbe ist die Farbe, die zuletzt für diese Zone rausging;
// so sieht man sofort, ob Aufnahme, Zuordnung und Ausgabe zusammenpassen.
import { ref } from "vue";
import type { Zone } from "../api";

const props = defineProps<{
  zones: Zone[];
  active: number;
  snap: boolean;
  colors: [number, number, number][];
  ranges: { start: number; count: number; label: string }[];
  /** Aktuelles Bild der Aufnahme; ohne Bild bleibt die Fläche leer. */
  image?: string | null;
}>();
const emit = defineEmits<{ "update:active": [index: number]; changed: [] }>();

const surface = ref<HTMLElement | null>(null);

interface Drag {
  index: number;
  mode: "move" | "resize";
  startX: number;
  startY: number;
  from: Zone;
  rect: DOMRect;
}
let drag: Drag | null = null;

const clamp = (value: number, low: number, high: number) => Math.min(high, Math.max(low, value));
// 0,5 % Raster: feiner als das Auge, grob genug zum Treffen.
const quantise = (value: number) => (props.snap ? Math.round(value * 200) / 200 : value);

function start(event: PointerEvent, index: number, mode: "move" | "resize") {
  event.preventDefault();
  event.stopPropagation();
  const rect = surface.value?.getBoundingClientRect();
  if (!rect) return;
  emit("update:active", index);
  drag = { index, mode, startX: event.clientX, startY: event.clientY, from: { ...props.zones[index] }, rect };
  window.addEventListener("pointermove", onMove);
  window.addEventListener("pointerup", onUp, { once: true });
}

function onMove(event: PointerEvent) {
  if (!drag) return;
  const zone = props.zones[drag.index];
  if (!zone) return;
  const dx = (event.clientX - drag.startX) / drag.rect.width;
  const dy = (event.clientY - drag.startY) / drag.rect.height;
  if (drag.mode === "move") {
    zone.x = clamp(quantise(drag.from.x + dx), 0, 1 - zone.w);
    zone.y = clamp(quantise(drag.from.y + dy), 0, 1 - zone.h);
  } else {
    zone.w = clamp(quantise(drag.from.w + dx), 0.02, 1 - zone.x);
    zone.h = clamp(quantise(drag.from.h + dy), 0.02, 1 - zone.y);
  }
}

function onUp() {
  window.removeEventListener("pointermove", onMove);
  if (drag) emit("changed");
  drag = null;
}

// Pfeiltasten verschieben, mit Shift in großen Schritten.
function nudge(event: KeyboardEvent, index: number) {
  const zone = props.zones[index];
  if (!zone) return;
  const step = event.shiftKey ? 0.05 : 0.005;
  const moves: Record<string, [number, number]> = {
    ArrowLeft: [-step, 0], ArrowRight: [step, 0], ArrowUp: [0, -step], ArrowDown: [0, step],
  };
  const move = moves[event.key];
  if (!move) return;
  event.preventDefault();
  zone.x = clamp(quantise(zone.x + move[0]), 0, 1 - zone.w);
  zone.y = clamp(quantise(zone.y + move[1]), 0, 1 - zone.h);
  emit("changed");
}

function style(zone: Zone, index: number) {
  const color = props.colors[index];
  const lit = color && color[0] + color[1] + color[2] > 6;
  return {
    left: `${zone.x * 100}%`,
    top: `${zone.y * 100}%`,
    width: `${zone.w * 100}%`,
    height: `${zone.h * 100}%`,
    background: lit ? `rgb(${color[0]} ${color[1]} ${color[2]} / 0.55)` : undefined,
  };
}
</script>

<template>
  <div ref="surface" class="editor">
    <img v-if="props.image" class="editor__image" :src="props.image" alt="" />
    <div v-else class="editor__screen">
      <span class="editor__label">Bildschirm</span>
    </div>
    <button
      v-for="(zone, i) in zones"
      :key="i"
      type="button"
      class="zone"
      :class="{ 'is-active': i === active, 'is-off': !zone.enabled }"
      :style="style(zone, i)"
      :aria-label="zone.name"
      @pointerdown="start($event, i, 'move')"
      @keydown="nudge($event, i)"
    >
      <span class="zone__num">{{ i + 1 }}</span>
      <span class="zone__label">{{ zone.name }} · {{ ranges[i]?.label ?? "" }}</span>
      <span class="zone__handle" title="Größe ändern" @pointerdown.stop="start($event, i, 'resize')" />
    </button>
  </div>
</template>

<style scoped>
.editor {
  position: relative;
  /* Groesse kommt von aussen (PreviewPanel), damit das Bild nicht verzerrt. */
  background: var(--wp-color-bg-input);
  border: var(--wp-size-border) solid var(--wp-color-border-strong);
  overflow: hidden;
  touch-action: none;
}
.editor__screen { position: absolute; inset: 0; display: flex; align-items: center; justify-content: center; }
.editor__image { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: fill; image-rendering: auto; }
.editor__label { font-family: var(--wp-font-family-mono); font-size: var(--wp-font-size-xs); color: var(--wp-color-text-faint); }
.zone {
  position: absolute;
  background: color-mix(in srgb, var(--wp-color-accent-default) 12%, transparent);
  border: 1px dashed var(--wp-color-accent-default);
  color: var(--wp-color-text-default);
  font-family: var(--wp-font-family-mono);
  font-size: 10px;
  padding: 2px 4px;
  cursor: grab;
  line-height: 1.4;
  text-align: left;
  overflow: hidden;
  text-shadow: 0 1px 2px rgb(0 0 0 / 0.85);
}
.zone:active { cursor: grabbing; }
.zone:focus-visible { outline: var(--wp-size-focus) solid var(--wp-color-focus-ring); outline-offset: -2px; }
.zone__num {
  display: inline-flex; align-items: center; justify-content: center;
  min-width: 13px; height: 13px; margin-right: 4px;
  background: var(--wp-color-accent-default); color: var(--wp-color-text-on-accent);
  font-size: 9px; font-weight: 600;
}
.zone__label { pointer-events: none; display: block; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.zone.is-active { border-style: solid; border-width: 2px; }
.zone.is-off { opacity: 0.4; }
.zone__handle {
  position: absolute; right: 0; bottom: 0; width: 10px; height: 10px;
  background: var(--wp-color-accent-default); cursor: nwse-resize;
}
</style>
