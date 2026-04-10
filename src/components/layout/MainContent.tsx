import { X, Terminal as TerminalIcon } from 'lucide-react';
import { cn } from '@/lib/utils';
import { useTerminalStore } from '@/stores/terminalStore';
import { TerminalView } from '@/components/terminal/TerminalView';
import { WelcomeView } from './WelcomeView';

export function MainContent() {
  const { terminals, activeTerminalId, setActiveTerminal, closeTerminal } =
    useTerminalStore();

  if (terminals.length === 0) {
    return <WelcomeView />;
  }

  return (
    <div className='flex-1 flex flex-col min-h-0 bg-[#0d1117]'>
      {/* Tab bar */}
      <div
        className={cn(
          'flex items-center h-9 bg-bg-elevated border-b border-border',
          'px-2 gap-0.5 flex-shrink-0',
        )}
      >
        <TerminalIcon size={13} className='text-text-muted mr-1.5' />

        <div className='flex items-center gap-0.5 flex-1 min-w-0 overflow-x-auto'>
          {terminals.map((t) => (
            <TabButton
              key={t.terminalId}
              label={t.connectionName}
              isActive={t.terminalId === activeTerminalId}
              onClick={() => setActiveTerminal(t.terminalId)}
              onClose={() => closeTerminal(t.terminalId)}
            />
          ))}
        </div>
      </div>

      {/* Terminal content */}
      <div className='flex-1 min-h-0'>
        {terminals.map((t) => (
          <TerminalView
            key={t.terminalId}
            terminalId={t.terminalId}
            isActive={t.terminalId === activeTerminalId}
          />
        ))}
      </div>
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
        'flex items-center gap-1.5 px-2.5 h-7 text-xs rounded-md cursor-pointer',
        'transition-colors group',
        isActive
          ? 'bg-[#0d1117] text-text-primary'
          : 'text-text-muted hover:text-text-secondary hover:bg-bg-card',
      )}
      onClick={onClick}
    >
      <span className='truncate max-w-[140px]'>{label}</span>
      <button
        onClick={(e) => {
          e.stopPropagation();
          onClose();
        }}
        className={cn(
          'w-4 h-4 flex items-center justify-center rounded',
          'opacity-0 group-hover:opacity-100',
          'hover:bg-danger/20 hover:text-danger',
          'transition-opacity',
        )}
      >
        <X size={10} />
      </button>
    </div>
  );
}
