<template>
  <div class="ssh-connection-form">
    <div class="form-header">
      <div class="header-left" v-if="isMacOS">
        <button class="close-btn macos-close" @click="onCancel">
          <span class="close-icon">●</span>
        </button>
      </div>
      <h3 class="form-title">新建SSH连接</h3>
      <div class="header-right" v-if="!isMacOS">
        <button class="close-btn windows-close" @click="onCancel">
          <span class="close-icon">×</span>
        </button>
      </div>
    </div>
    
    <form @submit.prevent="onSubmit">
      <div class="form-row">
        <div class="form-group">
          <label for="host">主机地址 *</label>
          <input 
            id="host" 
            v-model="formData.host" 
            type="text" 
            placeholder="例如: 192.168.1.100"
            :class="{ 'error': validationErrors.host }"
            required
          />
          <span v-if="validationErrors.host" class="error-message">{{ validationErrors.host }}</span>
        </div>
        
        <div class="form-group">
          <label for="port">端口</label>
          <input 
            id="port" 
            v-model.number="formData.port" 
            type="number" 
            min="1" 
            max="65535" 
            placeholder="22"
            :class="{ 'error': validationErrors.port }"
          />
          <span v-if="validationErrors.port" class="error-message">{{ validationErrors.port }}</span>
        </div>
      </div>
      
      <div class="form-row">
        <div class="form-group full-width">
          <label for="username">用户名 *</label>
          <input 
            id="username" 
            v-model="formData.username" 
            type="text" 
            placeholder="用户名"
            :class="{ 'error': validationErrors.username }"
            required
          />
          <span v-if="validationErrors.username" class="error-message">{{ validationErrors.username }}</span>
        </div>
      </div>
      
      <div class="form-row">
        <div class="form-group full-width">
          <label for="password">密码</label>
          <input 
            id="password" 
            v-model="formData.password" 
            type="password" 
            placeholder="密码(留空使用密钥认证)"
          />
        </div>
      </div>
      
      <div class="form-row">
        <div class="form-group">
          <label for="privateKey">私钥文件</label>
          <input 
            id="privateKey" 
            v-model="formData.privateKey" 
            type="text" 
            placeholder="私钥文件路径"
          />
        </div>
        
        <div class="form-group">
          <label for="keyPassphrase">密钥密码</label>
          <input 
            id="keyPassphrase" 
            v-model="formData.keyPassphrase" 
            type="password" 
            placeholder="私钥密码(可选)"
          />
        </div>
      </div>
      
      <div class="form-row">
        <div class="form-group full-width">
          <label for="name">连接名称</label>
          <input 
            id="name" 
            v-model="formData.name" 
            type="text" 
            placeholder="为连接命名(可选)"
          />
        </div>
      </div>
      
      <div class="form-actions">
        <button type="submit" class="btn-connect">连接</button>
        <button type="button" @click="onCancel" class="btn-cancel">取消</button>
      </div>
    </form>
  </div>
</template>

<script setup lang="ts">
import { reactive, ref, onMounted } from 'vue';
import { isMacOSBrowser } from '@/utils/app-utils';

interface SSHConnectionFormData {
  host: string;
  port: number | null;
  username: string;
  password: string;
  privateKey: string;
  keyPassphrase: string;
  name: string;
}

interface ValidationErrors {
  host?: string;
  port?: string;
  username?: string;
}

const formData = reactive<SSHConnectionFormData>({
  host: '',
  port: 22,
  username: '',
  password: '',
  privateKey: '',
  keyPassphrase: '',
  name: ''
});

const validationErrors = reactive<ValidationErrors>({});

const emit = defineEmits<{ 
  connect: [data: SSHConnectionFormData];
  cancel: [];
}>();

// 检测是否为macOS系统
const isMacOS = ref(false);

onMounted(async () => {
  try {
    isMacOS.value = await isMacOSBrowser();
  } catch (error) {
    console.error('Failed to detect platform:', error);
    isMacOS.value = false;
  }
});

const validateForm = (): boolean => {
  // 清除之前的错误
  Object.keys(validationErrors).forEach(key => {
    delete validationErrors[key as keyof ValidationErrors];
  });
  
  let isValid = true;
  
  // 验证主机地址
  if (!formData.host.trim()) {
    validationErrors.host = '主机地址不能为空';
    isValid = false;
  } else if (!isValidHost(formData.host)) {
    validationErrors.host = '请输入有效的主机地址(IP或域名)';
    isValid = false;
  }
  
  // 验证端口
  if (formData.port !== null && (formData.port < 1 || formData.port > 65535)) {
    validationErrors.port = '端口号必须在1-65535之间';
    isValid = false;
  }
  
  // 验证用户名
  if (!formData.username.trim()) {
    validationErrors.username = '用户名不能为空';
    isValid = false;
  }
  
  return isValid;
};

const isValidHost = (host: string): boolean => {
  // 检查是否为IP地址或有效域名
  const ipRegex = /^(?:(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)$/;
  const domainRegex = /^[a-zA-Z0-9]([a-zA-Z0-9\-]{0,61}[a-zA-Z0-9])?(\.[a-zA-Z0-9]([a-zA-Z0-9\-]{0,61}[a-zA-Z0-9])?)*$/;
  
  return ipRegex.test(host) || domainRegex.test(host);
};

const onSubmit = () => {
  if (!validateForm()) {
    return;
  }
  
  // 使用默认端口22，如果未指定的话
  const submitData = {
    ...formData,
    port: formData.port || 22
  };
  
  emit('connect', submitData);
};

const onCancel = () => {
  emit('cancel');
};
</script>

<style scoped>
.ssh-connection-form {
  width: 100%;
  max-width: 480px;
  padding: 16px;
  background: var(--color-bg-secondary);
  border-radius: var(--radius-md);
  box-shadow: var(--shadow-md);
}

.form-header {
  display: grid;
  grid-template-columns: 24px 1fr 24px;
  align-items: center;
  margin-bottom: 16px;
  gap: 8px;
}

.header-left, .header-right {
  width: 24px;
  height: 24px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.form-title {
  margin: 0;
  color: var(--color-text-primary);
  font-size: 1.1em;
  font-weight: 600;
  text-align: center;
}

.close-btn {
  border: none;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0;
  margin: 0;
  transition: var(--transition-fast);
  flex-shrink: 0;
}

.macos-close {
  background: #ff5f57;
  width: 12px;
  height: 12px;
  border-radius: 50%;
}

.macos-close .close-icon {
  color: transparent;
  font-size: 0;
  width: 12px;
  height: 12px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.windows-close {
  background: transparent;
  width: 24px;
  height: 24px;
  border-radius: var(--radius-sm);
  font-size: 16px;
}

.windows-close .close-icon {
  color: var(--color-text-secondary);
  width: 24px;
  height: 24px;
  display: flex;
  align-items: center;
  justify-content: center;
  line-height: 1;
}

.close-btn:hover {
  opacity: 0.8;
}

.windows-close:hover {
  background: var(--color-interactive-hover);
}

.form-row {
  display: flex;
  gap: 12px;
  margin-bottom: 12px;
}

.form-group {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.form-group.full-width {
  flex: 1 0 100%;
}

.form-group label {
  display: block;
  margin-bottom: 4px;
  color: var(--color-text-primary);
  font-size: 0.9em;
  font-weight: 500;
}

.form-group input {
  width: 100%;
  padding: 6px 8px;
  border: 1px solid var(--color-border-primary);
  border-radius: var(--radius-sm);
  background: var(--color-bg-primary);
  color: var(--color-text-primary);
  font-size: 0.9em;
  box-sizing: border-box;
}

.form-group input.error {
  border-color: #ff4757;
}

.form-group input:focus {
  outline: none;
  border-color: var(--color-primary);
}

.error-message {
  display: block;
  color: #ff4757;
  font-size: 0.75em;
  margin-top: 2px;
  margin-bottom: 4px;
}

.form-actions {
  display: flex;
  gap: 8px;
  justify-content: flex-end;
  margin-top: 8px;
  padding-top: 8px;
  border-top: 1px solid var(--color-border-secondary);
}

.btn-connect, .btn-cancel {
  padding: 6px 12px;
  border: none;
  border-radius: var(--radius-sm);
  cursor: pointer;
  font-size: 0.9em;
  transition: var(--transition-fast);
}

.btn-connect {
  background: var(--color-primary);
  color: white;
}

.btn-connect:hover {
  background: var(--color-primary-hover);
}

.btn-cancel {
  background: var(--color-bg-tertiary);
  color: var(--color-text-primary);
}

.btn-cancel:hover {
  background: var(--color-interactive-hover);
}
</style>
