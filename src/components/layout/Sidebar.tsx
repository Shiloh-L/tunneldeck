import { Network, Search, Plus, Settings, FileText } from 'lucide-react';
import { cn } from '@/lib/utils';
import { useConnectionStore } from '@/stores/tunnelStore';

interface SidebarProps {
  onNewConnection: () => void;
  onOpenSettings: () => void;
  onOpenLogs: () => void;
}

export function Sidebar({
  onNewConnection,
  onOpenSettings,
  onOpenLogs,
}: SidebarProps) {
  const {
    tags,
    connections,
    selectedTagId,
    setSelectedTag,
    searchQuery,
    setSearchQuery,
  } = useConnectionStore();

  const connectedCount = connections.filter((c) => c.status === 'connected').length;

  return (
    <aside
      className={cn(
        'w-56 flex flex-col h-full',
        'bg-bg-secondary border-r border-border',
      )}
    >
      {/* Search */}
      <div className='px-3 pt-3 pb-2'>
        <div
          className={cn(
            'flex items-center gap-2 px-2.5 h-8 rounded-lg',
            'bg-bg-card border border-border',
            'focus-within:border-border-focus transition-colors',
          )}
        >
          <Search size={13} className='text-text-muted flex-shrink-0' />
          <input
            type='text'
            placeholder='搜索连接…'
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className={cn(
              'flex-1 bg-transparent text-xs text-text-primary',
              'placeholder:text-text-muted outline-none',
            )}
          />
        </div>
      </div>

      {/* Navigation */}
      <nav className='flex-1 overflow-y-auto px-2 py-1 space-y-0.5'>
        {/* All connections */}
        <SidebarItem
          icon={<Network size={14} />}
          label='全部连接'
          count={connections.length}
          active={selectedTagId === null}
          onClick={() => setSelectedTag(null)}
          badge={
            connectedCount > 0 ? (
              <span className='text-[10px] font-medium text-success bg-success/10 px-1.5 py-0.5 rounded-full'>
                {connectedCount} 在线
              </span>
            ) : undefined
          }
        />

        {/* Tags section */}
        {tags.length > 0 && (
          <div className='pt-3 pb-1'>
            <span className='px-2 text-[10px] font-semibold uppercase tracking-wider text-text-muted'>
              标签
            </span>
          </div>
        )}

        {tags.map((tag) => {
          const tagConnCount = connections.filter((c) =>
            c.tag_ids.includes(tag.id),
          ).length;
          return (
            <SidebarItem
              key={tag.id}
              icon={
                <div
                  className='w-2.5 h-2.5 rounded-full flex-shrink-0'
                  style={{ background: tag.color }}
                />
              }
              label={tag.name}
              count={tagConnCount}
              active={selectedTagId === tag.id}
              onClick={() => setSelectedTag(tag.id)}
            />
          );
        })}
      </nav>

      {/* Bottom actions */}
      <div className='p-2 space-y-0.5 border-t border-border'>
        <SidebarItem
          icon={<Plus size={14} />}
          label='新建连接'
          onClick={onNewConnection}
          accent
        />
        <SidebarItem
          icon={<FileText size={14} />}
          label='审计日志'
          onClick={onOpenLogs}
        />
        <SidebarItem
          icon={<Settings size={14} />}
          label='设置'
          onClick={onOpenSettings}
        />
      </div>
    </aside>
  );
}

// ─── Sidebar Item ─────────────────────────────────────────────────

interface SidebarItemProps {
  icon: React.ReactNode;
  label: string;
  count?: number;
  active?: boolean;
  accent?: boolean;
  badge?: React.ReactNode;
  onClick?: () => void;
}

function SidebarItem({
  icon,
  label,
  count,
  active,
  accent,
  badge,
  onClick,
}: SidebarItemProps) {
  return (
    <button
      onClick={onClick}
      className={cn(
        'w-full flex items-center gap-2 px-2 h-8 rounded-lg text-xs',
        'transition-all duration-100 group',
        active
          ? 'bg-accent/10 text-accent'
          : accent
            ? 'text-accent hover:bg-accent/5'
            : 'text-text-secondary hover:text-text-primary hover:bg-bg-card',
      )}
    >
      <span className='flex-shrink-0'>{icon}</span>
      <span className='flex-1 text-left truncate font-medium'>{label}</span>
      {badge}
      {count !== undefined && !badge && (
        <span className='text-[10px] text-text-muted tabular-nums'>
          {count}
        </span>
      )}
    </button>
  );
}
