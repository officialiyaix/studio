import { useCallback, useEffect, useState } from 'react';
import { CheckCircle2, Loader2, PlugZap, RefreshCw, RotateCcw, ShieldAlert, Wand2 } from 'lucide-react';
import { api, asError, type OptimiserAction, type OptimiserView } from '../lib/api';
import { useStore } from '../lib/store';
import { cx, relativeTime } from '../lib/format';
import { Callout, ConfirmDialog } from '../components/ui';

export function OptimiserPage() {
  const { toast } = useStore();
  const [view, setView] = useState<OptimiserView | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [results, setResults] = useState<Record<string, { ok: boolean; message: string }>>({});
  const [confirm, setConfirm] = useState<OptimiserAction | null>(null);

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

  return (
    <div className="page optimiser scroll">
      <div className="opt-intro panel">
        <Wand2 size={22} />
        <div>
          <h2>Windows settings that affect games</h2>
          <p className="muted small">
            Nothing here changes on its own — not when Stabled opens and not when a Save launches. Each setting shows its current state, changes only when you choose Apply, records what it replaced, and can be restored.
          </p>
        </div>
        <button type="button" className="btn btn-sm" onClick={load}>
          <RefreshCw size={15} /> Re-check
        </button>
      </div>

      {view && (
        <Callout tone={view.originalOptimiser.connected ? 'success' : 'warning'}>
          <strong>
            <PlugZap size={14} style={{ verticalAlign: '-2px' }} /> Earlier optimiser not connected.
          </strong>{' '}
          {view.originalOptimiser.message}
        </Callout>
      )}
      {view?.platformMessage && <Callout tone="info">{view.platformMessage}</Callout>}

      <div className="opt-list">
        {!view && [0, 1, 2].map((i) => <div key={i} className="skeleton" style={{ height: 110 }} />)}
        {view?.actions.map((a) => {
          const result = results[a.id];
          const applied = a.state === 'applied';
          const unavailable = a.state === 'unavailable' || !view.supported;
          return (
            <article key={a.id} className={cx('opt-card panel', applied && 'applied')} aria-busy={busy === a.id}>
              <div className="opt-head">
                <div style={{ minWidth: 0 }}>
                  <h3>{a.title}</h3>
                  <p className="small muted">{a.description}</p>
                </div>
                <span className={cx('badge', applied ? 'success' : unavailable ? '' : a.state === 'unknown' ? 'warning' : 'info')}>
                  {applied ? 'Applied' : unavailable ? 'Unavailable' : a.state === 'unknown' ? 'Unknown' : 'Not applied'}
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
                  {result.ok && <CheckCircle2 size={14} />} {result.message}
                </div>
              )}
              <div className="opt-actions">
                <button type="button" className="btn btn-primary btn-sm" disabled={unavailable || applied || busy !== null} onClick={() => (a.requiresAdmin ? setConfirm(a) : run(a, 'apply'))}>
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
    </div>
  );
}
