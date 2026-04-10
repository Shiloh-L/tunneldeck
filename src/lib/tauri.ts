import { invoke } from '@tauri-apps/api/core';
import type {
  TunnelInfo,
  TunnelConfig,
  CreateTunnelRequest,
  Tag,
  AuditEntry,
  AppSettings,
} from '@/types';

// ─── Tunnel CRUD ──────────────────────────────────────────────────

export const listTunnels = () => invoke<TunnelInfo[]>('list_tunnels');

export const createTunnel = (req: CreateTunnelRequest) =>
  invoke<TunnelConfig>('create_tunnel', { req });

export const updateTunnel = (tunnel: TunnelConfig) =>
  invoke<void>('update_tunnel', { tunnel });

export const deleteTunnel = (tunnelId: string) =>
  invoke<void>('delete_tunnel', { tunnelId });

// ─── Tunnel Control ───────────────────────────────────────────────

export const startTunnel = (tunnelId: string, password?: string) =>
  invoke<void>('start_tunnel', { tunnelId, password: password ?? null });

export const stopTunnel = (tunnelId: string) =>
  invoke<void>('stop_tunnel', { tunnelId });

// ─── Tags ─────────────────────────────────────────────────────────

export const listTags = () => invoke<Tag[]>('list_tags');

export const createTag = (name: string, color: string) =>
  invoke<Tag>('create_tag', { name, color });

export const deleteTag = (tagId: string) =>
  invoke<void>('delete_tag', { tagId });

// ─── Password ─────────────────────────────────────────────────────

export const saveTunnelPassword = (tunnelId: string, password: string) =>
  invoke<void>('save_tunnel_password', { tunnelId, password });

export const hasStoredPassword = (tunnelId: string) =>
  invoke<boolean>('has_stored_password', { tunnelId });

// ─── Audit Logs ───────────────────────────────────────────────────

export const getAuditLogs = (days?: number) =>
  invoke<AuditEntry[]>('get_audit_logs', { days: days ?? null });

// ─── Settings ─────────────────────────────────────────────────────

export const getSettings = () => invoke<AppSettings>('get_settings');

export const updateSettings = (settings: AppSettings) =>
  invoke<void>('update_settings', { settings });

// ─── Import / Export ──────────────────────────────────────────────

export const exportConfig = () => invoke<string>('export_config');

export const importConfig = (json: string) =>
  invoke<number>('import_config', { json });
