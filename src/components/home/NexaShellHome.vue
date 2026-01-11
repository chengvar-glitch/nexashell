<template>
  <div class="nexashell-home">
    <div class="home-content-wrapper">
      <div class="home-header">
        <h2>NexaShell</h2>
        <p>Welcome to the modern SSH terminal tool</p>
      </div>

      <div class="home-content">
        <div class="quick-actions">
          <button class="action-btn" @click="handleNewConnection">
            <span class="btn-icon">+</span>
            <span class="btn-text">New SSH Connection</span>
          </button>

          <button class="action-btn" @click="handleOpenRecent">
            <span class="btn-icon">📁</span>
            <span class="btn-text">Open Recent Connection</span>
          </button>

          <button class="action-btn" @click="handleOpenSettings">
            <span class="btn-icon">⚙️</span>
            <span class="btn-text">Settings</span>
          </button>
        </div>

        <div class="recent-connections">
          <h3>Recent Connections</h3>
          <div class="connection-list">
            <div
              v-for="conn in recentConnections"
              :key="conn.id"
              class="connection-item"
            >
              <div class="conn-info">
                <div class="conn-name">
                  {{ conn.name }}
                </div>
                <div class="conn-details">
                  {{ conn.host }}:{{ conn.port }} | {{ conn.username }}
                </div>
              </div>
              <button class="connect-btn" @click="handleConnect(conn)">
                Connect
              </button>
            </div>
            <div v-if="recentConnections.length === 0" class="no-connections">
              No recent connections
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, inject } from 'vue';
import { OPEN_SSH_FORM_KEY } from '@/types';

interface SSHConnection {
  id: string;
  name: string;
  host: string;
  port: number;
  username: string;
}

// Props
const props = defineProps<{
  connections?: SSHConnection[];
}>();

// Emits - Send events to parent component, no specific business logic handled here
const emit = defineEmits<{
  newConnection: [];
  openRecent: [];
  openSettings: [];
  connect: [connection: SSHConnection];
}>();

// Use the passed connection list or an empty array
const recentConnections = ref<SSHConnection[]>(props.connections || []);

// Inject SSH form control method from App.vue
const openSSHForm = inject<() => void>(OPEN_SSH_FORM_KEY);

// Event handlers - only pass events upward
const handleNewConnection = () => {
  if (openSSHForm) {
    openSSHForm();
  }
  emit('newConnection');
};

const handleOpenRecent = () => {
  emit('openRecent');
};

const handleOpenSettings = () => {
  emit('openSettings');
};

const handleConnect = (conn: SSHConnection) => {
  emit('connect', conn);
};
</script>

<style scoped>
.nexashell-home {
  height: 100%;
  display: flex;
  flex-direction: column;
  background: var(--color-bg-primary);
  color: var(--color-text-primary);
  padding: 20px;
  box-sizing: border-box;
}

.home-header {
  text-align: center;
  margin-bottom: 30px;
}

.home-header h2 {
  font-size: 2em;
  margin: 0 0 10px 0;
  color: var(--color-text-primary);
}

.home-header p {
  color: var(--color-text-secondary);
  margin: 0;
}

.home-content {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 30px;
}

.quick-actions {
  display: flex;
  gap: 20px;
  justify-content: center;
}

.action-btn {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  width: 120px;
  height: 120px;
  background: var(--color-bg-secondary);
  border: 1px solid var(--color-border-primary);
  border-radius: var(--radius-md);
  cursor: pointer;
  transition: var(--transition-fast);
  padding: 15px;
  box-sizing: border-box;
}

.action-btn:hover {
  background: var(--color-interactive-hover);
  transform: translateY(-2px);
}

.btn-icon {
  font-size: 24px;
  margin-bottom: 8px;
}

.btn-text {
  font-size: 14px;
  text-align: center;
}

.recent-connections h3 {
  margin: 0 0 15px 0;
  color: var(--color-text-primary);
}

.connection-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.connection-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px;
  background: var(--color-bg-secondary);
  border-radius: var(--radius-sm);
  border: 1px solid var(--color-border-primary);
}

.conn-info {
  flex: 1;
}

.conn-name {
  font-weight: 500;
  color: var(--color-text-primary);
}

.conn-details {
  font-size: 0.9em;
  color: var(--color-text-secondary);
}

.connect-btn {
  padding: 6px 12px;
  background: var(--color-primary);
  color: white;
  border: none;
  border-radius: var(--radius-sm);
  cursor: pointer;
  transition: var(--transition-fast);
}

.connect-btn:hover {
  background: var(--color-primary-hover);
}

.no-connections {
  text-align: center;
  padding: 20px;
  color: var(--color-text-tertiary);
}

.home-content-wrapper {
  height: 100%;
}
</style>
