// Entry point for the standalone file-manager window. Mounts a slim root that
// hosts a full SFTP file manager (browse/upload/download) for a single session,
// so a new Tauri window never needs the full tab/app shell.
import { createApp } from 'vue';
import { createPinia } from 'pinia';
import FileManagerWindow from '@/components/filemanager/FileManagerWindow.vue';
import { i18n } from '@/core/i18n';
import { themeManager } from '@/core/utils/theme-manager';
import { setupLoggerDevTools } from '@/core/utils/logger-devtools';
import './styles/design-system.css';
import './styles/common.css';

// Apply the persisted theme (written by the main window) so this window opens
// with the same light/dark background instead of only following the OS
// preference.
themeManager.initialize();

const app = createApp(FileManagerWindow);

// Initialize Pinia for state management
app.use(createPinia());

// Use internationalization
app.use(i18n);

// Setup logger DevTools in development
setupLoggerDevTools();

app.mount('#app');
