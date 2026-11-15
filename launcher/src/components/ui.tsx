import { useEffect, useId, useRef, type ReactNode } from 'react';
import { AlertTriangle, CheckCircle2, Info, Loader2, X, XCircle } from 'lucide-react';
import type { Save } from '../lib/api';
import { cx, pct } from '../lib/format';

export function Toggle({ checked, onChange, label, disabled }: { checked: boolean; onChange: (v: boolean) => void; label: string; disabled?: boolean }) {
  return (
    <button
      type="button"
      role="switch"
      className="toggle"
      aria-checked={checked}
      aria-label={label}
      disabled={disabled}
      onClick={() => onChange(!checked)}
    />
  );
}

export function SettingRow({ label, desc, children, htmlFor }: { label: ReactNode; desc?: ReactNode; children: ReactNode; htmlFor?: string }) {
  return (
    <div className="setting-row">
      <div className="text">
        {htmlFor ? (
          <label className="label" htmlFor={htmlFor}>
            {label}
          </label>
        ) : (
          <span className="label">{label}</span>
        )}
        {desc && <span className="desc">{desc}</span>}
      </div>
      {children}
    </div>
  );
}

export function Segmented<T extends string | number>({
  value,
  options,
  onChange,
  label,
  disabled,
}: {
  value: T;
  options: Array<{ value: T; label: string }>;
  onChange: (v: T) => void;
  label: string;
  disabled?: boolean;
}) {
  const refs = useRef<Array<HTMLButtonElement | null>>([]);
  const move = (i: number) => {
    const n = (i + options.length) % options.length;
    onChange(options[n].value);
    refs.current[n]?.focus();
  };
  return (
    <div className="segmented" role="radiogroup" aria-label={label}>
      {options.map((o, i) => (
        <button
          key={String(o.value)}
          ref={(el) => {
            refs.current[i] = el;
          }}
          type="button"
          role="radio"
          aria-checked={o.value === value}
          tabIndex={o.value === value ? 0 : -1}
          disabled={disabled}
          onClick={() => onChange(o.value)}
          onKeyDown={(e) => {
            if (e.key === 'ArrowRight' || e.key === 'ArrowDown') {
              e.preventDefault();
              move(i + 1);
            } else if (e.key === 'ArrowLeft' || e.key === 'ArrowUp') {
              e.preventDefault();
              move(i - 1);
            }
          }}
        >
          {o.label}
        </button>
      ))}
    </div>
  );
}

export function Range({
  value,
  min,
  max,
  step = 1,
  onChange,
  onCommit,
  label,
  disabled,
  id,
}: {
  value: number;
  min: number;
  max: number;
  step?: number;
  onChange: (v: number) => void;
  onCommit?: (v: number) => void;
  label: string;
  disabled?: boolean;
  id?: string;
}) {
  const fill = ((value - min) / (max - min)) * 100;
  return (
    <input
      id={id}
      className="range"
      type="range"
      min={min}
      max={max}
      step={step}
      value={value}
      aria-label={label}
      disabled={disabled}
      style={{ ['--fill' as string]: `${fill}%` }}
      onChange={(e) => onChange(Number(e.target.value))}
      onPointerUp={(e) => onCommit?.(Number((e.target as HTMLInputElement).value))}
      onKeyUp={(e) => onCommit?.(Number((e.target as HTMLInputElement).value))}
    />
  );
}

export function Modal({
  title,
  onClose,
  children,
  footer,
  wide,
  closeLabel = 'Close',
  labelledBy,
  dismissible = true,
}: {
  title?: ReactNode;
  onClose: () => void;
  children: ReactNode;
  footer?: ReactNode;
  wide?: boolean;
  closeLabel?: string;
  labelledBy?: string;
  dismissible?: boolean;
}) {
  const ref = useRef<HTMLDivElement>(null);
  const titleId = useId();
  useEffect(() => {
    const prev = document.activeElement as HTMLElement | null;
    const el = ref.current;
    // Prefer the element marked data-autofocus (e.g. the first form field), not whatever comes first in the DOM.
    const first =
      el?.querySelector<HTMLElement>('[data-autofocus]') ?? el?.querySelector<HTMLElement>('input, button, [tabindex]:not([tabindex="-1"])');
    first?.focus();
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && dismissible) {
        e.stopPropagation();
        onClose();
      }
      if (e.key === 'Tab' && el) {
        const focusables = Array.from(el.querySelectorAll<HTMLElement>('button:not(:disabled), input:not(:disabled), select, textarea, [tabindex]:not([tabindex="-1"])'));
        if (focusables.length === 0) return;
        const f = focusables[0];
        const l = focusables[focusables.length - 1];
        if (e.shiftKey && document.activeElement === f) {
          e.preventDefault();
          l.focus();
        } else if (!e.shiftKey && document.activeElement === l) {
          e.preventDefault();
          f.focus();
        }
      }
    };
    window.addEventListener('keydown', onKey, true);
    return () => {
      window.removeEventListener('keydown', onKey, true);
      prev?.focus?.();
    };
  }, [onClose, dismissible]);
  return (
    <div className="modal-backdrop" onMouseDown={(e) => e.target === e.currentTarget && dismissible && onClose()}>
      <div ref={ref} className={cx('modal', wide && 'wide')} role="dialog" aria-modal="true" aria-labelledby={labelledBy ?? titleId}>
        {title !== undefined && (
          <div className="modal-head">
            <h2 id={titleId}>{title}</h2>
            {dismissible && (
              <button type="button" className="icon-btn plain sm" aria-label={closeLabel} onClick={onClose}>
                <X size={18} />
              </button>
            )}
          </div>
        )}
        <div className="modal-body">{children}</div>
        {footer && <div className="modal-foot">{footer}</div>}
      </div>
    </div>
  );
}

export function Callout({ tone = 'info', children }: { tone?: 'info' | 'warning' | 'danger' | 'success'; children: ReactNode }) {
  const Icon = tone === 'danger' ? XCircle : tone === 'warning' ? AlertTriangle : tone === 'success' ? CheckCircle2 : Info;
  return (
    <div className={cx('callout', tone)} role={tone === 'danger' ? 'alert' : undefined}>
      <Icon size={16} />
      <div>{children}</div>
    </div>
  );
}

export function SaveIcon({ save, size = 48 }: { save: Pick<Save, 'icon' | 'name'>; size?: number }) {
  return (
    <img
      className="save-icon"
      src={save.icon ?? '/mark.svg'}
      alt=""
      width={size}
      height={size}
      style={{ width: size, height: size, borderRadius: Math.round(size * 0.24) }}
      draggable={false}
    />
  );
}

export const STATUS_LABEL: Record<Save['status'], string> = {
  not_installed: 'Not installed',
  ready: 'Ready',
  downloading: 'Downloading',
  launching: 'Launching',
  running: 'Running',
  error: 'Error',
};

export function StatusBadge({ save }: { save: Save }) {
  const tone = { not_installed: '', ready: 'success', downloading: 'info', launching: 'info', running: 'accent', error: 'danger' }[save.status];
  const busy = save.status === 'downloading' || save.status === 'launching';
  let label = STATUS_LABEL[save.status];
  if (save.status === 'downloading' && save.phase?.kind === 'downloading' && save.phase.progress?.filesTotal) {
    label = `${label} ${pct(save.phase.progress.bytesTotal ? save.phase.progress.bytesDone : save.phase.progress.filesDone, save.phase.progress.bytesTotal || save.phase.progress.filesTotal)}%`;
  }
  return (
    <span className={cx('badge', tone)}>
      {busy ? <Loader2 size={12} className="spin" /> : <span className={cx('status-dot', save.status === 'running' && 'pulse')} />}
      {label}
    </span>
  );
}

export function ProgressBar({ value, indeterminate, label }: { value: number; indeterminate?: boolean; label: string }) {
  return (
    <div className={cx('progress', indeterminate && 'indeterminate')} role="progressbar" aria-label={label} aria-valuemin={0} aria-valuemax={100} aria-valuenow={indeterminate ? undefined : value}>
      <span style={{ width: `${value}%` }} />
    </div>
  );
}

export function ConfirmDialog({
  title,
  children,
  confirmLabel,
  danger,
  busy,
  onConfirm,
  onCancel,
}: {
  title: string;
  children: ReactNode;
  confirmLabel: string;
  danger?: boolean;
  busy?: boolean;
  onConfirm: () => void;
  onCancel: () => void;
}) {
  return (
    <Modal
      title={title}
      onClose={onCancel}
      footer={
        <>
          <button type="button" className="btn" onClick={onCancel} data-autofocus>
            Cancel
          </button>
          <button type="button" className={cx('btn', danger ? 'btn-danger' : 'btn-primary')} onClick={onConfirm} disabled={busy}>
            {busy && <Loader2 size={15} className="spin" />}
            {confirmLabel}
          </button>
        </>
      }
    >
      {children}
    </Modal>
  );
}
