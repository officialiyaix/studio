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
  /** Minecraft versions whose Stabled client jar is included in this launcher build. */
  bundledClientVersions: string[];
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
  /** Settings format version; sent back unchanged. */
  version: number;
  launcher: {
    afterLaunch: 'keep' | 'minimize' | 'hide';
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
  streamer: StreamerMode;
  fakeSpecs: FakeSpecs;
  backups: { afterSessions: boolean; beforeUpgrades: boolean; keep: number };
}
export interface StreamerMode {
  enabled: boolean;
  hideUsername: boolean;
  hideAvatar: boolean;
  hidePaths: boolean;
  redactLogs: boolean;
  hideSpecs: boolean;
}
export interface FakeSpecs {
  enabled: boolean;
  cpu: string;
  gpu: string;
  ramGb: number;
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
  customBackground: { file: string; focusX: number; focusY: number; dim: number; revision: number; source?: string } | null;
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
  clientBundled: boolean;
  /** Set on hidden test copies made by Find the broken mod. */
  testOf?: string;
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
  category: 'power' | 'graphics' | 'input' | 'background' | 'launch';
  impact: 'high' | 'medium' | 'low';
  requiresAdmin: boolean;
  requiresRestart: boolean;
  state: 'applied' | 'not_applied' | 'unavailable' | 'unknown';
  current: string;
  target: string;
  canRestore: boolean;
  appliedAt: string | null;
  unavailableReason: string | null;
}
export interface OptimiserBatchResult {
  id: string;
  title: string;
  ok: boolean;
  message: string;
}
export interface OptimiserView {
  supported: boolean;
  platformMessage: string | null;
  originalOptimiser: { connected: boolean; message: string };
  actions: OptimiserAction[];
}

// ---------- Compatibility, crashes, logs ----------
export interface CompatProblem {
  level: 'error' | 'warning';
  kind: 'minecraft-version' | 'loader' | 'duplicate' | 'dependency' | 'breaks' | 'known-conflict' | 'invalid' | 'other';
  file: string | null;
  modName: string | null;
  message: string;
  fix: string | null;
}
export interface CompatReport {
  problems: CompatProblem[];
  checkedMods: number;
}
export interface CrashFinding {
  id: string;
  title: string;
  explanation: string;
  fixes: string[];
  confidence: 'high' | 'medium' | 'low';
  evidence: string | null;
  mods: string[];
}
export interface CrashReport {
  file: string | null;
  exitCode: number | null;
  headline: string | null;
  findings: CrashFinding[];
  modsInTrace: string[];
}
export interface LogFile {
  name: string;
  kind: 'latest' | 'launch' | 'debug' | 'archive' | 'crash' | 'jvm-crash';
  size: number;
  modified: string | null;
}
export interface LogContent {
  name: string;
  text: string;
  truncated: boolean;
  size: number;
  modified: string | null;
}

// ---------- Backups ----------
export interface WorldBackup {
  id: string;
  world: string;
  reason: 'session' | 'upgrade' | 'manual';
  createdAt: string;
  size: number;
}
export interface BackupsView {
  backups: WorldBackup[];
  worlds: Array<{ folder: string; size: number; modified: string | null }>;
  totalSize: number;
}

// ---------- Find the broken mod ----------
export interface BisectCandidate {
  file: string;
  name: string;
  modId: string | null;
  requires: string[];
}

// ---------- Store ----------
export type StoreKind = 'mod' | 'resourcepack' | 'shader';
export interface StoreHit {
  projectId: string;
  slug: string;
  title: string;
  description: string;
  author: string;
  iconUrl: string | null;
  imageUrl: string | null;
  color: number | null;
  downloads: number;
  follows: number;
  categories: string[];
  dateModified: string | null;
}
export interface StoreSearch {
  hits: StoreHit[];
  total: number;
  offset: number;
  limit: number;
}
export interface StoreProject {
  id: string;
  slug: string;
  title: string;
  description: string;
  body: string;
  iconUrl: string | null;
  color: number | null;
  gallery: Array<{ url: string; title: string | null }>;
  downloads: number;
  follows: number;
  categories: string[];
  license: string | null;
  sourceUrl: string | null;
  issuesUrl: string | null;
  wikiUrl: string | null;
  discordUrl: string | null;
  published: string | null;
  updated: string | null;
  projectType: string;
  pageUrl: string;
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
  appearanceSetGalleryBackground: (id: string) => call<AppearanceView>('appearance_set_gallery_background', { id }),
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
  saveInstall: (id: string, repair = false) => call<void>('save_install', { id, repair }),
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
  optimiserApplyAll: () => call<OptimiserBatchResult[]>('optimiser_apply_all'),
  optimiserRestoreAll: () => call<OptimiserBatchResult[]>('optimiser_restore_all'),

  modsCheck: (id: string) => call<CompatReport>('mods_check', { id }),
  saveLogs: (id: string) => call<LogFile[]>('save_logs', { id }),
  saveLogRead: (id: string, name: string) => call<LogContent>('save_log_read', { id, name }),
  crashExplain: (id: string, name: string) => call<CrashReport>('crash_explain', { id, name }),

  backupsView: (id: string) => call<BackupsView>('backups_view', { id }),
  backupsCreate: (id: string, world?: string) => call<number>('backups_create', { id, world: world ?? null }),
  backupsRestore: (id: string, backup: string) => call<string>('backups_restore', { id, backup }),
  backupsDelete: (id: string, backup: string) => call<void>('backups_delete', { id, backup }),
  backupsOpenFolder: (id: string) => call<void>('backups_open_folder', { id }),

  bisectStart: (id: string, withWorlds: boolean) => call<{ testId: string; candidates: BisectCandidate[] }>('bisect_start', { id, withWorlds }),
  bisectApply: (testId: string, all: string[], keep: string[]) => call<string[]>('bisect_apply', { testId, all, keep }),
  bisectEnd: (testId: string) => call<void>('bisect_end', { testId }),

  storeSearch: (kind: StoreKind, query: string, sort: string, category: string | null, saveId: string | null, offset = 0) =>
    call<StoreSearch>('store_search', { kind, query, sort, category, saveId, offset }),
  storeProject: (id: string) => call<StoreProject>('store_project', { id }),
  storeInstalled: (saveId: string, kind: StoreKind) => call<string[]>('store_installed', { saveId, kind }),
  storeInstall: (saveId: string, kind: StoreKind, projectId: string) =>
    call<{ installed: string[]; already: string[]; message: string }>('store_install', { saveId, kind, projectId }),
};

export function onEvent<T>(name: string, handler: (payload: T) => void): Promise<UnlistenFn> {
  return listen<T>(name, (e) => handler(e.payload));
}
