import { invoke } from '@tauri-apps/api/core';

export async function quitApp() {
  try {
    await invoke('quit_app');
  } catch (error) {
    console.error('Failed to quit app:', error);
  }
}

export async function toggleMaximize() {
  try {
    await invoke('toggle_maximize');
  } catch (error) {
    console.error('Failed to toggle maximize:', error);
  }
}

export async function minimizeWindow() {
  try {
    await invoke('minimize_window');
  } catch (error) {
    console.error('Failed to minimize window:', error);
  }
}

export async function closeWindow() {
  try {
    await invoke('close_window');
  } catch (error) {
    console.error('Failed to close window:', error);
  }
}

export function createNewLocalTab() {
  window.dispatchEvent(new CustomEvent('app:new-local-tab'));
}

export function createNewSSHTab() {
  window.dispatchEvent(new CustomEvent('app:new-ssh-tab'));
}

export function focusSearch() {
  window.dispatchEvent(new CustomEvent('app:focus-search'));
}