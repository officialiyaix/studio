import { useEffect, useRef, useState } from 'react';
import { ChevronsLeft, ChevronsRight, Cpu, Home, Layers, Settings as SettingsIcon } from 'lucide-react';
import { useStore, type Page } from '../lib/store';
import { cx } from '../lib/format';

const ITEMS: Array<{ id: Page; label: string; icon: typeof Home }> = [
  { id: 'home', label: 'Home', icon: Home },
  { id: 'saves', label: 'Saves', icon: Layers },
  { id: 'optimiser', label: 'PC Optimiser', icon: Cpu },
  { id: 'settings', label: 'Settings', icon: SettingsIcon },
];

/**
 * Compact icon rail that expands over the content (never pushing it sideways):
 * on hover (with a short intent delay), while keyboard focus is inside, or when pinned with the toggle at the bottom.
 */
export function NavRail() {
  const { page, setPage, settings } = useStore();
  const hoverEnabled = settings?.launcher.expandNavOnHover ?? true;
  const [hover, setHover] = useState(false);
  const [keyboardFocus, setKeyboardFocus] = useState(false);
  const [pinned, setPinned] = useState(false);
  const [tip, setTip] = useState<{ text: string; x: number; y: number } | null>(null);
  const timer = useRef<number | undefined>(undefined);
  const itemRefs = useRef<Array<HTMLButtonElement | null>>([]);
  const open = pinned || keyboardFocus || (hoverEnabled && hover);

  useEffect(() => () => window.clearTimeout(timer.current), []);

  const enter = () => {
    window.clearTimeout(timer.current);
    timer.current = window.setTimeout(() => setHover(true), 90);
  };
  const leave = () => {
    window.clearTimeout(timer.current);
    timer.current = window.setTimeout(() => setHover(false), 140);
    setTip(null);
  };

  const showTip = (el: HTMLElement, text: string) => {
    if (open) return;
    const r = el.getBoundingClientRect();
    setTip({ text, x: r.right + 10, y: r.top + r.height / 2 - 14 });
  };

  const onKeyDown = (e: React.KeyboardEvent, i: number) => {
    const n = ITEMS.length;
    let next = -1;
    if (e.key === 'ArrowDown') next = (i + 1) % n;
    if (e.key === 'ArrowUp') next = (i - 1 + n) % n;
    if (e.key === 'Home') next = 0;
    if (e.key === 'End') next = n - 1;
    if (next >= 0) {
      e.preventDefault();
      itemRefs.current[next]?.focus();
    }
  };

  return (
    <div className="rail-slot">
      <nav
        className={cx('rail', open && 'open')}
        aria-label="Main navigation"
        onMouseEnter={enter}
        onMouseLeave={leave}
        onFocus={(e) => setKeyboardFocus((e.target as HTMLElement).matches(':focus-visible'))}
        onBlur={(e) => {
          if (!e.currentTarget.contains(e.relatedTarget as Node)) setKeyboardFocus(false);
        }}
      >
        <div className="rail-brand" aria-label="Stabled">
          <img src="/mark.svg" alt="" />
          <span className="word rail-label">Stabled</span>
        </div>
        {ITEMS.map((item, i) => {
          const Icon = item.icon;
          return (
            <button
              key={item.id}
              ref={(el) => {
                itemRefs.current[i] = el;
              }}
              type="button"
              className="nav-item"
              aria-current={page === item.id ? 'page' : undefined}
              aria-label={item.label}
              onClick={() => setPage(item.id)}
              onKeyDown={(e) => onKeyDown(e, i)}
              onMouseEnter={(e) => showTip(e.currentTarget, item.label)}
              onMouseLeave={() => setTip(null)}
              onFocus={(e) => e.currentTarget.matches(':focus-visible') && showTip(e.currentTarget, item.label)}
              onBlur={() => setTip(null)}
            >
              <Icon size={20} strokeWidth={2} />
              <span className="rail-label">{item.label}</span>
            </button>
          );
        })}
        <div className="rail-spacer" />
        <button
          type="button"
          className="nav-item rail-pin"
          aria-expanded={open}
          aria-label={pinned ? 'Collapse navigation' : 'Expand navigation'}
          onClick={() => {
            setPinned((p) => !p);
            setTip(null);
          }}
          onMouseEnter={(e) => showTip(e.currentTarget, pinned ? 'Collapse navigation' : 'Expand navigation')}
          onMouseLeave={() => setTip(null)}
        >
          {pinned ? <ChevronsLeft size={20} /> : <ChevronsRight size={20} />}
          <span className="rail-label">{pinned ? 'Collapse' : 'Keep expanded'}</span>
        </button>
      </nav>
      {tip && !open && (
        <div className="tip" style={{ left: tip.x, top: tip.y }} role="tooltip">
          {tip.text}
        </div>
      )}
    </div>
  );
}
