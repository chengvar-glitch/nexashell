import { ref } from 'vue';

/**
 * Transient inline message (toast) state: `show(msg)` displays the text and
 * auto-clears it after the timeout; `clear()` drops it immediately.
 */
export function useToastMessage(timeoutMs = 2200) {
  const message = ref('');
  let timer: ReturnType<typeof setTimeout> | null = null;

  const show = (msg: string) => {
    message.value = msg;
    if (timer) clearTimeout(timer);
    timer = setTimeout(() => {
      message.value = '';
      timer = null;
    }, timeoutMs);
  };

  const clear = () => {
    if (timer) clearTimeout(timer);
    timer = null;
    message.value = '';
  };

  return { message, show, clear };
}
