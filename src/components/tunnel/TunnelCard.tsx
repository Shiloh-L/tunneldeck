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
  Check,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import type { TunnelInfo, TunnelStatus } from '@/types';
import * as api from '@/lib/tauri';
import { useTunnelStore } from '@/stores/tunnelStore';

interface TunnelCardProps {
  tunnel: TunnelInfo;
  onEdit: (tunnel: TunnelInfo) => void;
  onConnect: (tunnel: TunnelInfo) => void;
}

export function TunnelCard({ tunnel, onEdit, onConnect }: TunnelCardProps) {
  const [showMenu, setShowMenu] = useState(false);
  const [copied, setCopied] = useState(false);
  const { tags, loadTunnels } = useTunnelStore();

  const tunnelTags = tags.filter((t) => tunnel.tag_ids.includes(t.id));

  const isActive =
    tunnel.status === 'connected' ||
    tunnel.status === 'connecting' ||
    tunnel.status === 'waitingduo' ||
    tunnel.status === 'reconnecting';

  const handleToggle = async () => {
    if (isActive) {
      await api.stopTunnel(tunnel.id);
    } else {
      onConnect(tunnel);
    }
  };

  const handleDelete = async () => {
    setShowMenu(false);
    await api.deleteTunnel(tunnel.id);
    await loadTunnels();
  };

  const handleCopyLocal = async () => {
    await navigator.clipboard.writeText(`localhost:${tunnel.local_port}`);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  };

  return (
    <div
      className={cn(
        'group relative rounded-xl border transition-all duration-200',
        'animate-fade-in',
        tunnel.status === 'connected'
          ? 'border-success/20 bg-success/[0.03]'
          : tunnel.status === 'error'
            ? 'border-danger/20 bg-danger/[0.03]'
            : 'border-border bg-bg-card hover:bg-bg-card-hover hover:border-border-focus/30',
      )}
    >
      <div className='p-4'>
        {/* Header row: name + status + controls */}
        <div className='flex items-start justify-between gap-3 mb-3'>
          <div className='flex items-center gap-2.5 min-w-0'>
            <StatusDot status={tunnel.status} />
            <h3 className='text-sm font-semibold text-text-primary truncate'>
              {tunnel.name}
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
                        onEdit(tunnel);
                      }}
                    />
                    <MenuButton
                      icon={<Copy size={12} />}
                      label='复制地址'
                      onClick={() => {
                        setShowMenu(false);
                        handleCopyLocal();
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

        {/* Route info */}
        <div
          className={cn(
            'flex items-center gap-2 text-xs font-mono',
            'text-text-secondary mb-2',
          )}
        >
          <button
            onClick={handleCopyLocal}
            className='hover:text-accent transition-colors flex items-center gap-1'
            title='点击复制'
          >
            {copied ? (
              <Check size={10} className='text-success' />
            ) : (
              <Copy size={10} className='opacity-0 group-hover:opacity-100' />
            )}
            <span>:{tunnel.local_port}</span>
          </button>
          <ArrowRight size={10} className='text-text-muted' />
          <span className='truncate'>
            {tunnel.target_host}:{tunnel.target_port}
          </span>
        </div>

        {/* Jump host */}
        <div className='text-[11px] text-text-muted mb-2.5'>
          via {tunnel.username}@{tunnel.jump_host}
        </div>

        {/* Footer: tags + uptime */}
        <div className='flex items-center justify-between'>
          <div className='flex items-center gap-1 flex-wrap'>
            {tunnelTags.map((tag) => (
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

          {tunnel.status === 'connected' && tunnel.uptime_secs != null && (
            <div className='flex items-center gap-1 text-[10px] text-text-muted'>
              <Clock size={10} />
              <span>{formatUptime(tunnel.uptime_secs)}</span>
            </div>
          )}

          {tunnel.status === 'error' && tunnel.error_message && (
            <span className='text-[10px] text-danger truncate max-w-[180px]'>
              {tunnel.error_message}
            </span>
          )}
        </div>
      </div>
    </div>
  );
}

// ─── Sub-components ───────────────────────────────────────────────

function StatusDot({ status }: { status: TunnelStatus }) {
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
