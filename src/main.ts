import { createApp } from 'vue';
import { createPinia } from 'pinia';
import App from './App.vue';
import { setupLoggerDevTools } from '@/core/utils/logger-devtools';
import './styles/design-system.css';
import './styles/common.css';

const app = createApp(App);

// Initialize Pinia for state management
app.use(createPinia());

// Setup logger DevTools in development
setupLoggerDevTools();

app.mount('#app');
