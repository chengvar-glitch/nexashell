import { invoke } from '@tauri-apps/api/core';
import { createLogger } from '@/core/utils/logger';
import type { TunnelDirection, TunnelRule, TunnelStatus } from './types';

const logger = createLogger('TUNNEL_API');

class TunnelAPI {
  /** Start every enabled rule persisted for the runtime session. */
  async startSessionTunnels(sessionId: string): Promise<TunnelStatus[]> {
    try {
      logger.debug('startSessionTunnels invoke', { sessionId });
      const result = await invoke<TunnelStatus[]>('start_session_tunnels', {
        sessionId,
      } as Record<string, unknown>);
      logger.info('Started session tunnels', { sessionId });
      return result || [];
    } catch (error) {
      logger.error('Failed to start session tunnels', error);
      throw error;
    }
  }

  /** Start a single tunnel rule. */
  async startTunnelRule(
    sessionId: string,
    ruleId: string
  ): Promise<TunnelStatus[]> {
    try {
      logger.debug('startTunnelRule invoke', { sessionId, ruleId });
      const result = await invoke<TunnelStatus[]>('start_tunnel_rule', {
        sessionId,
        ruleId,
      } as Record<string, unknown>);
      return result || [];
    } catch (error) {
      logger.error('Failed to start tunnel rule', error);
      throw error;
    }
  }

  /** Stop a single tunnel rule. */
  async stopTunnelRule(sessionId: string, ruleId: string): Promise<void> {
    try {
      await invoke<void>('stop_tunnel_rule', { sessionId, ruleId });
      logger.info('Stopped tunnel rule', { sessionId, ruleId });
    } catch (error) {
      logger.error('Failed to stop tunnel rule', error);
      throw error;
    }
  }

  /** Stop all running tunnels for the session. */
  async stopSessionTunnels(sessionId: string): Promise<void> {
    try {
      await invoke<void>('stop_session_tunnels', { sessionId });
      logger.info('Stopped session tunnels', { sessionId });
    } catch (error) {
      logger.error('Failed to stop session tunnels', error);
      throw error;
    }
  }

  /** List current runtime status for all tunnels of a session. */
  async listTunnelStatus(sessionId: string): Promise<TunnelStatus[]> {
    try {
      const result = await invoke<TunnelStatus[]>('list_tunnel_status', {
        sessionId,
      } as Record<string, unknown>);
      return result || [];
    } catch (error) {
      // Rethrow so the UI can distinguish "no running tunnels" from a backend
      // error instead of silently showing an empty status.
      logger.error('Failed to list tunnel status', error);
      throw error;
    }
  }

  /** Add a persisted tunnel rule; returns the new rule id. */
  async addTunnelRule(payload: {
    sessionId: string;
    direction: TunnelDirection;
    listenHost: string;
    listenPort: number;
    targetHost: string;
    targetPort: number;
    enabled: boolean;
  }): Promise<string> {
    try {
      logger.debug('addTunnelRule invoke', payload);
      const id = await invoke<string>('add_tunnel_rule', payload);
      logger.info('Tunnel rule added', { id });
      return id;
    } catch (error) {
      logger.error('Failed to add tunnel rule', error);
      throw error;
    }
  }

  /** List persisted tunnel rules for a session. */
  async listTunnelRules(sessionId: string): Promise<TunnelRule[]> {
    try {
      const rules = await invoke<TunnelRule[]>('list_tunnel_rules', {
        sessionId,
      } as Record<string, unknown>);
      return rules || [];
    } catch (error) {
      // Rethrow so the UI can distinguish "no configured rules" from a backend
      // error instead of silently returning an empty list.
      logger.error('Failed to list tunnel rules', error);
      throw error;
    }
  }

  /** Delete a persisted tunnel rule. */
  async deleteTunnelRule(id: string): Promise<void> {
    try {
      await invoke<void>('delete_tunnel_rule', { id });
      logger.info('Tunnel rule deleted', { id });
    } catch (error) {
      logger.error('Failed to delete tunnel rule', error);
      throw error;
    }
  }

  /** Update a persisted tunnel rule (currently just the enabled flag). */
  async updateTunnelRule(id: string, enabled: boolean): Promise<void> {
    try {
      await invoke<void>('update_tunnel_rule', { id, enabled });
      logger.debug('Tunnel rule updated', { id, enabled });
    } catch (error) {
      logger.error('Failed to update tunnel rule', error);
      throw error;
    }
  }
}

export const tunnelApi = new TunnelAPI();
