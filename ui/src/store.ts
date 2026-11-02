// Ein Zustand für die ganze Oberfläche. Quelle ist immer der Rust-Kern:
// `snapshot` beim Start, `telemetry` als Ereignis, `save_config` für Änderungen.
import { reactive, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { Config, FoundDevice, Snapshot, Telemetry, ThemeName } from "./api";
import { translator } from "./i18n";
import { zoneRanges as computeRanges } from "./lib/zones";

export const state = reactive({
  config: null as Config | null,
  telemetry: null as Telemetry | null,
  found: [] as FoundDevice[],
  scanning: false,
  busy: false,
  error: "" as string,
  notice: "" as string,
});

/** Aktuelles Bild der Aufnahme als data-URL (JPEG). */
export const preview = ref<string | null>(null);

export function t(key: Parameters<ReturnType<typeof translator>>[0]): string {
  return translator(state.config?.settings.language ?? "auto")(key);
}

export function themeName(): ThemeName {
  const theme = state.config?.settings.theme;
  return theme === "light" || theme === "mono" ? theme : "dark";
}

function applyTheme() {
  document.documentElement.dataset.theme = themeName();
}

export async function init() {
  try {
    const snapshot = await invoke<Snapshot>("snapshot");
    state.config = snapshot.config;
    state.telemetry = snapshot.telemetry;
    // Alte Themes hießen anders ("Crosspoint"). Es gibt nur noch drei; alles
    // Unbekannte wird zu dark, ohne dass die Datei angefasst wird, bevor der
    // Nutzer etwas ändert.
    applyTheme();
    await listen<Telemetry>("telemetry", (event) => {
      state.telemetry = event.payload;
    });
    // Vorschau getrennt von den Messwerten. 90 ms (~11 Bilder/s) sind flüssig
    // genug, um Bewegung zu sehen, und deutlich billiger als ein Videostrom.
    window.setInterval(async () => {
      try {
        const frame = await invoke<string | null>("preview_frame");
        if (frame) preview.value = frame;
      } catch {
        /* Fenster noch nicht bereit: beim nächsten Takt erneut versuchen */
      }
    }, 90);
  } catch (error) {
    state.error = String(error);
  }
}

let pending: number | undefined;

/** Konfiguration übernehmen: entprellt, damit Tippen keine Schreiborgie auslöst. */
export function commit() {
  if (!state.config) return;
  window.clearTimeout(pending);
  pending = window.setTimeout(async () => {
    if (!state.config) return;
    state.busy = true;
    try {
      const snapshot = await invoke<Snapshot>("save_config", {
        config: JSON.parse(JSON.stringify(state.config)),
      });
      state.telemetry = snapshot.telemetry;
      state.error = "";
      applyTheme();
    } catch (error) {
      state.error = String(error);
    } finally {
      state.busy = false;
    }
  }, 300);
}

export async function restartCapture() {
  try {
    await invoke("restart_capture");
  } catch (error) {
    state.error = String(error);
  }
}

export async function forgetPermission() {
  try {
    await invoke("forget_permission");
    await restartCapture();
  } catch (error) {
    state.error = String(error);
  }
}

export async function setPaused(paused: boolean) {
  try {
    await invoke("set_paused", { paused });
  } catch (error) {
    state.error = String(error);
  }
}

export async function scanNetwork() {
  state.scanning = true;
  state.found = [];
  try {
    state.found = await invoke<FoundDevice[]>("discover");
  } catch (error) {
    state.error = String(error);
  } finally {
    state.scanning = false;
  }
}

export function addFound(found: FoundDevice) {
  if (!state.config) return;
  if (state.config.devices.some((d) => d.id === found.id)) {
    state.notice = `${found.name} ist bereits eingerichtet.`;
    return;
  }
  state.config.devices.push({
    id: found.id,
    name: found.name,
    host: found.host,
    led_count: found.leds,
    enabled: true,
    protocol: "Ddp",
    kind: (found.kind === "wled"
      ? "Wled"
      : found.kind === "openrgb"
        ? "OpenRgb"
        : found.kind === "razer"
          ? "Razer"
          : found.kind === "govee"
            ? "Govee"
            : "Nanoleaf") as Config["devices"][number]["kind"],
    port: found.port,
    target: found.target,
    token: null,
    brightness: 1,
    reverse: false,
    bgr: false,
    max_fps: 0,
  });
  commit();
}

export function removeDevice(id: string) {
  if (!state.config) return;
  state.config.devices = state.config.devices.filter((d) => d.id !== id);
  state.config.zones = state.config.zones.filter((z) => z.device !== id);
  commit();
}

export function addZone(deviceId: string) {
  if (!state.config) return;
  state.config.zones.push({
    name: `Zone ${state.config.zones.length + 1}`,
    device: deviceId,
    x: 0.1, y: 0.1, w: 0.8, h: 0.2,
    count: 30,
    vertical: false,
    gain: 1,
    enabled: true,
    offset: null,
    color: [0, 0, 0],
  });
  commit();
}

export function removeZone(index: number) {
  if (!state.config) return;
  state.config.zones.splice(index, 1);
  commit();
}


/** WLED am Host fragen: Name, Firmware und LED-Anzahl. */
export async function probeDevice(host: string): Promise<{ name: string; leds: number; detail: string } | null> {
  try {
    const found = await invoke<FoundDevice>("probe_wled", { host });
    return { name: found.name, leds: found.leds, detail: found.detail };
  } catch (error) {
    state.error = String(error);
    return null;
  }
}

/** Nanoleaf paaren: der Controller muss blinken, dann liefert er den Token. */
export async function pairNanoleaf(host: string): Promise<string | null> {
  try {
    state.busy = true;
    return await invoke<string>("pair_nanoleaf", { host });
  } catch (error) {
    state.error = String(error);
    return null;
  } finally {
    state.busy = false;
  }
}

/** Controller eines OpenRGB-Servers holen (für die Auswahl im Dialog). */
export async function listOpenRgb(host: string, port: number): Promise<FoundDevice[]> {
  try {
    state.busy = true;
    return await invoke<FoundDevice[]>("list_openrgb", { host, port });
  } catch (error) {
    state.error = String(error);
    return [];
  } finally {
    state.busy = false;
  }
}

/** Farben der LEDs, die eine Zone gerade belegt (für das Lineal). */
export function zoneLeds(index: number): [number, number, number][] {
  const zone = state.config?.zones[index];
  if (!zone) return [];
  const deviceIndex = (state.config?.devices ?? []).findIndex((d) => d.id === zone.device);
  if (deviceIndex < 0) return [];
  const strip = state.telemetry?.device_leds?.[deviceIndex] ?? [];
  const { start, count } = zoneRanges()[index] ?? { start: 0, count: zone.count };
  return strip.slice(start, start + count);
}

/** LED-Bereich jeder Zone (getestete Logik in `lib/zones.ts`). */
export function zoneRanges() {
  return computeRanges(state.config?.zones ?? []);
}
