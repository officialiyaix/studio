import { useCallback, useEffect, useState } from 'react';
import { Lock, RotateCcw } from 'lucide-react';
import { api, asError, type Preset, type Save, type SaveSettingsView } from '../lib/api';
import { useStore } from '../lib/store';
import { rangePct } from '../lib/format';
import { Callout, PercentRange, Range, Segmented, SettingRow, Toggle } from '../components/ui';

const PRESET_DESC: Record<Preset, string> = {
  performance: 'Fast graphics, 8-chunk render distance. Best for low-end PCs and laptops.',
  balanced: 'Fancy graphics, 12-chunk render distance. A good default for most PCs.',
  quality: 'Fancy graphics, 16-chunk render distance.',
  custom: 'Stabled leaves your in-game video settings exactly as you set them.',
};

function discordStatus(view: SaveSettingsView, configured: boolean): string {
  if (!configured) return 'Not configured in this build (DISCORD_APPLICATION_ID is missing), so no activity is sent.';
  const s = view.runtime?.discord?.status;
  if (!view.locked || !s) return view.client.discord.enabled ? 'Connects to Discord when this Save is running.' : 'Off for this Save.';
  return (
    {
      connected: 'Live: connected to Discord.',
      connecting: 'Live: connecting to Discord…',
      waiting_for_discord: 'Live: Discord isn’t running — the game retries in the background.',
      rejected: 'Live: Discord rejected this build’s application ID.',
      disabled: 'Live: off for this Save.',
      not_configured: 'Not configured in this build.',
    }[s] ?? `Live: ${s}`
  );
}

export function SaveSettingsPanel({ save }: { save: Save }) {
  const { toast, info } = useStore();
  const [view, setView] = useState<SaveSettingsView | null>(null);
  const [memory, setMemory] = useState<number | null>(null);

  const load = useCallback(async () => {
    try {
      const v = await api.saveSettingsGet(save.id);
      setView(v);
      setMemory(v.memoryMb);
    } catch (e) {
      toast(asError(e).message, 'error');
    }
  }, [save.id, toast]);

  useEffect(() => {
    load();
  }, [load, save.status]);

  // While running, refresh the live status (written by the game) every few seconds.
  useEffect(() => {
    if (save.status !== 'running') return;
    const t = window.setInterval(load, 4000);
    return () => window.clearInterval(t);
  }, [save.status, load]);

  if (!view) return <div className="skeleton" style={{ height: 300 }} />;
  const locked = view.locked;
  const c = view.client;

  const patch = async (section: string, values: Record<string, unknown>) => {
    try {
      setView(await api.saveSettingsSet(save.id, { client: { [section]: values } }));
    } catch (e) {
      toast(asError(e).message, 'error');
    }
  };
  const setMem = async (mb: number | null) => {
    try {
      setView(await api.saveSettingsSet(save.id, { memoryMb: mb }));
    } catch (e) {
      toast(asError(e).message, 'error');
    }
  };

  return (
    <div className="save-settings">
      {locked && (
        <Callout tone="warning">
          <Lock size={13} style={{ verticalAlign: '-2px' }} /> Minecraft is using these settings right now. Change them in game under Stabled Settings (Right Shift or the pause menu); the launcher shows them read-only until the game closes.
        </Callout>
      )}
      <fieldset disabled={locked} className="plain-fieldset">
        <section className="group panel-alt">
          <h3>Game</h3>
          <SettingRow label="Memory" desc={memory == null ? `Using the default (${(view.defaultMemoryMb / 1024).toFixed(1)} GB from Settings → Save defaults).` : `${(memory / 1024).toFixed(1)} GB for this Save.`}>
            <div style={{ display: 'flex', alignItems: 'center', gap: 10, minWidth: 260 }}>
              <Range label="Memory" min={1024} max={16384} step={512} value={memory ?? view.defaultMemoryMb} onChange={setMemory} onCommit={(v) => setMem(v)} disabled={locked} />
              {memory != null && (
                <button type="button" className="icon-btn plain sm" aria-label="Use default memory" title="Use default" onClick={() => (setMemory(null), setMem(null))}>
                  <RotateCcw size={14} />
                </button>
              )}
            </div>
          </SettingRow>
          <SettingRow label="Performance preset" desc={PRESET_DESC[c.performance.preset]}>
            <Segmented
              label="Performance preset"
              value={c.performance.preset}
              onChange={(v) => patch('performance', { preset: v })}
              options={[
                { value: 'performance', label: 'Performance' },
                { value: 'balanced', label: 'Balanced' },
                { value: 'quality', label: 'Quality' },
                { value: 'custom', label: 'Custom' },
              ]}
            />
          </SettingRow>
          <SettingRow label="Stabled menu styling" desc="Stabled look for the title and pause menus. Other game screens keep the vanilla style.">
            <Toggle label="Stabled menu styling" checked={c.menus.styled} onChange={(v) => patch('menus', { styled: v })} />
          </SettingRow>
        </section>

        <section className="group panel-alt">
          <h3>FPS display</h3>
          <SettingRow label="Show FPS" desc="The game's own measured frame rate, in a corner of the screen.">
            <Toggle label="Show FPS" checked={c.fps.enabled} onChange={(v) => patch('fps', { enabled: v })} />
          </SettingRow>
          <SettingRow label="Placement">
            <Segmented
              label="FPS placement"
              value={c.fps.position}
              onChange={(v) => patch('fps', { position: v })}
              options={[
                { value: 'top-left', label: 'Top left' },
                { value: 'top-right', label: 'Top right' },
                { value: 'bottom-left', label: 'Bottom left' },
                { value: 'bottom-right', label: 'Bottom right' },
              ]}
            />
          </SettingRow>
          <SettingRow label="Size">
            <Segmented label="FPS size" value={c.fps.scale} onChange={(v) => patch('fps', { scale: v })} options={[0.75, 1, 1.25, 1.5, 2].map((s) => ({ value: s, label: `${Math.round(s * 100)}%` }))} />
          </SettingRow>
          <SettingRow label="Colour">
            <Segmented
              label="FPS colour"
              value={c.fps.color}
              onChange={(v) => patch('fps', { color: v })}
              options={['theme', 'white', 'green', 'yellow', 'cyan'].map((v) => ({ value: v, label: v[0].toUpperCase() + v.slice(1) }))}
            />
          </SettingRow>
          <SettingRow label="Background">
            <Segmented
              label="FPS background"
              value={c.fps.background}
              onChange={(v) => patch('fps', { background: v })}
              options={['panel', 'shadow', 'none'].map((v) => ({ value: v, label: v[0].toUpperCase() + v.slice(1) }))}
            />
          </SettingRow>
        </section>

        <section className="group panel-alt">
          <h3>Motion blur</h3>
          <SettingRow label="Motion blur" desc="Blends recent frames in the world renderer; the HUD and menus stay sharp. Costs one extra full-screen pass per frame.">
            <Toggle label="Motion blur" checked={c.motionBlur.enabled} onChange={(v) => patch('motionBlur', { enabled: v })} />
          </SettingRow>
          <SettingRow label="Strength" desc={`${rangePct(c.motionBlur.strength, 10, 100)}%`}>
            <div style={{ minWidth: 220 }}>
              <PercentRange
                label="Motion blur strength"
                min={10}
                max={100}
                value={c.motionBlur.strength}
                disabled={!c.motionBlur.enabled || locked}
                onChange={(v) => setView({ ...view, client: { ...c, motionBlur: { ...c.motionBlur, strength: v } } })}
                onCommit={(v) => patch('motionBlur', { strength: v })}
              />
            </div>
          </SettingRow>
          {view.runtime?.motionBlur?.available === false && <Callout tone="warning">{view.runtime.motionBlur.reason}</Callout>}
        </section>

        <section className="group panel-alt">
          <h3>Discord Presence</h3>
          <SettingRow label="Discord Presence" desc={discordStatus(view, info?.config.discordPresenceConfigured ?? false)}>
            <Toggle label="Discord Presence" checked={c.discord.enabled} onChange={(v) => patch('discord', { enabled: v })} />
          </SettingRow>
          <SettingRow label="Show elapsed time" desc="Shows how long you've been playing this session.">
            <Toggle label="Show elapsed time" checked={c.discord.showElapsed} onChange={(v) => patch('discord', { showElapsed: v })} />
          </SettingRow>
          <p className="tiny muted">Only general activity is shared — in menus, singleplayer, multiplayer or Realms. Server addresses, world names and account details are never sent.</p>
        </section>

        <section className="group panel-alt">
          <h3>Introduction</h3>
          <SettingRow
            label="First-launch introduction"
            desc={
              view.onboarding.completed
                ? `${view.onboarding.skipped ? 'Skipped' : 'Completed'}. Show it again to revisit appearance, performance, FPS, motion blur and Discord settings.`
                : `Not finished yet — it opens on the next launch${view.onboarding.step > 1 ? ` at step ${view.onboarding.step} of 5` : ''}.`
            }
          >
            <button
              type="button"
              className="btn btn-sm"
              disabled={!view.onboarding.completed}
              onClick={async () => {
                try {
                  await api.saveResetIntro(save.id);
                  toast('The introduction will open the next time you launch this Save.', 'success');
                  load();
                } catch (e) {
                  toast(asError(e).message, 'error');
                }
              }}
            >
              Show again next launch
            </button>
          </SettingRow>
        </section>
      </fieldset>
    </div>
  );
}
