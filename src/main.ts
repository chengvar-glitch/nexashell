import { createApp } from 'vue';
import { createPinia } from 'pinia';
import { getCurrentWindow } from '@tauri-apps/api/window';
import App from './App.vue';
import { i18n, initLocale } from '@/core/i18n';
import { createLogger } from '@/core/utils/logger';
import { setupLoggerDevTools } from '@/core/utils/logger-devtools';
import './styles/design-system.css';
import './styles/common.css';

const bootLogger = createLogger('BOOT');

// Route otherwise-unhandled async rejections and runtime errors into the
// central logger instead of leaving them silently unobserved.
window.addEventListener('unhandledrejection', event => {
  bootLogger.error('Unhandled promise rejection', event.reason);
});
window.addEventListener('error', event => {
  bootLogger.error(`Uncaught error in ${event.message}`, {
    filename: event.filename,
    lineno: event.lineno,
  });
});

const app = createApp(App);

// Initialize Pinia for state management
app.use(createPinia());

// Use internationalization
app.use(i18n);

// Setup logger DevTools in development
setupLoggerDevTools();

async function bootstrap() {
  // Ensure the persisted locale is loaded (messages resolved) before mount.
  await initLocale();
  app.mount('#app');

  // The main window is configured with `visible: false` so it never paints the
  // bare window background before the UI is ready. Reveal it after the first
  // frame renders (double rAF = after the initial paint), with a timeout
  // fallback so a stalled bundle cannot leave the window invisible forever.
  let shown = false;
  const reveal = () => {
    if (shown) return;
    shown = true;
    getCurrentWindow()
      .show()
      .catch(err => bootLogger.error('Failed to show main window', err));
  };
  requestAnimationFrame(() => requestAnimationFrame(reveal));
  setTimeout(reveal, 3000);
}

bootstrap();
