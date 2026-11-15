import { AlertTriangle, CheckCircle2, Gauge, Puzzle, Quote } from 'lucide-react';
import type { CrashReport } from '../lib/api';
import { cx } from '../lib/format';

const CONFIDENCE: Record<string, { label: string; level: number }> = {
  high: { label: 'Very likely', level: 3 },
  medium: { label: 'Likely', level: 2 },
  low: { label: 'Possible', level: 1 },
};

/** Crash findings: what happened, how sure Stabled is, and what to do. */
export function CrashFindings({ report }: { report: CrashReport }) {
  return (
    <div className="crash-findings">
      {report.headline && (
        <div className="crash-headline">
          <AlertTriangle size={15} />
          <span className="selectable">{report.headline}</span>
        </div>
      )}
      {report.findings.map((f, i) => {
        const c = CONFIDENCE[f.confidence] ?? CONFIDENCE.low;
        return (
          <article key={f.id + i} className={cx('crash-card', `conf-${f.confidence}`)} style={{ ['--i' as string]: i }}>
            <header className="crash-card-head">
              <h3>{f.title}</h3>
              <span className="confidence" title={`Confidence: ${c.label.toLowerCase()}`}>
                <Gauge size={13} />
                <span className="confidence-bars" aria-hidden="true">
                  {[1, 2, 3].map((n) => (
                    <i key={n} className={cx(n <= c.level && 'on')} />
                  ))}
                </span>
                {c.label}
              </span>
            </header>
            <p className="small">{f.explanation}</p>
            {f.fixes.length > 0 && (
              <ul className="crash-fixes">
                {f.fixes.map((x) => (
                  <li key={x}>
                    <CheckCircle2 size={14} />
                    <span>{x}</span>
                  </li>
                ))}
              </ul>
            )}
            {f.mods.length > 0 && (
              <div className="crash-mods">
                {f.mods.map((m) => (
                  <span key={m} className="chip">
                    <Puzzle size={12} /> {m}
                  </span>
                ))}
              </div>
            )}
            {f.evidence && (
              <blockquote className="crash-evidence selectable">
                <Quote size={12} />
                <code>{f.evidence}</code>
              </blockquote>
            )}
          </article>
        );
      })}
      {report.file && <p className="tiny muted">Based on {report.file}{report.exitCode != null ? ` · exit code ${report.exitCode}` : ''}.</p>}
    </div>
  );
}
