<script setup lang="ts">
// Die LED-Leiste zeigt, was tatsächlich auf die Leitung geht — nicht, was
// berechnet wurde. Leer heißt: es kommt nichts an.
const props = withDefaults(defineProps<{ colors?: [number, number, number][]; label?: string }>(),
  { colors: () => [] });
</script>

<template>
  <div class="strip" :class="{ 'is-empty': props.colors.length === 0 }" role="img"
       :aria-label="label ?? `${props.colors.length} LEDs`">
    <span v-for="(c, i) in props.colors" :key="i" class="strip__led"
          :style="{ background: `rgb(${c[0]} ${c[1]} ${c[2]})` }" />
    <span v-if="props.colors.length === 0" class="strip__empty">keine Daten</span>
  </div>
</template>

<style scoped>
.strip { display: flex; height: 14px; border: var(--wp-size-border) solid var(--wp-color-border-subtle); overflow: hidden; background: var(--wp-color-bg-input); }
.strip__led { flex: 1 1 0; min-width: 1px; }
.strip.is-empty { align-items: center; justify-content: center; }
.strip__empty { font-size: var(--wp-font-size-xs); color: var(--wp-color-text-faint); }
</style>
