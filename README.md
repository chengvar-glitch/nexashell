# NexaShell Architecture Design Document

## 📋 Table of Contents

- [Overview](#overview)
- [Architecture Principles](#architecture-principles)
- [Core Architecture](#core-architecture)
- [Component Responsibilities](#component-responsibilities)
- [Data Flow](#data-flow)
- [State Management](#state-management)
- [Event System](#event-system)
- [Extension Guide](#extension-guide)

---

## Overview

NexaShell is a modern SSH terminal tool based on Tauri 2 + Vue 3 + TypeScript, designed with a low-coupling, high-cohesion component architecture.

### Tech Stack

- **Frontend Framework**: Vue 3 (Composition API)
- **Build Tool**: Vite
- **Desktop Framework**: Tauri 2
- **Language**: TypeScript
- **Package Manager**: pnpm
- **Terminal Engine**: xterm.js

---

## Architecture Principles

### 1. Single Responsibility Principle

Each component is responsible for a clear functional domain, without taking on additional responsibilities.

### 2. Dependency Injection Pattern

Decouple components through Vue's `provide/inject` mechanism, avoiding direct dependencies.

### 3. Unidirectional Data Flow

- **Data flows downward**: From parent components to child components via props
- **Events propagate upward**: Child components notify parent components via emit

### 4. Type-Driven

Use TypeScript type identifiers (such as `type` field) instead of string matching to determine logic.

### 5. Event-Driven Architecture

Prioritize custom events for component communication to reduce coupling.

---

## Core Architecture

```
┌─────────────────────────────────────────────────────────┐
│                        App.vue                          │
│  ┌──────────────────────────────────────────────────┐  │
│  │  Global state management & dependency injection │  │
│  │  provider                                    │  │
│  │  - Tab management (tabManagement)              │  │
│  │  - SSH form control (openSSHForm/closeSSHForm) │  │
│  │  - Settings panel control (showSettings)        │  │
│  │  - Shortcut registration (shortcutManager)      │  │
│  │  - Theme management (themeManager)              │  │
│  └──────────────────────────────────────────────────┘  │
│                                                          │
│  ┌──────────────┐  ┌──────────────┐  ┌─────────────┐  │
│  │WindowTitleBar│  │   AppTabs    │  │ AppContent  │  │
│  │ (layout/)    │  │  (layout/)   │  │ (layout/)   │  │
│  └──────────────┘  └──────────────┘  └─────────────┘  │
│                                                          │
│  ┌──────────────────────────────────────────────────┐  │
│  │         Modal Overlay (SSH Form Popup)          │  │
│  │         - SSHConnectionForm (ssh/)              │  │
│  └──────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

---

## Directory Structure

```
src/
├── App.vue                  # Application root component
├── main.ts                  # Application entry
│
├── components/              # Components directory
│   ├── layout/             # Layout components
│   │   ├── AppContent.vue
│   │   ├── AppTabs.vue
│   │   └── WindowTitleBar.vue
│   │
│   ├── terminal/           # Terminal related
│   │   └── TerminalView.vue
│   │
│   ├── ssh/                # SSH connection related
│   │   └── SSHConnectionForm.vue
│   │
│   ├── home/               # Home related
│   │   └── NexaShellHome.vue
│   │
│   ├── settings/           # Settings related
│   │   └── SettingsPanel.vue
│   │
│   ├── search/             # Search related
│   │   ├── SearchBox.vue
│   │   └── SearchDropdown.vue
│   │
│   └── common/             # Common UI components
│       ├── DropdownMenu.vue
│       ├── ShortcutHint.vue
│       └── TabItem.vue
│
├── composables/             # Composable functions
│   ├── use-tab-management.ts
│   ├── use-modal.ts
│   └── index.ts
│
├── constants/               # Constants definition
│   ├── events.ts
│   ├── menu.ts
│   ├── tab.ts
│   └── index.ts
│
├── types/                   # Type definitions
│   ├── tab.ts
│   ├── ssh.ts
│   ├── injection-keys.ts
│   └── index.ts
│
├── utils/                   # Utility functions
│   ├── window/
│   │   └── window-operations.ts
│   ├── platform/
│   │   └── platform-detection.ts
│   ├── tab/
│   │   └── tab-operations.ts
│   ├── error-handler.ts
│   ├── event-bus.ts
│   └── app-utils.ts
│
├── config/                  # Configuration
│   ├── env.ts
│   └── index.ts
│
└── styles/                  # Style files
    ├── common.css
    └── design-system.css
```

### 📦 Packaging Principles

1. **Divide by functional domain**: Related components aggregated together
2. **Single responsibility**: Each directory contains components for specific functions only
3. **Low coupling**: Cross-directory dependencies clearly visible
4. **High cohesion**: Related code centrally managed

---

## Component Responsibilities

### 🎯 App.vue (Coordination Layer)

**Responsibilities**:

- Global state management (tabs, popups, settings)
- Dependency injection providers (provide)
- Shortcut registration and management
- Theme system initialization
- Global event listening and coordination

**Not Responsible For**:

- UI rendering details
- Business logic implementation

**Key Code**:

```typescript
import type { Tab } from '@/types/tab';
import {
  TAB_MANAGEMENT_KEY,
  OPEN_SSH_FORM_KEY,
  CLOSE_SSH_FORM_KEY,
} from '@/types/injection-keys';

// Using type-safe injection keys
provide(TAB_MANAGEMENT_KEY, {
  tabs,
  activeTabId,
  setActiveTab,
  addTab,
  closeTab,
});

provide(OPEN_SSH_FORM_KEY, openSSHForm);
provide(CLOSE_SSH_FORM_KEY, closeSSHForm);
```

---

### 🏠 WindowTitleBar.vue (Title Bar Layer)

**Responsibilities**:

- Window controls (close, minimize, maximize)
- Search box display
- Settings button
- Platform detection (macOS/Windows)

**Dependency Injection**:

- `showSettings` - Settings panel state

---

### 📑 AppTabs.vue (Tab Management Layer)

**Responsibilities**:

- Rendering and displaying tabs
- Tab switching, closing interactions
- New tab button and dropdown menu
- Tab scrolling control

**Dependency Injection**:

- `tabManagement` - Tab management methods
- `openSSHForm` - SSH form opening method

**Not Responsible For**:

- Tab state storage (managed by App.vue)
- Popup rendering (managed by App.vue)

**Key Logic**:

```typescript
// Creating tabs must specify type
const newTab = {
  id: `tab-${Date.now()}-${tabCounter++}`,
  label: `Terminal ${tabCounter}`,
  type: 'terminal' as const, // ✅ Type identifier
  closable: true,
};
```

---

### 📄 AppContent.vue (Content Rendering Layer)

**Responsibilities**:

- Render corresponding components based on the `type` field of the active tab
- Component switching logic

**Dependency Injection**:

- `tabManagement` - Get active tab information

**Core Logic**:

```typescript
const currentComponent = computed(() => {
  const activeTab = tabs.find(tab => tab.id === activeTabId.value);

  // ✅ Determine by type field (decoupled)
  switch (activeTab.type) {
    case 'terminal':
    case 'ssh':
      return TerminalView;
    case 'home':
    default:
      return NexaShellHome;
  }
});
```

**Not Responsible For**:

- Tab state management
- Understanding the meaning of tab labels

---

### 🏡 NexaShellHome.vue (Home UI Layer)

**Responsibilities**:

- Display welcome interface
- Quick action buttons (new connection, recent connections, settings)
- Recent connections list display

**Dependency Injection**:

- `openSSHForm` - Open SSH form method

**Event Passing**:

```typescript
emit('newConnection'); // Notify parent component
emit('openRecent');
emit('openSettings');
emit('connect', connection);
```

**Not Responsible For**:

- Business logic processing
- State management
- Popup control

---

### 📝 SSHConnectionForm.vue (Form Component)

**Responsibilities**:

- SSH connection form UI rendering
- Form validation
- Platform detection (macOS/Windows style differences)

**Event Passing**:

```typescript
emit('connect', formData); // Submit form data
emit('cancel'); // Cancel operation
```

**Not Responsible For**:

- Popup display/hide control (managed by App.vue)
- Actual SSH connection establishment

---

### 🖥️ TerminalView.vue (Terminal Component)

**Responsibilities**:

- Integrate xterm.js
- Terminal session management
- Terminal interaction

**Keep Pure**: Only handle terminal-related logic

---

## Data Flow

### Tab Management Process

```
User clicks "New Tab"
    ↓
AppTabs.handleMenuSelect
    ↓
tabManagement.addTab({ id, label, type, closable })
    ↓
App.vue updates tabs state
    ↓
AppTabs and AppContent automatically respond to updates
    ↓
AppContent renders corresponding component based on type
```

### SSH Form Process

```
User clicks "New Connection" (multiple entry points)
    ↓
Call openSSHForm() (obtained via inject)
    ↓
Trigger App.vue's openSSHForm
    ↓
App.vue sets showSSHForm = true
    ↓
Modal Overlay displays SSHConnectionForm
    ↓
User submits form
    ↓
SSHConnectionForm emit('connect', data)
    ↓
App.vue.handleSSHConnect(data)
    ↓
Close popup & handle connection logic
```

---

## State Management

### Global State (App.vue)

```typescript
// Tab state
const tabs = ref<Tab[]>([...]);
const activeTabId = ref<string>('...');

// Popup state
const showSSHForm = ref(false);
const showSettings = ref(false);
```

### Injection Dependencies

```typescript
// Provided to child components
import {
  TAB_MANAGEMENT_KEY,
  OPEN_SSH_FORM_KEY,
  CLOSE_SSH_FORM_KEY,
  SHOW_SETTINGS_KEY,
} from '@/types/injection-keys';

provide(TAB_MANAGEMENT_KEY, {
  tabs,
  activeTabId,
  setActiveTab,
  addTab,
  closeTab,
});
provide(OPEN_SSH_FORM_KEY, openSSHForm);
provide(CLOSE_SSH_FORM_KEY, closeSSHForm);
provide(SHOW_SETTINGS_KEY, showSettings);

// Used in child components
const openSSHForm = inject(OPEN_SSH_FORM_KEY);
const tabManagement = inject(TAB_MANAGEMENT_KEY);

if (!tabManagement) {
  throw new Error('tabManagement not provided');
}
```

---

## Event System

### Global Events

```typescript
// Register global shortcut events
import { APP_EVENTS } from '@/constants';
import { eventBus } from '@/utils/event-bus';

// Use event bus instead of native event listeners
eventBus.on(APP_EVENTS.OPEN_SETTINGS, () => {...});
eventBus.on(APP_EVENTS.CLOSE_DIALOG, () => {...});
eventBus.on(APP_EVENTS.OPEN_SSH_FORM, () => {...});
eventBus.on(APP_EVENTS.NEW_LOCAL_TAB, () => {...});
eventBus.on(APP_EVENTS.NEW_SSH_TAB, () => {...});
eventBus.on(APP_EVENTS.CLOSE_TAB, () => {...});
eventBus.on(APP_EVENTS.FOCUS_SEARCH, () => {...});

// Send events
import { APP_EVENTS } from '@/constants';
import { eventBus } from '@/utils/event-bus';

eventBus.emit(APP_EVENTS.OPEN_SSH_FORM);
```

### Component Events

```typescript
// Child components pass events to parent
emit('connect', data);
emit('cancel');
emit('click', id);
emit('close', id);
```

---

## Extension Guide

### Adding New Tab Types

#### 1. Extend Tab Interface

```typescript
// src/types/tab.ts
import type { TabType } from '@/types/tab';

export type TabType = 'home' | 'terminal' | 'ssh' | 'editor'; // Add 'editor'

export interface Tab {
  id: string;
  label: string;
  type: TabType;
  closable: boolean;
}
```

#### 2. Add rendering logic in AppContent

```typescript
// AppContent.vue
import CodeEditor from '@/components/ui/CodeEditor.vue';

const currentComponent = computed(() => {
  switch (activeTab.type) {
    case 'terminal':
    case 'ssh':
      return TerminalView;
    case 'editor': // Added
      return CodeEditor;
    case 'home':
    default:
      return NexaShellHome;
  }
});
```

#### 3. Specify type when creating tabs

```typescript
// AppTabs.vue
const newTab = {
  id: `tab-${Date.now()}-${tabCounter++}`,
  label: 'Code Editor',
  type: 'editor' as const,
  closable: true,
};
tabManagement.addTab(newTab);
```

### Adding New Global Shortcuts

#### 1. Define in shortcut-manager.ts

```typescript
export const PredefinedShortcuts = {
  // ... existing shortcuts
  NEW_FEATURE: {
    key: 'n',
    metaKey: true,
    description: 'New feature',
    handler: () => {
      window.dispatchEvent(new CustomEvent('app:new-feature'));
    },
  },
};
```

#### 2. Register and listen in App.vue

```typescript
import { APP_EVENTS } from '@/constants';
import { eventBus } from '@/utils/event-bus';

onMounted(() => {
  shortcutManager.register(PredefinedShortcuts.NEW_FEATURE);

  eventBus.on(APP_EVENTS.NEW_FEATURE, () => {
    // Handle new feature
  });
});
```

### Adding New Global Popups

#### 1. Define state in App.vue

```typescript
const {
  isOpen: showNewModal,
  openModal: openNewModal,
  closeModal: closeNewModal,
} = useModal();
provide('openNewModal', openNewModal);
provide('closeNewModal', closeNewModal);
```

#### 2. Add popup in template

```
<div v-if="showNewModal" class="modal-overlay" @click.self="closeNewModal">
  <NewModalComponent
    @confirm="handleNewModalConfirm"
    @cancel="closeNewModal"
  />
</div>
```

---

## Best Practices

### ✅ Recommended Practices

1. **Use Type Identifiers**: Determine type through `type` field instead of string matching
2. **Dependency Injection**: Pass methods and state via `provide/inject`
3. **Event Driven**: Child components pass events upward via emit
4. **Single Responsibility**: Each component is responsible for a clear function
5. **Type Safe**: Make full use of TypeScript's type system

### ❌ Avoid Practices

1. **String Matching**: Don't determine type via `label.includes('xxx')`
2. **Direct State Manipulation**: Child components shouldn't directly modify global state
3. **Mixed Responsibilities**: Don't handle business logic in UI components
4. **Circular Dependencies**: Avoid mutual references between components
5. **Hard Coding**: Don't repeat the same logic in multiple places

---

## Modular Improvements

To improve code maintainability and testability, we've made modular improvements to the original structure:

### 1. Composables Split

```typescript
// src/composables/use-tab-management.ts
export function useTabManagement() {
  // Tab management logic
}

// src/composables/use-modal.ts
export function useModal() {
  // Modal management logic
}
```

### 2. Utility Function Modularization

```
src/utils/
├── window/
│   └── window-operations.ts  # Window operations
├── platform/
│   └── platform-detection.ts # Platform detection
├── tab/
│   └── tab-operations.ts     # Tab operations
├── error-handler.ts          # Error handling
└── event-bus.ts              # Event bus
```

### 3. Constant Management

```typescript
// src/constants/events.ts
export const APP_EVENTS = {
  OPEN_SETTINGS: 'app:open-settings',
  CLOSE_DIALOG: 'app:close-dialog',
  // ...
} as const;

// src/constants/menu.ts
export const NEW_TAB_MENU_ITEMS = [
  { key: 'local', label: 'Local Terminal', shortcut: 'Cmd+Shift+T' },
  { key: 'ssh', label: 'Remote Connection', shortcut: 'Cmd+T' },
] as const;
```

### 4. Type Safety Improvements

- Use Symbol-typed injection keys
- Unified type definitions
- Strict type checking

---

## Architecture Advantages

### Low Coupling

- Components communicate via interfaces (provide/inject)
- Modifying one component doesn't affect others

### High Cohesion

- Each component has clear responsibilities
- Related functions aggregated together

### Extensible

- Adding new features requires minimal modifications
- Complies with open/closed principle

### Maintainable

- Clear code structure
- Easy to locate and fix issues

### Testable

- Single component responsibilities, easy to write unit tests
- Injectable dependencies, easy to mock

---

## Version Information

- **Document Version**: 2.0.0
- **Project Version**: 0.1.0
- **Update Date**: 2026-01-11
- **Update Content**: Introduced modular architecture, including Composables, utility function modularization, constant management, type safety improvements

---

## Related Resources

- [Vue 3 Documentation](https://vuejs.org/)
- [Tauri 2 Documentation](https://tauri.app/)
- [TypeScript Documentation](https://www.typescriptlang.org/)
- [xterm.js Documentation](https://xtermjs.org/)

---

**Note**: This architecture design follows modern frontend engineering best practices, aiming to build a maintainable and extensible desktop application.
