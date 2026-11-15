import { useEffect, useRef, useState } from 'react';
import { Cpu, ExternalLink, Home, Layers, Settings as SettingsIcon, ShoppingBag } from 'lucide-react';
import { useStore, type Page } from '../lib/store';
import { cx } from '../lib/format';
import { DiscordGlyph, InfoGlyph } from './icons';

const ITEMS: Array<{ id: Page; label: string; icon: typeof Home }> = [
  { id: 'home', label: 'Home', icon: Home },
  { id: 'saves', label: 'Saves', icon: Layers },
  { id: 'store', label: 'Store', icon: ShoppingBag },
  { id: 'optimiser', label: 'PC Optimiser', icon: Cpu },
  { id: 'settings', label: 'Settings', icon: SettingsIcon },
];

/** Widths from --rail-w / --rail-w-open and the floating gap (--rail-gap) in base.css; hover cards sit just right of the rail. */
const RAIL_W = 68;
const RAIL_W_OPEN = 224;
const RAIL_GAP = 12;

type CardKind = 'discord' | 'about';

/**
 * Floating icon rail that expands over the content (never pushing it sideways) on hover, with a short intent
 * delay, or while keyboard focus is inside. At the bottom: the Stabled Discord and About items, each with a
 * hover card. Opening the Discord invite always asks first (DiscordPrompt).
 */
export function NavRail() {
  const { page, setPage, info, askDiscord } = useStore();
  const [hover, setHover] = useState(false);
  const [keyboardFocus, setKeyboardFocus] = useState(false);
  const [tip, setTip] = useState<{ text: string; x: number; y: number } | null>(null);
  const [card, setCard] = useState<{ kind: CardKind; y: number } | null>(null);
  const timer = useRef<number | undefined>(undefined);
  const cardTimer = useRef<number | undefined>(undefined);
  const itemRefs = useRef<Array<HTMLButtonElement | null>>([]);
  const open = keyboardFocus || hover;
  const invite = info?.config.discordInviteUrl ?? null;

  useEffect(
    () => () => {
      window.clearTimeout(timer.current);
      window.clearTimeout(cardTimer.current);
    },
    [],
  );

  useEffect(() => {
    if (!card) return;
    const onKey = (e: KeyboardEvent) => e.key === 'Escape' && setCard(null);
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [card]);

  const enter = () => {
    window.clearTimeout(timer.current);
    timer.current = window.setTimeout(() => setHover(true), 90);
  };
  const leave = () => {
    window.clearTimeout(timer.current);
    timer.current = window.setTimeout(() => setHover(false), 160);
    setTip(null);
  };

  const showTip = (el: HTMLElement, text: string) => {
    if (open) return;
    const r = el.getBoundingClientRect();
    setTip({ text, x: r.right + 10, y: r.top + r.height / 2 - 14 });
  };

  const showCard = (el: HTMLElement, kind: CardKind) => {
    window.clearTimeout(cardTimer.current);
    const r = el.getBoundingClientRect();
    setTip(null);
    setCard({ kind, y: r.top + r.height / 2 });
  };
  const keepCard = () => window.clearTimeout(cardTimer.current);
  const hideCard = (delay = 180) => {
    window.clearTimeout(cardTimer.current);
    cardTimer.current = window.setTimeout(() => setCard(null), delay);
  };

  const openInvite = () => {
    if (!invite) return;
    setCard(null);
    askDiscord();
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

  const footerProps = (kind: CardKind) => ({
    onMouseEnter: (e: React.MouseEvent<HTMLButtonElement>) => showCard(e.currentTarget, kind),
    onMouseLeave: () => hideCard(),
    onFocus: (e: React.FocusEvent<HTMLButtonElement>) => e.currentTarget.matches(':focus-visible') && showCard(e.currentTarget, kind),
    onBlur: (e: React.FocusEvent<HTMLButtonElement>) => {
      if (!(e.relatedTarget as HTMLElement | null)?.closest('.rail-card')) hideCard(0);
    },
    'aria-expanded': card?.kind === kind,
    'aria-controls': `rail-card-${kind}`,
  });

  return (
    <div className="rail-slot">
      <nav
        className={cx('rail', open && 'open')}
        aria-label="Main navigation"
        onMouseEnter={enter}
        onMouseLeave={leave}
        onFocus={(e) => setKeyboardFocus((e.target as HTMLElement).matches(':focus-visible'))}
        onBlur={(e) => {
          if (!e.currentTarget.contains(e.relatedTarget as Node) && !(e.relatedTarget as HTMLElement | null)?.closest('.rail-card')) setKeyboardFocus(false);
        }}
      >
        <div className="rail-brand" aria-label="Stabled">
          <img src="/logo.png" alt="" />
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
        <button type="button" className="nav-item rail-foot rail-discord" aria-label="Stabled Discord" onClick={openInvite} {...footerProps('discord')}>
          <DiscordGlyph size={20} />
          <span className="rail-label">Discord</span>
        </button>
        <button
          type="button"
          className="nav-item rail-foot rail-about"
          aria-label={`About Stabled${info ? `, version ${info.version}` : ''}`}
          onClick={(e) => (card?.kind === 'about' ? setCard(null) : showCard(e.currentTarget, 'about'))}
          {...footerProps('about')}
        >
          <InfoGlyph size={22} />
          <span className="rail-label">About</span>
        </button>
      </nav>

      {tip && !open && !card && (
        <div className="tip" style={{ left: tip.x, top: tip.y }} role="tooltip">
          {tip.text}
        </div>
      )}

      {card && (
        <div
          id={`rail-card-${card.kind}`}
          key={card.kind}
          className={cx('rail-card', `rail-card-${card.kind}`)}
          role={card.kind === 'about' ? 'tooltip' : 'dialog'}
          aria-label={card.kind === 'about' ? 'About Stabled' : 'Stabled Discord'}
          style={{ left: RAIL_GAP + (open ? RAIL_W_OPEN : RAIL_W) + 12, top: card.y }}
          onMouseEnter={() => {
            keepCard();
            enter();
          }}
          onMouseLeave={() => {
            hideCard();
            leave();
          }}
          onBlur={(e) => {
            if (!e.currentTarget.contains(e.relatedTarget as Node)) hideCard(0);
          }}
        >
          {card.kind === 'discord' ? (
            <>
              <div className="rail-card-head">
                <span className="rail-card-badge discord">
                  <DiscordGlyph size={18} />
                </span>
                <div>
                  <div className="rail-card-title">Stabled Discord</div>
                  <div className="tiny muted">Help, updates and the community</div>
                </div>
              </div>
              {invite ? (
                <a
                  className="rail-card-link"
                  href={invite}
                  onClick={(e) => {
                    e.preventDefault();
                    openInvite();
                  }}
                >
                  {invite}
                  <ExternalLink size={13} />
                </a>
              ) : (
                <div className="tiny muted">The Discord invite isn't configured in this build.</div>
              )}
            </>
          ) : (
            <div className="rail-card-head">
              <img className="rail-card-logo" src="/logo.png" alt="" />
              <div>
                <div className="rail-card-title">Stabled</div>
                <div className="rail-card-version">Version {info?.version ?? '…'}</div>
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
