// ─── Connection types (mirror Rust types) ─────────────────────────

export interface ForwardRule {
  id: string;
  name: string;
  local_port: number;
  target_host: string;
  target_port: number;
  enabled: boolean;
}

export type AuthMethod = 'password' | 'key';

export interface Connection {
  id: string;
  name: string;
  host: string;
  port: number;
  username: string;
  auth_method: AuthMethod;
  private_key_path: string | null;
  forwards: ForwardRule[];
  auto_connect: boolean;
  tag_ids: string[];
  created_at: string;
  updated_at: string;
}

export type ConnectionStatus =
  | 'disconnected'
  | 'connecting'
  | 'waitingduo'
  | 'connected'
  | 'reconnecting'
  | 'error';

export interface ConnectionInfo extends Connection {
  status: ConnectionStatus;
  error_message?: string;
  uptime_secs?: number;
  running_forward_ids: string[];
}

export interface CreateConnectionRequest {
  name: string;
  host: string;
  port: number;
  username: string;
  password: string;
  auth_method: AuthMethod;
  private_key_path: string | null;
  forwards: Omit<ForwardRule, 'id'>[];
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
  connection_id: string;
  connection_name: string;
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

export interface ConnectionStatusEvent {
  connectionId: string;
  status: ConnectionStatus;
  error?: string;
}

export interface AuthStatusEvent {
  connectionId: string;
  status: 'prompting_password' | 'waiting_duo_push' | 'success' | 'failed';
  message: string;
}

// ─── Terminal types ───────────────────────────────────────────────

export interface TerminalSession {
  terminalId: string;
  connectionId: string;
  connectionName: string;
}

export interface TerminalDataEvent {
  terminalId: string;
  data: string;
}

export interface TerminalExitEvent {
  terminalId: string;
  connectionId: string;
}
