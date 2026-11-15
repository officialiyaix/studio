import { useEffect, useRef, useState } from 'react';
import { LogOut, RefreshCw, Settings as SettingsIcon, User, UserRoundCog } from 'lucide-react';
import { api, asError } from '../lib/api';
import { useStore, type Page } from '../lib/store';

const TITLES: Record<Page, { title: string; subtitle: string }> = {
  home: { title: 'Home', subtitle: 'Your Saves, launch controls and news' },
  saves: { title: 'Saves', subtitle: 'Separate Minecraft installations, each with its own mods, settings and worlds' },
  optimiser: { title: 'PC Optimiser', subtitle: 'Windows settings that affect games — applied only when you choose' },
  settings: { title: 'Settings', subtitle: 'Socials, appearance, launcher behaviour, accounts and more' },
};

export function Header() {
  const { page, setPage, auth, openSignIn, setAuth, toast } = useStore();
  const [menu, setMenu] = useState(false);
  const wrap = useRef<HTMLDivElement>(null);
  const signedIn = auth?.state === 'signed_in' || auth?.state === 'offline' || auth?.state === 'no_java';
  const t = TITLES[page];

  useEffect(() => {
    if (!menu) return;
    const close = (e: MouseEvent) => {
      if (!wrap.current?.contains(e.target as Node)) setMenu(false);
    };
    const esc = (e: KeyboardEvent) => e.key === 'Escape' && setMenu(false);
    window.addEventListener('mousedown', close);
    window.addEventListener('keydown', esc);
    return () => {
      window.removeEventListener('mousedown', close);
      window.removeEventListener('keydown', esc);
    };
  }, [menu]);

  const onProfile = () => {
    if (!signedIn || !auth?.profile) {
      openSignIn(auth?.message ?? null);
    } else {
      setMenu((m) => !m);
    }
  };

  const name = auth?.profile?.name ?? (auth?.state === 'unconfigured' ? 'Sign-in unavailable' : 'Sign in');

  return (
    <header className="header">
      <div style={{ minWidth: 0 }}>
        <h1 key={`t-${page}`} className="title-anim">
          {t.title}
        </h1>
        <p key={`s-${page}`} className="subtitle title-anim delayed">
          {t.subtitle}
        </p>
      </div>
      <div className="header-actions" ref={wrap} style={{ position: 'relative' }}>
        <button type="button" className="icon-btn" aria-label="Settings" title="Settings" onClick={() => setPage('settings')}>
          <SettingsIcon size={18} />
        </button>
        <button type="button" className="profile-btn" aria-haspopup={signedIn ? 'menu' : 'dialog'} aria-expanded={menu} onClick={onProfile}>
          <span className="avatar" aria-hidden="true">
            {auth?.avatar ? <img src={auth.avatar} alt="" /> : <User size={18} />}
          </span>
          <span className="name">{name}</span>
        </button>
        {menu && auth?.profile && (
          <div className="menu" role="menu" style={{ right: 0, top: 48 }}>
            <div style={{ padding: '8px 10px 6px' }}>
              <div style={{ fontWeight: 650 }}>{auth.profile.name}</div>
              <div className="tiny muted">
                {auth.state === 'signed_in' && 'Minecraft: Java Edition'}
                {auth.state === 'offline' && "Signed in — couldn't refresh the session"}
                {auth.state === 'no_java' && 'No Java Edition access on this account'}
              </div>
            </div>
            <hr />
            <button
              role="menuitem"
              onClick={() => {
                setMenu(false);
                setPage('settings');
                window.setTimeout(() => document.getElementById('settings-accounts')?.scrollIntoView({ behavior: 'smooth' }), 50);
              }}
            >
              <UserRoundCog size={16} /> Account settings
            </button>
            <button
              role="menuitem"
              onClick={() => {
                setMenu(false);
                openSignIn('Choose the Microsoft account to use with Stabled.');
              }}
            >
              <RefreshCw size={16} /> Switch account
            </button>
            <button
              role="menuitem"
              className="danger"
              onClick={async () => {
                setMenu(false);
                try {
                  setAuth(await api.authSignOut());
                  toast('Signed out. Your Saves and settings are unchanged.', 'success');
                } catch (e) {
                  toast(asError(e).message, 'error');
                }
              }}
            >
              <LogOut size={16} /> Sign out
            </button>
          </div>
        )}
      </div>
    </header>
  );
}
