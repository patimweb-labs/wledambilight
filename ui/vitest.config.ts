import { defineConfig } from "vitest/config";

// Nur reine Logik: keine Tauri-Aufrufe, kein DOM. Was das Fenster anzeigt,
// prueft spaeter Playwright.
export default defineConfig({
  test: { include: ["src/**/*.test.ts"], environment: "node" },
});
