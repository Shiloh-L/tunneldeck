import { useState } from 'react';
import { X, Server, Lock, Globe, Waypoints } from 'lucide-react';
import { cn } from '@/lib/utils';
import { useTunnelStore } from '@/stores/tunnelStore';
import * as api from '@/lib/tauri';
import type { TunnelInfo } from '@/types';

interface TunnelFormProps {
  tunnel?: TunnelInfo; // if provided, we're editing
  onClose: () => void;
}

export function TunnelForm({ tunnel, onClose }: TunnelFormProps) {
  const { tags, loadTunnels } = useTunnelStore();
  const isEditing = !!tunnel;

  const [form, setForm] = useState({
    name: tunnel?.name ?? '',
    jump_host: tunnel?.jump_host ?? '',
    jump_port: tunnel?.jump_port ?? 22,
    username: tunnel?.username ?? '',
    target_host: tunnel?.target_host ?? '',
    target_port: tunnel?.target_port ?? 3306,
    local_port: tunnel?.local_port ?? 13306,
    password: '',
    auto_connect: tunnel?.auto_connect ?? false,
    tag_ids: tunnel?.tag_ids ?? [],
  });

  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const update = (patch: Partial<typeof form>) =>
    setForm((prev) => ({ ...prev, ...patch }));

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    setSaving(true);

    try {
      if (isEditing && tunnel) {
        await api.updateTunnel({
          ...tunnel,
          name: form.name,
          jump_host: form.jump_host,
          jump_port: form.jump_port,
          username: form.username,
          target_host: form.target_host,
          target_port: form.target_port,
          local_port: form.local_port,
          auto_connect: form.auto_connect,
          tag_ids: form.tag_ids,
          updated_at: new Date().toISOString(),
        });
        if (form.password) {
          await api.saveTunnelPassword(tunnel.id, form.password);
        }
      } else {
        await api.createTunnel({
          name: form.name,
          jump_host: form.jump_host,
          jump_port: form.jump_port,
          username: form.username,
          target_host: form.target_host,
          target_port: form.target_port,
          local_port: form.local_port,
          password: form.password,
          auto_connect: form.auto_connect,
          tag_ids: form.tag_ids,
        });
      }
      await loadTunnels();
      onClose();
    } catch (err) {
      setError(String(err));
    } finally {
      setSaving(false);
    }
  };

  const toggleTag = (tagId: string) => {
    update({
      tag_ids: form.tag_ids.includes(tagId)
        ? form.tag_ids.filter((id) => id !== tagId)
        : [...form.tag_ids, tagId],
    });
  };

  return (
    <div className='fixed inset-0 z-50 flex items-center justify-center animate-fade-in'>
      {/* Backdrop */}
      <div
        className='absolute inset-0 bg-black/60 backdrop-blur-sm'
        onClick={onClose}
      />

      {/* Dialog */}
      <div
        className={cn(
          'relative w-[480px] max-h-[85vh] overflow-y-auto',
          'bg-bg-secondary border border-border rounded-2xl shadow-2xl',
          'animate-fade-in',
        )}
      >
        {/* Header */}
        <div className='flex items-center justify-between px-5 pt-5 pb-3'>
          <h2 className='text-base font-semibold text-text-primary'>
            {isEditing ? '编辑隧道' : '新建隧道'}
          </h2>
          <button
            onClick={onClose}
            className='w-7 h-7 flex items-center justify-center rounded-lg text-text-muted hover:text-text-primary hover:bg-bg-card transition-all'
          >
            <X size={14} />
          </button>
        </div>

        <form onSubmit={handleSubmit} className='px-5 pb-5 space-y-4'>
          {/* Name */}
          <FormField label='隧道名称' icon={<Waypoints size={13} />}>
            <input
              type='text'
              value={form.name}
              onChange={(e) => update({ name: e.target.value })}
              placeholder='生产环境 MySQL'
              required
              className={inputClass}
            />
          </FormField>

          {/* SSH Connection */}
          <div className='space-y-2'>
            <SectionLabel icon={<Server size={13} />} label='跳板机' />
            <div className='grid grid-cols-3 gap-2'>
              <div className='col-span-2'>
                <input
                  type='text'
                  value={form.jump_host}
                  onChange={(e) => update({ jump_host: e.target.value })}
                  placeholder='bastion.example.com'
                  required
                  className={inputClass}
                />
              </div>
              <input
                type='number'
                value={form.jump_port}
                onChange={(e) => update({ jump_port: Number(e.target.value) })}
                placeholder='22'
                min={1}
                max={65535}
                className={inputClass}
              />
            </div>
            <div className='grid grid-cols-2 gap-2'>
              <input
                type='text'
                value={form.username}
                onChange={(e) => update({ username: e.target.value })}
                placeholder='用户名'
                required
                className={inputClass}
              />
              <input
                type='password'
                value={form.password}
                onChange={(e) => update({ password: e.target.value })}
                placeholder={isEditing ? '不修改留空' : '密码 (Duo Push)'}
                required={!isEditing}
                className={inputClass}
              />
            </div>
          </div>

          {/* Target */}
          <div className='space-y-2'>
            <SectionLabel icon={<Globe size={13} />} label='目标服务' />
            <div className='grid grid-cols-3 gap-2'>
              <div className='col-span-2'>
                <input
                  type='text'
                  value={form.target_host}
                  onChange={(e) => update({ target_host: e.target.value })}
                  placeholder='10.0.1.50'
                  required
                  className={inputClass}
                />
              </div>
              <input
                type='number'
                value={form.target_port}
                onChange={(e) =>
                  update({ target_port: Number(e.target.value) })
                }
                placeholder='3306'
                min={1}
                max={65535}
                className={inputClass}
              />
            </div>
          </div>

          {/* Local port */}
          <FormField label='本地端口' icon={<Lock size={13} />}>
            <input
              type='number'
              value={form.local_port}
              onChange={(e) => update({ local_port: Number(e.target.value) })}
              placeholder='13306'
              min={1024}
              max={65535}
              required
              className={inputClass}
            />
          </FormField>

          {/* Tags */}
          {tags.length > 0 && (
            <div className='space-y-2'>
              <span className='text-xs font-medium text-text-secondary'>
                标签
              </span>
              <div className='flex flex-wrap gap-1.5'>
                {tags.map((tag) => (
                  <button
                    key={tag.id}
                    type='button'
                    onClick={() => toggleTag(tag.id)}
                    className={cn(
                      'text-[11px] font-medium px-2.5 py-1 rounded-full',
                      'border transition-all',
                      form.tag_ids.includes(tag.id)
                        ? 'border-transparent'
                        : 'border-border text-text-muted hover:text-text-secondary',
                    )}
                    style={
                      form.tag_ids.includes(tag.id)
                        ? { color: tag.color, background: `${tag.color}20` }
                        : undefined
                    }
                  >
                    {tag.name}
                  </button>
                ))}
              </div>
            </div>
          )}

          {/* Auto-connect toggle */}
          <label className='flex items-center gap-2.5 cursor-pointer group'>
            <div
              className={cn(
                'w-8 h-[18px] rounded-full transition-all relative',
                form.auto_connect
                  ? 'bg-accent'
                  : 'bg-bg-card border border-border',
              )}
              onClick={() => update({ auto_connect: !form.auto_connect })}
            >
              <div
                className={cn(
                  'absolute top-0.5 w-3.5 h-3.5 rounded-full',
                  'bg-white transition-all shadow-sm',
                  form.auto_connect ? 'left-[17px]' : 'left-0.5',
                )}
              />
            </div>
            <span className='text-xs text-text-secondary group-hover:text-text-primary transition-colors'>
              启动时自动连接
            </span>
          </label>

          {/* Error */}
          {error && (
            <div className='text-xs text-danger bg-danger/10 px-3 py-2 rounded-lg'>
              {error}
            </div>
          )}

          {/* Actions */}
          <div className='flex justify-end gap-2 pt-1'>
            <button
              type='button'
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
              type='submit'
              disabled={saving}
              className={cn(
                'px-4 h-8 text-xs font-medium rounded-lg',
                'text-white bg-accent hover:bg-accent-hover',
                'transition-all disabled:opacity-50',
              )}
            >
              {saving ? '保存中…' : isEditing ? '保存修改' : '创建隧道'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}

// ─── Helpers ──────────────────────────────────────────────────────

const inputClass = cn(
  'w-full h-8 px-2.5 text-xs rounded-lg',
  'bg-bg-card border border-border text-text-primary',
  'placeholder:text-text-muted',
  'focus:border-border-focus focus:outline-none',
  'transition-colors',
);

function FormField({
  label,
  icon,
  children,
}: {
  label: string;
  icon: React.ReactNode;
  children: React.ReactNode;
}) {
  return (
    <div className='space-y-1.5'>
      <div className='flex items-center gap-1.5'>
        <span className='text-text-muted'>{icon}</span>
        <span className='text-xs font-medium text-text-secondary'>{label}</span>
      </div>
      {children}
    </div>
  );
}

function SectionLabel({
  icon,
  label,
}: {
  icon: React.ReactNode;
  label: string;
}) {
  return (
    <div className='flex items-center gap-1.5'>
      <span className='text-text-muted'>{icon}</span>
      <span className='text-xs font-medium text-text-secondary'>{label}</span>
    </div>
  );
}
