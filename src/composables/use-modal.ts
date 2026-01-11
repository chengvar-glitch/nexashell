/**
 * Modal management Composable
 * Provides modal open/close state management
 */

import { ref } from 'vue';

export function useModal() {
  const isOpen = ref(false);

  const openModal = () => {
    isOpen.value = true;
  };

  const closeModal = () => {
    isOpen.value = false;
  };

  const toggleModal = () => {
    isOpen.value = !isOpen.value;
  };

  return {
    isOpen,
    openModal,
    closeModal,
    toggleModal,
  };
}
