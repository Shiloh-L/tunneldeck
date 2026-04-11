import { useState } from 'react';
import { X, Key, Loader2 } from 'lucide-react';
import { cn } from '@/lib/utils';
import * as api from '@/lib/tauri';
import type { ConnectionInfo } from '@/types';

interface ConnectDialogProps {
  connection: ConnectionInfo;
  onClose: () => void;
}

export function ConnectDialog({ connection, onClose }: ConnectDialogProps) {
  const [password, setPassword] = useState('');
  const [savePassword, setSavePassword] = useState(true);
  const [connecting, setConnecting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [hasStored, setHasStored] = useState<boolean | null>(null);

  // Check if password is already stored
  useState(() => {
    api.hasStoredPassword(connection.id).then((v) => setHasStored(v));
  });

  const handleConnect = async () => {
    setError(null);
    setConnecting(true);

    try {
      if (connection.auth_method === 'key') {
        // Key auth: password is optional (passphrase)
        if (password) {
          await api.startConnection(connection.id, password);
        } else {
          await api.startConnection(connection.id);
        }
      } else if (hasStored && !password) {
        await api.startConnection(connection.id);
      } else {
        if (!password) {
          setError('请输入密码');
          setConnecting(false);
          return;
        }
        if (savePassword) {
          await api.saveConnectionPassword(connection.id, password);
        }
        await api.startConnection(connection.id, password);
      }
      onClose();
    } catch (err) {
      setError(String(err));
      setConnecting(false);
    }
  };

  return (
    <div className='fixed inset-0 z-50 flex items-center justify-center animate-fade-in'>
      <div
        className='absolute inset-0 bg-black/60 backdrop-blur-sm'
        onClick={onClose}
      />
      <div
        className={cn(
          'relative w-[380px]',
          'bg-bg-secondary border border-border rounded-2xl shadow-2xl',
          'animate-fade-in',
        )}
      >
        {/* Header */}
        <div className='flex items-center justify-between px-5 pt-5 pb-2'>
          <div className='flex items-center gap-2'>
            <Key size={14} className='text-accent' />
            <h2 className='text-sm font-semibold text-text-primary'>连接</h2>
          </div>
          <button
            onClick={onClose}
            className='w-7 h-7 flex items-center justify-center rounded-lg text-text-muted hover:text-text-primary hover:bg-bg-card transition-all'
          >
            <X size={14} />
          </button>
        </div>

        <div className='px-5 pb-5 space-y-4'>
          {/* Connection name */}
          <p className='text-xs text-text-secondary'>
            正在连接{' '}
            <span className='font-medium text-text-primary'>
              {connection.name}
            </span>
            <span className='text-text-muted ml-1'>
              ({connection.forwards.filter((f) => f.enabled).length} 个转发规则)
            </span>
          </p>

          {/* Password field */}
          <div className='space-y-1.5'>
            <label className='text-xs text-text-secondary'>
              {connection.auth_method === 'key'
                ? '密钥密码 (如无加密可留空)'
                : hasStored
                  ? '密码 (已保存，留空使用已存密码)'
                  : 'SSH 密码'}
            </label>
            <input
              type='password'
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              placeholder={
                connection.auth_method === 'key'
                  ? '密钥未加密可留空'
                  : hasStored
                    ? '留空使用已存密码'
                    : '输入密码'
              }
              onKeyDown={(e) => e.key === 'Enter' && handleConnect()}
              autoFocus
              className={cn(
                'w-full h-9 px-3 text-xs rounded-lg',
                'bg-bg-card border border-border text-text-primary',
                'placeholder:text-text-muted',
                'focus:border-border-focus focus:outline-none transition-colors',
              )}
            />
          </div>

          {/* Save password toggle */}
          <label className='flex items-center gap-2.5 cursor-pointer'>
            <div
              onClick={() => setSavePassword(!savePassword)}
              className={cn(
                'w-8 h-[18px] rounded-full transition-all relative',
                savePassword ? 'bg-accent' : 'bg-bg-card border border-border',
              )}
            >
              <div
                className={cn(
                  'absolute top-0.5 w-3.5 h-3.5 rounded-full',
                  'bg-white transition-all shadow-sm',
                  savePassword ? 'left-[17px]' : 'left-0.5',
                )}
              />
            </div>
            <span className='text-xs text-text-secondary'>记住密码</span>
          </label>

          {/* Info about Duo Push */}
          <div className='bg-duo-purple/5 border border-duo-purple/15 rounded-lg px-3 py-2.5'>
            <p className='text-[11px] text-duo-purple leading-relaxed'>
              连接后将自动触发 Duo Push 验证。
              <br />
              请准备好手机，在 Duo Mobile 中批准请求。
            </p>
          </div>

          {/* Error */}
          {error && (
            <div className='text-xs text-danger bg-danger/10 px-3 py-2 rounded-lg'>
              {error}
            </div>
          )}

          {/* Actions */}
          <div className='flex justify-end gap-2'>
            <button
              onClick={onClose}
              className={cn(
                'px-4 h-8 text-xs font-medium rounded-lg',
                'text-text-secondary hover:text-text-primary',
                'bg-bg-card hover:bg-bg-elevated border border-border',
                'transition-all',
              )}
            >
              取消
            </button>
            <button
              onClick={handleConnect}
              disabled={connecting}
              className={cn(
                'px-4 h-8 text-xs font-medium rounded-lg',
                'text-white bg-accent hover:bg-accent-hover',
                'transition-all disabled:opacity-50',
                'flex items-center gap-1.5',
              )}
            >
              {connecting && <Loader2 size={12} className='animate-spin' />}
              {connecting ? '连接中…' : '连接'}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
