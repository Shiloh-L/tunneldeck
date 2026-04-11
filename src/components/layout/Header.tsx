import {
  getCurrentWindow,
  type Window as TauriWindow,
} from '@tauri-apps/api/window';
import { Minus, X, Square } from 'lucide-react';
import { cn } from '@/lib/utils';
import { useToastStore } from '@/stores/toastStore';

let appWindow: TauriWindow | null = null;
try {
  appWindow = getCurrentWindow();
} catch {
  // Not in Tauri webview (e.g. browser dev)
}

export function Header() {
  const handleClose = () => {
    useToastStore.getState().addToast('info', '已最小化到系统托盘', 2000);
    appWindow?.hide();
  };

  return (
    <header
      data-tauri-drag-region
      className={cn(
        'flex items-center justify-between h-11 px-4',
        'bg-bg-secondary/80 backdrop-blur-md border-b border-border',
        'select-none',
      )}
    >
      {/* App title */}
      <div data-tauri-drag-region className='flex items-center gap-2.5'>
        <div className='w-5 h-5 rounded-md bg-accent flex items-center justify-center'>
          <span className='text-[10px] font-bold text-white'>S</span>
        </div>
        <span className='text-sm font-semibold text-text-primary tracking-tight'>
          ShellDeck
        </span>
      </div>

      {/* Window controls */}
      <div className='flex items-center gap-0.5'>
        <button
          onClick={() => appWindow?.minimize()}
          className={cn(
            'w-8 h-7 flex items-center justify-center rounded-md',
            'text-text-secondary hover:text-text-primary hover:bg-bg-card',
            'transition-all duration-100',
          )}
        >
          <Minus size={14} />
        </button>
        <button
          onClick={() => appWindow?.toggleMaximize()}
          className={cn(
            'w-8 h-7 flex items-center justify-center rounded-md',
            'text-text-secondary hover:text-text-primary hover:bg-bg-card',
            'transition-all duration-100',
          )}
        >
          <Square size={11} />
        </button>
        <button
          onClick={handleClose}
          className={cn(
            'w-8 h-7 flex items-center justify-center rounded-md',
            'text-text-secondary hover:text-white hover:bg-danger',
            'transition-all duration-100',
          )}
        >
          <X size={14} />
        </button>
      </div>
    </header>
  );
}
