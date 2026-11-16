import { describe, expect, it } from "vitest";
import { ledsOf, zoneRanges } from "./zones";
import type { Zone } from "../api";

function zone(device: string, count: number, offset: number | null = null, enabled = true): Zone {
  return {
    name: device, device, x: 0, y: 0, w: 1, h: 0.1, count,
    vertical: false, gain: 1, enabled, offset, color: [0, 0, 0],
  };
}

describe("zoneRanges", () => {
  it("legt Zonen eines Geräts hintereinander", () => {
    const ranges = zoneRanges([zone("d1", 34), zone("d1", 26)]);
    expect(ranges.map((r) => r.start)).toEqual([0, 34]);
    expect(ranges[1].label).toBe("LED 34–59");
  });

  it("behandelt Geräte getrennt", () => {
    const ranges = zoneRanges([zone("d1", 10), zone("d2", 5), zone("d1", 10)]);
    expect(ranges.map((r) => r.start)).toEqual([0, 0, 10]);
  });

  it("nimmt einen ausdrücklichen Startindex", () => {
    const ranges = zoneRanges([zone("d1", 10), zone("d1", 10, 40)]);
    expect(ranges[1].start).toBe(40);
    // Wie im Kern (`Config::zone_starts`): die nächste Zone rückt hinter den
    // festen Block, nicht an den Anfang.
    expect(zoneRanges([zone("d1", 10, 40), zone("d1", 10)])[1].start).toBe(50);
    expect(zoneRanges([zone("d1", 10), zone("d1", 20, 50), zone("d1", 5)]).map((r) => r.start))
      .toEqual([0, 50, 70]);
  });

  it("lässt deaktivierte Zonen keinen Platz belegen", () => {
    const ranges = zoneRanges([zone("d1", 10, null, false), zone("d1", 10)]);
    expect(ranges[0].start).toBe(0);
    expect(ranges[1].start).toBe(0);
  });

  it("beschriftet eine einzelne LED ohne Bereich", () => {
    expect(zoneRanges([zone("d1", 1)])[0].label).toBe("LED 0");
  });
});

describe("ledsOf", () => {
  const wire: [number, number, number][] = [[1, 0, 0], [2, 0, 0], [3, 0, 0], [4, 0, 0]];

  it("schneidet den Bereich der Zone aus", () => {
    expect(ledsOf(wire, { reverse: false, bgr: false }, 1, 2)).toEqual([[2, 0, 0], [3, 0, 0]]);
  });

  it("rechnet einen umgedrehten Streifen zurück", () => {
    expect(ledsOf(wire, { reverse: true, bgr: false }, 0, 2)).toEqual([[4, 0, 0], [3, 0, 0]]);
  });

  it("tauscht Rot und Blau bei BGR zurück", () => {
    expect(ledsOf([[0, 0, 9]], { reverse: false, bgr: true }, 0, 1)).toEqual([[9, 0, 0]]);
  });

  it("bleibt innerhalb des Puffers", () => {
    expect(ledsOf(wire, { reverse: false, bgr: false }, 3, 10)).toEqual([[4, 0, 0]]);
    expect(ledsOf(wire, { reverse: true, bgr: false }, 8, 2)).toEqual([]);
  });
});
