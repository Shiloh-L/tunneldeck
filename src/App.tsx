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
import { useTunnelStore, initEventListeners } from '@/stores/tunnelStore';
import type { TunnelInfo } from '@/types';

type Dialog =
  | { type: 'none' }
  | { type: 'new-tunnel' }
  | { type: 'edit-tunnel'; tunnel: TunnelInfo }
  | { type: 'connect'; tunnel: TunnelInfo }
  | { type: 'tags' }
  | { type: 'logs' }
  | { type: 'settings' };

export default function App() {
  const { loadTunnels, loadTags } = useTunnelStore();
  const [dialog, setDialog] = useState<Dialog>({ type: 'none' });

  useEffect(() => {
    // Initialize event listeners and load data
    initEventListeners();
    loadTunnels();
    loadTags();
  }, []);

  return (
    <div className='h-screen flex flex-col bg-bg-primary rounded-xl overflow-hidden border border-border/50'>
      <Header />

      <div className='flex flex-1 min-h-0'>
        <Sidebar
          onNewTunnel={() => setDialog({ type: 'new-tunnel' })}
          onOpenSettings={() => setDialog({ type: 'settings' })}
          onOpenLogs={() => setDialog({ type: 'logs' })}
        />

        <main className='flex-1 flex flex-col min-w-0'>
          <TunnelList
            onEdit={(tunnel) => setDialog({ type: 'edit-tunnel', tunnel })}
            onConnect={(tunnel) => setDialog({ type: 'connect', tunnel })}
          />
        </main>
      </div>

      {/* ─── Dialogs ───────────────────────────────────────────── */}
      {dialog.type === 'new-tunnel' && (
        <TunnelForm onClose={() => setDialog({ type: 'none' })} />
      )}

      {dialog.type === 'edit-tunnel' && (
        <TunnelForm
          tunnel={dialog.tunnel}
          onClose={() => setDialog({ type: 'none' })}
        />
      )}

      {dialog.type === 'connect' && (
        <ConnectDialog
          tunnel={dialog.tunnel}
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

      {/* Duo Push dialog is global — shows whenever DuoPush is active */}
      <DuoPushDialog />
    </div>
  );
}
