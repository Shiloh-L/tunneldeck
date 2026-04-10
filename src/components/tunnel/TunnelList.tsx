import { useConnectionStore } from '@/stores/tunnelStore';
import { TunnelCard } from './TunnelCard';
import { cn } from '@/lib/utils';
import { Network, Plug } from 'lucide-react';
import type { ConnectionInfo } from '@/types';

interface TunnelListProps {
  onEdit: (conn: ConnectionInfo) => void;
  onConnect: (conn: ConnectionInfo) => void;
}

export function TunnelList({ onEdit, onConnect }: TunnelListProps) {
  const { filteredConnections, selectedTagId, tags, isLoading } =
    useConnectionStore();

  const connections = filteredConnections();
  const tagName =
    selectedTagId && tags.find((t) => t.id === selectedTagId)?.name;

  if (isLoading) {
    return (
      <div className='flex-1 flex items-center justify-center'>
        <div className='animate-spin-slow w-6 h-6 border-2 border-accent/30 border-t-accent rounded-full' />
      </div>
    );
  }

  if (connections.length === 0) {
    return (
      <div className='flex-1 flex items-center justify-center'>
        <div className='text-center space-y-3'>
          <div className='w-12 h-12 rounded-2xl bg-bg-card border border-border flex items-center justify-center mx-auto'>
            <Plug size={20} className='text-text-muted' />
          </div>
          <div>
            <p className='text-sm font-medium text-text-secondary'>
              {tagName ? `"${tagName}" 标签下没有连接` : '还没有连接'}
            </p>
            <p className='text-xs text-text-muted mt-1'>
              点击左侧「新建连接」开始使用
            </p>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className='flex-1 overflow-y-auto p-4'>
      {/* Header */}
      <div className='flex items-center justify-between mb-4'>
        <div className='flex items-center gap-2'>
          <Network size={14} className='text-text-muted' />
          <h2 className='text-sm font-semibold text-text-primary'>
            {tagName ?? '全部连接'}
          </h2>
          <span className='text-xs text-text-muted'>
            ({connections.length})
          </span>
        </div>
      </div>

      {/* Grid */}
      <div
        className={cn(
          'grid gap-3',
          'grid-cols-1 lg:grid-cols-2 xl:grid-cols-3',
        )}
      >
        {connections.map((conn) => (
          <TunnelCard
            key={conn.id}
            connection={conn}
            onEdit={onEdit}
            onConnect={onConnect}
          />
        ))}
      </div>
    </div>
  );
}
