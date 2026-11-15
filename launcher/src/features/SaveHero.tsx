import { useState } from 'react';
import { FileText, FolderOpen, Loader2, Play, Puzzle, Square, Wrench, X } from 'lucide-react';
import { api, asError, type Save } from '../lib/api';
import { useStore } from '../lib/store';
import { formatBytes, pct, relativeTime } from '../lib/format';
import { Callout, ConfirmDialog, ProgressBar, SaveIcon, StatusBadge } from '../components/ui';
import { SaveMenu } from './SaveActions';
import { GameModePill } from './GameMode';

/**
 * The selected Save: identity, real status, progress and launch controls. Launch installs whatever is missing
 * first, so there is no separate Install step.
 */
export function SaveHero({ save, onOpenMods, onOpenLogs }: { save: Save; onOpenMods?: () => void; onOpenLogs?: () => void }) {
  const { launch, toast, auth } = useStore();
  const [repairAsk, setRepairAsk] = useState(false);
  const phase = save.phase;
  const progress = phase?.kind === 'downloading' ? phase.progress : null;
  const busy = save.status === 'downloading' || save.status === 'launching';
  const running = save.status === 'running';
  const value = progress ? pct(progress.bytesTotal ? progress.bytesDone : progress.filesDone, progress.bytesTotal || progress.filesTotal) : 0;
  const canLaunch = save.supported && save.clientBundled && !busy && !running;
  const signedIn = auth?.state === 'signed_in' || auth?.state === 'offline';

  return (
    <section className="hero panel" aria-label={`${save.name} details`}>
      <div className="hero-top">
        <SaveIcon save={save} size={84} />
        <div className="hero-id">
          <div className="hero-name-row">
            <h2 className="hero-name">{save.name}</h2>
            <StatusBadge save={save} />
            <GameModePill save={save} />
          </div>
          <p className="muted">
            Minecraft {save.minecraftVersion} · Fabric {save.loaderVersion} · Stabled client
          </p>
          <p className="tiny muted">
            {save.lastPlayedAt ? `Last played ${relativeTime(save.lastPlayedAt)}` : 'Not played yet'}
            {save.launches > 0 && ` · ${save.launches} launch${save.launches === 1 ? '' : 'es'}`}
          </p>
        </div>
        <SaveMenu save={save} />
      </div>

      {busy && (
        <div className="hero-progress" aria-live="polite">
          <div className="hero-progress-row">
            <span className="small">{save.status === 'launching' ? 'Starting Minecraft…' : progress?.label ?? 'Preparing…'}</span>
            <span className="tiny muted">
              {progress?.bytesTotal ? `${formatBytes(progress.bytesDone)} of ${formatBytes(progress.bytesTotal)}` : progress?.filesTotal ? `${progress.filesDone} of ${progress.filesTotal} files` : ''}
            </span>
          </div>
          <ProgressBar value={value} indeterminate={!progress?.filesTotal || save.status === 'launching'} label="Install progress" />
        </div>
      )}

      {phase?.kind === 'error' && (
        <Callout tone="danger">
          <div style={{ display: 'flex', gap: 8, alignItems: 'flex-start', justifyContent: 'space-between' }}>
            <span>{phase.message}</span>
            <button type="button" className="icon-btn plain sm" aria-label="Dismiss error" onClick={() => api.saveClearError(save.id)}>
              <X size={14} />
            </button>
          </div>
        </Callout>
      )}
      {!save.supported && <Callout tone="warning">Minecraft {save.minecraftVersion} isn't supported by this version of the Stabled client.</Callout>}
      {save.supported && !save.clientBundled && (
        <Callout tone="warning">This launcher build doesn't include the Stabled client for Minecraft {save.minecraftVersion}, so this Save can't be launched.</Callout>
      )}

      <div className="hero-actions">
        {running ? (
          <button type="button" className="btn btn-lg btn-danger" onClick={() => api.saveStop(save.id).catch((e) => toast(asError(e).message, 'error'))}>
            <Square size={17} /> Force close
          </button>
        ) : (
          <button type="button" className="btn btn-lg btn-cta launch-btn" disabled={!canLaunch} onClick={() => launch(save)}>
            {busy ? <Loader2 size={18} className="spin" /> : <Play size={18} fill="currentColor" />}
            {busy ? (save.status === 'launching' ? 'Launching' : 'Installing') : 'Launch'}
          </button>
        )}
        {(save.status === 'ready' || save.status === 'error') && (
          <button type="button" className="btn btn-lg" title="Check every game file and replace damaged ones" onClick={() => setRepairAsk(true)}>
            <Wrench size={17} /> Repair
          </button>
        )}
        {onOpenMods && (
          <button type="button" className="btn btn-lg" onClick={onOpenMods}>
            <Puzzle size={17} /> Mods
          </button>
        )}
        {onOpenLogs && (
          <button type="button" className="btn btn-lg" onClick={onOpenLogs}>
            <FileText size={17} /> Logs
          </button>
        )}
        <button type="button" className="btn btn-lg" onClick={() => api.saveOpenFolder(save.id).catch((e) => toast(asError(e).message, 'error'))}>
          <FolderOpen size={17} /> Folder
        </button>
      </div>
      {!signedIn && !running && <p className="tiny muted">Launching needs a Microsoft account with Minecraft: Java Edition. Everything else works without signing in.</p>}
      {signedIn && save.status === 'not_installed' && !busy && <p className="tiny muted">Not installed yet: Launch downloads Minecraft, Fabric and the Stabled client first.</p>}
      {repairAsk && (
        <ConfirmDialog
          title="Repair this Save?"
          confirmLabel="Repair"
          onCancel={() => setRepairAsk(false)}
          onConfirm={() => {
            setRepairAsk(false);
            api.saveInstall(save.id, true).catch((e) => toast(asError(e).message, 'error'));
          }}
        >
          <p>Stabled checks every game file (Minecraft, libraries, assets and Java) against its official checksum and downloads again any that are missing or damaged.</p>
          <p className="muted small">Your worlds, screenshots, settings and the mods you added are not touched.</p>
        </ConfirmDialog>
      )}
      {running && <p className="tiny muted">Mods and Save settings are locked while Minecraft is running. Change Stabled options in game with Stabled Settings (Right Shift).</p>}
    </section>
  );
}
