<script setup lang="ts">
// Fensteraufbau wie bisher: Kopfzeile, links Geräte, Mitte Bildschirm mit Zonen,
// rechts Inspektor, unten Ausgabe. Die Gestaltung kommt aus den Tokens.
import { computed, ref } from "vue";
import * as UI from "@weppelmann/components";
import { state } from "./store";
import TopBar from "./components/TopBar.vue";
import DevicesPanel from "./components/DevicesPanel.vue";
import PreviewPanel from "./components/PreviewPanel.vue";
import InspectorPanel from "./components/InspectorPanel.vue";
import OutputBar from "./components/OutputBar.vue";

const active = ref(0);
const ready = computed(() => state.config !== null);
</script>

<template>
  <div class="app">
    <TopBar />
    <div v-if="!ready" class="boot">
      <UI.WpSpinner size="md" />
      <UI.WpText tone="muted">{{ state.error || "…" }}</UI.WpText>
    </div>
    <template v-else>
      <div class="body">
        <DevicesPanel />
        <PreviewPanel :active="active" @update:active="active = $event" />
        <InspectorPanel :active="active" @update:active="active = $event" />
      </div>
      <OutputBar />
    </template>
    <div v-if="state.error" class="error">{{ state.error }}</div>
  </div>
</template>

<style scoped>
.app { display: flex; flex-direction: column; height: 100%; background: var(--wp-color-bg-canvas); }
.body { flex: 1; display: flex; min-height: 0; }
.boot { flex: 1; display: flex; align-items: center; justify-content: center; gap: var(--wp-space-2); }
.error { padding: 4px var(--wp-space-3); background: var(--wp-color-status-error); color: var(--wp-color-bg-canvas); font-size: var(--wp-font-size-xs); }
</style>
