import { create } from 'zustand';
import { listen } from '@tauri-apps/api/event';
import type {
  ConnectionInfo,
  ConnectionStatus,
  Tag,
  ConnectionStatusEvent,
  AuthStatusEvent,
} from '@/types';
import * as api from '@/lib/tauri';
import { useToastStore } from './toastStore';

interface ConnectionStore {
  // ─── State ──────────────────────────────────
  connections: ConnectionInfo[];
  tags: Tag[];
  selectedTagId: string | null; // null = "All"
  searchQuery: string;
  isLoading: boolean;
  duoPushConnectionId: string | null;

  // ─── Actions ────────────────────────────────
  loadConnections: () => Promise<void>;
  loadTags: () => Promise<void>;
  setSelectedTag: (tagId: string | null) => void;
  setSearchQuery: (query: string) => void;
  setDuoPushConnectionId: (id: string | null) => void;

  // ─── Derived ────────────────────────────────
  filteredConnections: () => ConnectionInfo[];

  // ─── Event handling ─────────────────────────
  updateConnectionStatus: (
    connectionId: string,
    status: ConnectionStatus,
    error?: string,
  ) => void;
}

export const useConnectionStore = create<ConnectionStore>((set, get) => ({
  connections: [],
  tags: [],
  selectedTagId: null,
  searchQuery: '',
  isLoading: false,
  duoPushConnectionId: null,

  loadConnections: async () => {
    set({ isLoading: true });
    try {
      const connections = await api.listConnections();
      set({ connections });
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
  setDuoPushConnectionId: (id) => set({ duoPushConnectionId: id }),

  filteredConnections: () => {
    const { connections, selectedTagId, searchQuery } = get();
    let filtered = connections;

    if (selectedTagId) {
      filtered = filtered.filter((c) => c.tag_ids.includes(selectedTagId));
    }

    if (searchQuery.trim()) {
      const q = searchQuery.toLowerCase();
      filtered = filtered.filter(
        (c) =>
          c.name.toLowerCase().includes(q) ||
          c.host.toLowerCase().includes(q) ||
          c.forwards.some(
            (f) =>
              f.target_host.toLowerCase().includes(q) ||
              f.local_port.toString().includes(q) ||
              f.name.toLowerCase().includes(q),
          ),
      );
    }

    return filtered;
  },

  updateConnectionStatus: (connectionId, status, error) => {
    set((state) => ({
      connections: state.connections.map((c) =>
        c.id === connectionId
          ? { ...c, status, error_message: error ?? undefined }
          : c,
      ),
    }));
  },
}));

// ─── Tauri Event Listeners (call once on app init) ────────────────

export async function initEventListeners(): Promise<() => void> {
  const unlistenStatus = await listen<ConnectionStatusEvent>(
    'connection-status',
    (event) => {
      const { connectionId, status, error } = event.payload;
      const store = useConnectionStore.getState();
      store.updateConnectionStatus(connectionId, status, error);

      // Toast notifications
      const conn = store.connections.find((c) => c.id === connectionId);
      const name = conn?.name ?? connectionId.slice(0, 8);
      const toast = useToastStore.getState().addToast;

      switch (status) {
        case 'connected':
          toast('success', `${name} 已连接`);
          break;
        case 'disconnected':
          toast('info', `${name} 已断开`);
          break;
        case 'error':
          toast('error', `${name} 连接失败${error ? ': ' + error : ''}`);
          break;
        case 'reconnecting':
          toast('warning', `${name} 正在重连…`);
          break;
      }
    },
  );

  const unlistenAuth = await listen<AuthStatusEvent>(
    'connection-auth-status',
    (event) => {
      const { connectionId, status } = event.payload;
      if (status === 'waiting_duo_push') {
        useConnectionStore.getState().setDuoPushConnectionId(connectionId);
      } else if (status === 'success' || status === 'failed') {
        useConnectionStore.getState().setDuoPushConnectionId(null);
      }
    },
  );

  return () => {
    unlistenStatus();
    unlistenAuth();
  };
}
