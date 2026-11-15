import { useEffect, useState } from 'react';
import { AlertCircle, Archive, Cpu, ExternalLink, EyeOff, FolderOpen, Loader2, LogOut, MemoryStick, Monitor, RefreshCw, RotateCcw, Sparkles, Trash2, User } from 'lucide-react';
import { api, asError, type AppInfo, type FakeSpecs, type Settings, type VersionOption } from '../lib/api';
import { usePrivacy } from '../lib/privacy';
import { useStore } from '../lib/store';
import { cx, formatBytes } from '../lib/format';
import { Callout, ConfirmDialog, Range, Segmented, SettingRow, Toggle } from '../components/ui';
import { CustomBackgroundEditor, SurfaceStyle, ThemeGrid } from '../features/Appearance';
import { DiscordGlyph } from '../components/icons';
import { BackgroundGallery } from '../features/BackgroundGallery';
import { GameModeCards } from '../features/GameMode';

export function SettingsPage() {
  const { settings, updateSettings, info, auth, openSignIn, setAuth, toast, askDiscord } = useStore();
  const privacy = usePrivacy();
  const [jvm, setJvm] = useState(settings?.advanced.jvmArgs ?? '');
  const [jvmError, setJvmError] = useState<string | null>(null);
  const [mem, setMem] = useState(settings?.saveDefaults.memoryMb ?? 4096);
  const [versions, setVersions] = useState<VersionOption[]>([]);
  const [clearing, setClearing] = useState<'confirm' | 'busy' | null>(null);

  useEffect(() => {
    api.versionsList().then((v) => setVersions(v.versions.filter((x) => x.supported))).catch(() => undefined);
  }, []);
  useEffect(() => {
    if (settings) {
      setJvm(settings.advanced.jvmArgs);
      setMem(settings.saveDefaults.memoryMb);
    }
  }, [settings]);

  if (!settings) return null;
  const set = <K extends Exclude<keyof Settings, 'version'>>(section: K, patch: Partial<Settings[K]>) =>
    updateSettings({ ...settings, [section]: { ...settings[section], ...patch } }).catch(() => undefined);

  const invite = info?.config.discordInviteUrl ?? null;

  return (
    <div className="page settings scroll">
      {/* ---------- Accounts ---------- */}
      <section className="settings-section" id="settings-accounts" aria-labelledby="accounts-h">
        <h2 id="accounts-h" className="section-title">
          Accounts
        </h2>
        <div className={cx('panel settings-panel account-panel', !(auth?.state === 'signed_in' || auth?.state === 'offline') && 'signed-out')}>
          {!(auth?.state === 'signed_in' || auth?.state === 'offline') && (
            <div className="account-warning" role="status">
              <AlertCircle size={15} />
              {auth?.state === 'no_java' ? 'This account can\'t launch Minecraft: Java Edition.' : 'You aren\'t signed in. Sign in with Microsoft to launch Minecraft.'}
            </div>
          )}
          <div className="account-row">
            <span className="avatar" style={{ width: 44, height: 44 }}>
              {auth?.avatar && !privacy.hideAvatar ? <img src={auth.avatar} alt="" /> : <User size={20} />}
            </span>
            <div style={{ minWidth: 0, flex: 1 }}>
              <div style={{ fontWeight: 650 }}>{privacy.name(auth?.profile?.name) ?? 'Not signed in'}</div>
              <div className="tiny muted">
                {auth?.state === 'signed_in' && 'Microsoft account · Minecraft: Java Edition'}
                {auth?.state === 'offline' && "Signed in, but the session couldn't be refreshed. Launching will try again."}
                {auth?.state === 'no_java' && (auth.message ?? 'This account has no Java Edition access.')}
                {auth?.state === 'signed_out' && 'Saves, mods and settings work without an account; launching needs one.'}
                {auth?.state === 'unconfigured' && "Microsoft sign-in isn't configured in this build (MICROSOFT_CLIENT_ID)."}
                {auth?.state === 'signing_in' && 'Waiting for the browser…'}
              </div>
            </div>
            {auth?.profile ? (
              <div style={{ display: 'flex', gap: 8 }}>
                <button type="button" className="btn btn-sm" onClick={() => openSignIn('Choose the Microsoft account to use with Stabled.')}>
                  <RefreshCw size={14} /> Switch account
                </button>
                <button
                  type="button"
                  className="btn btn-sm btn-danger"
                  onClick={async () => {
                    setAuth(await api.authSignOut());
                    toast('Signed out. Your Saves and settings are unchanged.', 'success');
                  }}
                >
                  <LogOut size={14} /> Sign out
                </button>
              </div>
            ) : (
              <button type="button" className="btn btn-primary btn-sm" onClick={() => openSignIn()} disabled={auth?.state === 'unconfigured'}>
                Sign in with Microsoft
              </button>
            )}
          </div>
          <p className="tiny muted">
            {auth?.persistent === false
              ? 'On this platform your session is kept only while Stabled is open.'
              : 'Your session is encrypted with Windows (DPAPI) for your user account and is never stored in settings files or logs.'}
          </p>
        </div>
      </section>

      {/* ---------- Socials ---------- */}
      <section className="settings-section" aria-labelledby="socials-h">
        <h2 id="socials-h" className="section-title">
          Socials
        </h2>
        <div className="discord-card panel">
          <span className="discord-logo">
            <DiscordGlyph size={26} />
          </span>
          <div style={{ minWidth: 0 }}>
            <h3>Join the Stabled community</h3>
            <p className="muted small">Get help, share your Saves and hear about updates on the Stabled Discord server.</p>
            {!invite && <p className="tiny muted" style={{ marginTop: 4 }}>The community invite isn't configured in this build (DISCORD_INVITE_URL), so there's nothing to open yet.</p>}
          </div>
          <button type="button" className="btn btn-lg discord-btn" disabled={!invite} onClick={askDiscord}>
            Join Discord <ExternalLink size={15} />
          </button>
        </div>
      </section>

      {/* ---------- Appearance ---------- */}
      <section className="settings-section" aria-labelledby="appearance-h">
        <h2 id="appearance-h" className="section-title">
          Appearance
        </h2>
        <div className="panel settings-panel">
          {/* One block: pre-themes (bundled backgrounds by category) first, then the basic themes. */}
          <div className="themes-block" role="group" aria-label="Themes">
            <BackgroundGallery />
            <div className="themes-sep" role="separator" />
            <div className="themes-basic">
              <div>
                <div className="field-label">Basic themes</div>
                <p className="tiny muted">Eight colour themes, and Custom Background for an image of your own.</p>
              </div>
              <ThemeGrid />
            </div>
          </div>
          <CustomBackgroundEditor />
          <div className="field-label" style={{ marginTop: 8 }}>
            Surface style
          </div>
          <SurfaceStyle />
          <SettingRow label="Reduce motion" desc="Turns off launcher animations and transitions.">
            <Toggle label="Reduce motion" checked={settings.launcher.reduceMotion} onChange={(v) => set('launcher', { reduceMotion: v })} />
          </SettingRow>
        </div>
      </section>

      {/* ---------- Launcher behaviour ---------- */}
      <section className="settings-section" aria-labelledby="behaviour-h">
        <h2 id="behaviour-h" className="section-title">
          Launcher behaviour
        </h2>
        <div className="panel settings-panel">
          <SettingRow label="When a Save starts" desc="What the Stabled window does while you play. Hidden windows come back when the game closes.">
            <Segmented
              label="When a Save starts"
              value={settings.launcher.afterLaunch}
              onChange={(v) => set('launcher', { afterLaunch: v })}
              options={[
                { value: 'keep', label: 'Keep open' },
                { value: 'minimize', label: 'Minimise' },
                { value: 'hide', label: 'Hide' },
              ]}
            />
          </SettingRow>
          <SettingRow label="Open Stabled on" desc="The page shown when Stabled starts.">
            <Segmented label="Start page" value={settings.launcher.startPage} onChange={(v) => set('launcher', { startPage: v })} options={[{ value: 'home', label: 'Home' }, { value: 'saves', label: 'Saves' }]} />
          </SettingRow>
          <SettingRow label="Show Minecraft release notes" desc="Adds Minecraft: Java Edition release notes from Mojang to the news panel.">
            <Toggle label="Show Minecraft release notes" checked={settings.launcher.showMinecraftNews} onChange={(v) => set('launcher', { showMinecraftNews: v })} />
          </SettingRow>
        </div>
      </section>

      {/* ---------- Save defaults ---------- */}
      <section className="settings-section" aria-labelledby="defaults-h">
        <h2 id="defaults-h" className="section-title">
          Save defaults
        </h2>
        <div className="panel settings-panel">
          <p className="tiny muted">Used for new Saves. Each Save can change these in its own settings.</p>
          <div className="field">
            <span className="field-label">Game mode</span>
            <span className="tiny muted">The mods new Saves start with, and the mode Saves use until you pick one for them. CPVP is the default.</span>
            <GameModeCards value={settings.saveDefaults.gameMode} onChange={(v) => set('saveDefaults', { gameMode: v })} minecraft={settings.saveDefaults.minecraftVersion} />
          </div>
          <SettingRow label="Minecraft version" desc="Preselected in Create Save.">
            <select className="input" style={{ width: 200 }} value={settings.saveDefaults.minecraftVersion} onChange={(e) => set('saveDefaults', { minecraftVersion: e.target.value })} aria-label="Default Minecraft version">
              {(versions.length ? versions : [{ id: settings.saveDefaults.minecraftVersion } as VersionOption]).map((v) => (
                <option key={v.id} value={v.id}>
                  {v.id}
                </option>
              ))}
            </select>
          </SettingRow>
          <SettingRow label="Memory" desc={`${(mem / 1024).toFixed(1)} GB for Saves that don't set their own.`}>
            <div style={{ minWidth: 240 }}>
              <Range label="Default memory" min={1024} max={16384} step={512} value={mem} onChange={setMem} onCommit={(v) => set('saveDefaults', { memoryMb: v })} />
            </div>
          </SettingRow>
          <SettingRow label="Performance preset">
            <Segmented
              label="Default performance preset"
              value={settings.saveDefaults.performancePreset}
              onChange={(v) => set('saveDefaults', { performancePreset: v })}
              options={[
                { value: 'performance', label: 'Performance' },
                { value: 'balanced', label: 'Balanced' },
                { value: 'quality', label: 'Quality' },
                { value: 'custom', label: 'Custom' },
              ]}
            />
          </SettingRow>
        </div>
      </section>

      {/* ---------- Game features ---------- */}
      <section className="settings-section" aria-labelledby="features-h">
        <h2 id="features-h" className="section-title">
          Game features
        </h2>
        <div className="panel settings-panel">
          <p className="tiny muted">Starting values for the Stabled client in new Saves. Change them per Save, or in game under Stabled Settings.</p>
          <SettingRow label="FPS display" desc="Shows the game's measured frame rate.">
            <Toggle label="FPS display for new Saves" checked={settings.saveDefaults.fpsEnabled} onChange={(v) => set('saveDefaults', { fpsEnabled: v })} />
          </SettingRow>
          <SettingRow label="Motion blur" desc="Off by default; it costs a little performance.">
            <Toggle label="Motion blur for new Saves" checked={settings.saveDefaults.motionBlurEnabled} onChange={(v) => set('saveDefaults', { motionBlurEnabled: v })} />
          </SettingRow>
          <SettingRow
            label="Discord Presence"
            desc={info?.config.discordPresenceConfigured ? 'Shows Stabled Client activity on your Discord profile while you play.' : "On by default, but this build has no Discord application ID (DISCORD_APPLICATION_ID), so no activity is sent until it's configured."}
          >
            <Toggle label="Discord Presence for new Saves" checked={settings.saveDefaults.discordEnabled} onChange={(v) => set('saveDefaults', { discordEnabled: v })} />
          </SettingRow>
        </div>
      </section>

      {/* ---------- World backups ---------- */}
      <section className="settings-section" aria-labelledby="backups-h">
        <h2 id="backups-h" className="section-title">
          World backups
        </h2>
        <div className="panel settings-panel">
          <p className="tiny muted">
            <Archive size={13} style={{ verticalAlign: '-2px' }} /> Stabled zips your worlds into each Save's backups folder. Restore them from a Save's Backups tab; restoring adds a copy and never overwrites a world.
          </p>
          <SettingRow label="Back up after playing" desc="When Minecraft closes (so it has finished saving), worlds you changed during that session are backed up.">
            <Toggle label="Back up after playing" checked={settings.backups.afterSessions} onChange={(v) => set('backups', { afterSessions: v })} />
          </SettingRow>
          <SettingRow label="Back up before updates" desc="Before Stabled updates a Save's Minecraft files or mods, every world in it is backed up. These backups are kept until you delete them.">
            <Toggle label="Back up before updates" checked={settings.backups.beforeUpgrades} onChange={(v) => set('backups', { beforeUpgrades: v })} />
          </SettingRow>
          <SettingRow label="Backups to keep" desc={`The newest ${settings.backups.keep} after-playing backup${settings.backups.keep === 1 ? '' : 's'} per world are kept; older ones are deleted.`}>
            <div style={{ minWidth: 220 }}>
              <Range label="Backups to keep" min={1} max={20} value={settings.backups.keep} onChange={(v) => set('backups', { keep: v })} />
            </div>
          </SettingRow>
        </div>
      </section>

      {/* ---------- Advanced ---------- */}
      <section className="settings-section" aria-labelledby="advanced-h">
        <h2 id="advanced-h" className="section-title">
          Advanced
        </h2>
        <div className="panel settings-panel">
          <div className="field">
            <label htmlFor="jvm">Extra Java arguments</label>
            <textarea
              id="jvm"
              className="input"
              rows={2}
              spellCheck={false}
              value={jvm}
              placeholder="e.g. -XX:+UseZGC"
              aria-invalid={!!jvmError}
              onChange={(e) => setJvm(e.target.value)}
              onBlur={async () => {
                if (jvm === settings.advanced.jvmArgs) return;
                try {
                  setJvmError(null);
                  await updateSettings({ ...settings, advanced: { ...settings.advanced, jvmArgs: jvm } });
                } catch (e) {
                  setJvmError(asError(e).message);
                }
              }}
            />
            {jvmError ? <span className="error">{jvmError}</span> : <span className="hint">Added to every Save. Memory, class path and Stabled's own properties are managed for you.</span>}
          </div>
          <SettingRow label="Parallel downloads" desc={`${settings.advanced.concurrentDownloads} files at a time while installing.`}>
            <div style={{ minWidth: 200 }}>
              <Range label="Parallel downloads" min={1} max={16} value={settings.advanced.concurrentDownloads} onChange={(v) => set('advanced', { concurrentDownloads: v })} />
            </div>
          </SettingRow>
          <SettingRow label="Data folders" desc={<span className="selectable">{info ? privacy.path(info.dataDir) : ''}</span>}>
            <div style={{ display: 'flex', gap: 8 }}>
              <button type="button" className="btn btn-sm" onClick={() => api.openDataFolder('data')}>
                <FolderOpen size={14} /> Data
              </button>
              <button type="button" className="btn btn-sm" onClick={() => api.openDataFolder('logs')}>
                <FolderOpen size={14} /> Logs
              </button>
            </div>
          </SettingRow>
          <SettingRow label="Clear downloaded game files" desc="Frees space used by shared game files, libraries, Java and mod downloads. Saves, worlds and settings are kept; files download again on the next launch.">
            <button type="button" className="btn btn-sm btn-danger" onClick={() => setClearing('confirm')} disabled={clearing === 'busy'}>
              {clearing === 'busy' ? <Loader2 size={14} className="spin" /> : <Trash2 size={14} />} Clear
            </button>
          </SettingRow>
          <div className="about tiny muted">
            <div>
              Stabled {info?.version} · {info?.platform} · {clientSummary(info)}
            </div>
            <div>
              Microsoft sign-in: {info?.config.microsoftConfigured ? 'configured' : 'not configured'} · Discord Presence: {info?.config.discordPresenceConfigured ? 'configured' : 'not configured'} · Community invite:{' '}
              {info?.config.discordInviteUrl ? 'configured' : 'not configured'} · News feed: {info?.config.newsFeedConfigured ? 'configured' : 'not configured'}
            </div>
            <div>Not an official Minecraft product. Not approved by or associated with Mojang or Microsoft.</div>
          </div>
          {info && !info.clientBundled && <Callout tone="warning">This build doesn't include any Stabled client jars, so Saves can't be launched.</Callout>}
        </div>
      </section>

      {/* ---------- Streamer mode ---------- */}
      <section className="settings-section" aria-labelledby="streamer-h">
        <h2 id="streamer-h" className="section-title">
          Streamer mode
        </h2>
        <div className={cx('panel settings-panel streamer-panel', settings.streamer.enabled && 'on')}>
          <SettingRow
            label={
              <span className="streamer-label">
                <EyeOff size={16} /> Streamer mode {settings.streamer.enabled && <span className="live-dot">On</span>}
              </span>
            }
            desc="Hides personal details while you stream or record. Features added to Stabled later that show personal details follow these switches too."
          >
            <Toggle label="Streamer mode" checked={settings.streamer.enabled} onChange={(v) => set('streamer', { enabled: v })} />
          </SettingRow>
          <div className={cx('streamer-options', !settings.streamer.enabled && 'off')} aria-disabled={!settings.streamer.enabled}>
            {(
              [
                ['hideUsername', 'Hide my Minecraft username', 'Shows "Player" instead of your name in the header, greeting and account settings, and hides it on the in-game title screen.'],
                ['hideAvatar', 'Hide my skin', 'Replaces your skin face with a plain icon.'],
                ['hidePaths', 'Hide folder locations', 'Folder paths contain your Windows user name; it is replaced with •••.'],
                ['redactLogs', 'Clean up logs', 'Hides your username, user folder and server addresses in the Logs tab and crash explanations.'],
                ['hideSpecs', 'Hide computer specifications', 'The specs bar shows "Hidden" (unless fake specs are on).'],
              ] as const
            ).map(([key, label, desc]) => (
              <SettingRow key={key} label={label} desc={desc}>
                <Toggle label={label} checked={settings.streamer[key]} disabled={!settings.streamer.enabled} onChange={(v) => set('streamer', { [key]: v })} />
              </SettingRow>
            ))}
          </div>
        </div>
      </section>

      {/* ---------- Fake specs ---------- */}
      <FakeSpecsSection settings={settings} onSave={(fake) => updateSettings({ ...settings, fakeSpecs: fake }).catch(() => undefined)} />

      {clearing === 'confirm' && (
        <ConfirmDialog
          title="Clear downloaded game files?"
          confirmLabel="Clear files"
          danger
          onCancel={() => setClearing(null)}
          onConfirm={async () => {
            setClearing('busy');
            try {
              const freed = await api.sharedCacheClear();
              toast(`Freed ${formatBytes(freed)}. Saves and worlds were not touched.`, 'success');
            } catch (e) {
              toast(asError(e).message, 'error');
            } finally {
              setClearing(null);
            }
          }}
        >
          <p>Shared downloads (Minecraft, libraries, sounds, Java and mod files) will be removed and downloaded again the next time you launch. Your Saves, worlds, mods folders and settings are not affected.</p>
        </ConfirmDialog>
      )}
    </div>
  );
}

const DREAM_PC: FakeSpecs = { enabled: true, cpu: 'AMD Ryzen 9 9950X3D', gpu: 'NVIDIA GeForce RTX 5090', ramGb: 192 };

/** Display-only specs for the specs bar, to flex on your friends. Nothing else uses them. */
function FakeSpecsSection({ settings, onSave }: { settings: Settings; onSave: (f: FakeSpecs) => void }) {
  const real = settings.fakeSpecs;
  const [draft, setDraft] = useState<FakeSpecs>(real);
  useEffect(() => setDraft(real), [real]);
  const commit = (next: FakeSpecs) => {
    setDraft(next);
    onSave(next);
  };
  const changed = real.enabled || real.cpu || real.gpu || real.ramGb;
  return (
    <section className="settings-section" aria-labelledby="fake-h">
      <h2 id="fake-h" className="section-title">
        Fake specs
      </h2>
      <div className={cx('panel settings-panel fake-panel', draft.enabled && 'on')}>
        <SettingRow
          label={
            <span className="streamer-label">
              <Sparkles size={16} /> Show fake specs
            </span>
          }
          desc="Flex on your friends about your crazy computer. Type any processor, graphics card and memory, and the specs bar at the bottom shows them instead of the real ones. Only the display changes: Stabled still uses your real hardware for memory suggestions and performance."
        >
          <Toggle label="Show fake specs" checked={draft.enabled} onChange={(v) => commit({ ...draft, enabled: v })} />
        </SettingRow>
        <div className="fake-grid">
          <label className="field">
            <span className="field-label">
              <Cpu size={14} /> Processor
            </span>
            <input className="input" maxLength={80} placeholder="Your real CPU" value={draft.cpu} onChange={(e) => setDraft({ ...draft, cpu: e.target.value })} onBlur={() => draft.cpu !== real.cpu && commit(draft)} />
          </label>
          <label className="field">
            <span className="field-label">
              <Monitor size={14} /> Graphics card
            </span>
            <input className="input" maxLength={80} placeholder="Your real GPU" value={draft.gpu} onChange={(e) => setDraft({ ...draft, gpu: e.target.value })} onBlur={() => draft.gpu !== real.gpu && commit(draft)} />
          </label>
          <label className="field">
            <span className="field-label">
              <MemoryStick size={14} /> Memory (GB)
            </span>
            <input
              className="input"
              type="number"
              min={0}
              max={4096}
              placeholder="Your real RAM"
              value={draft.ramGb || ''}
              onChange={(e) => setDraft({ ...draft, ramGb: Math.max(0, Math.min(4096, Math.round(Number(e.target.value) || 0))) })}
              onBlur={() => draft.ramGb !== real.ramGb && commit(draft)}
            />
          </label>
        </div>
        <p className="tiny muted">Empty fields keep showing your real hardware.</p>
        <div className="fake-actions">
          <button type="button" className="btn btn-sm" onClick={() => commit(DREAM_PC)}>
            <Sparkles size={14} /> Fill in a dream PC
          </button>
          <button type="button" className="btn btn-sm" disabled={!changed} onClick={() => commit({ enabled: false, cpu: '', gpu: '', ramGb: 0 })}>
            <RotateCcw size={14} /> Reset to my real specs
          </button>
        </div>
      </div>
    </section>
  );
}

/** "Stabled client included for 23 of 23 supported Minecraft versions (1.20.1–26.3)" */
function clientSummary(info: AppInfo | null): string {
  if (!info) return '';
  const included = info.supportedVersions.filter((v) => info.bundledClientVersions.includes(v));
  if (!included.length) return 'Stabled client not included in this build';
  const sorted = [...included].sort(compareVersions);
  const range = sorted.length > 1 ? `${sorted[0]}–${sorted[sorted.length - 1]}` : sorted[0];
  return `Stabled client included for ${included.length} of ${info.supportedVersions.length} supported Minecraft versions (${range})`;
}

function compareVersions(a: string, b: string): number {
  const pa = a.split('.').map(Number);
  const pb = b.split('.').map(Number);
  for (let i = 0; i < Math.max(pa.length, pb.length); i++) {
    const d = (pa[i] ?? 0) - (pb[i] ?? 0);
    if (d) return d;
  }
  return 0;
}
