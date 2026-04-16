import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  server: {
    port: 5173,
    strictPort: true,
  },
  clearScreen: false,
  envPrefix: ["VITE_", "TAURI_"],
  build: {
    outDir: "dist",
    target:
      process.env.TAURI_ENV_PLATFORM === "windows" ? "chrome105" : "safari14",
    sourcemap: !!process.env.TAURI_ENV_DEBUG,
    minify: !process.env.TAURI_ENV_DEBUG ? "esbuild" : false,
  },
});
