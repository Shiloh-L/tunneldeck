import { useEffect, useState } from 'react';
import { Header } from '@/components/layout/Header';
import { Sidebar } from '@/components/layout/Sidebar';
import { MainContent } from '@/components/layout/MainContent';
import { ConnectionForm } from '@/components/connection/ConnectionForm';
import { ConnectDialog } from '@/components/connection/ConnectDialog';
import { DuoPushDialog } from '@/components/connection/DuoPushDialog';
import { TagManager } from '@/components/tags/TagManager';
import { LogViewer } from '@/components/logs/LogViewer';
import { Settings } from '@/components/settings/Settings';
import {
  useConnectionStore,
  initEventListeners,
} from '@/stores/connectionStore';
import { initTerminalEventListeners } from '@/stores/terminalStore';
import type { ConnectionInfo } from '@/types';

type Dialog =
  | { type: 'none' }
  | { type: 'new-connection' }
  | { type: 'edit-connection'; connection: ConnectionInfo }
  | { type: 'connect'; connection: ConnectionInfo }
  | { type: 'tags' }
  | { type: 'logs' }
  | { type: 'settings' };

export default function App() {
  const { loadConnections, loadTags } = useConnectionStore();
  const [dialog, setDialog] = useState<Dialog>({ type: 'none' });

  useEffect(() => {
    initEventListeners();
    initTerminalEventListeners();
    loadConnections();
    loadTags();
  }, []);

  return (
    <div className='h-screen flex flex-col bg-bg-primary rounded-xl overflow-hidden border border-border/50'>
      <Header />

      <div className='flex flex-1 min-h-0'>
        <Sidebar
          onNewConnection={() => setDialog({ type: 'new-connection' })}
          onEditConnection={(conn) =>
            setDialog({ type: 'edit-connection', connection: conn })
          }
          onConnectDialog={(conn) =>
            setDialog({ type: 'connect', connection: conn })
          }
          onOpenSettings={() => setDialog({ type: 'settings' })}
          onOpenLogs={() => setDialog({ type: 'logs' })}
          onOpenTags={() => setDialog({ type: 'tags' })}
        />

        <MainContent />
      </div>

      {/* ─── Dialogs ───────────────────────────────────────────── */}
      {dialog.type === 'new-connection' && (
        <ConnectionForm onClose={() => setDialog({ type: 'none' })} />
      )}

      {dialog.type === 'edit-connection' && (
        <ConnectionForm
          connection={dialog.connection}
          onClose={() => setDialog({ type: 'none' })}
        />
      )}

      {dialog.type === 'connect' && (
        <ConnectDialog
          connection={dialog.connection}
          onClose={() => setDialog({ type: 'none' })}
        />
      )}

      {dialog.type === 'tags' && (
        <TagManager onClose={() => setDialog({ type: 'none' })} />
      )}

      {dialog.type === 'logs' && (
        <LogViewer onClose={() => setDialog({ type: 'none' })} />
      )}

      {dialog.type === 'settings' && (
        <Settings onClose={() => setDialog({ type: 'none' })} />
      )}

      <DuoPushDialog />
    </div>
  );
}
