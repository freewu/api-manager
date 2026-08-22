import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
  envPrefix: ["VITE_", "TAURI_"],
  build: {
    target: "es2021",
    minify: "esbuild",
    sourcemap: false,
    rollupOptions: {
      input: {
        main: "index.html",
        loading: "loading.html",
      },
      output: {
        // 分包策略：react 全家桶 / tauri 插件 / 其余第三方依赖独立成块，
        // 便于浏览器长期缓存（升级依赖时只有对应块失效）；highlight.js 单独一块
        // （配合 CodeTab 懒加载，首次进入不加载高亮代码）
        manualChunks(id: string) {
          if (!id.includes("node_modules")) return undefined;
          if (id.includes("highlight.js")) return "highlight";
          if (id.includes("@tauri-apps")) return "tauri-vendor";
          if (id.includes("react") || id.includes("scheduler")) return "react-vendor";
          return "vendor";
        },
      },
    },
  },
});
