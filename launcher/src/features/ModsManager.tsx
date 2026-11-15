import { useCallback, useEffect, useState } from 'react';
import { FolderOpen, Lock, Plus, RefreshCw, SearchCode, ShieldCheck, ShoppingBag, Trash2 } from 'lucide-react';
import { open } from '@tauri-apps/plugin-dialog';
import { api, asError, type ModEntry, type ModList, type Save } from '../lib/api';
import { useStore } from '../lib/store';
import { cx, formatBytes } from '../lib/format';
import { Callout, ConfirmDialog, Toggle } from '../components/ui';

export function ModsManager({ save, onFindBroken }: { save: Save; onFindBroken?: () => void }) {
  const { toast, setPage } = useStore();
  const [list, setList] = useState<ModList | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [removing, setRemoving] = useState<ModEntry | null>(null);
  const locked = save.status === 'running' || save.status === 'downloading' || save.status === 'launching';

  const load = useCallback(async () => {
    try {
      setList(await api.modsList(save.id));
      setError(null);
    } catch (e) {
      setError(asError(e).message);
    }
  }, [save.id]);

  useEffect(() => {
    load();
  }, [load, save.status]);

  const act = async (fn: () => Promise<unknown>, ok?: string) => {
    try {
      await fn();
      if (ok) toast(ok, 'success');
    } catch (e) {
      toast(asError(e).message, 'error');
    }
    load();
  };

  const add = async () => {
    const picked = await open({ multiple: true, filters: [{ name: 'Fabric mods', extensions: ['jar'] }] });
    if (!picked) return;
    const paths = Array.isArray(picked) ? picked : [picked];
    act(() => api.modsAdd(save.id, paths), `Added ${paths.length} mod${paths.length === 1 ? '' : 's'} to "${save.name}" only.`);
  };

  const managed = list?.mods.filter((m) => m.managed) ?? [];
  const user = list?.mods.filter((m) => !m.managed) ?? [];
  const problems = (list?.issues.filter((i) => i.level === 'error').length ?? 0) + (list?.mods.reduce((n, m) => n + (m.enabled ? m.issues.filter((i) => i.level === 'error').length : 0), 0) ?? 0);

  return (
    <div className="mods">
      <div className="mods-toolbar">
        <button type="button" className="btn btn-primary" onClick={add} disabled={locked}>
          <Plus size={16} /> Add mods
        </button>
        <button type="button" className="btn" onClick={() => api.saveOpenFolder(save.id, 'mods')}>
          <FolderOpen size={16} /> Open mods folder
        </button>
        <button type="button" className="btn" onClick={() => setPage('store')}>
          <ShoppingBag size={16} /> Browse the Store
        </button>
        {onFindBroken && (
          <button type="button" className="btn" onClick={onFindBroken} disabled={locked} title="Test your mods in a copy of this Save to find the one causing a crash">
            <SearchCode size={16} /> Find the broken mod
          </button>
        )}
        <button type="button" className="icon-btn" aria-label="Refresh mod list" onClick={load}>
          <RefreshCw size={16} />
        </button>
        <span className="tiny muted" style={{ marginLeft: 'auto' }}>
          Changes here affect "{save.name}" only.
        </span>
      </div>
      {locked && (
        <Callout tone="warning">
          <Lock size={13} style={{ verticalAlign: '-2px' }} /> Mods are locked while this Save is running or installing, so files can't change under the game.
        </Callout>
      )}
      {error && <Callout tone="danger">{error}</Callout>}
      {list?.issues.map((i) => (
        <Callout key={i.message} tone={i.level === 'error' ? 'danger' : 'warning'}>
          {i.message}
        </Callout>
      ))}
      {problems > 0 && (
        <p className="tiny muted">
          {problems} problem{problems === 1 ? '' : 's'} found from mod metadata. Stabled checks loaders, Minecraft versions, dependencies, duplicates, declared incompatibilities and known conflicts, and checks again before every launch. It can't predict every conflict: if the game still crashes, use Find the broken mod.
        </p>
      )}

      <div className="section-title" style={{ marginTop: 6 }}>
        <ShieldCheck size={14} /> Stabled components
      </div>
      <div className="mod-list">
        {!list && <div className="skeleton" style={{ height: 120 }} />}
        {list && managed.length === 0 && <p className="muted small">Installed the first time this Save is launched or installed.</p>}
        {managed.map((m) => (
          <ModRow key={m.file} mod={m} locked={locked} onToggle={(v) => act(() => api.modsSetEnabled(save.id, m.file, v))} onRemove={() => setRemoving(m)} />
        ))}
        {list?.removed.map((r) => (
          <div key={r.modId} className="mod-row disabled">
            <div className="mod-main">
              <div className="mod-title">
                <span className="mod-name">{r.title}</span>
                <span className="badge">Removed</span>
              </div>
              <div className="tiny muted mod-sub">You removed this Stabled component. Restore it to reinstall it on the next launch.</div>
            </div>
            <div className="mod-actions">
              <button type="button" className="btn btn-sm" disabled={locked} onClick={() => act(() => api.modsRestoreComponent(save.id, r.modId), `${r.title} will be reinstalled on the next launch.`)}>
                Restore
              </button>
            </div>
          </div>
        ))}
      </div>

      <div className="section-title" style={{ marginTop: 10 }}>
        Your mods
      </div>
      <div className="mod-list">
        {list && user.length === 0 && (
          <div className="empty">
            <p>No mods of your own yet. Add Fabric mods made for Minecraft {save.minecraftVersion}.</p>
          </div>
        )}
        {user.map((m) => (
          <ModRow key={m.file} mod={m} locked={locked} onToggle={(v) => act(() => api.modsSetEnabled(save.id, m.file, v))} onRemove={() => setRemoving(m)} />
        ))}
      </div>

      {removing && (
        <ConfirmDialog
          title={`Remove ${removing.name}?`}
          confirmLabel="Move to Recycle Bin"
          danger
          onCancel={() => setRemoving(null)}
          onConfirm={() => {
            const m = removing;
            setRemoving(null);
            act(() => api.modsRemove(save.id, m.file), `${m.name} was moved to the Recycle Bin.`);
          }}
        >
          <p>
            <code>{removing.file}</code> will be removed from "{save.name}". Other Saves are not affected.
          </p>
          {removing.managed && <p className="muted small">This is a Stabled-managed performance mod. Stabled won't reinstall it unless you choose Restore.</p>}
        </ConfirmDialog>
      )}
    </div>
  );
}

function ModRow({ mod, locked, onToggle, onRemove }: { mod: ModEntry; locked: boolean; onToggle: (v: boolean) => void; onRemove: () => void }) {
  const required = mod.managed?.required ?? false;
  const errors = mod.issues.filter((i) => i.level === 'error');
  return (
    <div className={cx('mod-row', !mod.enabled && 'disabled')}>
      <div className="mod-main">
        <div className="mod-title">
          <span className="mod-name">{mod.name}</span>
          {mod.version && <span className="tiny muted">{mod.version}</span>}
          {mod.managed && <span className="badge accent">{required ? 'Required' : 'Stabled'}</span>}
          {mod.managed?.beta && <span className="badge warning">Beta</span>}
          {!mod.enabled && <span className="badge">Disabled</span>}
        </div>
        <div className="tiny muted mod-sub">
          {mod.managed?.purpose ?? mod.description ?? mod.file}
          <span> · {formatBytes(mod.size)}</span>
        </div>
        {mod.enabled &&
          errors.map((i) => (
            <div key={i.message} className="mod-issue">
              {i.message}
            </div>
          ))}
      </div>
      <div className="mod-actions">
        {required ? (
          <span className="tiny muted" title="Required by the Stabled client">
            <Lock size={14} />
          </span>
        ) : (
          <>
            <Toggle label={`${mod.enabled ? 'Disable' : 'Enable'} ${mod.name}`} checked={mod.enabled} disabled={locked} onChange={onToggle} />
            <button type="button" className="icon-btn plain sm" aria-label={`Remove ${mod.name}`} disabled={locked} onClick={onRemove}>
              <Trash2 size={15} />
            </button>
          </>
        )}
      </div>
    </div>
  );
}
