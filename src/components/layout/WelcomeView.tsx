import { Terminal, ArrowLeftRight } from 'lucide-react';
import { cn } from '@/lib/utils';

export function WelcomeView() {
  return (
    <div
      className={cn(
        'flex-1 flex flex-col items-center justify-center',
        'bg-bg-primary text-text-muted select-none',
      )}
    >
      <div className='w-14 h-14 rounded-2xl bg-accent/10 flex items-center justify-center mb-5'>
        <Terminal size={28} className='text-accent' />
      </div>

      <h2 className='text-lg font-semibold text-text-primary mb-1.5'>
        欢迎使用 ShellDeck
      </h2>
      <p className='text-xs text-text-secondary mb-6'>双击左侧主机开始连接</p>

      <div className='flex items-center gap-6 text-[11px] text-text-muted'>
        <div className='flex items-center gap-1.5'>
          <Terminal size={12} />
          <span>SSH 终端</span>
        </div>
        <div className='flex items-center gap-1.5'>
          <ArrowLeftRight size={12} />
          <span>端口转发</span>
        </div>
      </div>
    </div>
  );
}
