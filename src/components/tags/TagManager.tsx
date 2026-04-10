import { useState } from 'react';
import { Plus, X, Palette } from 'lucide-react';
import { cn } from '@/lib/utils';
import { useConnectionStore } from '@/stores/connectionStore';
import * as api from '@/lib/tauri';

const PRESET_COLORS = [
  '#ef4444',
  '#f97316',
  '#f59e0b',
  '#22c55e',
  '#06b6d4',
  '#3b82f6',
  '#6366f1',
  '#8b5cf6',
  '#ec4899',
  '#64748b',
];

interface TagManagerProps {
  onClose: () => void;
}

export function TagManager({ onClose }: TagManagerProps) {
  const { tags, loadTags, loadConnections } = useConnectionStore();
  const [newName, setNewName] = useState('');
  const [newColor, setNewColor] = useState(PRESET_COLORS[0]);
  const [adding, setAdding] = useState(false);

  const handleAdd = async () => {
    if (!newName.trim()) return;
    setAdding(true);
    try {
      await api.createTag(newName.trim(), newColor);
      await loadTags();
      setNewName('');
    } finally {
      setAdding(false);
    }
  };

  const handleDelete = async (tagId: string) => {
    await api.deleteTag(tagId);
    await loadTags();
    await loadConnections();
  };

  return (
    <div className='fixed inset-0 z-50 flex items-center justify-center animate-fade-in'>
      <div
        className='absolute inset-0 bg-black/60 backdrop-blur-sm'
        onClick={onClose}
      />
      <div
        className={cn(
          'relative w-[380px] max-h-[70vh]',
          'bg-bg-secondary border border-border rounded-2xl shadow-2xl',
          'animate-fade-in',
        )}
      >
        {/* Header */}
        <div className='flex items-center justify-between px-5 pt-5 pb-3'>
          <div className='flex items-center gap-2'>
            <Palette size={14} className='text-accent' />
            <h2 className='text-sm font-semibold text-text-primary'>
              管理标签
            </h2>
          </div>
          <button
            onClick={onClose}
            className='w-7 h-7 flex items-center justify-center rounded-lg text-text-muted hover:text-text-primary hover:bg-bg-card transition-all'
          >
            <X size={14} />
          </button>
        </div>

        <div className='px-5 pb-5 space-y-4'>
          {/* Add new tag */}
          <div className='space-y-2'>
            <div className='flex gap-2'>
              <input
                type='text'
                value={newName}
                onChange={(e) => setNewName(e.target.value)}
                placeholder='标签名称'
                onKeyDown={(e) => e.key === 'Enter' && handleAdd()}
                className={cn(
                  'flex-1 h-8 px-2.5 text-xs rounded-lg',
                  'bg-bg-card border border-border text-text-primary',
                  'placeholder:text-text-muted focus:border-border-focus focus:outline-none',
                  'transition-colors',
                )}
              />
              <button
                onClick={handleAdd}
                disabled={!newName.trim() || adding}
                className={cn(
                  'h-8 px-3 text-xs font-medium rounded-lg',
                  'text-white bg-accent hover:bg-accent-hover',
                  'transition-all disabled:opacity-50',
                  'flex items-center gap-1.5',
                )}
              >
                <Plus size={12} />
                添加
              </button>
            </div>

            {/* Color picker */}
            <div className='flex items-center gap-1.5'>
              {PRESET_COLORS.map((color) => (
                <button
                  key={color}
                  onClick={() => setNewColor(color)}
                  className={cn(
                    'w-5 h-5 rounded-full transition-all',
                    newColor === color
                      ? 'ring-2 ring-offset-2 ring-offset-bg-secondary'
                      : 'hover:scale-110',
                  )}
                  style={{
                    background: color,
                    outlineColor: newColor === color ? color : undefined,
                  }}
                />
              ))}
            </div>
          </div>

          {/* Existing tags */}
          <div className='space-y-1 max-h-[300px] overflow-y-auto'>
            {tags.length === 0 && (
              <p className='text-xs text-text-muted py-4 text-center'>
                还没有创建标签
              </p>
            )}
            {tags.map((tag) => (
              <div
                key={tag.id}
                className={cn(
                  'flex items-center justify-between px-3 py-2 rounded-lg',
                  'bg-bg-card hover:bg-bg-card-hover transition-colors group',
                )}
              >
                <div className='flex items-center gap-2'>
                  <div
                    className='w-3 h-3 rounded-full'
                    style={{ background: tag.color }}
                  />
                  <span className='text-xs font-medium text-text-primary'>
                    {tag.name}
                  </span>
                </div>
                <button
                  onClick={() => handleDelete(tag.id)}
                  className='w-6 h-6 flex items-center justify-center rounded text-text-muted hover:text-danger hover:bg-danger/10 opacity-0 group-hover:opacity-100 transition-all'
                >
                  <X size={12} />
                </button>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}
