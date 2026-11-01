// Ein Zustand für die ganze Oberfläche. Quelle ist immer der Rust-Kern:
// `snapshot` beim Start, `telemetry` als Ereignis, `save_config` für Änderungen.
import { reactive } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { Config, FoundDevice, Snapshot, Telemetry, ThemeName } from "./api";
import { translator } from "./i18n";

export const state = reactive({
  config: null as Config | null,
  telemetry: null as Telemetry | null,
  found: [] as FoundDevice[],
  scanning: false,
  busy: false,
  error: "" as string,
  notice: "" as string,
});

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

/**
 * LED-Bereich jeder Zone auf ihrem Gerät: dieselbe Rechnung wie im Kern
 * (`zone_colors`). Ohne Startindex rücken die Zonen eines Geräts aneinander.
 */
export function zoneRanges(): { start: number; count: number; label: string }[] {
  const zones = state.config?.zones ?? [];
  const cursor = new Map<string, number>();
  return zones.map((zone) => {
    const start = zone.offset ?? (cursor.get(zone.device) ?? 0);
    if (zone.enabled && zone.offset === null) {
      cursor.set(zone.device, start + zone.count);
    }
    return {
      start,
      count: zone.count,
      label: zone.count > 1 ? `LED ${start}–${start + zone.count - 1}` : `LED ${start}`,
    };
  });
}
