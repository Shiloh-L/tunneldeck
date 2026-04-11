import { X, CheckCircle, AlertCircle, AlertTriangle, Info } from 'lucide-react';
import { cn } from '@/lib/utils';
import { useToastStore, type ToastType } from '@/stores/toastStore';

const iconMap: Record<ToastType, React.ReactNode> = {
  success: <CheckCircle size={14} />,
  error: <AlertCircle size={14} />,
  warning: <AlertTriangle size={14} />,
  info: <Info size={14} />,
};

const styleMap: Record<ToastType, string> = {
  success: 'border-success/30 bg-success/5 text-success',
  error: 'border-danger/30 bg-danger/5 text-danger',
  warning: 'border-warning/30 bg-warning/5 text-warning',
  info: 'border-accent/30 bg-accent/5 text-accent',
};

export function ToastContainer() {
  const { toasts, removeToast } = useToastStore();

  if (toasts.length === 0) return null;

  return (
    <div className='fixed bottom-4 right-4 z-[100] flex flex-col gap-2 max-w-sm'>
      {toasts.map((toast) => (
        <div
          key={toast.id}
          className={cn(
            'flex items-start gap-2.5 px-3.5 py-2.5 rounded-xl border',
            'shadow-lg backdrop-blur-sm animate-fade-in',
            'bg-bg-elevated/95',
            styleMap[toast.type],
          )}
        >
          <span className='mt-0.5 flex-shrink-0'>{iconMap[toast.type]}</span>
          <p className='text-xs leading-relaxed flex-1 text-text-primary'>
            {toast.message}
          </p>
          <button
            onClick={() => removeToast(toast.id)}
            className='mt-0.5 flex-shrink-0 text-text-muted hover:text-text-primary transition-colors'
          >
            <X size={12} />
          </button>
        </div>
      ))}
    </div>
  );
}
