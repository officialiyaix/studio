// Typed wrappers around the Stabled backend commands (src-tauri/src/commands.rs).
// The backend never returns credentials; the types below are the complete data the UI can see.
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

export interface AppError {
  code: string;
  message: string;
}

export function asError(e: unknown): AppError {
  if (e && typeof e === 'object' && 'message' in e) {
    const o = e as { code?: string; message: string };
    return { code: o.code ?? 'ERROR', message: String(o.message) };
  }
  return { code: 'ERROR', message: String(e) };
}

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(cmd, args);
  } catch (e) {
    throw asError(e);
  }
}

// ---------- App ----------
export interface ConfigStatus {
  microsoftConfigured: boolean;
  discordPresenceConfigured: boolean;
  discordInviteUrl: string | null;
  newsFeedConfigured: boolean;
}
export interface AppInfo {
  version: string;
  platform: string;
  isWindows: boolean;
  config: ConfigStatus;
  dataDir: string;
  sharedDir: string;
  clientBundled: boolean;
  supportedVersions: string[];
}

export interface Gpu {
  name: string;
  dedicatedMemoryBytes: number | null;
}
export interface SystemStatic {
  cpu: string | null;
  cpuCores: number | null;
  cpuThreads: number | null;
  gpus: Gpu[];
  memoryInstalledBytes: number | null;
  memoryUsableBytes: number | null;
  os: { name: string; version: string | null; build: string | null; architecture: string };
}
export interface SystemDynamic {
  memoryUsedBytes: number | null;
  memoryUsableBytes: number | null;
}

export interface Settings {
  launcher: {
    afterLaunch: 'keep' | 'minimize' | 'hide';
    expandNavOnHover: boolean;
    reduceMotion: boolean;
    startPage: 'home' | 'saves';
    showMinecraftNews: boolean;
  };
  saveDefaults: {
    minecraftVersion: string;
    memoryMb: number;
    performancePreset: Preset;
    fpsEnabled: boolean;
    motionBlurEnabled: boolean;
    discordEnabled: boolean;
  };
  advanced: { jvmArgs: string; concurrentDownloads: number };
}

// ---------- Appearance ----------
export type Preset = 'performance' | 'balanced' | 'quality' | 'custom';
export interface ThemeColors {
  bg: string; bgAlt: string; surface: string; surfaceAlt: string; surfaceHover: string; border: string;
  text: string; textMuted: string; accent: string; accentAlt: string; accentText: string; cta: string; ctaText: string;
  success: string; warning: string; danger: string; info: string;
}
export interface ThemeDef {
  id: string;
  name: string;
  description: string;
  light: boolean;
  colors: ThemeColors;
}
export interface ThemesFile {
  defaultTheme: string;
  themes: ThemeDef[];
  custom: { id: 'custom'; name: string; description: string; baseTheme: string; defaultDim: number };
}
export interface Appearance {
  theme: string;
  surface: 'simple' | 'glass';
  glassTransparency: number;
  customBackground: { file: string; focusX: number; focusY: number; dim: number; revision: number } | null;
}
export interface AppearanceView {
  appearance: Appearance;
  customBackground: string | null;
  customBackgroundError: string | null;
  themes: ThemesFile;
}

// ---------- Saves ----------
export interface Progress {
  phase: string;
  label: string;
  filesDone: number;
  filesTotal: number;
  bytesDone: number;
  bytesTotal: number;
}
export type RunPhase =
  | { kind: 'downloading'; progress: Progress | null }
  | { kind: 'launching' }
  | { kind: 'running'; pid: number; startedAt: string }
  | { kind: 'error'; message: string };
export type SaveStatus = 'not_installed' | 'ready' | 'downloading' | 'launching' | 'running' | 'error';
export interface Save {
  id: string;
  name: string;
  minecraftVersion: string;
  loader: string;
  loaderVersion: string;
  createdAt: string;
  lastPlayedAt: string | null;
  hasIcon: boolean;
  iconRevision: number;
  memoryMb: number | null;
  installedStack: string | null;
  launches: number;
  icon: string | null;
  status: SaveStatus;
  phase: RunPhase | null;
  supported: boolean;
}
export interface SaveSummary {
  worlds: number;
  screenshots: number;
  mods: number;
  sizeBytes: number;
}
export interface ClientSettings {
  fps: { enabled: boolean; position: string; scale: number; color: string; background: string };
  motionBlur: { enabled: boolean; strength: number };
  discord: { enabled: boolean; showElapsed: boolean };
  performance: { preset: Preset };
  menus: { styled: boolean };
}
export interface SaveSettingsView {
  client: ClientSettings;
  onboarding: { completed: boolean; skipped: boolean; step: number; completedAt: string | null };
  memoryMb: number | null;
  defaultMemoryMb: number;
  runtime: {
    discord?: { status: string; detail?: string };
    motionBlur?: { available: boolean; reason?: string };
    introductionOpen?: boolean;
  } | null;
  locked: boolean;
}
export interface VersionOption {
  id: string;
  releaseTime: string | null;
  supported: boolean;
  loader: string | null;
  note: string;
  hasBetaComponents: boolean;
}
export interface VersionList {
  versions: VersionOption[];
  source: 'live' | 'cached' | 'offline';
}

export interface ModIssue {
  level: 'error' | 'warning';
  message: string;
}
export interface ModEntry {
  file: string;
  enabled: boolean;
  modId: string | null;
  name: string;
  version: string | null;
  description: string | null;
  loader: string;
  size: number;
  managed: { title: string; required: boolean; purpose: string; beta: boolean; modified: boolean } | null;
  issues: ModIssue[];
}
export interface ModList {
  mods: ModEntry[];
  issues: ModIssue[];
  removed: Array<{ modId: string; title: string }>;
}

// ---------- Account ----------
export type AuthState = 'unconfigured' | 'signed_out' | 'signing_in' | 'signed_in' | 'no_java' | 'offline';
export interface AuthStatus {
  state: AuthState;
  profile: { id: string; name: string } | null;
  avatar: string | null;
  message: string | null;
  persistent: boolean;
}

// ---------- News ----------
export interface NewsItem {
  id: string;
  source: 'stabled' | 'minecraft' | 'guide';
  title: string;
  summary: string;
  category: string;
  date: string | null;
  image: string | null;
  url: string | null;
  body: string | null;
}
export interface NewsResult {
  items: NewsItem[];
  state: 'live' | 'cached' | 'local' | 'failed';
  fetchedAt: string | null;
  error: string | null;
  feedConfigured: boolean;
}

// ---------- Optimiser ----------
export interface OptimiserAction {
  id: string;
  title: string;
  description: string;
  details: string;
  requiresAdmin: boolean;
  requiresRestart: boolean;
  state: 'applied' | 'not_applied' | 'unavailable' | 'unknown';
  current: string;
  target: string;
  canRestore: boolean;
  appliedAt: string | null;
  unavailableReason: string | null;
}
export interface OptimiserView {
  supported: boolean;
  platformMessage: string | null;
  originalOptimiser: { connected: boolean; message: string };
  actions: OptimiserAction[];
}

export const api = {
  appInfo: () => call<AppInfo>('app_info'),
  systemStatic: () => call<SystemStatic>('system_static'),
  systemDynamic: () => call<SystemDynamic>('system_dynamic'),
  settingsGet: () => call<Settings>('settings_get'),
  settingsSet: (settings: Settings) => call<Settings>('settings_set', { settings }),

  appearanceGet: () => call<AppearanceView>('appearance_get'),
  appearanceSet: (patch: Partial<Appearance> | Record<string, unknown>) => call<AppearanceView>('appearance_set', { patch }),
  appearanceSetBackground: (path: string) => call<AppearanceView>('appearance_set_background', { path }),
  appearanceResetBackground: () => call<AppearanceView>('appearance_reset_background'),
  imageInspect: (path: string) => call<{ dataUrl: string; width: number; height: number }>('image_inspect', { path }),

  versionsList: () => call<VersionList>('versions_list'),
  savesList: () => call<Save[]>('saves_list'),
  saveGet: (id: string) => call<Save>('save_get', { id }),
  saveValidateName: (name: string, except?: string) => call<string>('save_validate_name', { name, except: except ?? null }),
  saveCreate: (request: { name: string; minecraftVersion: string; iconPngBase64: string | null; memoryMb: number | null; performancePreset: Preset | null }) =>
    call<Save>('save_create', { request }),
  saveRename: (id: string, name: string) => call<Save>('save_rename', { id, name }),
  saveSetIcon: (id: string, pngBase64: string | null) => call<Save>('save_set_icon', { id, pngBase64 }),
  saveDuplicate: (id: string) => call<Save>('save_duplicate', { id }),
  saveDelete: (id: string) => call<void>('save_delete', { id }),
  saveSummary: (id: string) => call<SaveSummary>('save_summary', { id }),
  saveOpenFolder: (id: string, sub?: string) => call<void>('save_open_folder', { id, sub: sub ?? null }),
  saveSettingsGet: (id: string) => call<SaveSettingsView>('save_settings_get', { id }),
  saveSettingsSet: (id: string, patch: { client?: Record<string, Record<string, unknown>>; memoryMb?: number | null }) =>
    call<SaveSettingsView>('save_settings_set', { id, patch }),
  saveResetIntro: (id: string) => call<void>('save_reset_intro', { id }),
  saveInstall: (id: string) => call<void>('save_install', { id }),
  saveLaunch: (id: string) => call<void>('save_launch', { id }),
  saveStop: (id: string) => call<void>('save_stop', { id }),
  saveClearError: (id: string) => call<void>('save_clear_error', { id }),

  modsList: (id: string) => call<ModList>('mods_list', { id }),
  modsAdd: (id: string, paths: string[]) => call<string[]>('mods_add', { id, paths }),
  modsSetEnabled: (id: string, file: string, enabled: boolean) => call<void>('mods_set_enabled', { id, file, enabled }),
  modsRemove: (id: string, file: string) => call<void>('mods_remove', { id, file }),
  modsRestoreComponent: (id: string, modId: string) => call<void>('mods_restore_component', { id, modId }),

  authStatus: () => call<AuthStatus>('auth_status'),
  authRestore: () => call<AuthStatus>('auth_restore'),
  authSignIn: () => call<AuthStatus>('auth_sign_in'),
  authCancel: () => call<void>('auth_cancel'),
  authSignOut: () => call<AuthStatus>('auth_sign_out'),

  newsGet: (cacheOnly = false) => call<NewsResult>('news_get', { cacheOnly }),
  newsDetail: (id: string) => call<string>('news_detail', { id }),

  openExternal: (url: string) => call<void>('open_external', { url }),
  openDiscordInvite: () => call<void>('open_discord_invite'),
  openDataFolder: (kind: 'data' | 'logs' | 'shared') => call<void>('open_data_folder', { kind }),
  sharedCacheClear: () => call<number>('shared_cache_clear'),

  optimiserView: () => call<OptimiserView>('optimiser_view'),
  optimiserApply: (id: string) => call<string>('optimiser_apply', { id }),
  optimiserRestore: (id: string) => call<string>('optimiser_restore', { id }),
};

export function onEvent<T>(name: string, handler: (payload: T) => void): Promise<UnlistenFn> {
  return listen<T>(name, (e) => handler(e.payload));
}
