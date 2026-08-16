<template>
  <div v-if="visible" class="palette-overlay" @click.self="closePalette">
    <div class="palette" role="dialog" :aria-label="t('palette.title')">
      <!-- Search header -->
      <div class="palette-search">
        <Search :size="18" class="palette-search-icon" :stroke-width="1.75" />
        <input
          ref="searchInput"
          v-model="query"
          class="palette-input"
          :placeholder="t('palette.searchPlaceholder')"
          autocomplete="off"
          spellcheck="false"
          @input="highlightedIndex = 0"
          @keydown="onSearchKeydown"
        />
        <button class="palette-esc" title="ESC" @click="closePalette">ESC</button>
      </div>

      <!-- Add/edit snippet form -->
      <form v-if="editingId !== null" class="palette-form" @submit.prevent="saveSnippet">
        <input
          v-model="draft.name"
          class="palette-input"
          :placeholder="t('palette.namePlaceholder')"
          required
        />
        <input
          v-model="draft.command"
          class="palette-input"
          :placeholder="t('palette.commandPlaceholder')"
          required
        />
        <textarea
          v-model="draft.description"
          class="palette-input palette-form-textarea"
          :placeholder="t('palette.descriptionPlaceholder')"
          rows="2"
        ></textarea>
        <div class="palette-form-actions">
          <button type="submit" class="palette-btn-primary">
            {{ editingId === 'new' ? t('palette.add') : t('palette.save') }}
          </button>
          <button type="button" class="palette-btn" @click="editingId = null">
            {{ t('common.cancel') }}
          </button>
        </div>
      </form>

      <!-- Snippet list -->
      <div class="palette-body">
        <div v-if="filteredSnippets.length === 0" class="palette-empty">
          <Terminal :size="28" class="palette-empty-icon" :stroke-width="1.5" />
          <p>{{ query ? t('palette.noMatches') : t('palette.empty') }}</p>
          <button type="button" class="palette-btn-primary palette-empty-add" @click="startAdd">
            {{ t('palette.add') }}
          </button>
        </div>
        <div v-else>
          <div
            v-for="(snippet, i) in filteredSnippets"
            :key="snippet.id"
            class="palette-item"
            :class="{ 'palette-item-active': i === highlightedIndex }"
            @click="executeSnippet(snippet)"
            @mouseenter="highlightedIndex = i"
          >
            <div class="palette-item-main">
              <Terminal :size="16" class="palette-item-icon" :stroke-width="1.75" />
              <div class="palette-item-text">
                <span class="palette-item-name">{{ snippet.name }}</span>
                <span v-if="snippet.description" class="palette-item-desc">
                  {{ snippet.description }}
                </span>
              </div>
            </div>
            <div class="palette-item-actions" @click.stop>
              <button
                type="button"
                class="palette-icon-btn"
                :title="t('palette.edit')"
                @click="startEdit(snippet)"
              >
                <Pencil :size="14" :stroke-width="1.75" />
              </button>
              <button
                type="button"
                class="palette-icon-btn palette-icon-btn-danger"
                :title="t('palette.delete')"
                @click="removeSnippet(snippet)"
              >
                <Trash2 :size="14" :stroke-width="1.75" />
              </button>
            </div>
          </div>
        </div>
      </div>

      <!-- Footer hint -->
      <div class="palette-footer">
        <span v-if="query">{{ filteredSnippets.length }} {{ t('palette.matches') }}</span>
        <span v-else>{{ t('palette.footerHint') }}</span>
        <span class="palette-footer-spacer" />
        <span v-if="activeSessionHint" class="palette-session-hint">
          {{ activeSessionHint }}
        </span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, inject, nextTick, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { Pencil, Search, Terminal, Trash2 } from 'lucide-vue-next';
import { TAB_MANAGEMENT_KEY } from '@/core/types';
import { sessionApi, useSessionStore } from '@/features/session';
import { snippetApi } from '@/features/snippet';
import type { Snippet, SnippetDraft } from '@/features/snippet';
import { createLogger } from '@/core/utils/logger';

const logger = createLogger('COMMAND_PALETTE');

const props = defineProps<{ visible: boolean }>();
const emit = defineEmits<{ 'update:visible': [value: boolean] }>();

const { t } = useI18n();
const searchInput = ref<HTMLInputElement | null>(null);
const query = ref('');
const snippets = ref<Snippet[]>([]);
const highlightedIndex = ref(0);
/** Null = not editing; 'new' = adding; a string id = editing that snippet. */
const editingId = ref<string | null | 'new'>(null);
const draft = ref<SnippetDraft>({ name: '', command: '', description: '' });
const activeSessionHint = ref('');

interface TabLike {
  id: string;
  type: string;
  panes?: Array<{ id: string; type?: string }>;
}
interface TabManagementLike {
  tabs: { value: TabLike[] };
  activeTabId: { value: string };
}
const tabManagement = (inject(TAB_MANAGEMENT_KEY) as TabManagementLike | null) ?? null;
const sessionStore = useSessionStore();

const filteredSnippets = computed<Snippet[]>(() => {
  const q = query.value.trim().toLowerCase();
  if (!q) return snippets.value;
  return snippets.value.filter(
    s =>
      s.name.toLowerCase().includes(q) ||
      s.command.toLowerCase().includes(q) ||
      (s.description || '').toLowerCase().includes(q)
  );
});

/** Resolve the id of the currently-focused SSH session, if any. */
function activeSshSessionId(): string | null {
  if (!tabManagement) return null;
  const tab = tabManagement.tabs.value.find(
    it => it.id === tabManagement.activeTabId.value
  );
  if (!tab || tab.type !== 'ssh') return null;
  const panes = tab.panes || [];
  if (panes.length === 0) return null;
  // The active pane, falling back to the first pane. SSH pane ids equal the
  // runtime session ids (see App.vue connect flow).
  const pane = panes[panes.length - 1];
  const session = sessionStore.getSession(pane.id);
  return session && session.type === 'ssh' ? pane.id : null;
}

async function refreshActiveHint() {
  const sid = activeSshSessionId();
  if (sid) activeSessionHint.value = t('palette.executingInto', {
    session: sessionStore.getSession(sid)?.connectionParams?.serverName || sid,
  });
  else activeSessionHint.value = t('palette.noActiveSession');
}

async function refreshSnippets() {
  snippets.value = await snippetApi.listSnippets();
}

const clearQuery = () => {
  query.value = '';
  editingId.value = null;
  highlightedIndex.value = 0;
};

const closePalette = () => {
  clearQuery();
  emit('update:visible', false);
};

function onSearchKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape') {
    if (editingId.value !== null) {
      editingId.value = null;
    } else {
      closePalette();
    }
    e.preventDefault();
    return;
  }
  if (editingId.value !== null) return;
  if (e.key === 'ArrowDown') {
    highlightedIndex.value = Math.min(
      highlightedIndex.value + 1,
      Math.max(filteredSnippets.value.length - 1, 0)
    );
    e.preventDefault();
  } else if (e.key === 'ArrowUp') {
    highlightedIndex.value = Math.max(highlightedIndex.value - 1, 0);
    e.preventDefault();
  } else if (e.key === 'Enter') {
    const target = filteredSnippets.value[highlightedIndex.value];
    if (target) executeSnippet(target);
    e.preventDefault();
  }
}

async function executeSnippet(snippet: Snippet) {
  const sid = activeSshSessionId();
  if (!sid) return;
  try {
    // Send the snippet command followed by Enter, mirroring "paste + run".
    await sessionApi.sendSSHInput(sid, `${snippet.command}\r`);
    logger.info('Executed snippet', { id: snippet.id, name: snippet.name });
    closePalette();
  } catch (error) {
    logger.error('Failed to execute snippet', error);
  }
}

const startAdd = () => {
  editingId.value = 'new';
  draft.value = { name: '', command: '', description: '' };
};

const startEdit = (snippet: Snippet) => {
  editingId.value = snippet.id;
  draft.value = {
    name: snippet.name,
    command: snippet.command,
    description: snippet.description,
  };
};

async function saveSnippet() {
  if (!draft.value.name.trim() || !draft.value.command.trim()) return;
  try {
    if (editingId.value === 'new') {
      await snippetApi.addSnippet(
        draft.value.name,
        draft.value.command,
        draft.value.description || ''
      );
    } else if (editingId.value) {
      await snippetApi.updateSnippet(editingId.value, {
        name: draft.value.name,
        command: draft.value.command,
        description: draft.value.description || '',
      });
    }
    editingId.value = null;
    await refreshSnippets();
  } catch (error) {
    logger.error('Failed to save snippet', error);
  }
}

async function removeSnippet(snippet: Snippet) {
  try {
    await snippetApi.deleteSnippet(snippet.id);
    await refreshSnippets();
  } catch (error) {
    logger.error('Failed to delete snippet', error);
  }
}

watch(
  () => props.visible,
  async shown => {
    if (shown) {
      clearQuery();
      await refreshSnippets();
      await refreshActiveHint();
      await nextTick();
      searchInput.value?.focus();
    }
  }
);
</script>

<style scoped>
.palette-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: var(--color-bg-overlay);
  display: flex;
  align-items: flex-start;
  justify-content: center;
  padding-top: 12vh;
  z-index: 2000;
  animation: palette-fade var(--transition-fast);
}

@keyframes palette-fade {
  from {
    opacity: 0;
  }
  to {
    opacity: 1;
  }
}

.palette {
  width: min(600px, 90vw);
  max-height: 70vh;
  background: var(--color-bg-elevated);
  border: 1px solid var(--color-border-primary);
  border-radius: var(--radius-lg);
  box-shadow: var(--shadow-xl);
  overflow: hidden;
  display: flex;
  flex-direction: column;
  animation: palette-drop var(--transition-base);
}

@keyframes palette-drop {
  from {
    opacity: 0;
    transform: translateY(-8px) scale(0.98);
  }
  to {
    opacity: 1;
    transform: translateY(0) scale(1);
  }
}

.palette-search {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 14px 16px;
  border-bottom: 1px solid var(--color-border-secondary);
}

.palette-search-icon {
  color: var(--color-text-tertiary);
  flex-shrink: 0;
}

.palette-input {
  flex: 1;
  min-width: 0;
  border: none;
  outline: none;
  background: transparent;
  color: var(--color-text-primary);
  font-size: 15px;
  padding: 4px 0;
}

.palette-input::placeholder {
  color: var(--color-text-tertiary);
}

.palette-esc {
  border: none;
  background: var(--color-bg-secondary);
  color: var(--color-text-secondary);
  border-radius: var(--radius-xs);
  padding: 2px 8px;
  font-size: 11px;
  cursor: pointer;
  font-family: var(--font-mono);
}

.palette-form {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 12px 16px;
  border-bottom: 1px solid var(--color-border-secondary);
  background: var(--color-bg-secondary);
}

.palette-form-input,
.palette-form-textarea {
  border: 1px solid var(--color-border-secondary);
  border-radius: var(--radius-sm);
  padding: 7px 10px;
  background: var(--color-bg-elevated);
}

.palette-form-textarea {
  resize: vertical;
  font-family: inherit;
  line-height: 1.5;
}

.palette-form-actions {
  display: flex;
  gap: 8px;
  justify-content: flex-end;
}

.palette-btn,
.palette-btn-primary {
  border: none;
  border-radius: var(--radius-sm);
  padding: 7px 14px;
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  transition: var(--transition-fast);
}

.palette-btn {
  background: var(--color-bg-secondary);
  color: var(--color-text-primary);
  border: 1px solid var(--color-border-secondary);
}

.palette-btn:hover {
  background: var(--color-interactive-hover);
}

.palette-btn-primary {
  background: var(--color-primary);
  color: white;
}

.palette-btn-primary:hover {
  background: var(--color-primary-hover);
}

.palette-body {
  flex: 1;
  overflow-y: auto;
  padding: 6px;
  min-height: 120px;
}

.palette-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 10px;
  padding: 32px 16px;
  color: var(--color-text-secondary);
  font-size: 14px;
}

.palette-empty-icon {
  color: var(--color-text-tertiary);
}

.palette-empty-add {
  margin-top: 4px;
}

.palette-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  padding: 9px 12px;
  border-radius: var(--radius-sm);
  cursor: pointer;
}

.palette-item-active {
  background: var(--color-interactive-selected);
}

.palette-item-main {
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 0;
}

.palette-item-icon {
  color: var(--color-text-tertiary);
  flex-shrink: 0;
}

.palette-item-text {
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.palette-item-name {
  color: var(--color-text-primary);
  font-size: 14px;
  font-weight: 500;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.palette-item-desc {
  color: var(--color-text-tertiary);
  font-size: 12px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.palette-item-actions {
  display: flex;
  gap: 4px;
  opacity: 0;
  transition: opacity var(--transition-fast);
}

.palette-item:hover .palette-item-actions {
  opacity: 1;
}

.palette-icon-btn {
  border: none;
  background: transparent;
  color: var(--color-text-secondary);
  padding: 4px;
  border-radius: var(--radius-xs);
  cursor: pointer;
  display: flex;
  align-items: center;
  transition: var(--transition-fast);
}

.palette-icon-btn:hover {
  background: var(--color-interactive-hover);
  color: var(--color-text-primary);
}

.palette-icon-btn-danger:hover {
  background: rgba(255, 59, 48, 0.12);
  color: var(--color-danger);
}

.palette-footer {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 16px;
  border-top: 1px solid var(--color-border-secondary);
  color: var(--color-text-tertiary);
  font-size: 12px;
}

.palette-footer-spacer {
  flex: 1;
}

.palette-session-hint {
  color: var(--color-text-secondary);
  max-width: 60%;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.palette-kbd {
  font-family: var(--font-mono);
  font-size: 11px;
  background: var(--color-bg-secondary);
  border: 1px solid var(--color-border-secondary);
  border-radius: var(--radius-xs);
  padding: 1px 6px;
  color: var(--color-text-secondary);
}
</style>

