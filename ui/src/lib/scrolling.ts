// Scrollleisten erscheinen nur, solange gescrollt wird. CSS kennt keinen
// Zustand „scrollt gerade“, deshalb setzt dieser Helfer an jedem Element, das
// scrollt, die Klasse `is-scrolling` und nimmt sie nach einer Ruhepause wieder
// weg. Die Darstellung selbst steht in base.css.

export const SCROLLING = "is-scrolling";

interface Marked {
  classList: { add(c: string): void; remove(c: string): void };
}

/**
 * Beobachtet alle Scroll-Ereignisse unter `root` (Scroll-Ereignisse steigen
 * nicht auf, daher die Capture-Phase). Gibt eine Abmeldefunktion zurück.
 */
export function markScrolling(root: EventTarget, idleMs = 800): () => void {
  const timers = new Map<Marked, ReturnType<typeof setTimeout>>();
  const onScroll = (e: Event) => {
    const el = e.target as Partial<Marked> | null;
    if (!el?.classList) return; // das Dokument selbst scrollt hier nie
    const m = el as Marked;
    m.classList.add(SCROLLING);
    const old = timers.get(m);
    if (old !== undefined) clearTimeout(old);
    timers.set(
      m,
      setTimeout(() => {
        m.classList.remove(SCROLLING);
        timers.delete(m);
      }, idleMs),
    );
  };
  root.addEventListener("scroll", onScroll, { capture: true, passive: true });
  return () => {
    root.removeEventListener("scroll", onScroll, { capture: true });
    for (const [m, t] of timers) {
      clearTimeout(t);
      m.classList.remove(SCROLLING);
    }
    timers.clear();
  };
}
