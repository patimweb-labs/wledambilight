import { describe, expect, it } from "vitest";
import { zoneRanges } from "./zones";
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
    // Der ausdrückliche Index verschiebt den Zähler nicht.
    expect(zoneRanges([zone("d1", 10, 40), zone("d1", 10)])[1].start).toBe(0);
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
