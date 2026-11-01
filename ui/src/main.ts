import { createApp } from "vue";
import "@weppelmann/tokens-fonts";
import "@weppelmann/tokens-css";
import "@weppelmann/components/style.css";
import "./base.css";
import App from "./App.vue";
import { init } from "./store";

// Erst den Zustand holen, dann zeichnen: die Oberfläche soll beim ersten Bild
// schon die Wahrheit zeigen und nicht kurz einen leeren Rahmen.
init().finally(() => createApp(App).mount("#app"));
