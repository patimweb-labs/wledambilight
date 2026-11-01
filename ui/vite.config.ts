import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

// Tauri liefert die Dateien im Betrieb selbst aus; im Entwicklungsmodus kommt die
// Oberfläche von hier. Port 1420 ist in tauri.conf.json fest verdrahtet.
//
// Die Komponentenbibliothek liegt in einem eigenen Repository. Lokal wird sie über
// den Nachbarordner eingebunden, im Flatpak-Build über WPM_UI_DIR (die Quelle wird
// dort mitgeliefert). Beides zeigt auf dieselben Quellen — kein zweiter Stand.
const here = dirname(fileURLToPath(import.meta.url));
const library = process.env.WPM_UI_DIR
  ? resolve(process.env.WPM_UI_DIR)
  : resolve(here, "../../weppelmann-ui");
const lib = (...p: string[]) => resolve(library, "packages", ...p);

export default defineConfig({
  plugins: [vue()],
  clearScreen: false,
  server: { port: 1420, strictPort: true, host: "127.0.0.1", fs: { allow: [resolve(library), resolve(here)] } },
  resolve: {
    alias: [
      { find: /^@weppelmann\/components$/, replacement: lib("components/src/index.ts") },
      { find: /^@weppelmann\/components\/(.*)$/, replacement: lib("components/src/$1") },
      { find: /^@weppelmann\/icons$/, replacement: lib("icons/src/index.ts") },
      { find: /^@weppelmann\/tokens-fonts$/, replacement: lib("tokens/dist/css/fonts.css") },
      { find: /^@weppelmann\/tokens-css$/, replacement: lib("tokens/dist/css/vars.css") },
      { find: /^@weppelmann\/tokens$/, replacement: lib("tokens/dist/json/tokens.json") },
    ],
  },
  build: { target: "es2022", outDir: "dist", emptyOutDir: true },
});
