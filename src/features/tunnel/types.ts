/**
 * Tunnel types mirroring the Rust backend (camelCase via serde).
 */

export type TunnelDirection = 'local' | 'dynamic';

/**
 * Persistent tunnel rule row (from `list_tunnel_rules`).
 */
export interface TunnelRule {
  id: string;
  sessionId: string;
  direction: TunnelDirection;
  listenHost: string;
  listenPort: number;
  targetHost: string;
  targetPort: number;
  enabled: boolean;
}

/**
 * Runtime status for a tunnel (from `list_tunnel_status` / start commands).
 */
export interface TunnelStatus {
  ruleId: string;
  direction: TunnelDirection;
  listenHost: string;
  listenPort: number;
  targetHost: string;
  targetPort: number;
  state: 'starting' | 'listening' | 'failed' | 'stopped' | string;
  accepted: number;
  error?: string | null;
}

/**
 * UI-only draft for the "add rule" form.
 */
export interface NewTunnelRule {
  direction: TunnelDirection;
  listenHost: string;
  listenPort: number;
  targetHost?: string;
  targetPort?: number;
}
