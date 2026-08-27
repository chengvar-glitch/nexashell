/**
 * Session data model matching backend database schema
 */
export type AuthType = 'password' | 'key';

export interface SavedSession {
  id: string;
  addr: string;
  port: number;
  server_name: string;
  username: string;
  auth_type: AuthType;
  private_key_path?: string | null;
  is_favorite: boolean;
  is_pinned: boolean;
  pinned_at?: string | null;
  last_connected_at?: string | null;
  created_at: string;
  updated_at: string;
}

/**
 * Extended session type for UI (including relationships)
 */
export interface SavedSessionDisplay extends SavedSession {
  groups?: string[]; // names
  group_ids?: string[];
  // Transient/UI only properties
  password?: string;
  key_passphrase?: string;
}

/**
 * Result of importing XTerminal-format session text.
 */
export interface ImportResult {
  imported: number;
  skipped: number;
  failed: string[];
}

/**
 * Payload emitted by the SSH connection form (create/edit/save-only flows).
 */
export interface SSHConnectionFormData {
  id?: string; // session ID, set when editing existing session
  server_name: string;
  addr: string;
  port: number | null;
  username: string;
  // Optional: omitted (undefined) in edit mode when the user left it blank so
  // the backend keeps the stored ciphertext ("unchanged"); `null` means the
  // user explicitly asked to clear the stored value.
  password?: string | null;
  private_key_path: string;
  key_passphrase?: string | null;
  save_session: boolean;
  groups?: string[];
  /** True when the user explicitly asked to clear any stored credentials. */
  clearCredentials?: boolean;
}
