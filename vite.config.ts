/// <reference types="vitest/config" />
import { defineConfig } from 'vite';
import vue from '@vitejs/plugin-vue';
import { fileURLToPath } from 'node:url';
import { dirname } from 'node:path';

const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig({
  plugins: [vue()],
  esbuild: {
    // @xterm/xterm@6.0.0 ships PRE-MINIFIED ESM (lib/xterm.mjs). Vite's
    // production minify pass (esbuild) re-mangles it and corrupts a closure in
    // InputHandler.requestMode — the enum initializer ends up referencing an
    // identifier that no longer exists. The moment a TUI (vim/tmux/htop/less)
    // sends a DECRQM mode query (`CSI ? Ps $ p`), requestMode throws
    // `ReferenceError` inside xterm's async write pipeline, the write queue
    // stalls, and the terminal freezes permanently (no output, no reply).
    // Only reproducible in production builds: `vite dev` does not mangle deps,
    // which is why vim works in dev but the packaged app dies.
    //
    // esbuild's mangler renames `let r;(P=>…)(r||={})` to reference a name it
    // then eliminates (`(void 0||(r={}))`), and its syntax pass does the same
    // even when identifier mangling is disabled — so BOTH passes must be off
    // for already-minified vendor code (xtermjs/xterm.js#5800). Whitespace
    // minification is kept; the size cost is limited to this vendor chunk.
    minifyIdentifiers: false,
    minifySyntax: false,
  },
  test: {
    globals: true,
    environment: 'happy-dom',
    include: ['src/**/*.test.ts'],
    css: false,
  },
  build: {
    // Aligned with the TS compiler target in tsconfig.json (ES2020).
    target: 'es2020',
    cssCodeSplit: true,
    cssMinify: 'esbuild',
    chunkSizeWarningLimit: 700,
    rollupOptions: {
      input: {
        main: fileURLToPath(new URL('./index.html', import.meta.url)),
        filemanager: fileURLToPath(new URL('./filemanager.html', import.meta.url)),
      },
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
    // VITE_PORT can override the dev port (see .env.example); default 1420.
    port: Number(process.env.VITE_PORT) || 1420,
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
