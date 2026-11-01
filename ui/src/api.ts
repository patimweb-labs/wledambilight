// Vertrag mit dem Rust-Kern. Die Namen entsprechen den Feldern aus config.rs —
// dort ist die Wahrheit, hier nur die Sicht darauf.

export type DeviceKind = "Wled" | "OpenRgb" | "Razer" | "Govee" | "Nanoleaf";
export type Protocol = "Ddp" | "Drgb";
export type ThemeName = "dark" | "light" | "mono";

export interface DeviceCfg {
  id: string;
  name: string;
  host: string;
  led_count: number;
  enabled: boolean;
  protocol: Protocol;
  kind: DeviceKind;
  port: number;
  target: string;
  token: string | null;
  brightness: number;
  reverse: boolean;
  bgr: boolean;
  max_fps: number;
}

export interface Zone {
  name: string;
  device: string;
  x: number; y: number; w: number; h: number;
  count: number;
  vertical: boolean;
  gain: number;
  enabled: boolean;
  offset: number | null;
  color: [number, number, number];
}

export interface Settings {
  target_fps: number;
  capture_width: number;
  smoothing_ms: number;
  spatial: number;
  slew: number;
  pause_when_focused: boolean;
  gamma: number;
  saturation: number;
  brightness: number;
  restore_token: string | null;
  test_image: boolean;
  snap: boolean;
  monitor: number;
  language: string;
  theme: string;
}

export interface Config {
  devices: DeviceCfg[];
  zones: Zone[];
  settings: Settings;
}

export type CaptureKind = "idle" | "starting" | "running" | "error";
export type DeviceState = "connecting" | "ok" | "error";

export interface DeviceStatus {
  id: string;
  state: DeviceState;
  error: string | null;
}

export interface Telemetry {
  capture: CaptureKind;
  capture_error: string | null;
  screen: [number, number];
  dims: [number, number];
  fps_in: number;
  fps_out: number;
  sample_us: number;
  paused: boolean;
  devices: DeviceStatus[];
  device_leds: [number, number, number][][];
  /** Eine Farbe je Zone, in der Reihenfolge der Konfiguration. */
  zone_colors: [number, number, number][];
}

export interface Snapshot {
  config: Config;
  telemetry: Telemetry;
}

export interface FoundDevice {
  kind: Lowercase<DeviceKind>;
  id: string;
  name: string;
  host: string;
  port: number;
  target: string;
  leds: number;
  detail: string;
}

export const DEVICE_KINDS: { value: DeviceKind; key: Lowercase<DeviceKind> }[] = [
  { value: "Wled", key: "wled" },
  { value: "OpenRgb", key: "openrgb" },
  { value: "Razer", key: "razer" },
  { value: "Govee", key: "govee" },
  { value: "Nanoleaf", key: "nanoleaf" },
];
