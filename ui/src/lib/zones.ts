// Zonen-Rechnung, ohne Fenster und ohne Tauri: der LED-Bereich jeder Zone auf
// ihrem Gerät. Dieselbe Regel wie im Kern (`commands::zone_colors`): ohne
// Startindex rücken die Zonen eines Geräts in der Listenreihenfolge aneinander,
// deaktivierte Zonen belegen keinen Platz.
import type { Zone } from "../api";

export interface ZoneRange {
  start: number;
  count: number;
  label: string;
}

export function zoneRanges(zones: Zone[]): ZoneRange[] {
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
