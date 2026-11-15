import { useCallback, useEffect, useMemo, useState } from 'react';
import { CheckCircle2, Gamepad2, Gauge, Loader2, MonitorCog, MousePointer2, PlugZap, RefreshCw, RotateCcw, Rocket, ShieldAlert, Wand2, XCircle, Zap } from 'lucide-react';
import { api, asError, type OptimiserAction, type OptimiserBatchResult, type OptimiserView } from '../lib/api';
import { useStore } from '../lib/store';
import { cx, relativeTime } from '../lib/format';
import { Callout, ConfirmDialog } from '../components/ui';

const CATEGORIES: Array<{ id: OptimiserAction['category']; title: string; blurb: string; icon: typeof Zap }> = [
  { id: 'launch', title: 'Minecraft launch', blurb: 'Applied by Stabled each time it starts a Save.', icon: Rocket },
  { id: 'power', title: 'Power', blurb: 'Keep the CPU running at full speed.', icon: Zap },
  { id: 'graphics', title: 'Graphics', blurb: 'Make sure Minecraft gets the right GPU and frame delivery.', icon: MonitorCog },
  { id: 'input', title: 'Mouse and keyboard', blurb: 'Precise aim and no pop-ups mid-game.', icon: MousePointer2 },
  { id: 'background', title: 'Windows and background apps', blurb: 'Less going on behind the game.', icon: Gamepad2 },
];

const IMPACT = { high: 'Big impact', medium: 'Helps', low: 'Small boost' } as const;

export function OptimiserPage() {
  const { toast } = useStore();
  const [view, setView] = useState<OptimiserView | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [results, setResults] = useState<Record<string, { ok: boolean; message: string }>>({});
  const [confirm, setConfirm] = useState<OptimiserAction | null>(null);
  const [confirmAll, setConfirmAll] = useState<'apply' | 'restore' | null>(null);
  const [batch, setBatch] = useState<OptimiserBatchResult[] | null>(null);

  const load = useCallback(async () => {
    try {
      setView(await api.optimiserView());
    } catch (e) {
      toast(asError(e).message, 'error');
    }
  }, [toast]);

  useEffect(() => {
    load();
  }, [load]);

  const run = async (a: OptimiserAction, op: 'apply' | 'restore') => {
    setBusy(a.id);
    try {
      const message = op === 'apply' ? await api.optimiserApply(a.id) : await api.optimiserRestore(a.id);
      setResults((r) => ({ ...r, [a.id]: { ok: true, message } }));
    } catch (e) {
      setResults((r) => ({ ...r, [a.id]: { ok: false, message: asError(e).message } }));
    } finally {
      setBusy(null);
      load();
    }
  };

  const runAll = async (op: 'apply' | 'restore') => {
    setBusy(op === 'apply' ? '*apply' : '*restore');
    setBatch(null);
    try {
      const r = op === 'apply' ? await api.optimiserApplyAll() : await api.optimiserRestoreAll();
      setBatch(r);
      setResults((old) => ({ ...old, ...Object.fromEntries(r.map((x) => [x.id, { ok: x.ok, message: x.message }])) }));
      const ok = r.filter((x) => x.ok).length;
      if (r.length === 0) toast(op === 'apply' ? 'Everything available is already applied.' : 'Nothing to restore.', 'info');
      else toast(`${op === 'apply' ? 'Applied' : 'Restored'} ${ok} of ${r.length} optimisation${r.length === 1 ? '' : 's'}.`, ok === r.length ? 'success' : 'warning');
    } catch (e) {
      toast(asError(e).message, 'error');
    } finally {
      setBusy(null);
      load();
    }
  };

  const actions = view?.actions ?? [];
  const available = actions.filter((a) => a.state !== 'unavailable');
  const applied = available.filter((a) => a.state === 'applied').length;
  const pending = available.filter((a) => a.state === 'not_applied');
  const restorable = actions.filter((a) => a.canRestore).length;
  const pct = available.length ? Math.round((applied / available.length) * 100) : 0;
  const pendingAdmin = pending.filter((a) => a.requiresAdmin);
  const pendingRestart = pending.filter((a) => a.requiresRestart);
  const grouped = useMemo(() => CATEGORIES.map((c) => ({ ...c, items: actions.filter((a) => a.category === c.id) })).filter((c) => c.items.length), [actions]);

  return (
    <div className="page optimiser scroll">
      <section className="opt-hero panel">
        <div className="opt-score" style={{ ['--p' as string]: pct }} aria-label={`${applied} of ${available.length} optimisations applied`}>
          <svg viewBox="0 0 44 44" aria-hidden="true">
            <circle cx="22" cy="22" r="19" className="track" />
            <circle cx="22" cy="22" r="19" className="fill" pathLength={100} />
          </svg>
          <span>
            <strong>{applied}</strong>/{available.length || '–'}
          </span>
        </div>
        <div className="opt-hero-text">
          <h2>
            <Wand2 size={20} /> {pending.length === 0 && available.length ? 'Your PC is tuned for Minecraft' : 'Tune your PC for Minecraft'}
          </h2>
          <p className="muted small">
            Nothing here changes on its own. Each setting shows its current state, changes only when you apply it, records what it replaced, and can be restored.
          </p>
        </div>
        <div className="opt-hero-actions">
          <button
            type="button"
            className={cx('btn btn-cta apply-all', busy === '*apply' && 'busy')}
            disabled={!view || pending.length === 0 || busy !== null}
            onClick={() => setConfirmAll('apply')}
          >
            {busy === '*apply' ? <Loader2 size={20} className="spin" /> : <Zap size={20} fill="currentColor" />}
            <span>
              Apply all
              <small>{pending.length ? `${pending.length} optimisation${pending.length === 1 ? '' : 's'}` : 'All applied'}</small>
            </span>
          </button>
          <div className="opt-hero-small">
            <button type="button" className="btn btn-sm" onClick={() => setConfirmAll('restore')} disabled={!restorable || busy !== null}>
              {busy === '*restore' ? <Loader2 size={14} className="spin" /> : <RotateCcw size={14} />} Restore all
            </button>
            <button type="button" className="btn btn-sm" onClick={load} disabled={busy !== null}>
              <RefreshCw size={14} /> Re-check
            </button>
          </div>
        </div>
      </section>

      {view && (
        <Callout tone={view.originalOptimiser.connected ? 'success' : 'warning'}>
          <strong>
            <PlugZap size={14} style={{ verticalAlign: '-2px' }} /> Earlier optimiser not connected.
          </strong>{' '}
          {view.originalOptimiser.message}
        </Callout>
      )}
      {view?.platformMessage && <Callout tone="info">{view.platformMessage}</Callout>}
      {batch && batch.some((b) => !b.ok) && (
        <Callout tone="warning">
          Some optimisations weren't applied:
          <ul className="opt-batch">
            {batch
              .filter((b) => !b.ok)
              .map((b) => (
                <li key={b.id}>
                  <strong>{b.title}:</strong> {b.message}
                </li>
              ))}
          </ul>
        </Callout>
      )}

      {!view && (
        <div className="opt-list">
          {[0, 1, 2, 3].map((i) => (
            <div key={i} className="skeleton" style={{ height: 150 }} />
          ))}
        </div>
      )}
      {grouped.map((g, gi) => {
        const Icon = g.icon;
        const done = g.items.filter((a) => a.state === 'applied').length;
        return (
          <section key={g.id} className="opt-group" style={{ ['--g' as string]: gi }}>
            <header className="opt-group-head">
              <span className="opt-group-icon">
                <Icon size={17} />
              </span>
              <div>
                <h3>{g.title}</h3>
                <p className="tiny muted">{g.blurb}</p>
              </div>
              <span className="tiny muted opt-group-count">
                {done} of {g.items.length} applied
              </span>
            </header>
            <div className="opt-list">
              {g.items.map((a, i) => {
                const result = results[a.id];
                const isApplied = a.state === 'applied';
                const unavailable = a.state === 'unavailable';
                return (
                  <article key={a.id} className={cx('opt-card panel', isApplied && 'applied')} aria-busy={busy === a.id} style={{ ['--i' as string]: i }}>
                    <div className="opt-head">
                      <div style={{ minWidth: 0 }}>
                        <h3>{a.title}</h3>
                        <p className="small muted">{a.description}</p>
                      </div>
                      <span className={cx('badge', isApplied ? 'success' : unavailable ? '' : a.state === 'unknown' ? 'warning' : 'info')}>
                        {isApplied ? (
                          <>
                            <CheckCircle2 size={12} /> Applied
                          </>
                        ) : unavailable ? (
                          'Unavailable'
                        ) : a.state === 'unknown' ? (
                          'Unknown'
                        ) : (
                          'Not applied'
                        )}
                      </span>
                    </div>
                    <dl className="opt-state">
                      <dt>Current</dt>
                      <dd>{a.current || '—'}</dd>
                      {a.target && (
                        <>
                          <dt>Stabled sets</dt>
                          <dd>{a.target}</dd>
                        </>
                      )}
                    </dl>
                    <div className="opt-tags">
                      <span className={cx('impact', `impact-${a.impact}`)}>
                        <Gauge size={12} /> {IMPACT[a.impact]}
                      </span>
                      {a.requiresAdmin && (
                        <span className="badge warning">
                          <ShieldAlert size={12} /> Needs administrator
                        </span>
                      )}
                      {a.requiresRestart && <span className="badge">Restart Windows to take effect</span>}
                      {a.appliedAt && <span className="tiny muted">Applied by Stabled {relativeTime(a.appliedAt)}</span>}
                    </div>
                    {a.unavailableReason && <p className="tiny muted">{a.unavailableReason}</p>}
                    <details className="opt-details">
                      <summary className="tiny">What exactly changes</summary>
                      <p className="tiny muted selectable">{a.details}</p>
                    </details>
                    {result && (
                      <div className={cx('opt-result', result.ok ? 'ok' : 'bad')} role="status">
                        {result.ok ? <CheckCircle2 size={14} /> : <XCircle size={14} />} {result.message}
                      </div>
                    )}
                    <div className="opt-actions">
                      <button type="button" className="btn btn-primary btn-sm" disabled={unavailable || isApplied || busy !== null} onClick={() => (a.requiresAdmin ? setConfirm(a) : run(a, 'apply'))}>
                        {busy === a.id ? <Loader2 size={14} className="spin" /> : null} Apply
                      </button>
                      <button type="button" className="btn btn-sm" disabled={!a.canRestore || busy !== null} onClick={() => run(a, 'restore')} title={a.canRestore ? 'Put back the setting Stabled replaced' : 'Nothing recorded to restore'}>
                        <RotateCcw size={14} /> Restore previous
                      </button>
                    </div>
                  </article>
                );
              })}
            </div>
          </section>
        );
      })}

      {confirm && (
        <ConfirmDialog
          title={`Apply ${confirm.title}?`}
          confirmLabel="Continue"
          onCancel={() => setConfirm(null)}
          onConfirm={() => {
            const a = confirm;
            setConfirm(null);
            run(a, 'apply');
          }}
        >
          <p>Windows will ask for administrator permission because this is a machine-wide setting. Stabled records the current value first so you can restore it here.</p>
          {confirm.requiresRestart && <p className="muted small">It takes effect after you restart Windows.</p>}
        </ConfirmDialog>
      )}
      {confirmAll === 'apply' && (
        <ConfirmDialog
          title={`Apply ${pending.length} optimisation${pending.length === 1 ? '' : 's'}?`}
          confirmLabel="Apply all"
          onCancel={() => setConfirmAll(null)}
          onConfirm={() => {
            setConfirmAll(null);
            runAll('apply');
          }}
        >
          <ul className="opt-confirm-list">
            {pending.map((a) => (
              <li key={a.id}>
                <CheckCircle2 size={14} /> {a.title}
              </li>
            ))}
          </ul>
          <p className="muted small">
            Stabled records every current value first, so Restore all puts everything back.
            {pendingAdmin.length > 0 && ` Windows asks for administrator permission once, for ${pendingAdmin.map((a) => a.title).join(' and ')}.`}
            {pendingRestart.length > 0 && ' Some changes take effect after you restart Windows.'}
          </p>
        </ConfirmDialog>
      )}
      {confirmAll === 'restore' && (
        <ConfirmDialog
          title="Restore all previous settings?"
          confirmLabel="Restore all"
          onCancel={() => setConfirmAll(null)}
          onConfirm={() => {
            setConfirmAll(null);
            runAll('restore');
          }}
        >
          <p>Every setting Stabled changed goes back to the value it had before ({restorable} in total).</p>
        </ConfirmDialog>
      )}
    </div>
  );
}
