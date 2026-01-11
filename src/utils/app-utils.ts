// 此文件已废弃，请使用以下模块:
// - @/composables/use-modal
// - @/utils/window/window-operations
// - @/utils/tab/tab-operations
// - @/utils/platform/platform-detection
// - @/utils/event-bus

// 保留此文件是为了向后兼容
// 临时提供向后兼容的函数

import { closeWindow as closeWindowImpl, quitApp as quitAppImpl } from './window/window-operations';
import { createNewLocalTab as createNewLocalTabImpl } from './tab/tab-operations';
import { isMacOS as isMacOSImpl, isWindows as isWindowsImpl } from './platform/platform-detection';
import { eventBus } from './event-bus';
import { APP_EVENTS } from '@/constants';

export const quitApp = quitAppImpl;
export const closeWindow = closeWindowImpl;
export const createNewLocalTab = createNewLocalTabImpl;
export const focusSearch = () => eventBus.emit(APP_EVENTS.FOCUS_SEARCH);
export const isMacOSBrowser = isMacOSImpl;
export const isWindowsBrowser = isWindowsImpl;
