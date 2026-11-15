import { Loader2 } from 'lucide-react';
import { useStore } from './lib/store';
import { cx } from './lib/format';
import { NavRail } from './components/NavRail';
import { Header } from './components/Header';
import { SpecsPanel } from './components/SpecsPanel';
import { SignInDialog } from './components/SignInDialog';
import { Toasts } from './components/Toasts';
import { HomePage } from './pages/Home';
import { SavesPage } from './pages/Saves';
import { OptimiserPage } from './pages/Optimiser';
import { SettingsPage } from './pages/Settings';
import { StorePage } from './pages/Store';
import { CrashDialog, DiscordPrompt, LaunchCheckDialog } from './features/GlobalDialogs';

function Backdrop() {
  const { appearance } = useStore();
  const a = appearance?.appearance;
  const custom = a?.theme === 'custom' && appearance?.customBackground;
  return (
    <div className="app-bg" aria-hidden="true">
      {custom && (
        <>
          <img key={a!.customBackground?.revision ?? 0} className="bg-fade" src={appearance!.customBackground!} alt="" style={{ objectPosition: `${(a!.customBackground?.focusX ?? 0.5) * 100}% ${(a!.customBackground?.focusY ?? 0.5) * 100}%` }} />
          <div className="dim" style={{ opacity: a!.customBackground?.dim ?? 0.45 }} />
        </>
      )}
    </div>
  );
}

function ActivityStatus() {
  const { saves } = useStore();
  const active = saves.filter((s) => s.status === 'downloading' || s.status === 'launching' || s.status === 'running');
  if (active.length === 0) return null;
  return (
    <div className="footer-status" aria-live="polite">
      {active.map((s) => (
        <span key={s.id} style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
          {s.status === 'running' ? <span className="status-dot pulse" style={{ color: 'var(--success)' }} /> : <Loader2 size={13} className="spin" />}
          {s.name}: {s.status === 'running' ? 'running' : s.status === 'launching' ? 'launching' : 'installing'}
        </span>
      ))}
    </div>
  );
}

export function App() {
  const { page, signIn, appearance } = useStore();
  if (!appearance) {
    return (
      <div className="boot">
        <img src="/logo.png" alt="" width={64} height={64} />
      </div>
    );
  }
  return (
    <>
      <Backdrop />
      <div className={cx('shell', signIn.open && 'blocked')} aria-hidden={signIn.open || undefined}>
        <NavRail />
        <Header />
        <main className="main" id="main">
          {page === 'home' && <HomePage key="home" />}
          {page === 'saves' && <SavesPage key="saves" />}
          {page === 'store' && <StorePage key="store" />}
          {page === 'optimiser' && <OptimiserPage key="optimiser" />}
          {page === 'settings' && <SettingsPage key="settings" />}
        </main>
        <footer className="footer">
          <SpecsPanel />
          <ActivityStatus />
        </footer>
      </div>
      <SignInDialog />
      <DiscordPrompt />
      <LaunchCheckDialog />
      <CrashDialog />
      <Toasts />
    </>
  );
}
