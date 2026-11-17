import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { markScrolling, SCROLLING } from "./scrolling";

// Ein Element, das scrollen kann: EventTarget mit Klassenliste.
class Box extends EventTarget {
  classes = new Set<string>();
  classList = {
    add: (c: string) => void this.classes.add(c),
    remove: (c: string) => void this.classes.delete(c),
  };
}

function scroll(root: EventTarget, target: Box) {
  // im Browser kommt das Ereignis in der Capture-Phase am Wurzelknoten vorbei
  const e = new Event("scroll");
  Object.defineProperty(e, "target", { value: target });
  root.dispatchEvent(e);
}

describe("markScrolling", () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it("zeigt die Leiste beim Scrollen und blendet sie nach der Ruhepause aus", () => {
    const root = new EventTarget();
    const box = new Box();
    markScrolling(root, 800);
    expect(box.classes.has(SCROLLING)).toBe(false);
    scroll(root, box);
    expect(box.classes.has(SCROLLING)).toBe(true);
    vi.advanceTimersByTime(799);
    expect(box.classes.has(SCROLLING)).toBe(true);
    vi.advanceTimersByTime(1);
    expect(box.classes.has(SCROLLING)).toBe(false);
  });

  it("weiteres Scrollen verlängert die Anzeige", () => {
    const root = new EventTarget();
    const box = new Box();
    markScrolling(root, 800);
    scroll(root, box);
    vi.advanceTimersByTime(600);
    scroll(root, box);
    vi.advanceTimersByTime(600);
    expect(box.classes.has(SCROLLING)).toBe(true);
    vi.advanceTimersByTime(200);
    expect(box.classes.has(SCROLLING)).toBe(false);
  });

  it("jedes Element hat seine eigene Pause", () => {
    const root = new EventTarget();
    const a = new Box();
    const b = new Box();
    markScrolling(root, 800);
    scroll(root, a);
    vi.advanceTimersByTime(500);
    scroll(root, b);
    vi.advanceTimersByTime(300);
    expect(a.classes.has(SCROLLING)).toBe(false);
    expect(b.classes.has(SCROLLING)).toBe(true);
  });

  it("Abmelden räumt auf und reagiert nicht mehr", () => {
    const root = new EventTarget();
    const box = new Box();
    const stop = markScrolling(root, 800);
    scroll(root, box);
    stop();
    expect(box.classes.has(SCROLLING)).toBe(false);
    scroll(root, box);
    expect(box.classes.has(SCROLLING)).toBe(false);
  });
});
