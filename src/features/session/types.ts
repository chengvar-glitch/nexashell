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
  last_connected_at?: string | null;
  created_at: string;
  updated_at: string;
}

/**
 * Extended session type for UI (including relationships)
 */
export interface SavedSessionDisplay extends SavedSession {
  groups?: string[]; // names
  tags?: string[]; // names
  group_ids?: string[];
  tag_ids?: string[];
  // Transient/UI only properties
  password?: string;
  key_passphrase?: string;
}
