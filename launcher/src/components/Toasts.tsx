import { AlertTriangle, CheckCircle2, Info, X, XCircle } from 'lucide-react';
import { useStore } from '../lib/store';

export function Toasts() {
  const { toasts, dismissToast } = useStore();
  return (
    <div className="toasts" aria-live="polite" aria-relevant="additions">
      {toasts.map((t) => {
        const Icon = t.tone === 'success' ? CheckCircle2 : t.tone === 'error' ? XCircle : t.tone === 'warning' ? AlertTriangle : Info;
        return (
          <div key={t.id} className={`toast ${t.tone}`} role={t.tone === 'error' ? 'alert' : 'status'}>
            <Icon size={17} />
            <div style={{ flex: 1 }}>{t.message}</div>
            <button type="button" className="icon-btn plain sm" aria-label="Dismiss" onClick={() => dismissToast(t.id)} style={{ width: 22, height: 22 }}>
              <X size={14} />
            </button>
          </div>
        );
      })}
    </div>
  );
}
