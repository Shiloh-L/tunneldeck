import { useState, useEffect } from 'react';
import {
  X,
  Settings as SettingsIcon,
  Globe,
  Shield,
  Heart,
  FileText,
  Download,
  Upload,
  Copy,
  Check,
  RefreshCw,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import * as api from '@/lib/tauri';
import type { AppSettings } from '@/types';

interface SettingsProps {
  onClose: () => void;
}

export function Settings({ onClose }: SettingsProps) {
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [copiedToken, setCopiedToken] = useState(false);
  const [importResult, setImportResult] = useState<string | null>(null);

  useEffect(() => {
    api.getSettings().then((s) => {
      setSettings(s);
      setLoading(false);
    });
  }, []);

  const update = (patch: Partial<AppSettings>) => {
    if (settings) setSettings({ ...settings, ...patch });
  };

  const handleSave = async () => {
    if (!settings) return;
    setSaving(true);
    try {
      await api.updateSettings(settings);
    } finally {
      setSaving(false);
    }
  };

  const handleExport = async () => {
    const json = await api.exportConfig();
    const blob = new Blob([json], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `shelldeck-export-${new Date().toISOString().slice(0, 10)}.json`;
    a.click();
    URL.revokeObjectURL(url);
  };

  const handleImport = async () => {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = '.json';
    input.onchange = async () => {
      const file = input.files?.[0];
      if (!file) return;
      const text = await file.text();
      try {
        const count = await api.importConfig(text);
        setImportResult(`成功导入 ${count} 条隧道配置`);
        setTimeout(() => setImportResult(null), 3000);
      } catch (err) {
        setImportResult(`导入失败: ${err}`);
      }
    };
    input.click();
  };

  const generateToken = () => {
    const chars =
      'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
    const token = Array.from({ length: 32 }, () =>
      chars.charAt(Math.floor(Math.random() * chars.length)),
    ).join('');
    update({ api_token: token });
  };

  if (loading || !settings) {
    return null;
  }

  return (
    <div className='fixed inset-0 z-50 flex items-center justify-center animate-fade-in'>
      <div
        className='absolute inset-0 bg-black/60 backdrop-blur-sm'
        onClick={onClose}
      />
      <div
        className={cn(
          'relative w-[480px] max-h-[80vh] overflow-y-auto',
          'bg-bg-secondary border border-border rounded-2xl shadow-2xl',
          'animate-fade-in',
        )}
      >
        {/* Header */}
        <div className='flex items-center justify-between px-5 pt-5 pb-3'>
          <div className='flex items-center gap-2'>
            <SettingsIcon size={14} className='text-accent' />
            <h2 className='text-sm font-semibold text-text-primary'>设置</h2>
          </div>
          <button
            onClick={onClose}
            className='w-7 h-7 flex items-center justify-center rounded-lg text-text-muted hover:text-text-primary hover:bg-bg-card transition-all'
          >
            <X size={14} />
          </button>
        </div>

        <div className='px-5 pb-5 space-y-5'>
          {/* Health Check */}
          <Section icon={<Heart size={13} />} title='健康检查'>
            <SettingRow label='心跳间隔 (秒)'>
              <input
                type='number'
                value={settings.health_check_interval_secs}
                onChange={(e) =>
                  update({
                    health_check_interval_secs: Number(e.target.value),
                  })
                }
                min={10}
                max={300}
                className={inputClass}
              />
            </SettingRow>
            <SettingRow label='最大重连次数'>
              <input
                type='number'
                value={settings.max_reconnect_attempts}
                onChange={(e) =>
                  update({ max_reconnect_attempts: Number(e.target.value) })
                }
                min={0}
                max={100}
                className={inputClass}
              />
            </SettingRow>
          </Section>

          {/* Logs */}
          <Section icon={<FileText size={13} />} title='日志'>
            <SettingRow label='日志保留天数'>
              <input
                type='number'
                value={settings.log_retention_days}
                onChange={(e) =>
                  update({ log_retention_days: Number(e.target.value) })
                }
                min={1}
                max={365}
                className={inputClass}
              />
            </SettingRow>
          </Section>

          {/* API */}
          <Section icon={<Globe size={13} />} title='REST API'>
            <SettingRow label='启用 API'>
              <Toggle
                value={settings.api_enabled}
                onChange={(v) => update({ api_enabled: v })}
              />
            </SettingRow>
            {settings.api_enabled && (
              <>
                <SettingRow label='端口 (0=随机)'>
                  <input
                    type='number'
                    value={settings.api_port}
                    onChange={(e) =>
                      update({ api_port: Number(e.target.value) })
                    }
                    min={0}
                    max={65535}
                    className={inputClass}
                  />
                </SettingRow>
                <SettingRow label='Bearer Token'>
                  <div className='flex gap-1.5'>
                    <input
                      type='text'
                      value={settings.api_token ?? ''}
                      readOnly
                      className={cn(inputClass, 'flex-1 font-mono text-[10px]')}
                    />
                    <button
                      onClick={() => {
                        if (settings.api_token) {
                          navigator.clipboard.writeText(settings.api_token);
                          setCopiedToken(true);
                          setTimeout(() => setCopiedToken(false), 1500);
                        }
                      }}
                      className='w-7 h-7 flex items-center justify-center rounded-lg bg-bg-card border border-border text-text-muted hover:text-text-primary transition-all'
                    >
                      {copiedToken ? (
                        <Check size={11} className='text-success' />
                      ) : (
                        <Copy size={11} />
                      )}
                    </button>
                    <button
                      onClick={generateToken}
                      className='w-7 h-7 flex items-center justify-center rounded-lg bg-bg-card border border-border text-text-muted hover:text-text-primary transition-all'
                      title='生成新 Token'
                    >
                      <RefreshCw size={11} />
                    </button>
                  </div>
                </SettingRow>
              </>
            )}
          </Section>

          {/* Import / Export */}
          <Section icon={<Shield size={13} />} title='数据'>
            <div className='flex gap-2'>
              <button
                onClick={handleExport}
                className={cn(
                  'flex-1 h-8 text-xs font-medium rounded-lg',
                  'bg-bg-card border border-border text-text-secondary',
                  'hover:text-text-primary hover:bg-bg-elevated',
                  'flex items-center justify-center gap-1.5 transition-all',
                )}
              >
                <Download size={12} />
                导出配置
              </button>
              <button
                onClick={handleImport}
                className={cn(
                  'flex-1 h-8 text-xs font-medium rounded-lg',
                  'bg-bg-card border border-border text-text-secondary',
                  'hover:text-text-primary hover:bg-bg-elevated',
                  'flex items-center justify-center gap-1.5 transition-all',
                )}
              >
                <Upload size={12} />
                导入配置
              </button>
            </div>
            {importResult && (
              <p className='text-xs text-accent mt-2'>{importResult}</p>
            )}
          </Section>

          {/* Save button */}
          <button
            onClick={handleSave}
            disabled={saving}
            className={cn(
              'w-full h-9 text-xs font-medium rounded-lg',
              'text-white bg-accent hover:bg-accent-hover',
              'transition-all disabled:opacity-50',
            )}
          >
            {saving ? '保存中…' : '保存设置'}
          </button>
        </div>
      </div>
    </div>
  );
}

// ─── Helpers ──────────────────────────────────────────────────────

const inputClass = cn(
  'h-7 px-2 text-xs rounded-lg',
  'bg-bg-card border border-border text-text-primary',
  'focus:border-border-focus focus:outline-none transition-colors',
  'w-24',
);

function Section({
  icon,
  title,
  children,
}: {
  icon: React.ReactNode;
  title: string;
  children: React.ReactNode;
}) {
  return (
    <div className='space-y-2.5'>
      <div className='flex items-center gap-1.5'>
        <span className='text-text-muted'>{icon}</span>
        <span className='text-xs font-semibold text-text-secondary uppercase tracking-wide'>
          {title}
        </span>
      </div>
      <div className='space-y-2 pl-5'>{children}</div>
    </div>
  );
}

function SettingRow({
  label,
  children,
}: {
  label: string;
  children: React.ReactNode;
}) {
  return (
    <div className='flex items-center justify-between'>
      <span className='text-xs text-text-secondary'>{label}</span>
      {children}
    </div>
  );
}

function Toggle({
  value,
  onChange,
}: {
  value: boolean;
  onChange: (v: boolean) => void;
}) {
  return (
    <button
      type='button'
      onClick={() => onChange(!value)}
      className={cn(
        'w-8 h-[18px] rounded-full transition-all relative',
        value ? 'bg-accent' : 'bg-bg-card border border-border',
      )}
    >
      <div
        className={cn(
          'absolute top-0.5 w-3.5 h-3.5 rounded-full',
          'bg-white transition-all shadow-sm',
          value ? 'left-[17px]' : 'left-0.5',
        )}
      />
    </button>
  );
}
