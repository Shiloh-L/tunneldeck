import { create } from 'zustand';
import { listen } from '@tauri-apps/api/event';
import type {
  TunnelInfo,
  TunnelStatus,
  Tag,
  TunnelStatusEvent,
  AuthStatusEvent,
} from '@/types';
import * as api from '@/lib/tauri';

interface TunnelStore {
  // ─── State ──────────────────────────────────
  tunnels: TunnelInfo[];
  tags: Tag[];
  selectedTagId: string | null; // null = "All"
  searchQuery: string;
  isLoading: boolean;
  duoPushTunnelId: string | null; // tunnel currently waiting for Duo

  // ─── Actions ────────────────────────────────
  loadTunnels: () => Promise<void>;
  loadTags: () => Promise<void>;
  setSelectedTag: (tagId: string | null) => void;
  setSearchQuery: (query: string) => void;
  setDuoPushTunnelId: (id: string | null) => void;

  // ─── Derived ────────────────────────────────
  filteredTunnels: () => TunnelInfo[];

  // ─── Event handling ─────────────────────────
  updateTunnelStatus: (
    tunnelId: string,
    status: TunnelStatus,
    error?: string,
  ) => void;
}

export const useTunnelStore = create<TunnelStore>((set, get) => ({
  tunnels: [],
  tags: [],
  selectedTagId: null,
  searchQuery: '',
  isLoading: false,
  duoPushTunnelId: null,

  loadTunnels: async () => {
    set({ isLoading: true });
    try {
      const tunnels = await api.listTunnels();
      set({ tunnels });
    } finally {
      set({ isLoading: false });
    }
  },

  loadTags: async () => {
    const tags = await api.listTags();
    set({ tags });
  },

  setSelectedTag: (tagId) => set({ selectedTagId: tagId }),
  setSearchQuery: (query) => set({ searchQuery: query }),
  setDuoPushTunnelId: (id) => set({ duoPushTunnelId: id }),

  filteredTunnels: () => {
    const { tunnels, selectedTagId, searchQuery } = get();
    let filtered = tunnels;

    if (selectedTagId) {
      filtered = filtered.filter((t) => t.tag_ids.includes(selectedTagId));
    }

    if (searchQuery.trim()) {
      const q = searchQuery.toLowerCase();
      filtered = filtered.filter(
        (t) =>
          t.name.toLowerCase().includes(q) ||
          t.jump_host.toLowerCase().includes(q) ||
          t.target_host.toLowerCase().includes(q) ||
          t.local_port.toString().includes(q),
      );
    }

    return filtered;
  },

  updateTunnelStatus: (tunnelId, status, error) => {
    set((state) => ({
      tunnels: state.tunnels.map((t) =>
        t.id === tunnelId
          ? { ...t, status, error_message: error ?? undefined }
          : t,
      ),
    }));
  },
}));

// ─── Tauri Event Listeners (call once on app init) ────────────────

export async function initEventListeners() {
  await listen<TunnelStatusEvent>('tunnel-status', (event) => {
    const { tunnelId, status, error } = event.payload;
    useTunnelStore.getState().updateTunnelStatus(tunnelId, status, error);
  });

  await listen<AuthStatusEvent>('tunnel-auth-status', (event) => {
    const { tunnelId, status } = event.payload;
    if (status === 'waiting_duo_push') {
      useTunnelStore.getState().setDuoPushTunnelId(tunnelId);
    } else if (status === 'success' || status === 'failed') {
      useTunnelStore.getState().setDuoPushTunnelId(null);
    }
  });
}
