import { invoke } from '@tauri-apps/api/core';
import type {
  ConnectionInfo,
  Connection,
  CreateConnectionRequest,
  Tag,
  AuditEntry,
  AppSettings,
} from '@/types';

// ─── Connection CRUD ──────────────────────────────────────────────

export const listConnections = () =>
  invoke<ConnectionInfo[]>('list_connections');

export const createConnection = (req: CreateConnectionRequest) =>
  invoke<Connection>('create_connection', { req });

export const updateConnection = (connection: Connection) =>
  invoke<void>('update_connection', { connection });

export const deleteConnection = (connectionId: string) =>
  invoke<void>('delete_connection', { connectionId });

// ─── Connection Control ───────────────────────────────────────────

export const startConnection = (connectionId: string, password?: string) =>
  invoke<void>('start_connection', {
    connectionId,
    password: password ?? null,
  });

export const stopConnection = (connectionId: string) =>
  invoke<void>('stop_connection', { connectionId });

// ─── Tags ─────────────────────────────────────────────────────────

export const listTags = () => invoke<Tag[]>('list_tags');

export const createTag = (name: string, color: string) =>
  invoke<Tag>('create_tag', { name, color });

export const deleteTag = (tagId: string) =>
  invoke<void>('delete_tag', { tagId });

// ─── Password ─────────────────────────────────────────────────────

export const saveConnectionPassword = (
  connectionId: string,
  password: string,
) => invoke<void>('save_connection_password', { connectionId, password });

export const hasStoredPassword = (connectionId: string) =>
  invoke<boolean>('has_stored_password', { connectionId });

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

// ─── Terminal ─────────────────────────────────────────────────────

export const openTerminal = (
  connectionId: string,
  cols: number,
  rows: number,
) => invoke<string>('open_terminal', { connectionId, cols, rows });

export const writeTerminal = (terminalId: string, data: string) =>
  invoke<void>('write_terminal', { terminalId, data });

export const resizeTerminal = (
  terminalId: string,
  cols: number,
  rows: number,
) => invoke<void>('resize_terminal', { terminalId, cols, rows });

export const closeTerminal = (terminalId: string) =>
  invoke<void>('close_terminal', { terminalId });
