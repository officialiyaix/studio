import { useEffect, useMemo, useState } from 'react';
import { Plus, Search } from 'lucide-react';
import { api, asError, type Save, type SaveSummary } from '../lib/api';
import { useStore, type SavesTab } from '../lib/store';
import { cx, formatBytes, formatDate } from '../lib/format';
import { SaveIcon, StatusBadge } from '../components/ui';
import { CreateSaveDialog } from '../features/CreateSaveDialog';
import { ModsManager } from '../features/ModsManager';
import { SaveHero } from '../features/SaveHero';
import { SaveSettingsPanel } from '../features/SaveSettingsPanel';
import { LogsViewer } from '../features/LogsViewer';
import { BackupsPanel } from '../features/BackupsPanel';
import { BisectWizard } from '../features/BisectWizard';
import { GameModeCards } from '../features/GameMode';

const TABS: Array<{ id: SavesTab; label: string }> = [
  { id: 'overview', label: 'Overview' },
  { id: 'mods', label: 'Mods' },
  { id: 'logs', label: 'Logs' },
  { id: 'backups', label: 'Backups' },
  { id: 'settings', label: 'Save settings' },
];

export function SavesPage() {
  const { saves, selectedId, select, savesTab: tab, setSavesTab: setTab, bisectSave, setBisectSave } = useStore();
  const [creating, setCreating] = useState(false);
  const [query, setQuery] = useState('');
  const selected = saves.find((s) => s.id === selectedId) ?? null;
  const filtered = useMemo(() => saves.filter((s) => s.name.toLowerCase().includes(query.trim().toLowerCase())), [saves, query]);

  return (
    <div className="page saves-page">
      <div className="toolbar">
        <button type="button" className="btn btn-cta create-btn" onClick={() => setCreating(true)}>
          <Plus size={17} strokeWidth={2.5} /> Create Save
        </button>
        <p className="tiny muted toolbar-note">Each Save is a separate Minecraft installation: mods, settings and worlds in one never affect another.</p>
      </div>
      <div className="saves-grid">
        <div className="saves-list panel">
          <div className="saves-search">
            <Search size={15} />
            <input className="input" placeholder="Search Saves" aria-label="Search Saves" value={query} onChange={(e) => setQuery(e.target.value)} />
          </div>
          <div className="scroll saves-items" role="listbox" aria-label="Saves">
            {filtered.length === 0 && <p className="muted small" style={{ padding: 12 }}>{saves.length === 0 ? 'No Saves yet.' : 'No Saves match your search.'}</p>}
            {filtered.map((s) => (
              <button type="button" key={s.id} role="option" aria-selected={s.id === selectedId} className={cx('saves-item', s.id === selectedId && 'selected')} onClick={() => select(s.id)}>
                <SaveIcon save={s} size={38} />
                <span className="saves-item-text">
                  <span className="save-card-name">{s.name}</span>
                  <span className="tiny muted">Minecraft {s.minecraftVersion}</span>
                </span>
                <StatusBadge save={s} />
              </button>
            ))}
          </div>
        </div>
        <div className="saves-detail scroll">
          {selected ? (
            <>
              <SaveHero key={selected.id} save={selected} onOpenMods={() => setTab('mods')} />
              <div className="tabs" role="tablist" aria-label="Save sections">
                {TABS.map((t) => (
                  <button key={t.id} type="button" role="tab" aria-selected={tab === t.id} className={cx('tab', tab === t.id && 'active')} onClick={() => setTab(t.id)}>
                    {t.label}
                  </button>
                ))}
              </div>
              <div role="tabpanel" key={`${selected.id}-${tab}`} className="tab-panel">
                {tab === 'overview' && <Overview save={selected} />}
                {tab === 'mods' && <ModsManager save={selected} onFindBroken={() => setBisectSave(selected.id)} />}
                {tab === 'logs' && <LogsViewer save={selected} />}
                {tab === 'backups' && <BackupsPanel save={selected} />}
                {tab === 'settings' && <SaveSettingsPanel save={selected} />}
              </div>
            </>
          ) : (
            <div className="empty panel">
              <p>Create a Save to get started.</p>
            </div>
          )}
        </div>
      </div>
      {creating && <CreateSaveDialog onClose={() => setCreating(false)} />}
      {bisectSave && saves.some((s) => s.id === bisectSave) && <BisectWizard save={saves.find((s) => s.id === bisectSave)!} onClose={() => setBisectSave(null)} />}
    </div>
  );
}

function Overview({ save }: { save: Save }) {
  const { upsertSave, toast } = useStore();
  const [summary, setSummary] = useState<SaveSummary | null>(null);
  const locked = save.status === 'running' || save.status === 'downloading' || save.status === 'launching';
  const switchMode = async (mode: string) => {
    if (mode === save.mode) return;
    try {
      upsertSave(await api.saveSetGameMode(save.id, mode));
      toast(save.status === 'not_installed' ? 'Game mode changed. Its mods are installed on the first launch.' : 'Game mode changed. Switching mods now.', 'success');
    } catch (e) {
      toast(asError(e).message, 'error');
    }
  };
  useEffect(() => {
    api.saveSummary(save.id).then(setSummary).catch(() => undefined);
  }, [save.id, save.status]);
  const folders: Array<[string, string]> = [
    ['saves', 'Worlds'],
    ['mods', 'Mods'],
    ['resourcepacks', 'Resource packs'],
    ['shaderpacks', 'Shader packs'],
    ['screenshots', 'Screenshots'],
    ['logs', 'Logs'],
  ];
  return (
    <div className="overview">
      <div className="stat-grid">
        <div className="stat panel-alt">
          <span className="k">Worlds</span>
          <span className="v">{summary?.worlds ?? '–'}</span>
        </div>
        <div className="stat panel-alt">
          <span className="k">Mods</span>
          <span className="v">{summary?.mods ?? '–'}</span>
        </div>
        <div className="stat panel-alt">
          <span className="k">Screenshots</span>
          <span className="v">{summary?.screenshots ?? '–'}</span>
        </div>
        <div className="stat panel-alt">
          <span className="k">Size on disk</span>
          <span className="v">{summary ? formatBytes(summary.sizeBytes) : '–'}</span>
        </div>
      </div>
      <div className="panel-alt" style={{ padding: 14 }}>
        <div className="section-title" style={{ marginBottom: 10 }}>
          Game mode
        </div>
        <GameModeCards value={save.mode} onChange={switchMode} minecraft={save.minecraftVersion} disabled={locked} />
        <p className="tiny muted" style={{ marginTop: 8 }}>
          {locked ? 'Close Minecraft to change the game mode.' : "Switching installs the new mode's mods and removes the old mode's (only files Stabled installed and you haven't changed)."}
        </p>
      </div>
      <div className="panel-alt" style={{ padding: 14 }}>
        <div className="section-title" style={{ marginBottom: 10 }}>
          Open a folder in this Save
        </div>
        <div style={{ display: 'flex', gap: 8, flexWrap: 'wrap' }}>
          {folders.map(([sub, label]) => (
            <button key={sub} type="button" className="btn btn-sm" onClick={() => api.saveOpenFolder(save.id, sub)}>
              {label}
            </button>
          ))}
        </div>
      </div>
      <p className="tiny muted">
        Created {formatDate(save.createdAt)} · Fabric Loader {save.loaderVersion} · {save.installedStack ? `Installed (${save.installedStack})` : 'Not installed yet'}
      </p>
    </div>
  );
}
