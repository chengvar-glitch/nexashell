import { invoke } from '@tauri-apps/api/core';
import { createLogger } from '@/core/utils/logger';
import type { Snippet } from './types';

const logger = createLogger('SNIPPET_API');

class SnippetAPI {
  async addSnippet(
    name: string,
    command: string,
    description?: string
  ): Promise<string> {
    try {
      const id = await invoke<string>('add_snippet', {
        name,
        command,
        description: description || null,
      });
      logger.info('Snippet added', { id, name });
      return id;
    } catch (error) {
      logger.error('Failed to add snippet', error);
      throw error;
    }
  }

  async listSnippets(): Promise<Snippet[]> {
    try {
      const snippets = await invoke<Snippet[]>('list_snippets');
      return snippets || [];
    } catch (error) {
      logger.error('Failed to list snippets', error);
      return [];
    }
  }

  async updateSnippet(
    id: string,
    patch: Partial<
      Pick<Snippet, 'name' | 'command' | 'description' | 'sort'>
    >
  ): Promise<void> {
    try {
      await invoke('update_snippet', {
        id,
        name: patch.name ?? null,
        command: patch.command ?? null,
        description: patch.description ?? null,
        sort: patch.sort ?? null,
      });
      logger.info('Snippet updated', { id });
    } catch (error) {
      logger.error('Failed to update snippet', error);
      throw error;
    }
  }

  async deleteSnippet(id: string): Promise<void> {
    try {
      await invoke('delete_snippet', { id });
      logger.info('Snippet deleted', { id });
    } catch (error) {
      logger.error('Failed to delete snippet', error);
      throw error;
    }
  }
}

export const snippetApi = new SnippetAPI();
