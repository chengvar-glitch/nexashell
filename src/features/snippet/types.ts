/**
 * Command snippet data model matching the backend `snippets` table.
 */
export interface Snippet {
  id: string;
  name: string;
  command: string;
  description: string;
  sort: number;
  createdAt: string;
  updatedAt: string;
}

/** Draft used by the snippet-editing UI. */
export interface SnippetDraft {
  name: string;
  command: string;
  description?: string;
}
