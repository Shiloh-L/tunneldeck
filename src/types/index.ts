// ─── Tunnel types (mirror Rust types) ─────────────────────────────

export interface TunnelConfig {
  id: string;
  name: string;
  jump_host: string;
  jump_port: number;
  username: string;
  target_host: string;
  target_port: number;
  local_port: number;
  auto_connect: boolean;
  tag_ids: string[];
  created_at: string;
  updated_at: string;
}

export type TunnelStatus =
  | 'disconnected'
  | 'connecting'
  | 'waitingduo'
  | 'connected'
  | 'reconnecting'
  | 'error';

export interface TunnelInfo extends TunnelConfig {
  status: TunnelStatus;
  error_message?: string;
  uptime_secs?: number;
}

export interface CreateTunnelRequest {
  name: string;
  jump_host: string;
  jump_port: number;
  username: string;
  target_host: string;
  target_port: number;
  local_port: number;
  password: string;
  auto_connect: boolean;
  tag_ids: string[];
}

// ─── Tag types ────────────────────────────────────────────────────

export interface Tag {
  id: string;
  name: string;
  color: string;
}

// ─── Audit types ──────────────────────────────────────────────────

export type AuditEvent =
  | 'connected'
  | 'disconnected'
  | 'reconnected'
  | 'error'
  | 'created'
  | 'deleted'
  | 'updated';

export interface AuditEntry {
  tunnel_id: string;
  tunnel_name: string;
  event: AuditEvent;
  message: string;
  ts: string;
}

// ─── Settings ─────────────────────────────────────────────────────

export interface AppSettings {
  api_enabled: boolean;
  api_token: string | null;
  api_port: number;
  auto_start_tunnels: boolean;
  health_check_interval_secs: number;
  max_reconnect_attempts: number;
  log_retention_days: number;
}

// ─── Auth status events from Rust ─────────────────────────────────

export interface AuthStatusEvent {
  tunnelId: string;
  status: 'prompting_password' | 'waiting_duo_push' | 'success' | 'failed';
  message: string;
}

export interface TunnelStatusEvent {
  tunnelId: string;
  status: TunnelStatus;
  error?: string;
}
