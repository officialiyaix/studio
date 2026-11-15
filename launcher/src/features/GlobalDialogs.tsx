import { AlertOctagon, AlertTriangle, ExternalLink, FileText, Play, Puzzle, SearchCode, ShieldCheck, Wrench } from 'lucide-react';
import { api, asError } from '../lib/api';
import { useStore } from '../lib/store';
import { cx } from '../lib/format';
import { Modal } from '../components/ui';
import { DiscordGlyph } from '../components/icons';
import { CrashFindings } from './CrashExplain';

/** Confirmation before the Discord invite opens in the browser (from the rail and from Settings). */
export function DiscordPrompt() {
  const { discordPrompt, closeDiscord, info, toast } = useStore();
  if (!discordPrompt) return null;
  const invite = info?.config.discordInviteUrl ?? '';
  let host = invite;
  try {
    const u = new URL(invite);
    host = `${u.host}${u.pathname}`;
  } catch {
    /* shown as is */
  }
  const go = () => {
    closeDiscord();
    api.openDiscordInvite().catch((e) => toast(asError(e).message, 'error'));
  };
  return (
    <Modal
      title="Leaving Stabled"
      onClose={closeDiscord}
      footer={
        <>
          <button type="button" className="btn" onClick={closeDiscord}>
            Stay here
          </button>
          <button type="button" className="btn discord-btn" data-autofocus onClick={go}>
            Continue to Discord <ExternalLink size={14} />
          </button>
        </>
      }
    >
      <div className="leave-card">
        <span className="leave-badge">
          <DiscordGlyph size={26} />
        </span>
        <div style={{ minWidth: 0 }}>
          <p>
            You're about to be directed to the <strong>Stabled Discord server</strong> in your web browser.
          </p>
          <div className="leave-url selectable">{host}</div>
        </div>
      </div>
      <ul className="leave-points">
        <li>
          <ShieldCheck size={15} /> Stabled doesn't send any of your information to Discord. Your browser opens the invite and you decide whether to join.
        </li>
        <li>
          <ExternalLink size={15} /> Discord is a separate service with its own terms and privacy policy.
        </li>
        <li>
          <AlertTriangle size={15} /> Staff will never ask for your Microsoft password or a Minecraft account code. Don't share them with anyone.
        </li>
      </ul>
    </Modal>
  );
}

/** The pre-launch mod check found errors: show them and let the player decide. */
export function LaunchCheckDialog() {
  const { launchCheck, resolveLaunchCheck, openSave } = useStore();
  if (!launchCheck) return null;
  const { save, report } = launchCheck;
  const errors = report.problems.filter((p) => p.level === 'error');
  const warnings = report.problems.filter((p) => p.level === 'warning');
  return (
    <Modal
      title="Problems with this Save's mods"
      onClose={() => resolveLaunchCheck(false)}
      wide
      footer={
        <>
          <button type="button" className="btn" onClick={() => resolveLaunchCheck(false)}>
            Cancel
          </button>
          <button
            type="button"
            className="btn"
            onClick={() => {
              resolveLaunchCheck(false);
              openSave(save.id, 'mods');
            }}
          >
            <Puzzle size={15} /> Fix in Mods
          </button>
          <button type="button" className="btn btn-danger" onClick={() => resolveLaunchCheck(true)}>
            <Play size={15} /> Launch anyway
          </button>
        </>
      }
    >
      <p className="muted small">
        Stabled checked the {report.checkedMods} enabled mods in <strong>{save.name}</strong> before launching. Minecraft will probably crash or skip these mods if you launch now.
      </p>
      <ul className="compat-list">
        {[...errors, ...warnings].map((p, i) => (
          <li key={i} className={cx('compat-item', p.level)} style={{ ['--i' as string]: i }}>
            {p.level === 'error' ? <AlertOctagon size={16} /> : <AlertTriangle size={16} />}
            <div style={{ minWidth: 0 }}>
              {p.modName && <div className="compat-mod">{p.modName}</div>}
              <div className="small">{p.message}</div>
              {p.fix && <div className="tiny muted compat-fix">{p.fix}</div>}
            </div>
          </li>
        ))}
      </ul>
    </Modal>
  );
}

/** After a crash: the explanation, with the next steps one click away. */
export function CrashDialog() {
  const { crash, showCrash, openSave, setBisectSave, toast } = useStore();
  if (!crash) return null;
  const close = () => showCrash(null);
  return (
    <Modal
      title={`${crash.save.name} crashed`}
      onClose={close}
      wide
      footer={
        <>
          <button
            type="button"
            className="btn"
            onClick={() => {
              close();
              api.saveInstall(crash.save.id, true).catch((e) => toast(asError(e).message, 'error'));
            }}
          >
            <Wrench size={15} /> Repair
          </button>
          <button
            type="button"
            className="btn"
            onClick={() => {
              close();
              openSave(crash.save.id, 'mods');
              setBisectSave(crash.save.id);
            }}
          >
            <SearchCode size={15} /> Find the broken mod
          </button>
          <button
            type="button"
            className="btn btn-primary"
            onClick={() => {
              close();
              openSave(crash.save.id, 'logs');
            }}
          >
            <FileText size={15} /> Open Logs
          </button>
        </>
      }
    >
      <p className="muted small">Stabled read the crash report and logs. Here's what most likely happened and what to try, most likely first.</p>
      <CrashFindings report={crash.report} />
    </Modal>
  );
}
