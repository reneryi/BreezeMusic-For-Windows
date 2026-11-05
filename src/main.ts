import { createApp } from "vue";
import { createPinia } from "pinia";
import App from "./App.vue";
import DesktopLyricWindow from "./components/DesktopLyricWindow.vue";
import "./styles/main.css";

const isStandaloneDesktopLyricWindow =
  typeof window !== "undefined" &&
  window.location.search.includes("window=desktop_lyric");

const app = createApp(
  isStandaloneDesktopLyricWindow ? DesktopLyricWindow : App
);
if (!isStandaloneDesktopLyricWindow) {
  app.use(createPinia());
}
app.mount("#app");
