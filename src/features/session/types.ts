/**
 * Session data model matching backend database schema
 */
export interface SavedSession {
  id: string;
  addr: string;
  port: number;
  server_name: string;
  username: string;
  auth_type: string; // 'password' | 'key'
  private_key_path?: string | null;
  is_favorite: boolean;
  created_at: string;
  updated_at: string;
}

/**
 * Extended session type for UI (including relationships)
 */
export interface SavedSessionDisplay extends SavedSession {
  groups?: string[];
  tags?: string[];
  // Transient/UI only properties
  password?: string;
  key_passphrase?: string;
}
