import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// https://vitejs.dev/config/
export default defineConfig(async () => ({
  plugins: [svelte()],

  // 防止 Vite 遮蔽 Rust 的端口
  clearScreen: false,

  // Tauri 期望固定端口，如果端口被占用则失败
  server: {
    port: 1420,
    strictPort: true,
    watch: {
      // 监听 src-tauri 目录变化时自动重载
      ignored: ["**/src-tauri/**"],
    },
  },
}));
