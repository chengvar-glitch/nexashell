/**
 * 依赖注入键类型定义
 * 提供类型安全的 provide/inject 机制
 */

import type { InjectionKey, Ref } from 'vue';
import type { TabManagement } from './tab';

// 标签管理注入键
export const TAB_MANAGEMENT_KEY: InjectionKey<TabManagement> = Symbol('tabManagement');

// SSH 表单控制注入键
export const OPEN_SSH_FORM_KEY: InjectionKey<() => void> = Symbol('openSSHForm');
export const CLOSE_SSH_FORM_KEY: InjectionKey<() => void> = Symbol('closeSSHForm');
export const SHOW_SSH_FORM_KEY: InjectionKey<Ref<boolean>> = Symbol('showSSHForm');

// 设置面板控制注入键
export const SHOW_SETTINGS_KEY: InjectionKey<Ref<boolean>> = Symbol('showSettings');
