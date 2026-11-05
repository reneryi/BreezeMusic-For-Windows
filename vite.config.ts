import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";

const host = process.env.TAURI_DEV_HOST;

export default defineConfig(async () => ({
  plugins: [vue()],
  clearScreen: false,
  build: {
    rollupOptions: {
      output: {
        manualChunks(id) {
          if (id.includes("node_modules")) {
            if (id.includes("@tauri-apps")) {
              return "vendor-tauri";
            }
            if (id.includes("lucide-vue-next")) {
              return "vendor-icons";
            }
            if (id.includes("vue") || id.includes("pinia")) {
              return "vendor-vue";
            }
            return "vendor";
          }
          if (id.includes("/src/stores/appStore")) {
            return "app-store";
          }
        },
      },
    },
  },
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
}));
