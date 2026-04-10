import { create } from 'zustand';
import { listen } from '@tauri-apps/api/event';
import type { TerminalSession, TerminalExitEvent } from '@/types';
import * as api from '@/lib/tauri';

interface TerminalStore {
  terminals: TerminalSession[];
  activeTerminalId: string | null;
  showPanel: boolean;

  openTerminal: (connectionId: string, connectionName: string) => Promise<void>;
  closeTerminal: (terminalId: string) => Promise<void>;
  removeTerminal: (terminalId: string) => void;
  setActiveTerminal: (terminalId: string) => void;
  setShowPanel: (show: boolean) => void;
  togglePanel: () => void;
}

export const useTerminalStore = create<TerminalStore>((set, get) => ({
  terminals: [],
  activeTerminalId: null,
  showPanel: false,

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
      showPanel: true,
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
      return {
        terminals,
        activeTerminalId,
        showPanel: terminals.length > 0 ? state.showPanel : false,
      };
    });
  },

  setActiveTerminal: (terminalId) => set({ activeTerminalId: terminalId }),
  setShowPanel: (show) => set({ showPanel: show }),
  togglePanel: () => set((state) => ({ showPanel: !state.showPanel })),
}));

// ─── Tauri Event Listeners ────────────────────────────────────────

export async function initTerminalEventListeners() {
  await listen<TerminalExitEvent>('terminal-exit', (event) => {
    const state = useTerminalStore.getState();
    state.closeTerminal(event.payload.terminalId);
  });
}
