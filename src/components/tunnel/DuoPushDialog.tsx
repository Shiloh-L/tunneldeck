import { useState, useEffect } from 'react';
import { Smartphone, Shield, Loader2, X } from 'lucide-react';
import { cn } from '@/lib/utils';
import { useConnectionStore } from '@/stores/tunnelStore';

const DUO_TIMEOUT_SECS = 60;

export function DuoPushDialog() {
  const { duoPushConnectionId, connections, setDuoPushConnectionId } =
    useConnectionStore();
  const [countdown, setCountdown] = useState(DUO_TIMEOUT_SECS);

  const connection = connections.find((c) => c.id === duoPushConnectionId);

  useEffect(() => {
    if (!duoPushConnectionId) {
      setCountdown(DUO_TIMEOUT_SECS);
      return;
    }

    setCountdown(DUO_TIMEOUT_SECS);
    const timer = setInterval(() => {
      setCountdown((prev) => {
        if (prev <= 1) {
          clearInterval(timer);
          return 0;
        }
        return prev - 1;
      });
    }, 1000);

    return () => clearInterval(timer);
  }, [duoPushConnectionId]);

  if (!duoPushConnectionId || !connection) return null;

  return (
    <div className='fixed inset-0 z-50 flex items-center justify-center animate-fade-in'>
      {/* Backdrop */}
      <div className='absolute inset-0 bg-black/60 backdrop-blur-sm' />

      {/* Dialog */}
      <div
        className={cn(
          'relative w-[360px]',
          'bg-bg-secondary border border-border rounded-2xl shadow-2xl',
          'p-6 text-center animate-fade-in',
        )}
      >
        {/* Close */}
        <button
          onClick={() => setDuoPushConnectionId(null)}
          className='absolute top-3 right-3 w-7 h-7 flex items-center justify-center rounded-lg text-text-muted hover:text-text-primary hover:bg-bg-card transition-all'
        >
          <X size={14} />
        </button>

        {/* Animated icon */}
        <div className='mx-auto w-16 h-16 rounded-2xl bg-duo-purple/10 flex items-center justify-center mb-4 relative'>
          <Smartphone size={28} className='text-duo-purple' />
          <div className='absolute -top-1 -right-1 w-5 h-5 rounded-full bg-duo-purple flex items-center justify-center'>
            <Shield size={10} className='text-white' />
          </div>
          {/* Pulsing ring */}
          <div className='absolute inset-0 rounded-2xl border-2 border-duo-purple/30 animate-pulse-glow' />
        </div>

        {/* Title */}
        <h3 className='text-sm font-semibold text-text-primary mb-1'>
          等待 Duo Push 验证
        </h3>

        {/* Connection name */}
        <p className='text-xs text-text-secondary mb-4'>
          正在连接{' '}
          <span className='font-medium text-text-primary'>
            {connection.name}
          </span>
        </p>

        {/* Instructions */}
        <div
          className={cn(
            'bg-bg-card border border-border rounded-xl px-4 py-3 mb-4',
            'text-xs text-text-secondary leading-relaxed',
          )}
        >
          <p>请在手机上打开 Duo Mobile</p>
          <p className='mt-1'>
            点击 <span className='font-medium text-success'>Approve</span>{' '}
            批准登录请求
          </p>
        </div>

        {/* Countdown */}
        <div className='flex items-center justify-center gap-2 mb-3'>
          <Loader2 size={12} className='text-duo-purple animate-spin' />
          <span className='text-xs text-text-muted tabular-nums'>
            {countdown > 0 ? `${countdown}s 后超时` : '已超时'}
          </span>
        </div>

        {/* Progress bar */}
        <div className='h-1 bg-bg-card rounded-full overflow-hidden'>
          <div
            className='h-full bg-duo-purple rounded-full transition-all duration-1000 ease-linear'
            style={{
              width: `${(countdown / DUO_TIMEOUT_SECS) * 100}%`,
            }}
          />
        </div>
      </div>
    </div>
  );
}
