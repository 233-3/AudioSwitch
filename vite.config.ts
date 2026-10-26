import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";

// Tauri 期望前端 dev server 使用固定端口，并在固定端口不可用时直接失败，
// 避免 Tauri 连接到错误的端口。
const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  plugins: [vue()],
  clearScreen: false,
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
      // src-tauri 由 cargo 自己监听，避免 vite 重复触发
      ignored: ["**/src-tauri/**"],
    },
  },
  // Tauri 使用固定的 Chromium 版本，可以安全地以现代语法为目标
  build: {
    target: "chrome110",
    minify: "esbuild",
    sourcemap: false,
  },
});
