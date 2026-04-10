import { create } from 'zustand';
import { listen } from '@tauri-apps/api/event';
import type {
  TerminalSession,
  TerminalExitEvent,
  ConnectionStatusEvent,
} from '@/types';
import * as api from '@/lib/tauri';

interface TerminalStore {
  terminals: TerminalSession[];
  activeTerminalId: string | null;

  // Pending terminal: when double-clicking a disconnected host, we set this
  // so that after connection succeeds we auto-open a terminal.
  pendingTerminalConnectionId: string | null;
  pendingTerminalConnectionName: string | null;

  openTerminal: (connectionId: string, connectionName: string) => Promise<void>;
  closeTerminal: (terminalId: string) => Promise<void>;
  removeTerminal: (terminalId: string) => void;
  setActiveTerminal: (terminalId: string) => void;
  setPendingTerminal: (
    connectionId: string | null,
    connectionName: string | null,
  ) => void;
}

export const useTerminalStore = create<TerminalStore>((set, get) => ({
  terminals: [],
  activeTerminalId: null,
  pendingTerminalConnectionId: null,
  pendingTerminalConnectionName: null,

  openTerminal: async (connectionId, connectionName) => {
    const terminalId = await api.openTerminal(connectionId, 120, 30);
    const session: TerminalSession = {
      terminalId,
      connectionId,
      connectionName,
    };
    set((state) => ({
      terminals: [...state.terminals, session],
      activeTerminalId: terminalId,
    }));
  },

  closeTerminal: async (terminalId) => {
    try {
      await api.closeTerminal(terminalId);
    } catch {
      // Terminal may already be closed
    }
    get().removeTerminal(terminalId);
  },

  removeTerminal: (terminalId) => {
    set((state) => {
      const terminals = state.terminals.filter(
        (t) => t.terminalId !== terminalId,
      );
      const activeTerminalId =
        state.activeTerminalId === terminalId
          ? (terminals[terminals.length - 1]?.terminalId ?? null)
          : state.activeTerminalId;
      return { terminals, activeTerminalId };
    });
  },

  setActiveTerminal: (terminalId) => set({ activeTerminalId: terminalId }),

  setPendingTerminal: (connectionId, connectionName) =>
    set({
      pendingTerminalConnectionId: connectionId,
      pendingTerminalConnectionName: connectionName,
    }),
}));

// ─── Tauri Event Listeners ────────────────────────────────────────

export async function initTerminalEventListeners() {
  // Handle terminal exit
  await listen<TerminalExitEvent>('terminal-exit', (event) => {
    const state = useTerminalStore.getState();
    state.closeTerminal(event.payload.terminalId);
  });

  // Handle pending terminal: when a connection becomes 'connected' and we have a pending request
  await listen<ConnectionStatusEvent>('connection-status', async (event) => {
    const { connectionId, status } = event.payload;
    const state = useTerminalStore.getState();

    if (
      status === 'connected' &&
      state.pendingTerminalConnectionId === connectionId
    ) {
      const name = state.pendingTerminalConnectionName ?? connectionId;
      state.setPendingTerminal(null, null);
      try {
        await state.openTerminal(connectionId, name);
      } catch {
        // Terminal open failed — connection may have dropped
      }
    }
  });
}
