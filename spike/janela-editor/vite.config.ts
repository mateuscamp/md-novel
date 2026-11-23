import { defineConfig } from "vite";

// Porta fixa: o tauri.conf.json aponta para ela.
export default defineConfig({
  clearScreen: false,
  server: { port: 1420, strictPort: true },
});
