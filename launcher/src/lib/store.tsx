import { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState, type ReactNode } from 'react';
import { api, asError, onEvent, type AppInfo, type AppearanceView, type AuthStatus, type CompatReport, type CrashReport, type Save, type Settings } from './api';
import { applyAppearance } from './theme';

export type Page = 'home' | 'saves' | 'store' | 'optimiser' | 'settings';
export type SavesTab = 'overview' | 'mods' | 'logs' | 'backups' | 'settings';
export type ToastTone = 'info' | 'success' | 'warning' | 'error';
export interface Toast {
  id: number;
  tone: ToastTone;
  message: string;
}

interface Store {
  info: AppInfo | null;
  settings: Settings | null;
  appearance: AppearanceView | null;
  auth: AuthStatus | null;
  saves: Save[];
  savesLoaded: boolean;
  selectedId: string | null;
  page: Page;
  signIn: { open: boolean; reason: string | null };
  toasts: Toast[];
  savesTab: SavesTab;
  setSavesTab: (t: SavesTab) => void;
  /** Go to the Saves page with this Save selected and a tab open. */
  openSave: (id: string, tab?: SavesTab) => void;
  discordPrompt: boolean;
  askDiscord: () => void;
  closeDiscord: () => void;
  /** Problems found by the pre-launch mod check, waiting for the player's decision. */
  launchCheck: { save: Save; report: CompatReport } | null;
  resolveLaunchCheck: (go: boolean) => void;
  /** Crash explanations for a Save that just closed unexpectedly. */
  crash: { save: Save; report: CrashReport } | null;
  showCrash: (c: { save: Save; report: CrashReport } | null) => void;
  /** Save whose Find the broken mod wizard is open. */
  bisectSave: string | null;
  setBisectSave: (id: string | null) => void;
  setPage: (p: Page) => void;
  select: (id: string | null) => void;
  refreshSaves: () => Promise<void>;
  upsertSave: (s: Save) => void;
  removeSave: (id: string) => void;
  updateSettings: (next: Settings) => Promise<void>;
  updateAppearance: (patch: Record<string, unknown>) => Promise<void>;
  setAppearanceView: (v: AppearanceView) => void;
  setAuth: (a: AuthStatus) => void;
  openSignIn: (reason?: string | null) => void;
  closeSignIn: () => void;
  toast: (message: string, tone?: ToastTone) => void;
  dismissToast: (id: number) => void;
  launch: (save: Save) => Promise<void>;
}

const Ctx = createContext<Store | null>(null);

export function useStore(): Store {
  const s = useContext(Ctx);
  if (!s) throw new Error('StoreProvider missing');
  return s;
}

export function StoreProvider({ children }: { children: ReactNode }) {
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [settings, setSettings] = useState<Settings | null>(null);
  const [appearance, setAppearance] = useState<AppearanceView | null>(null);
  const [auth, setAuth] = useState<AuthStatus | null>(null);
  const [saves, setSaves] = useState<Save[]>([]);
  const [savesLoaded, setSavesLoaded] = useState(false);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [page, setPage] = useState<Page>('home');
  const [signIn, setSignIn] = useState<{ open: boolean; reason: string | null }>({ open: false, reason: null });
  const [toasts, setToasts] = useState<Toast[]>([]);
  const [savesTab, setSavesTab] = useState<SavesTab>('overview');
  const [discordPrompt, setDiscordPrompt] = useState(false);
  const [launchCheck, setLaunchCheck] = useState<{ save: Save; report: CompatReport } | null>(null);
  const [crash, setCrash] = useState<{ save: Save; report: CrashReport } | null>(null);
  const [bisectSave, setBisectSave] = useState<string | null>(null);
  const toastId = useRef(1);
  const savesRef = useRef<Save[]>([]);
  savesRef.current = saves;

  const toast = useCallback((message: string, tone: ToastTone = 'info') => {
    const id = toastId.current++;
    setToasts((t) => [...t.slice(-3), { id, tone, message }]);
    window.setTimeout(() => setToasts((t) => t.filter((x) => x.id !== id)), tone === 'error' ? 8000 : 4500);
  }, []);
  const dismissToast = useCallback((id: number) => setToasts((t) => t.filter((x) => x.id !== id)), []);

  const refreshSaves = useCallback(async () => {
    const list = await api.savesList();
    setSaves(list);
    setSavesLoaded(true);
    setSelectedId((cur) => (cur && list.some((s) => s.id === cur) ? cur : list[0]?.id ?? null));
  }, []);

  const upsertSave = useCallback((s: Save) => {
    if (s.testOf) return; // test copies are hidden
    setSaves((list) => {
      const i = list.findIndex((x) => x.id === s.id);
      if (i === -1) return [...list, s];
      const copy = list.slice();
      copy[i] = s;
      return copy;
    });
  }, []);

  const removeSave = useCallback((id: string) => {
    setSaves((list) => {
      const next = list.filter((s) => s.id !== id);
      setSelectedId((cur) => (cur === id ? next[0]?.id ?? null : cur));
      return next;
    });
  }, []);

  // Startup: load everything, then check for a restorable Microsoft session.
  useEffect(() => {
    let cancelled = false;
    (async () => {
      const [i, s, a] = await Promise.all([api.appInfo(), api.settingsGet(), api.appearanceGet()]);
      if (cancelled) return;
      setInfo(i);
      setSettings(s);
      setAppearance(a);
      setPage(s.launcher.startPage);
      await refreshSaves().catch(() => setSavesLoaded(true));
      setAuth(await api.authStatus());
      const restored = await api.authRestore().catch(() => null);
      if (cancelled || !restored) return;
      setAuth(restored);
      // Signed out on startup → show the sign-in popup once for this session.
      if (restored.state !== 'signed_in' && restored.state !== 'offline') {
        setSignIn({ open: true, reason: null });
      }
    })().catch((e) => toast(asError(e).message, 'error'));
    return () => {
      cancelled = true;
    };
  }, [refreshSaves, toast]);

  useEffect(() => {
    if (appearance) applyAppearance(appearance, settings?.launcher.reduceMotion ?? false);
  }, [appearance, settings?.launcher.reduceMotion]);

  // Backend events.
  useEffect(() => {
    const subs = [
      onEvent<Save>('save-updated', (s) => upsertSave(s)),
      onEvent<AuthStatus>('auth-changed', (a) => setAuth(a)),
      onEvent<{ id: string; code: number | null; crash: CrashReport | null }>('game-exited', (e) => {
        // The game may have changed the shared appearance (theme picked in game) — pick it up.
        api.appearanceGet().then(setAppearance).catch(() => undefined);
        const save = savesRef.current.find((s) => s.id === e.id);
        if (e.crash && save) setCrash({ save, report: e.crash });
      }),
      onEvent<{ id: string; checked: number; replaced: number }>('repair-done', (e) => {
        toast(
          e.replaced > 0
            ? `Repair finished: checked ${e.checked.toLocaleString()} game files and replaced ${e.replaced}. Worlds and your mods weren't touched.`
            : `Repair finished: all ${e.checked.toLocaleString()} game files are intact.`,
          'success',
        );
      }),
      onEvent<{ id: string; worlds: number; reason: string; error: string | null }>('backups-done', (e) => {
        if (e.error) toast(`World backup failed: ${e.error}`, 'warning');
        else if (e.worlds > 0) toast(`Backed up ${e.worlds} world${e.worlds === 1 ? '' : 's'} ${e.reason === 'upgrade' ? 'before the update' : 'from your session'}.`, 'success');
      }),
    ];
    return () => {
      subs.forEach((p) => p.then((un) => un()));
    };
  }, [upsertSave, toast]);

  // Re-read appearance when the window regains focus (the game may have changed it while running).
  useEffect(() => {
    const onFocus = () => api.appearanceGet().then(setAppearance).catch(() => undefined);
    window.addEventListener('focus', onFocus);
    return () => window.removeEventListener('focus', onFocus);
  }, []);

  const updateSettings = useCallback(
    async (next: Settings) => {
      try {
        setSettings(await api.settingsSet(next));
      } catch (e) {
        toast(asError(e).message, 'error');
        throw e;
      }
    },
    [toast],
  );

  const updateAppearance = useCallback(
    async (patch: Record<string, unknown>) => {
      try {
        setAppearance(await api.appearanceSet(patch));
      } catch (e) {
        toast(asError(e).message, 'error');
      }
    },
    [toast],
  );

  const openSignIn = useCallback((reason: string | null = null) => setSignIn({ open: true, reason }), []);
  const closeSignIn = useCallback(() => setSignIn({ open: false, reason: null }), []);

  const launchNow = useCallback(
    async (save: Save) => {
      try {
        await api.saveLaunch(save.id);
      } catch (e) {
        const err = asError(e);
        if (err.code === 'SIGN_IN_REQUIRED' || err.code === 'NO_JAVA_ACCESS' || err.code === 'AUTH_UNCONFIGURED') {
          openSignIn(err.message);
        } else {
          toast(err.message, 'error');
        }
      }
    },
    [openSignIn, toast],
  );

  const launch = useCallback(
    async (save: Save) => {
      // The backend enforces this too; the UI check only avoids a round trip and opens sign-in directly.
      if (!auth || (auth.state !== 'signed_in' && auth.state !== 'offline')) {
        openSignIn(auth?.state === 'no_java' ? auth.message : 'Sign in with Microsoft to launch Minecraft.');
        return;
      }
      // Mod compatibility check: wrong Minecraft versions, other loaders, duplicates and known conflicts.
      const report = await api.modsCheck(save.id).catch(() => null);
      if (report && report.problems.some((p) => p.level === 'error')) {
        setLaunchCheck({ save, report });
        return;
      }
      await launchNow(save);
    },
    [auth, openSignIn, launchNow],
  );

  const launchCheckRef = useRef(launchCheck);
  launchCheckRef.current = launchCheck;
  const resolveLaunchCheck = useCallback(
    (go: boolean) => {
      const c = launchCheckRef.current;
      setLaunchCheck(null);
      if (c && go) void launchNow(c.save);
    },
    [launchNow],
  );

  const openSave = useCallback((id: string, tab: SavesTab = 'overview') => {
    setSelectedId(id);
    setSavesTab(tab);
    setPage('saves');
  }, []);

  const value = useMemo<Store>(
    () => ({
      info,
      settings,
      appearance,
      auth,
      saves,
      savesLoaded,
      selectedId,
      page,
      signIn,
      toasts,
      savesTab,
      setSavesTab,
      openSave,
      discordPrompt,
      askDiscord: () => setDiscordPrompt(true),
      closeDiscord: () => setDiscordPrompt(false),
      launchCheck,
      resolveLaunchCheck,
      crash,
      showCrash: setCrash,
      bisectSave,
      setBisectSave,
      setPage,
      select: setSelectedId,
      refreshSaves,
      upsertSave,
      removeSave,
      updateSettings,
      updateAppearance,
      setAppearanceView: setAppearance,
      setAuth,
      openSignIn,
      closeSignIn,
      toast,
      dismissToast,
      launch,
    }),
    [info, settings, appearance, auth, saves, savesLoaded, selectedId, page, signIn, toasts, savesTab, openSave, discordPrompt, launchCheck, resolveLaunchCheck, crash, bisectSave, refreshSaves, upsertSave, removeSave, updateSettings, updateAppearance, openSignIn, closeSignIn, toast, dismissToast, launch],
  );

  return <Ctx.Provider value={value}>{children}</Ctx.Provider>;
}
