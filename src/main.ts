import { createApp } from 'vue';
import { createPinia } from 'pinia';
import App from './App.vue';
import { i18n } from '@/core/i18n';
import { setupLoggerDevTools } from '@/core/utils/logger-devtools';
import './styles/design-system.css';
import './styles/common.css';

const app = createApp(App);

// Initialize Pinia for state management
app.use(createPinia());

// Use internationalization
app.use(i18n);

// Setup logger DevTools in development
setupLoggerDevTools();

app.mount('#app');
