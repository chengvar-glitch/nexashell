/**
 * Dependency injection key type definitions
 * Provides type-safe provide/inject mechanism
 */

import type { InjectionKey, Ref } from 'vue';
import type { TabManagement } from '@/features/tabs';
import type { SplitNode } from '@/features/tabs';

// Tab management injection key
export const TAB_MANAGEMENT_KEY: InjectionKey<TabManagement> =
  Symbol('tabManagement');

// Split-pane drag resize commit — writes dragged sizes back into the tree
export const PANE_SIZES_COMMIT_KEY: InjectionKey<
  (node: SplitNode, sizes: number[]) => void
> = Symbol('paneSizesCommit');

// SSH form control injection keys
export const OPEN_SSH_FORM_KEY: InjectionKey<() => void> =
  Symbol('openSSHForm');
export const CLOSE_SSH_FORM_KEY: InjectionKey<() => void> =
  Symbol('closeSSHForm');
export const SHOW_SSH_FORM_KEY: InjectionKey<Ref<boolean>> =
  Symbol('showSSHForm');

// Settings panel control injection key
export const SHOW_SETTINGS_KEY: InjectionKey<Ref<boolean>> =
  Symbol('showSettings');
