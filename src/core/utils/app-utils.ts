// This file is deprecated, please use the following modules:
// - @/composables/use-modal
// - @/core/utils/window/window-operations
// - @/features/tabs
// - @/core/utils/platform/platform-detection
// - @/core/utils/event-bus

// This file is kept for backward compatibility
// Temporarily provides backward compatible functions

import {
  closeWindow as closeWindowImpl,
  quitApp as quitAppImpl,
} from '@/features/window';
import { createNewLocalTab as createNewLocalTabImpl } from '@/features/tabs';
import {
  isMacOS as isMacOSImpl,
  isWindows as isWindowsImpl,
} from './platform/platform-detection';
import { eventBus } from './event-bus';
import { APP_EVENTS } from '@/core/constants';

export const quitApp = quitAppImpl;
export const closeWindow = closeWindowImpl;
export const createNewLocalTab = createNewLocalTabImpl;
export const focusSearch = () => eventBus.emit(APP_EVENTS.FOCUS_SEARCH);
export const isMacOSBrowser = isMacOSImpl;
export const isWindowsBrowser = isWindowsImpl;
