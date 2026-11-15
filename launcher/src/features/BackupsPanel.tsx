import { useCallback, useEffect, useState } from 'react';
import { Archive, ArchiveRestore, FolderOpen, Globe2, History, Loader2, Plus, Settings2, ShieldCheck, Trash2 } from 'lucide-react';
import { api, asError, type BackupsView, type Save, type WorldBackup } from '../lib/api';
import { useStore } from '../lib/store';
import { cx, formatBytes, relativeTime } from '../lib/format';
import { Callout, ConfirmDialog } from '../components/ui';

const REASON: Record<WorldBackup['reason'], string> = {
  session: 'After playing',
  upgrade: 'Before update',
  manual: 'Manual',
};

/** World backups for one Save: automatic ones (after sessions, before updates) and manual ones. */
export function BackupsPanel({ save }: { save: Save }) {
  const { toast, settings, setPage } = useStore();
  const [view, setView] = useState<BackupsView | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [restore, setRestore] = useState<WorldBackup | null>(null);
  const [remove, setRemove] = useState<WorldBackup | null>(null);
  const locked = save.status === 'running' || save.status === 'downloading' || save.status === 'launching';

  const load = useCallback(async () => {
    try {
      setView(await api.backupsView(save.id));
    } catch (e) {
      toast(asError(e).message, 'error');
    }
  }, [save.id, toast]);

  useEffect(() => {
    load();
  }, [load, save.status]);

  const backUp = async (world?: string) => {
    setBusy(world ?? 'all');
    try {
      const n = await api.backupsCreate(save.id, world);
      toast(n ? `Backed up ${n} world${n === 1 ? '' : 's'}.` : 'There are no worlds to back up yet.', n ? 'success' : 'info');
    } catch (e) {
      toast(asError(e).message, 'error');
    } finally {
      setBusy(null);
      load();
    }
  };

  const auto = settings?.backups;
  const byWorld = new Map<string, WorldBackup[]>();
  view?.backups.forEach((b) => byWorld.set(b.world, [...(byWorld.get(b.world) ?? []), b]));
  const worldNames = Array.from(new Set([...(view?.worlds.map((w) => w.folder) ?? []), ...byWorld.keys()]));

  return (
    <div className="backups">
      <div className="backups-head panel-alt">
        <span className="backups-icon">
          <ShieldCheck size={20} />
        </span>
        <div style={{ minWidth: 0, flex: 1 }}>
          <div className="backups-title">World backups</div>
          <p className="tiny muted">
            {auto?.afterSessions || auto?.beforeUpgrades
              ? `Automatic: ${[auto.afterSessions && 'after you play (worlds you changed)', auto.beforeUpgrades && 'before Stabled updates this Save'].filter(Boolean).join(' and ')}. Keeps the last ${auto.keep} automatic backups per world.`
              : 'Automatic backups are off. Turn them on in Settings → World backups.'}
          </p>
        </div>
        <button type="button" className="btn btn-sm" onClick={() => setPage('settings')} title="Backup settings">
          <Settings2 size={14} /> Settings
        </button>
        <button type="button" className="btn btn-primary btn-sm" onClick={() => backUp()} disabled={locked || busy !== null}>
          {busy === 'all' ? <Loader2 size={14} className="spin" /> : <Plus size={14} />} Back up all now
        </button>
      </div>
      {locked && <Callout tone="info">Backups and restores wait until Minecraft closes, so worlds aren't copied while they're being written.</Callout>}

      {!view && <div className="skeleton" style={{ height: 160 }} />}
      {view && worldNames.length === 0 && (
        <div className="logs-empty">
          <Globe2 size={28} />
          <p>No worlds yet. Backups appear here once you've played in this Save.</p>
        </div>
      )}
      <div className="backup-worlds">
        {worldNames.map((w, wi) => {
          const info = view?.worlds.find((x) => x.folder === w);
          const list = byWorld.get(w) ?? [];
          return (
            <section key={w} className="backup-world panel-alt" style={{ ['--i' as string]: wi }}>
              <header className="backup-world-head">
                <Globe2 size={17} />
                <div style={{ minWidth: 0, flex: 1 }}>
                  <div className="backup-world-name">{w}</div>
                  <div className="tiny muted">
                    {info ? `${formatBytes(info.size)}${info.modified ? ` · played ${relativeTime(info.modified)}` : ''}` : 'World no longer in this Save'} · {list.length} backup{list.length === 1 ? '' : 's'}
                  </div>
                </div>
                {info && (
                  <button type="button" className="btn btn-sm" disabled={locked || busy !== null} onClick={() => backUp(w)}>
                    {busy === w ? <Loader2 size={14} className="spin" /> : <Archive size={14} />} Back up
                  </button>
                )}
              </header>
              {list.length > 0 && (
                <ul className="backup-list">
                  {list.map((b) => (
                    <li key={b.id} className="backup-item">
                      <History size={14} />
                      <span className="backup-when" title={new Date(b.createdAt).toLocaleString()}>
                        {relativeTime(b.createdAt)}
                      </span>
                      <span className={cx('badge', b.reason === 'upgrade' ? 'info' : b.reason === 'manual' ? 'success' : '')}>{REASON[b.reason] ?? b.reason}</span>
                      <span className="tiny muted">{formatBytes(b.size)}</span>
                      <span className="backup-actions">
                        <button type="button" className="btn btn-sm" disabled={locked} onClick={() => setRestore(b)}>
                          <ArchiveRestore size={14} /> Restore
                        </button>
                        <button type="button" className="icon-btn plain sm" aria-label="Delete backup" onClick={() => setRemove(b)}>
                          <Trash2 size={14} />
                        </button>
                      </span>
                    </li>
                  ))}
                </ul>
              )}
            </section>
          );
        })}
      </div>
      {view && view.backups.length > 0 && (
        <div className="tiny muted backups-foot">
          {view.backups.length} backup{view.backups.length === 1 ? '' : 's'} · {formatBytes(view.totalSize)}
          <button type="button" className="btn btn-ghost btn-sm" onClick={() => api.backupsOpenFolder(save.id).catch((e) => toast(asError(e).message, 'error'))}>
            <FolderOpen size={13} /> Open backups folder
          </button>
        </div>
      )}

      {restore && (
        <ConfirmDialog
          title={`Restore "${restore.world}"?`}
          confirmLabel="Restore as a copy"
          onCancel={() => setRestore(null)}
          onConfirm={async () => {
            const b = restore;
            setRestore(null);
            try {
              const name = await api.backupsRestore(save.id, b.id);
              toast(`Restored as "${name}". It's in your world list next to the original.`, 'success');
            } catch (e) {
              toast(asError(e).message, 'error');
            }
            load();
          }}
        >
          <p>
            Stabled adds the backup from {new Date(restore.createdAt).toLocaleString()} as a new world called "{restore.world} (restored …)". Your current world is not changed or replaced.
          </p>
        </ConfirmDialog>
      )}
      {remove && (
        <ConfirmDialog
          title="Delete this backup?"
          confirmLabel="Delete backup"
          danger
          onCancel={() => setRemove(null)}
          onConfirm={async () => {
            const b = remove;
            setRemove(null);
            try {
              await api.backupsDelete(save.id, b.id);
            } catch (e) {
              toast(asError(e).message, 'error');
            }
            load();
          }}
        >
          <p>
            The {REASON[remove.reason]?.toLowerCase() ?? ''} backup of "{remove.world}" from {new Date(remove.createdAt).toLocaleString()} ({formatBytes(remove.size)}) will be deleted permanently. The world itself isn't affected.
          </p>
        </ConfirmDialog>
      )}
    </div>
  );
}
