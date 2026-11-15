import { useState } from 'react';
import { Plus } from 'lucide-react';
import { useStore } from '../lib/store';
import { cx, relativeTime } from '../lib/format';
import { SaveIcon, StatusBadge } from '../components/ui';
import { CreateSaveDialog } from '../features/CreateSaveDialog';
import { NewsPanel } from '../features/NewsPanel';
import { SaveHero } from '../features/SaveHero';

export function HomePage() {
  const { saves, savesLoaded, selectedId, select, openSave } = useStore();
  const [creating, setCreating] = useState(false);
  const selected = saves.find((s) => s.id === selectedId) ?? null;

  return (
    <div className="page home">
      <div className="toolbar">
        <button type="button" className="btn btn-cta create-btn" onClick={() => setCreating(true)}>
          <Plus size={17} strokeWidth={2.5} /> Create Save
        </button>
      </div>
      <div className="home-grid">
        <div className="home-main scroll">
          {selected ? (
            <SaveHero key={selected.id} save={selected} onOpenMods={() => openSave(selected.id, 'mods')} onOpenLogs={() => openSave(selected.id, 'logs')} />
          ) : (
            savesLoaded && (
              <section className="hero panel welcome">
                <img src="/logo.png" alt="" width={72} height={72} />
                <div>
                  <h2>Create your first Save</h2>
                  <p className="muted">
                    A Save is a complete Minecraft installation with its own mods, settings and worlds. Stabled installs Minecraft, Fabric, the Stabled client and a tested set of performance mods for you.
                  </p>
                </div>
                <button type="button" className="btn btn-cta btn-lg" onClick={() => setCreating(true)}>
                  <Plus size={18} /> Create Save
                </button>
              </section>
            )
          )}
          {saves.length > 0 && (
            <section aria-label="Your Saves">
              <div className="section-title" style={{ margin: '4px 2px 10px' }}>
                Your Saves · {saves.length}
              </div>
              <div className="save-grid" role="listbox" aria-label="Saves">
                {saves.map((s) => (
                  <button
                    type="button"
                    key={s.id}
                    role="option"
                    aria-selected={s.id === selectedId}
                    className={cx('save-card', s.id === selectedId && 'selected')}
                    onClick={() => select(s.id)}
                  >
                    <SaveIcon save={s} size={44} />
                    <div className="save-card-text">
                      <span className="save-card-name">{s.name}</span>
                      <span className="tiny muted">
                        {s.minecraftVersion} · {s.lastPlayedAt ? relativeTime(s.lastPlayedAt) : 'not played yet'}
                      </span>
                      <StatusBadge save={s} />
                    </div>
                  </button>
                ))}
              </div>
            </section>
          )}
          {!savesLoaded && <div className="skeleton" style={{ height: 220 }} />}
        </div>
        <NewsPanel />
      </div>
      {creating && <CreateSaveDialog onClose={() => setCreating(false)} />}
    </div>
  );
}
