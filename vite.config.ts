import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

// Tauri 期望固定端口 1420；dev server 不可抢占时直接失败而非换端口。
export default defineConfig({
  plugins: [react(), tailwindcss()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
  },
  envPrefix: ["VITE_", "TAURI_ENV_*"],
  build: {
    target: "esnext",
    // Tauri webview 下无需 polyfill，用现代 ES 即可。
    minify: "esbuild",
    sourcemap: false,
  },
});
