import { useState, useRef, useEffect } from 'react';
import {
  Network,
  Search,
  Plus,
  Settings,
  FileText,
  Tags,
  Terminal,
  Pencil,
  Copy,
  Trash2,
  Power,
  PowerOff,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import { useConnectionStore } from '@/stores/connectionStore';
import { useTerminalStore } from '@/stores/terminalStore';
import * as api from '@/lib/tauri';
import type { ConnectionInfo } from '@/types';

interface SidebarProps {
  onNewConnection: () => void;
  onEditConnection: (conn: ConnectionInfo) => void;
  onConnectDialog: (conn: ConnectionInfo) => void;
  onOpenSettings: () => void;
  onOpenLogs: () => void;
  onOpenTags: () => void;
}

export function Sidebar({
  onNewConnection,
  onEditConnection,
  onConnectDialog,
  onOpenSettings,
  onOpenLogs,
  onOpenTags,
}: SidebarProps) {
  const {
    tags,
    connections,
    selectedTagId,
    setSelectedTag,
    searchQuery,
    setSearchQuery,
    filteredConnections,
    loadConnections,
  } = useConnectionStore();

  const { openTerminal, setPendingTerminal } = useTerminalStore();

  const connectedCount = connections.filter(
    (c) => c.status === 'connected',
  ).length;

  const filtered = filteredConnections();

  // ─── Context Menu ─────────────────────────────────────────────
  const [contextMenu, setContextMenu] = useState<{
    x: number;
    y: number;
    connection: ConnectionInfo;
  } | null>(null);
  const menuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handleOutsideClick = () => setContextMenu(null);
    if (contextMenu) {
      document.addEventListener('click', handleOutsideClick);
      return () => document.removeEventListener('click', handleOutsideClick);
    }
  }, [contextMenu]);

  // ─── Double-click: auto-connect + open terminal ───────────────
  const handleDoubleClick = async (conn: ConnectionInfo) => {
    if (conn.status === 'connected') {
      // Already connected → just open a new terminal tab
      await openTerminal(conn.id, conn.name);
      return;
    }

    // Not connected → need to connect first, then open terminal
    try {
      const hasPassword = await api.hasStoredPassword(conn.id);
      if (hasPassword) {
        setPendingTerminal(conn.id, conn.name);
        await api.startConnection(conn.id);
      } else {
        // No saved password → show connect dialog (auto-connect terminal flow)
        setPendingTerminal(conn.id, conn.name);
        onConnectDialog(conn);
      }
    } catch {
      setPendingTerminal(null, null);
    }
  };

  // ─── Context menu actions ─────────────────────────────────────
  const handleConnect = async (conn: ConnectionInfo) => {
    setContextMenu(null);
    try {
      const hasPassword = await api.hasStoredPassword(conn.id);
      if (hasPassword) {
        await api.startConnection(conn.id);
      } else {
        onConnectDialog(conn);
      }
    } catch {
      /* handled by status events */
    }
  };

  const handleDisconnect = async (conn: ConnectionInfo) => {
    setContextMenu(null);
    await api.stopConnection(conn.id);
  };

  const handleOpenTerminal = async (conn: ConnectionInfo) => {
    setContextMenu(null);
    if (conn.status === 'connected') {
      await openTerminal(conn.id, conn.name);
    } else {
      await handleDoubleClick(conn);
    }
  };

  const handleCopyAddress = (conn: ConnectionInfo) => {
    setContextMenu(null);
    navigator.clipboard.writeText(`${conn.username}@${conn.host}:${conn.port}`);
  };

  const handleDelete = async (conn: ConnectionInfo) => {
    setContextMenu(null);
    await api.deleteConnection(conn.id);
    loadConnections();
  };

  return (
    <aside
      className={cn(
        'w-64 flex flex-col h-full',
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
            placeholder='搜索主机…'
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className={cn(
              'flex-1 bg-transparent text-xs text-text-primary',
              'placeholder:text-text-muted outline-none',
            )}
          />
        </div>
      </div>

      {/* Navigation + Host List */}
      <nav className='flex-1 overflow-y-auto px-2 py-1 space-y-0.5'>
        {/* All connections filter */}
        <NavItem
          icon={<Network size={14} />}
          label='全部主机'
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
          <>
            <div className='pt-3 pb-1'>
              <span className='px-2 text-[10px] font-semibold uppercase tracking-wider text-text-muted'>
                标签
              </span>
            </div>
            {tags.map((tag) => {
              const tagConnCount = connections.filter((c) =>
                c.tag_ids.includes(tag.id),
              ).length;
              return (
                <NavItem
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
          </>
        )}

        {/* Host list */}
        <div className='pt-3 pb-1'>
          <span className='px-2 text-[10px] font-semibold uppercase tracking-wider text-text-muted'>
            主机
          </span>
        </div>

        {filtered.length === 0 ? (
          <div className='px-2 py-4 text-center'>
            <p className='text-xs text-text-muted'>暂无主机</p>
            <p className='text-[10px] text-text-muted mt-1'>
              点击下方「新建主机」开始
            </p>
          </div>
        ) : (
          filtered.map((conn) => (
            <HostItem
              key={conn.id}
              connection={conn}
              onDoubleClick={() => handleDoubleClick(conn)}
              onContextMenu={(e) => {
                e.preventDefault();
                setContextMenu({
                  x: e.clientX,
                  y: e.clientY,
                  connection: conn,
                });
              }}
            />
          ))
        )}
      </nav>

      {/* Bottom actions */}
      <div className='p-2 space-y-0.5 border-t border-border'>
        <NavItem
          icon={<Plus size={14} />}
          label='新建主机'
          onClick={onNewConnection}
          accent
        />
        <NavItem
          icon={<Tags size={14} />}
          label='标签管理'
          onClick={onOpenTags}
        />
        <NavItem
          icon={<FileText size={14} />}
          label='审计日志'
          onClick={onOpenLogs}
        />
        <NavItem
          icon={<Settings size={14} />}
          label='设置'
          onClick={onOpenSettings}
        />
      </div>

      {/* ─── Context Menu Portal ─────────────────────────────── */}
      {contextMenu && (
        <div
          ref={menuRef}
          className={cn(
            'fixed z-50 min-w-[160px] py-1 rounded-lg',
            'bg-bg-elevated border border-border shadow-xl',
            'animate-fade-in',
          )}
          style={{ left: contextMenu.x, top: contextMenu.y }}
        >
          {contextMenu.connection.status === 'connected' ? (
            <>
              <ContextMenuItem
                icon={<Terminal size={13} />}
                label='打开终端'
                onClick={() => handleOpenTerminal(contextMenu.connection)}
              />
              <ContextMenuItem
                icon={<PowerOff size={13} />}
                label='断开连接'
                onClick={() => handleDisconnect(contextMenu.connection)}
                danger
              />
            </>
          ) : (
            <ContextMenuItem
              icon={<Power size={13} />}
              label='连接'
              onClick={() => handleConnect(contextMenu.connection)}
            />
          )}
          <div className='my-1 border-t border-border' />
          <ContextMenuItem
            icon={<Pencil size={13} />}
            label='编辑'
            onClick={() => {
              setContextMenu(null);
              onEditConnection(contextMenu.connection);
            }}
          />
          <ContextMenuItem
            icon={<Copy size={13} />}
            label='复制地址'
            onClick={() => handleCopyAddress(contextMenu.connection)}
          />
          <div className='my-1 border-t border-border' />
          <ContextMenuItem
            icon={<Trash2 size={13} />}
            label='删除'
            onClick={() => handleDelete(contextMenu.connection)}
            danger
          />
        </div>
      )}
    </aside>
  );
}

// ─── NavItem ──────────────────────────────────────────────────────

function NavItem({
  icon,
  label,
  count,
  active,
  accent,
  badge,
  onClick,
}: {
  icon: React.ReactNode;
  label: string;
  count?: number;
  active?: boolean;
  accent?: boolean;
  badge?: React.ReactNode;
  onClick?: () => void;
}) {
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

// ─── HostItem ─────────────────────────────────────────────────────

const statusColor: Record<string, string> = {
  connected: 'bg-success',
  connecting: 'bg-warning animate-pulse',
  waitingduo: 'bg-duo-purple animate-pulse',
  reconnecting: 'bg-warning animate-pulse',
  error: 'bg-danger',
  disconnected: 'bg-text-muted',
};

function HostItem({
  connection,
  onDoubleClick,
  onContextMenu,
}: {
  connection: ConnectionInfo;
  onDoubleClick: () => void;
  onContextMenu: (e: React.MouseEvent) => void;
}) {
  const forwardCount =
    connection.forwards?.filter((f) => f.enabled).length ?? 0;

  return (
    <button
      className={cn(
        'w-full text-left px-2 py-1.5 rounded-lg',
        'hover:bg-bg-card transition-colors',
        'group cursor-default',
      )}
      onDoubleClick={onDoubleClick}
      onContextMenu={onContextMenu}
    >
      <div className='flex items-center gap-2'>
        <div
          className={cn(
            'w-2 h-2 rounded-full flex-shrink-0',
            statusColor[connection.status] ?? 'bg-text-muted',
          )}
        />
        <span className='text-xs font-medium text-text-primary truncate flex-1'>
          {connection.name}
        </span>
        {forwardCount > 0 && (
          <span className='text-[9px] text-text-muted tabular-nums'>
            {forwardCount} 转发
          </span>
        )}
      </div>
      <div className='ml-4 mt-0.5 text-[10px] text-text-muted truncate font-mono'>
        {connection.username}@{connection.host}:{connection.port}
      </div>
    </button>
  );
}

// ─── ContextMenuItem ──────────────────────────────────────────────

function ContextMenuItem({
  icon,
  label,
  onClick,
  danger,
}: {
  icon: React.ReactNode;
  label: string;
  onClick: () => void;
  danger?: boolean;
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
      <span>{label}</span>
    </button>
  );
}
