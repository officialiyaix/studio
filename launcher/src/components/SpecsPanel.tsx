import { useEffect, useRef, useState } from 'react';
import { ChevronUp, Cpu, MemoryStick, Monitor } from 'lucide-react';
import { api, type SystemDynamic, type SystemStatic } from '../lib/api';
import { formatGB } from '../lib/format';

const POLL_MS = 5000;

/** Persistent bottom-left panel with the computer's real hardware. Static data is read once; memory every 5 s. */
export function SpecsPanel() {
  const [info, setInfo] = useState<SystemStatic | null>(null);
  const [dyn, setDyn] = useState<SystemDynamic | null>(null);
  const [failed, setFailed] = useState(false);
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    api.systemStatic().then(setInfo).catch(() => setFailed(true));
    let timer: number | undefined;
    const tick = () => {
      if (document.visibilityState === 'visible') {
        api.systemDynamic().then(setDyn).catch(() => undefined);
      }
      timer = window.setTimeout(tick, POLL_MS);
    };
    tick();
    return () => window.clearTimeout(timer);
  }, []);

  useEffect(() => {
    if (!open) return;
    const close = (e: MouseEvent) => !ref.current?.contains(e.target as Node) && setOpen(false);
    const esc = (e: KeyboardEvent) => e.key === 'Escape' && setOpen(false);
    window.addEventListener('mousedown', close);
    window.addEventListener('keydown', esc);
    return () => {
      window.removeEventListener('mousedown', close);
      window.removeEventListener('keydown', esc);
    };
  }, [open]);

  const unavailable = failed ? 'Unavailable' : 'Reading…';
  const gpu = info?.gpus[0]?.name ?? (info ? 'Not detected' : unavailable);
  const extraGpus = (info?.gpus.length ?? 0) - 1;
  const used = dyn?.memoryUsedBytes ?? null;
  const usable = dyn?.memoryUsableBytes ?? info?.memoryUsableBytes ?? null;
  const usedPct = used != null && usable ? Math.round((used / usable) * 100) : null;

  return (
    <div className="specs panel" ref={ref} aria-label="Computer specifications">
      <div className="spec cpu" title={info?.cpu ?? undefined}>
        <Cpu size={18} />
        <div className="body">
          <span className="k">CPU</span>
          <span className="v">{info ? info.cpu ?? 'Not detected' : unavailable}</span>
        </div>
      </div>
      <div className="spec gpu" title={info?.gpus.map((g) => g.name).join('\n')}>
        <Monitor size={18} />
        <div className="body">
          <span className="k">GPU{extraGpus > 0 ? ` · ${extraGpus + 1} adapters` : ''}</span>
          <span className="v">{gpu}</span>
        </div>
      </div>
      <div className="spec ram">
        <MemoryStick size={18} />
        <div className="body">
          <span className="k">RAM</span>
          <span className="v">
            {info?.memoryInstalledBytes != null ? `${formatGB(info.memoryInstalledBytes, 0)} installed` : unavailable}
          </span>
          {usedPct != null && (
            <span className="mem-bar" title={`${formatGB(used)} of ${formatGB(usable)} in use`}>
              <span style={{ width: `${usedPct}%` }} />
            </span>
          )}
        </div>
      </div>
      <button type="button" className="icon-btn plain sm" aria-expanded={open} aria-label="Show full system details" onClick={() => setOpen((o) => !o)}>
        <ChevronUp size={16} style={{ transform: open ? 'rotate(180deg)' : undefined, transition: 'transform var(--speed)' }} />
      </button>
      {open && (
        <div className="specs-pop panel" role="dialog" aria-label="System details">
          <div className="section-title">This computer</div>
          <dl>
            <dt>Operating system</dt>
            <dd>
              {info ? [info.os.name, info.os.version].filter(Boolean).join(' ') : unavailable}
              {info?.os.build && <span className="muted"> · build {info.os.build}</span>}
              {info && <span className="muted"> · {info.os.architecture}</span>}
            </dd>
            <dt>Processor</dt>
            <dd>
              {info?.cpu ?? 'Not detected'}
              {info?.cpuCores != null && (
                <span className="muted">
                  {' '}
                  · {info.cpuCores} cores{info.cpuThreads ? `, ${info.cpuThreads} threads` : ''}
                </span>
              )}
            </dd>
            <dt>Graphics</dt>
            <dd>
              {info && info.gpus.length > 0
                ? info.gpus.map((g) => (
                    <div key={g.name}>
                      {g.name}
                      {g.dedicatedMemoryBytes != null && <span className="muted"> · {formatGB(g.dedicatedMemoryBytes, 0)} dedicated</span>}
                    </div>
                  ))
                : 'Not detected'}
            </dd>
            <dt>Memory</dt>
            <dd>
              {info?.memoryInstalledBytes != null ? `${formatGB(info.memoryInstalledBytes, 0)} installed` : 'Installed amount not reported'}
              {usable != null && <span className="muted"> · {formatGB(usable)} usable</span>}
              {used != null && usable != null && (
                <div>
                  {formatGB(used)} in use now ({usedPct}%)
                </div>
              )}
            </dd>
          </dl>
          <p className="tiny muted">Read from this PC. Memory in use refreshes every 5 seconds; GB = 1024³ bytes, as Windows shows.</p>
        </div>
      )}
    </div>
  );
}
