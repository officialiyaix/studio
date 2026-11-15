import { useEffect, useMemo, useRef, useState } from 'react';
import { CheckCircle2, CircleHelp, FlaskConical, Loader2, Play, PowerOff, SearchCode, Trash2, XCircle } from 'lucide-react';
import { api, asError, onEvent, type BisectCandidate, type Save } from '../lib/api';
import { useStore } from '../lib/store';
import { cx } from '../lib/format';
import { Callout, Modal, ProgressBar } from '../components/ui';

type Step =
  | { kind: 'intro' }
  | { kind: 'preparing' }
  | { kind: 'test'; round: number; test: string[]; enabled: string[]; purpose: 'baseline' | 'half' | 'confirm' }
  | { kind: 'found'; file: string; confirmed: boolean }
  | { kind: 'not-mods' }
  | { kind: 'combo' };

/**
 * Find the broken mod: a test copy of the Save (the original is never changed) is launched with groups of mods
 * switched off. After each test the player says whether the problem happened, and the suspects are halved.
 */
export function BisectWizard({ save, onClose }: { save: Save; onClose: () => void }) {
  const { openSignIn, toast } = useStore();
  const [withWorlds, setWithWorlds] = useState(false);
  const [step, setStep] = useState<Step>({ kind: 'intro' });
  const [testId, setTestId] = useState<string | null>(null);
  const [candidates, setCandidates] = useState<BisectCandidate[]>([]);
  const [suspects, setSuspects] = useState<string[]>([]);
  const [status, setStatus] = useState<Save['status'] | null>(null);
  const [crashed, setCrashed] = useState<boolean | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const testRef = useRef<string | null>(null);
  testRef.current = testId;

  const all = useMemo(() => candidates.map((c) => c.file), [candidates]);
  const name = (f: string) => candidates.find((c) => c.file === f)?.name ?? f;
  const totalRounds = 2 + Math.ceil(Math.log2(Math.max(2, all.length)));

  // The test copy's status and exit.
  useEffect(() => {
    const subs = [
      onEvent<Save>('save-updated', (s) => {
        if (s.id === testRef.current) setStatus(s.status);
      }),
      onEvent<{ id: string; code: number | null; crash: unknown }>('game-exited', (e) => {
        if (e.id === testRef.current) setCrashed(!!e.crash || (e.code != null && e.code !== 0));
      }),
    ];
    return () => {
      subs.forEach((p) => p.then((un) => un()));
    };
  }, []);

  // The test copy is deleted when the wizard closes, whatever happened.
  useEffect(
    () => () => {
      if (testRef.current) void api.bisectEnd(testRef.current).catch(() => undefined);
    },
    [],
  );

  const runTest = async (id: string, keep: string[], purpose: 'baseline' | 'half' | 'confirm', round: number) => {
    setBusy(true);
    setError(null);
    setCrashed(null);
    setStatus(null);
    try {
      const enabled = await api.bisectApply(id, all.length ? all : keep, keep);
      setStep({ kind: 'test', round, test: keep, enabled, purpose });
    } catch (e) {
      setError(asError(e).message);
    } finally {
      setBusy(false);
    }
  };

  const start = async () => {
    setStep({ kind: 'preparing' });
    setError(null);
    try {
      const r = await api.bisectStart(save.id, withWorlds);
      setTestId(r.testId);
      setCandidates(r.candidates);
      const files = r.candidates.map((c) => c.file);
      setSuspects(files);
      // Round 1: none of the player's mods, to check the problem really comes from one of them.
      setBusy(true);
      const enabled = await api.bisectApply(r.testId, files, []);
      setStep({ kind: 'test', round: 1, test: [], enabled, purpose: 'baseline' });
    } catch (e) {
      setError(asError(e).message);
      setStep({ kind: 'intro' });
    } finally {
      setBusy(false);
    }
  };

  const launch = async () => {
    if (!testId) return;
    setCrashed(null);
    try {
      await api.saveLaunch(testId);
    } catch (e) {
      const err = asError(e);
      if (err.code === 'SIGN_IN_REQUIRED' || err.code === 'NO_JAVA_ACCESS' || err.code === 'AUTH_UNCONFIGURED') openSignIn(err.message);
      else toast(err.message, 'error');
    }
  };

  const answer = async (happened: boolean) => {
    if (step.kind !== 'test' || !testId) return;
    const round = step.round + 1;
    if (step.purpose === 'baseline') {
      if (happened) return setStep({ kind: 'not-mods' });
      return half(suspects, round);
    }
    if (step.purpose === 'confirm') {
      return setStep(happened ? { kind: 'found', file: step.test[0], confirmed: true } : { kind: 'combo' });
    }
    const next = happened ? suspects.filter((f) => step.test.includes(f)) : suspects.filter((f) => !step.test.includes(f));
    setSuspects(next);
    if (next.length === 0) return setStep({ kind: 'combo' });
    if (next.length === 1) return runTest(testId, next, 'confirm', round);
    return half(next, round);
  };

  const half = (list: string[], round: number) => {
    if (!testId) return;
    if (list.length === 1) return runTest(testId, list, 'confirm', round);
    return runTest(testId, list.slice(0, Math.ceil(list.length / 2)), 'half', round);
  };

  const running = status === 'running' || status === 'launching' || status === 'downloading';
  const finished = step.kind === 'found' || step.kind === 'not-mods' || step.kind === 'combo';
  const round = step.kind === 'test' ? step.round : finished ? totalRounds : 0;

  const applyToSave = async (file: string, op: 'disable' | 'remove') => {
    try {
      if (op === 'disable') await api.modsSetEnabled(save.id, file, false);
      else await api.modsRemove(save.id, file);
      toast(`${name(file)} was ${op === 'disable' ? 'disabled' : 'moved to the Recycle Bin'} in "${save.name}".`, 'success');
      onClose();
    } catch (e) {
      toast(asError(e).message, 'error');
    }
  };

  return (
    <Modal
      title="Find the broken mod"
      onClose={onClose}
      wide
      dismissible={!running}
      footer={
        <>
          <button type="button" className="btn" onClick={onClose} disabled={running}>
            {finished ? 'Done' : 'Cancel'}
          </button>
          {step.kind === 'intro' && (
            <button type="button" className="btn btn-primary" onClick={start}>
              <FlaskConical size={15} /> Make test copy and start
            </button>
          )}
        </>
      }
    >
      {round > 0 && (
        <div className="bisect-progress">
          <ProgressBar value={Math.min(100, (round / totalRounds) * 100)} label="Progress" />
          <span className="tiny muted">{finished ? 'Finished' : `Test ${round} of about ${totalRounds}`}</span>
        </div>
      )}
      {error && <Callout tone="danger">{error}</Callout>}

      <div className="bisect-step" key={`${step.kind}-${round}`}>
        {step.kind === 'intro' && (
          <>
            <div className="bisect-hero">
              <SearchCode size={34} />
              <div>
                <h3>Let Stabled narrow it down</h3>
                <p className="muted small">
                  Stabled makes a hidden test copy of <strong>{save.name}</strong> and launches it with groups of your mods switched off. After each test you tell Stabled whether the problem happened, and the list of suspects halves each time. Your Save is never changed while testing.
                </p>
              </div>
            </div>
            <ol className="bisect-how">
              <li>Launch the test copy and try to make the problem happen (start the game, join a world, do what crashed it).</li>
              <li>Close Minecraft (or let it crash) and answer: did it happen?</li>
              <li>Repeat a few times. Mods that others need stay on automatically.</li>
            </ol>
            <label className="check-row">
              <input type="checkbox" checked={withWorlds} onChange={(e) => setWithWorlds(e.target.checked)} />
              <span>
                Copy my worlds into the test copy
                <span className="tiny muted" style={{ display: 'block' }}>
                  Needed if the problem only happens inside a world. Testing never touches your real worlds.
                </span>
              </span>
            </label>
          </>
        )}

        {step.kind === 'preparing' && (
          <div className="bisect-wait">
            <Loader2 size={22} className="spin" /> Making the test copy…
          </div>
        )}

        {step.kind === 'test' && (
          <>
            <h3 className="bisect-q">
              {step.purpose === 'baseline'
                ? 'First, check the problem goes away without your mods'
                : step.purpose === 'confirm'
                  ? `Last check: only ${name(step.test[0])}`
                  : `Testing ${step.test.length} of ${suspects.length} suspects`}
            </h3>
            <p className="muted small">
              {step.purpose === 'baseline'
                ? 'This test runs with all of your own mods off (Stabled components stay on). If the problem still happens, a mod isn\'t the cause.'
                : 'Only the mods marked "On" are enabled in the test copy, plus anything they need.'}
            </p>
            <div className="bisect-mods">
              {candidates.map((c, i) => {
                const on = step.enabled.includes(c.file);
                const suspect = suspects.includes(c.file);
                return (
                  <span key={c.file} className={cx('bisect-mod', on && 'on', !suspect && 'cleared')} style={{ ['--i' as string]: i }} title={c.file}>
                    {on ? <CheckCircle2 size={12} /> : <PowerOff size={12} />} {c.name}
                  </span>
                );
              })}
            </div>
            <div className="bisect-run">
              <button type="button" className="btn btn-cta" onClick={launch} disabled={running || busy}>
                {running ? <Loader2 size={16} className="spin" /> : <Play size={16} fill="currentColor" />}
                {status === 'running' ? 'Test running…' : running ? 'Starting…' : crashed !== null ? 'Launch again' : 'Launch test'}
              </button>
              <span className="tiny muted">
                {status === 'running'
                  ? 'Try to make the problem happen, then close Minecraft.'
                  : crashed === true
                    ? 'The test crashed.'
                    : crashed === false
                      ? 'The test closed normally.'
                      : 'Or answer straight away if you already know.'}
              </span>
            </div>
            <div className="bisect-answer">
              <span className="small">Did the problem happen?</span>
              <button type="button" className={cx('btn', crashed === true && 'btn-danger')} disabled={running || busy} onClick={() => answer(true)}>
                <XCircle size={15} /> Yes, it happened
              </button>
              <button type="button" className={cx('btn', crashed === false && 'btn-primary')} disabled={running || busy} onClick={() => answer(false)}>
                <CheckCircle2 size={15} /> No, it worked
              </button>
            </div>
          </>
        )}

        {step.kind === 'found' && (
          <div className="bisect-result found">
            <span className="bisect-result-icon">
              <SearchCode size={28} />
            </span>
            <h3>{name(step.file)} is causing the problem</h3>
            <p className="muted small">The problem happened with only this mod on (and what it needs), and not without it. Check for an update made for Minecraft {save.minecraftVersion}, or keep it off.</p>
            <div className="bisect-result-actions">
              <button type="button" className="btn btn-primary" onClick={() => applyToSave(step.file, 'disable')}>
                <PowerOff size={15} /> Disable it in {save.name}
              </button>
              <button type="button" className="btn btn-danger" onClick={() => applyToSave(step.file, 'remove')}>
                <Trash2 size={15} /> Remove it
              </button>
            </div>
          </div>
        )}
        {step.kind === 'not-mods' && (
          <div className="bisect-result">
            <span className="bisect-result-icon ok">
              <CheckCircle2 size={28} />
            </span>
            <h3>Your mods aren't the cause</h3>
            <p className="muted small">The problem happened with all of your own mods off. Try Repair to rule out damaged game files, check your graphics driver, or look at the explanation in the Logs tab.</p>
          </div>
        )}
        {step.kind === 'combo' && (
          <div className="bisect-result">
            <span className="bisect-result-icon">
              <CircleHelp size={28} />
            </span>
            <h3>It takes a combination of mods</h3>
            <p className="muted small">No single mod causes it on its own: two or more mods conflict with each other. The last mods tested were {suspects.map(name).join(', ') || 'none'}. Try disabling recently added mods in pairs, or ask in the Stabled Discord.</p>
          </div>
        )}
      </div>
    </Modal>
  );
}
