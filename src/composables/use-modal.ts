/**
 * 模态框管理 Composable
 * 提供模态框的打开/关闭状态管理
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
