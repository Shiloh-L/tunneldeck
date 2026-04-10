import { useState } from 'react';
import {
  Play,
  Square,
  Pencil,
  Trash2,
  MoreHorizontal,
  ArrowRight,
  Clock,
  Copy,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import type { ConnectionInfo, ConnectionStatus } from '@/types';
import * as api from '@/lib/tauri';
import { useConnectionStore } from '@/stores/tunnelStore';

interface TunnelCardProps {
  connection: ConnectionInfo;
  onEdit: (conn: ConnectionInfo) => void;
  onConnect: (conn: ConnectionInfo) => void;
}

export function TunnelCard({ connection, onEdit, onConnect }: TunnelCardProps) {
  const [showMenu, setShowMenu] = useState(false);
  const { tags, loadConnections } = useConnectionStore();

  const connTags = tags.filter((t) => connection.tag_ids.includes(t.id));
  const enabledForwards = connection.forwards.filter((f) => f.enabled);

  const isActive =
    connection.status === 'connected' ||
    connection.status === 'connecting' ||
    connection.status === 'waitingduo' ||
    connection.status === 'reconnecting';

  const handleToggle = async () => {
    if (isActive) {
      await api.disconnectTunnel(connection.id);
    } else {
      onConnect(connection);
    }
  };

  const handleDelete = async () => {
    setShowMenu(false);
    await api.deleteConnection(connection.id);
    await loadConnections();
  };

  const handleCopyAll = async () => {
    const text = enabledForwards
      .map((f) => `localhost:${f.local_port}`)
      .join('\n');
    await navigator.clipboard.writeText(text);
  };

  return (
    <div
      className={cn(
        'group relative rounded-xl border transition-all duration-200',
        'animate-fade-in',
        connection.status === 'connected'
          ? 'border-success/20 bg-success/[0.03]'
          : connection.status === 'error'
            ? 'border-danger/20 bg-danger/[0.03]'
            : 'border-border bg-bg-card hover:bg-bg-card-hover hover:border-border-focus/30',
      )}
    >
      <div className='p-4'>
        {/* Header row: name + status + controls */}
        <div className='flex items-start justify-between gap-3 mb-3'>
          <div className='flex items-center gap-2.5 min-w-0'>
            <StatusDot status={connection.status} />
            <h3 className='text-sm font-semibold text-text-primary truncate'>
              {connection.name}
            </h3>
          </div>

          <div className='flex items-center gap-1 flex-shrink-0'>
            {/* Connect/Disconnect button */}
            <button
              onClick={handleToggle}
              className={cn(
                'w-7 h-7 flex items-center justify-center rounded-lg',
                'transition-all duration-150',
                isActive
                  ? 'bg-danger/10 text-danger hover:bg-danger/20'
                  : 'bg-accent/10 text-accent hover:bg-accent/20',
              )}
              title={isActive ? '断开连接' : '连接'}
            >
              {isActive ? <Square size={12} /> : <Play size={12} />}
            </button>

            {/* More menu */}
            <div className='relative'>
              <button
                onClick={() => setShowMenu(!showMenu)}
                className={cn(
                  'w-7 h-7 flex items-center justify-center rounded-lg',
                  'text-text-muted hover:text-text-secondary hover:bg-bg-elevated',
                  'transition-all opacity-0 group-hover:opacity-100',
                )}
              >
                <MoreHorizontal size={14} />
              </button>

              {showMenu && (
                <>
                  <div
                    className='fixed inset-0 z-10'
                    onClick={() => setShowMenu(false)}
                  />
                  <div
                    className={cn(
                      'absolute right-0 top-8 z-20 w-36',
                      'bg-bg-elevated border border-border rounded-lg shadow-xl',
                      'py-1 animate-fade-in',
                    )}
                  >
                    <MenuButton
                      icon={<Pencil size={12} />}
                      label='编辑'
                      onClick={() => {
                        setShowMenu(false);
                        onEdit(connection);
                      }}
                    />
                    <MenuButton
                      icon={<Copy size={12} />}
                      label='复制地址'
                      onClick={() => {
                        setShowMenu(false);
                        handleCopyAll();
                      }}
                    />
                    <div className='h-px bg-border mx-2 my-1' />
                    <MenuButton
                      icon={<Trash2 size={12} />}
                      label='删除'
                      danger
                      onClick={handleDelete}
                    />
                  </div>
                </>
              )}
            </div>
          </div>
        </div>

        {/* Forward rules */}
        <div className='space-y-1 mb-2'>
          {enabledForwards.map((fwd) => (
            <div
              key={fwd.id}
              className={cn(
                'flex items-center gap-2 text-xs font-mono',
                'text-text-secondary',
              )}
            >
              <span className='text-text-muted truncate max-w-[60px]'>{fwd.name || '—'}</span>
              <span>:{fwd.local_port}</span>
              <ArrowRight size={10} className='text-text-muted flex-shrink-0' />
              <span className='truncate'>
                {fwd.target_host}:{fwd.target_port}
              </span>
            </div>
          ))}
        </div>

        {/* Jump host */}
        <div className='text-[11px] text-text-muted mb-2.5'>
          via {connection.username}@{connection.host}
        </div>

        {/* Footer: tags + uptime */}
        <div className='flex items-center justify-between'>
          <div className='flex items-center gap-1 flex-wrap'>
            {connTags.map((tag) => (
              <span
                key={tag.id}
                className='text-[10px] font-medium px-1.5 py-0.5 rounded-full'
                style={{
                  color: tag.color,
                  background: `${tag.color}15`,
                }}
              >
                {tag.name}
              </span>
            ))}
          </div>

          {connection.status === 'connected' && connection.uptime_secs != null && (
            <div className='flex items-center gap-1 text-[10px] text-text-muted'>
              <Clock size={10} />
              <span>{formatUptime(connection.uptime_secs)}</span>
            </div>
          )}

          {connection.status === 'error' && connection.error_message && (
            <span className='text-[10px] text-danger truncate max-w-[180px]'>
              {connection.error_message}
            </span>
          )}
        </div>
      </div>
    </div>
  );
}

// ─── Sub-components ───────────────────────────────────────────────

function StatusDot({ status }: { status: ConnectionStatus }) {
  return <div className={cn('status-dot', `status-dot-${status}`)} />;
}

function MenuButton({
  icon,
  label,
  danger,
  onClick,
}: {
  icon: React.ReactNode;
  label: string;
  danger?: boolean;
  onClick: () => void;
}) {
  return (
    <button
      onClick={onClick}
      className={cn(
        'w-full flex items-center gap-2 px-3 py-1.5 text-xs',
        'transition-colors',
        danger
          ? 'text-danger hover:bg-danger/10'
          : 'text-text-secondary hover:text-text-primary hover:bg-bg-card',
      )}
    >
      {icon}
      {label}
    </button>
  );
}

function formatUptime(secs: number): string {
  if (secs < 60) return `${secs}s`;
  if (secs < 3600) return `${Math.floor(secs / 60)}m`;
  const h = Math.floor(secs / 3600);
  const m = Math.floor((secs % 3600) / 60);
  return `${h}h ${m}m`;
}
