import { useEffect, useRef, useState } from 'react';
import { Check, ChevronDown, Feather, Loader2, Package, Swords } from 'lucide-react';
import { api, asError, type GameMode, type Save } from '../lib/api';
import { useStore } from '../lib/store';
import { cx } from '../lib/format';

let cache: Promise<GameMode[]> | null = null;

/** The game modes (sets of extra mods a Save can switch between), loaded once. */
export function useGameModes(): GameMode[] | null {
  const [modes, setModes] = useState<GameMode[] | null>(null);
  useEffect(() => {
    cache ??= api.gameModes().catch((e) => {
      cache = null;
      throw e;
    });
    let live = true;
    cache.then((m) => live && setModes(m)).catch(() => undefined);
    return () => {
      live = false;
    };
  }, []);
  return modes;
}

export function GameModeIcon({ id, size = 16 }: { id: string; size?: number }) {
  return id === 'cpvp' ? <Swords size={size} /> : <Feather size={size} />;
}

/** Mode cards for choosing a game mode; each lists the mods it adds for the given Minecraft version. */
export function GameModeCards({ value, onChange, minecraft, disabled }: { value: string; onChange: (id: string) => void; minecraft: string | null; disabled?: boolean }) {
  const modes = useGameModes();
  if (!modes) return <div className="skeleton" style={{ height: 120 }} />;
  return (
    <div className="mode-cards" role="radiogroup" aria-label="Game mode">
      {modes.map((m, i) => {
        const v = minecraft ? m.versions[minecraft] : undefined;
        const mods = v?.mods.filter((x) => !x.library) ?? [];
        const libs = v?.mods.filter((x) => x.library) ?? [];
        const selected = value === m.id;
        return (
          <button
            key={m.id}
            type="button"
            role="radio"
            aria-checked={selected}
            disabled={disabled}
            className={cx('mode-card', `mode-${m.id}`, selected && 'selected')}
            style={{ ['--i' as string]: i }}
            onClick={() => onChange(m.id)}
          >
            <span className="mode-card-head">
              <span className="mode-icon">
                <GameModeIcon id={m.id} size={18} />
              </span>
              <span className="mode-name">{m.name}</span>
              {m.isDefault && <span className="badge accent">Default</span>}
              {selected && (
                <span className="mode-check" aria-hidden="true">
                  <Check size={14} />
                </span>
              )}
            </span>
            <span className="tiny muted mode-desc">{m.description}</span>
            {mods.length > 0 && (
              <span className="mode-mods">
                {mods.map((x) => (
                  <span key={x.title} className="mode-mod" title={x.purpose}>
                    <Package size={11} /> {x.title}
                  </span>
                ))}
              </span>
            )}
            {libs.length > 0 && <span className="tiny muted">Plus {libs.map((x) => x.title).join(', ')}, which they need.</span>}
            {v?.notes.map((n) => (
              <span key={n} className="tiny mode-note">
                {n}
              </span>
            ))}
          </button>
        );
      })}
    </div>
  );
}

/** The Save's game mode as a pill in its header, with a menu to switch. */
export function GameModePill({ save }: { save: Save }) {
  const { upsertSave, toast } = useStore();
  const modes = useGameModes();
  const [open, setOpen] = useState(false);
  const [busy, setBusy] = useState(false);
  const wrap = useRef<HTMLDivElement>(null);
  const locked = save.status === 'running' || save.status === 'downloading' || save.status === 'launching';
  const current = modes?.find((m) => m.id === save.mode);

  useEffect(() => {
    if (!open) return;
    const close = (e: MouseEvent) => !wrap.current?.contains(e.target as Node) && setOpen(false);
    const esc = (e: KeyboardEvent) => e.key === 'Escape' && setOpen(false);
    window.addEventListener('mousedown', close);
    window.addEventListener('keydown', esc);
    return () => {
      window.removeEventListener('mousedown', close);
      window.removeEventListener('keydown', esc);
    };
  }, [open]);

  const choose = async (m: GameMode) => {
    setOpen(false);
    if (m.id === save.mode) return;
    setBusy(true);
    try {
      upsertSave(await api.saveSetGameMode(save.id, m.id));
      const installed = save.status !== 'not_installed';
      toast(`"${save.name}" is now in ${m.name} mode. ${installed ? 'Switching its mods now.' : 'Its mods are installed on the first launch.'}`, 'success');
    } catch (e) {
      toast(asError(e).message, 'error');
    } finally {
      setBusy(false);
    }
  };

  if (!current) return null;
  return (
    <div className="mode-pill-wrap" ref={wrap}>
      <button
        type="button"
        className={cx('mode-pill', `mode-${current.id}`)}
        aria-haspopup="menu"
        aria-expanded={open}
        disabled={locked || busy}
        title={locked ? 'Close Minecraft to change the game mode' : 'Game mode'}
        onClick={() => setOpen((o) => !o)}
      >
        {busy ? <Loader2 size={13} className="spin" /> : <GameModeIcon id={current.id} size={13} />}
        {current.name}
        <ChevronDown size={13} style={{ transform: open ? 'rotate(180deg)' : undefined, transition: 'transform var(--speed)' }} />
      </button>
      {open && modes && (
        <div className="menu mode-menu" role="menu">
          <div className="tiny muted mode-menu-title">Game mode</div>
          {modes.map((m) => {
            const v = m.versions[save.minecraftVersion];
            const count = v?.mods.filter((x) => !x.library).length ?? 0;
            return (
              <button key={m.id} role="menuitemradio" aria-checked={m.id === save.mode} className={cx('mode-menu-item', m.id === save.mode && 'active')} onClick={() => choose(m)}>
                <span className={cx('mode-icon', `mode-${m.id}`)}>
                  <GameModeIcon id={m.id} size={15} />
                </span>
                <span style={{ minWidth: 0 }}>
                  <span className="mode-name">{m.name}</span>
                  <span className="tiny muted" style={{ display: 'block' }}>
                    {count ? `${count} mod${count === 1 ? '' : 's'}: ${v!.mods.filter((x) => !x.library).map((x) => x.title).join(', ')}` : m.description}
                  </span>
                </span>
                {m.id === save.mode && <Check size={15} className="mode-menu-check" />}
              </button>
            );
          })}
        </div>
      )}
    </div>
  );
}
