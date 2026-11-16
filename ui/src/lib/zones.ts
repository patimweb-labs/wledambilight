// Zonen-Rechnung, ohne Fenster und ohne Tauri: der LED-Bereich jeder Zone auf
// ihrem Gerät. Dieselbe Regel wie im Kern (`Config::zone_starts`, daraus liest
// auch der Sampler): ohne Startindex rückt eine Zone hinter die vorige
// eingeschaltete Zone desselben Geräts — auch hinter eine mit festem Start.
// Deaktivierte Zonen belegen keinen Platz.
import type { DeviceCfg, Zone } from "../api";

export type Rgb = [number, number, number];

export interface ZoneRange {
  start: number;
  count: number;
  label: string;
}

export function zoneRanges(zones: Zone[]): ZoneRange[] {
  const cursor = new Map<string, number>();
  return zones.map((zone) => {
    const start = zone.offset ?? (cursor.get(zone.device) ?? 0);
    if (zone.enabled) {
      cursor.set(zone.device, start + zone.count);
    }
    return {
      start,
      count: zone.count,
      label: zone.count > 1 ? `LED ${start}–${start + zone.count - 1}` : `LED ${start}`,
    };
  });
}

/**
 * Farben der LEDs `start … start + count` eines Geräts in Bildfarben.
 *
 * `wire` ist der Puffer, wie er auf die Leitung ging: bei `reverse` liegt LED 0
 * am Ende, bei `bgr` sind Rot und Blau getauscht. Beides wird zurückgerechnet.
 */
export function ledsOf(
  wire: Rgb[],
  device: Pick<DeviceCfg, "reverse" | "bgr">,
  start: number,
  count: number,
): Rgb[] {
  const out: Rgb[] = [];
  const end = Math.min(start + count, wire.length);
  for (let led = Math.max(0, start); led < end; led++) {
    const c = wire[device.reverse ? wire.length - 1 - led : led];
    if (!c) continue;
    out.push(device.bgr ? [c[2], c[1], c[0]] : [c[0], c[1], c[2]]);
  }
  return out;
}
