import { useState } from 'react';
import {
  X,
  Server,
  Globe,
  Waypoints,
  Plus,
  Trash2,
  Key,
  Lock,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import { useConnectionStore } from '@/stores/connectionStore';
import * as api from '@/lib/tauri';
import type { ConnectionInfo, ForwardRule, AuthMethod } from '@/types';

interface ConnectionFormProps {
  connection?: ConnectionInfo; // if provided, we're editing
  onClose: () => void;
}

interface ForwardDraft {
  id?: string; // exists if editing
  name: string;
  local_port: number;
  target_host: string;
  target_port: number;
  enabled: boolean;
}

export function ConnectionForm({ connection, onClose }: ConnectionFormProps) {
  const { tags, loadConnections } = useConnectionStore();
  const isEditing = !!connection;

  const [form, setForm] = useState({
    name: connection?.name ?? '',
    host: connection?.host ?? '',
    port: connection?.port ?? 22,
    username: connection?.username ?? '',
    password: '',
    auth_method: (connection?.auth_method ?? 'password') as AuthMethod,
    private_key_path: connection?.private_key_path ?? '',
    auto_connect: connection?.auto_connect ?? false,
    tag_ids: connection?.tag_ids ?? [],
  });

  const [forwards, setForwards] = useState<ForwardDraft[]>(
    connection?.forwards?.map((f) => ({
      id: f.id,
      name: f.name,
      local_port: f.local_port,
      target_host: f.target_host,
      target_port: f.target_port,
      enabled: f.enabled,
    })) ?? [],
  );

  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const update = (patch: Partial<typeof form>) =>
    setForm((prev) => ({ ...prev, ...patch }));

  const updateForward = (index: number, patch: Partial<ForwardDraft>) =>
    setForwards((prev) =>
      prev.map((f, i) => (i === index ? { ...f, ...patch } : f)),
    );

  const addForward = () =>
    setForwards((prev) => [
      ...prev,
      {
        name: '',
        local_port: 0,
        target_host: '',
        target_port: 0,
        enabled: true,
      },
    ]);

  const removeForward = (index: number) =>
    setForwards((prev) => prev.filter((_, i) => i !== index));

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    setSaving(true);

    try {
      if (isEditing && connection) {
        const updatedForwards: ForwardRule[] = forwards.map((f) => ({
          id: f.id ?? crypto.randomUUID(),
          name: f.name,
          local_port: f.local_port,
          target_host: f.target_host,
          target_port: f.target_port,
          enabled: f.enabled,
        }));
        await api.updateConnection({
          ...connection,
          name: form.name,
          host: form.host,
          port: form.port,
          username: form.username,
          auth_method: form.auth_method,
          private_key_path: form.private_key_path || null,
          forwards: updatedForwards,
          auto_connect: form.auto_connect,
          tag_ids: form.tag_ids,
          updated_at: new Date().toISOString(),
        });
        if (form.password) {
          await api.saveConnectionPassword(connection.id, form.password);
        }
      } else {
        await api.createConnection({
          name: form.name,
          host: form.host,
          port: form.port,
          username: form.username,
          password: form.password,
          auth_method: form.auth_method,
          private_key_path: form.private_key_path || null,
          forwards: forwards.map((f) => ({
            name: f.name,
            local_port: f.local_port,
            target_host: f.target_host,
            target_port: f.target_port,
            enabled: f.enabled,
          })),
          auto_connect: form.auto_connect,
          tag_ids: form.tag_ids,
        });
      }
      await loadConnections();
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
          'relative w-[520px] max-h-[85vh] overflow-y-auto',
          'bg-bg-secondary border border-border rounded-2xl shadow-2xl',
          'animate-fade-in',
        )}
      >
        {/* Header */}
        <div className='flex items-center justify-between px-5 pt-5 pb-3'>
          <h2 className='text-base font-semibold text-text-primary'>
            {isEditing ? '编辑主机' : '新建主机'}
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
          <FormField label='连接名称' icon={<Waypoints size={13} />}>
            <input
              type='text'
              value={form.name}
              onChange={(e) => update({ name: e.target.value })}
              placeholder='生产环境服务器'
              required
              className={inputClass}
            />
          </FormField>

          {/* SSH Connection */}
          <div className='space-y-2'>
            <SectionLabel icon={<Server size={13} />} label='服务器' />
            <div className='grid grid-cols-3 gap-2'>
              <div className='col-span-2'>
                <input
                  type='text'
                  value={form.host}
                  onChange={(e) => update({ host: e.target.value })}
                  placeholder='bastion.example.com'
                  required
                  className={inputClass}
                />
              </div>
              <input
                type='number'
                value={form.port}
                onChange={(e) => update({ port: Number(e.target.value) })}
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
                placeholder={
                  isEditing
                    ? '不修改留空'
                    : form.auth_method === 'key'
                      ? '密钥密码 (可选)'
                      : '密码 (Duo Push)'
                }
                required={!isEditing && form.auth_method === 'password'}
                className={inputClass}
              />
            </div>
          </div>

          {/* Auth Method */}
          <div className='space-y-2'>
            <SectionLabel icon={<Key size={13} />} label='认证方式' />
            <div className='flex gap-2'>
              <button
                type='button'
                onClick={() => update({ auth_method: 'password' })}
                className={cn(
                  'flex-1 flex items-center justify-center gap-1.5 h-8 text-xs font-medium rounded-lg border transition-all',
                  form.auth_method === 'password'
                    ? 'border-accent bg-accent/10 text-accent'
                    : 'border-border text-text-muted hover:text-text-secondary hover:border-border-focus',
                )}
              >
                <Lock size={12} />
                密码
              </button>
              <button
                type='button'
                onClick={() => update({ auth_method: 'key' })}
                className={cn(
                  'flex-1 flex items-center justify-center gap-1.5 h-8 text-xs font-medium rounded-lg border transition-all',
                  form.auth_method === 'key'
                    ? 'border-accent bg-accent/10 text-accent'
                    : 'border-border text-text-muted hover:text-text-secondary hover:border-border-focus',
                )}
              >
                <Key size={12} />
                密钥
              </button>
            </div>
            {form.auth_method === 'key' && (
              <input
                type='text'
                value={form.private_key_path}
                onChange={(e) => update({ private_key_path: e.target.value })}
                placeholder='私钥路径 (如: C:\Users\xxx\.ssh\id_ed25519)'
                required
                className={inputClass}
              />
            )}
          </div>

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

          {/* Forward Rules (Optional) */}
          <div className='space-y-2'>
            <div className='flex items-center justify-between'>
              <SectionLabel
                icon={<Globe size={13} />}
                label='端口转发规则 (可选)'
              />
              <button
                type='button'
                onClick={addForward}
                className='flex items-center gap-1 text-[11px] text-accent hover:text-accent-hover transition-colors'
              >
                <Plus size={12} />
                添加规则
              </button>
            </div>

            {forwards.length === 0 && (
              <p className='text-[11px] text-text-muted px-1'>
                不需要端口转发？可直接创建，仅使用 SSH 终端。
              </p>
            )}

            {forwards.map((fwd, i) => (
              <div
                key={i}
                className='relative rounded-lg border border-border bg-bg-card p-3 space-y-2'
              >
                {forwards.length > 0 && (
                  <button
                    type='button'
                    onClick={() => removeForward(i)}
                    className='absolute top-2 right-2 w-6 h-6 flex items-center justify-center rounded text-text-muted hover:text-danger hover:bg-danger/10 transition-all'
                  >
                    <Trash2 size={11} />
                  </button>
                )}
                <input
                  type='text'
                  value={fwd.name}
                  onChange={(e) => updateForward(i, { name: e.target.value })}
                  placeholder={`规则名称 (如: MySQL #${i + 1})`}
                  className={cn(inputClass, 'pr-8')}
                />
                <div className='grid grid-cols-5 gap-2 items-center'>
                  <div className='col-span-1'>
                    <input
                      type='number'
                      value={fwd.local_port || ''}
                      onChange={(e) =>
                        updateForward(i, { local_port: Number(e.target.value) })
                      }
                      placeholder='本地端口'
                      min={1024}
                      max={65535}
                      className={inputClass}
                    />
                  </div>
                  <div className='flex items-center justify-center text-text-muted text-xs'>
                    →
                  </div>
                  <div className='col-span-2'>
                    <input
                      type='text'
                      value={fwd.target_host}
                      onChange={(e) =>
                        updateForward(i, { target_host: e.target.value })
                      }
                      placeholder='目标主机'
                      className={inputClass}
                    />
                  </div>
                  <div className='col-span-1'>
                    <input
                      type='number'
                      value={fwd.target_port || ''}
                      onChange={(e) =>
                        updateForward(i, {
                          target_port: Number(e.target.value),
                        })
                      }
                      placeholder='端口'
                      min={1}
                      max={65535}
                      className={inputClass}
                    />
                  </div>
                </div>
              </div>
            ))}
          </div>

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
              {saving ? '保存中…' : isEditing ? '保存修改' : '创建主机'}
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
