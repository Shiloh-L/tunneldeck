import { useEffect, useState } from 'react';
import { Header } from '@/components/layout/Header';
import { Sidebar } from '@/components/layout/Sidebar';
import { TunnelList } from '@/components/tunnel/TunnelList';
import { TunnelForm } from '@/components/tunnel/TunnelForm';
import { ConnectDialog } from '@/components/tunnel/ConnectDialog';
import { DuoPushDialog } from '@/components/tunnel/DuoPushDialog';
import { TagManager } from '@/components/tags/TagManager';
import { LogViewer } from '@/components/logs/LogViewer';
import { Settings } from '@/components/settings/Settings';
import { useConnectionStore, initEventListeners } from '@/stores/tunnelStore';
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
    loadConnections();
    loadTags();
  }, []);

  return (
    <div className='h-screen flex flex-col bg-bg-primary rounded-xl overflow-hidden border border-border/50'>
      <Header />

      <div className='flex flex-1 min-h-0'>
        <Sidebar
          onNewConnection={() => setDialog({ type: 'new-connection' })}
          onOpenSettings={() => setDialog({ type: 'settings' })}
          onOpenLogs={() => setDialog({ type: 'logs' })}
        />

        <main className='flex-1 flex flex-col min-w-0'>
          <TunnelList
            onEdit={(conn) =>
              setDialog({ type: 'edit-connection', connection: conn })
            }
            onConnect={(conn) =>
              setDialog({ type: 'connect', connection: conn })
            }
          />
        </main>
      </div>

      {/* ─── Dialogs ───────────────────────────────────────────── */}
      {dialog.type === 'new-connection' && (
        <TunnelForm onClose={() => setDialog({ type: 'none' })} />
      )}

      {dialog.type === 'edit-connection' && (
        <TunnelForm
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
