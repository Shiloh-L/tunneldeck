import { useState, useEffect } from 'react';
import {
  X,
  FileText,
  RefreshCw,
  CheckCircle2,
  XCircle,
  AlertTriangle,
  RotateCw,
  Plus,
  Trash2,
  Pencil,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import * as api from '@/lib/tauri';
import type { AuditEntry, AuditEvent } from '@/types';

interface LogViewerProps {
  onClose: () => void;
}

const EVENT_CONFIG: Record<
  AuditEvent,
  { icon: React.ReactNode; color: string; label: string }
> = {
  connected: {
    icon: <CheckCircle2 size={12} />,
    color: 'text-success',
    label: '已连接',
  },
  disconnected: {
    icon: <XCircle size={12} />,
    color: 'text-text-muted',
    label: '已断开',
  },
  reconnected: {
    icon: <RotateCw size={12} />,
    color: 'text-warning',
    label: '重连',
  },
  error: {
    icon: <AlertTriangle size={12} />,
    color: 'text-danger',
    label: '错误',
  },
  created: {
    icon: <Plus size={12} />,
    color: 'text-accent',
    label: '创建',
  },
  deleted: {
    icon: <Trash2 size={12} />,
    color: 'text-danger',
    label: '删除',
  },
  updated: {
    icon: <Pencil size={12} />,
    color: 'text-accent',
    label: '更新',
  },
};

export function LogViewer({ onClose }: LogViewerProps) {
  const [logs, setLogs] = useState<AuditEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [days, setDays] = useState(7);
  const [filterEvent, setFilterEvent] = useState<AuditEvent | 'all'>('all');

  const load = async () => {
    setLoading(true);
    try {
      const data = await api.getAuditLogs(days);
      setLogs(data);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    load();
  }, [days]);

  const filtered =
    filterEvent === 'all' ? logs : logs.filter((l) => l.event === filterEvent);

  return (
    <div className='fixed inset-0 z-50 flex items-center justify-center animate-fade-in'>
      <div
        className='absolute inset-0 bg-black/60 backdrop-blur-sm'
        onClick={onClose}
      />
      <div
        className={cn(
          'relative w-[600px] max-h-[80vh] flex flex-col',
          'bg-bg-secondary border border-border rounded-2xl shadow-2xl',
          'animate-fade-in',
        )}
      >
        {/* Header */}
        <div className='flex items-center justify-between px-5 pt-5 pb-3 flex-shrink-0'>
          <div className='flex items-center gap-2'>
            <FileText size={14} className='text-accent' />
            <h2 className='text-sm font-semibold text-text-primary'>
              审计日志
            </h2>
            <span className='text-xs text-text-muted'>({filtered.length})</span>
          </div>
          <div className='flex items-center gap-2'>
            <button
              onClick={load}
              className='w-7 h-7 flex items-center justify-center rounded-lg text-text-muted hover:text-text-primary hover:bg-bg-card transition-all'
            >
              <RefreshCw size={13} className={loading ? 'animate-spin' : ''} />
            </button>
            <button
              onClick={onClose}
              className='w-7 h-7 flex items-center justify-center rounded-lg text-text-muted hover:text-text-primary hover:bg-bg-card transition-all'
            >
              <X size={14} />
            </button>
          </div>
        </div>

        {/* Filters */}
        <div className='px-5 pb-3 flex items-center gap-2 flex-shrink-0'>
          {/* Days selector */}
          <select
            value={days}
            onChange={(e) => setDays(Number(e.target.value))}
            className={cn(
              'h-7 px-2 text-[11px] rounded-lg',
              'bg-bg-card border border-border text-text-secondary',
              'focus:outline-none focus:border-border-focus',
            )}
          >
            <option value={1}>今天</option>
            <option value={7}>近 7 天</option>
            <option value={30}>近 30 天</option>
          </select>

          {/* Event filter */}
          <select
            value={filterEvent}
            onChange={(e) =>
              setFilterEvent(e.target.value as AuditEvent | 'all')
            }
            className={cn(
              'h-7 px-2 text-[11px] rounded-lg',
              'bg-bg-card border border-border text-text-secondary',
              'focus:outline-none focus:border-border-focus',
            )}
          >
            <option value='all'>全部事件</option>
            <option value='connected'>连接</option>
            <option value='disconnected'>断开</option>
            <option value='error'>错误</option>
            <option value='created'>创建</option>
            <option value='deleted'>删除</option>
          </select>
        </div>

        {/* Log entries */}
        <div className='flex-1 overflow-y-auto px-5 pb-5'>
          {loading ? (
            <div className='flex items-center justify-center py-12'>
              <div className='animate-spin-slow w-5 h-5 border-2 border-accent/30 border-t-accent rounded-full' />
            </div>
          ) : filtered.length === 0 ? (
            <div className='text-center py-12 text-xs text-text-muted'>
              没有日志记录
            </div>
          ) : (
            <div className='space-y-0.5'>
              {filtered.map((entry, i) => {
                const cfg = EVENT_CONFIG[entry.event];
                return (
                  <div
                    key={i}
                    className={cn(
                      'flex items-start gap-3 px-3 py-2 rounded-lg',
                      'hover:bg-bg-card transition-colors',
                    )}
                  >
                    <span className={cn('mt-0.5 flex-shrink-0', cfg.color)}>
                      {cfg.icon}
                    </span>
                    <div className='flex-1 min-w-0'>
                      <div className='flex items-center gap-2'>
                        <span className='text-xs font-medium text-text-primary truncate'>
                          {entry.tunnel_name}
                        </span>
                        <span
                          className={cn(
                            'text-[10px] font-medium px-1.5 py-0.5 rounded-full',
                            cfg.color,
                            'bg-current/5',
                          )}
                          style={{
                            backgroundColor: `color-mix(in srgb, currentColor 8%, transparent)`,
                          }}
                        >
                          {cfg.label}
                        </span>
                      </div>
                      <p className='text-[11px] text-text-muted truncate mt-0.5'>
                        {entry.message}
                      </p>
                    </div>
                    <span className='text-[10px] text-text-muted flex-shrink-0 tabular-nums'>
                      {formatTime(entry.ts)}
                    </span>
                  </div>
                );
              })}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

function formatTime(iso: string): string {
  try {
    const d = new Date(iso);
    return d.toLocaleString('zh-CN', {
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
    });
  } catch {
    return iso;
  }
}
