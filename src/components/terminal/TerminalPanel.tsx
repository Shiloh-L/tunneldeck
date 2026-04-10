import {
  X,
  Terminal as TerminalIcon,
  ChevronDown,
  ChevronUp,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import { useTerminalStore } from '@/stores/terminalStore';
import { TerminalView } from './TerminalView';

export function TerminalPanel() {
  const {
    terminals,
    activeTerminalId,
    setActiveTerminal,
    closeTerminal,
    showPanel,
    togglePanel,
  } = useTerminalStore();

  if (terminals.length === 0) return null;

  return (
    <div
      className='flex flex-col border-t border-border bg-[#0d1117]'
      style={{ height: showPanel ? '280px' : '32px' }}
    >
      {/* Tab bar */}
      <div className='flex items-center h-8 bg-bg-elevated border-b border-border px-2 gap-1 flex-shrink-0'>
        <button
          onClick={togglePanel}
          className='flex items-center gap-1 text-text-muted hover:text-text-secondary transition-colors mr-1'
        >
          <TerminalIcon size={12} />
          {showPanel ? <ChevronDown size={12} /> : <ChevronUp size={12} />}
        </button>

        {terminals.map((t) => (
          <TabButton
            key={t.terminalId}
            label={t.connectionName}
            isActive={t.terminalId === activeTerminalId}
            onClick={() => {
              setActiveTerminal(t.terminalId);
              if (!showPanel) togglePanel();
            }}
            onClose={() => closeTerminal(t.terminalId)}
          />
        ))}
      </div>

      {/* Terminal content */}
      {showPanel && (
        <div className='flex-1 min-h-0'>
          {terminals.map((t) => (
            <TerminalView
              key={t.terminalId}
              terminalId={t.terminalId}
              isActive={t.terminalId === activeTerminalId}
            />
          ))}
        </div>
      )}
    </div>
  );
}

function TabButton({
  label,
  isActive,
  onClick,
  onClose,
}: {
  label: string;
  isActive: boolean;
  onClick: () => void;
  onClose: () => void;
}) {
  return (
    <div
      className={cn(
        'flex items-center gap-1 px-2 h-6 text-xs rounded cursor-pointer',
        'transition-colors',
        isActive
          ? 'bg-[#0d1117] text-text-primary'
          : 'text-text-muted hover:text-text-secondary hover:bg-bg-card',
      )}
      onClick={onClick}
    >
      <span className='truncate max-w-[120px]'>{label}</span>
      <button
        onClick={(e) => {
          e.stopPropagation();
          onClose();
        }}
        className='w-4 h-4 flex items-center justify-center rounded hover:bg-danger/20 hover:text-danger'
      >
        <X size={10} />
      </button>
    </div>
  );
}
