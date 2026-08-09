import { defineConfig } from 'vite';
import vue from '@vitejs/plugin-vue';
import { fileURLToPath } from 'node:url';
import { dirname } from 'node:path';

const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig({
  plugins: [vue()],
  build: {
    target: 'es2021',
    cssCodeSplit: true,
    cssMinify: 'esbuild',
    chunkSizeWarningLimit: 700,
    rollupOptions: {
      output: {
        manualChunks: {
          vue: ['vue', 'pinia', 'vue-i18n'],
          xterm: ['@xterm/xterm', '@xterm/addon-fit', '@xterm/addon-search', '@xterm/addon-webgl'],
          ui: ['lucide-vue-next'],
          tauri: ['@tauri-apps/api'],
        },
      },
    },
  },
  resolve: {
    alias: {
      '@': dirname(fileURLToPath(import.meta.url)) + '/src',
    },
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: 'ws',
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ['**/src-tauri/**'],
    },
  },
});
