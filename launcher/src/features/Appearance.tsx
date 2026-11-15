import { useRef, useState } from 'react';
import { Check, ImagePlus, Loader2, RotateCcw } from 'lucide-react';
import { open } from '@tauri-apps/plugin-dialog';
import { api, asError, type ThemeDef } from '../lib/api';
import { useStore } from '../lib/store';
import { cx } from '../lib/format';
import { Callout, Range, Toggle } from '../components/ui';

function Swatch({ theme, image, dim }: { theme: ThemeDef; image?: string | null; dim?: number }) {
  const c = theme.colors;
  return (
    <div className="swatch" style={{ background: image ? undefined : `linear-gradient(150deg, ${c.bg}, ${c.bgAlt})` }}>
      {image && <img src={image} alt="" />}
      {image && <div className="swatch-dim" style={{ opacity: dim ?? 0.45 }} />}
      <div className="sw-rail" style={{ background: c.surface, borderColor: c.border }}>
        <i style={{ background: c.accent }} />
        <i style={{ background: c.textMuted }} />
        <i style={{ background: c.textMuted }} />
      </div>
      <div className="sw-panel" style={{ background: c.surface, borderColor: c.border }}>
        <b style={{ background: c.text }} />
        <b style={{ background: c.textMuted, width: '55%' }} />
        <span className="sw-btn" style={{ background: c.cta }} />
        <span className="sw-chip" style={{ background: c.accent }} />
      </div>
    </div>
  );
}

/** Exactly nine tiles: eight built-in themes and the single Custom Background slot. */
export function ThemeGrid() {
  const { appearance, updateAppearance, setAppearanceView, toast } = useStore();
  const [busy, setBusy] = useState(false);
  if (!appearance) return null;
  const { themes } = appearance;
  const current = appearance.appearance.theme;
  const base = themes.themes.find((t) => t.id === themes.custom.baseTheme)!;
  const hasImage = !!appearance.customBackground;

  const pickImage = async () => {
    const path = await open({ multiple: false, filters: [{ name: 'Images', extensions: ['png', 'jpg', 'jpeg', 'webp'] }] });
    if (!path || Array.isArray(path)) return;
    setBusy(true);
    try {
      setAppearanceView(await api.appearanceSetBackground(path));
      toast('Custom background set. Stabled saved its own copy of the image.', 'success');
    } catch (e) {
      toast(asError(e).message, 'error');
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="theme-grid" role="radiogroup" aria-label="Theme">
      {themes.themes.map((t) => (
        <button
          key={t.id}
          type="button"
          role="radio"
          aria-checked={current === t.id}
          className={cx('theme-tile', current === t.id && 'selected')}
          onClick={() => updateAppearance({ theme: t.id })}
        >
          <Swatch theme={t} />
          <span className="theme-name">
            {t.name}
            {current === t.id && <Check size={15} />}
          </span>
          <span className="tiny muted theme-desc">{t.description}</span>
        </button>
      ))}
      <button
        type="button"
        role="radio"
        aria-checked={current === 'custom'}
        className={cx('theme-tile', current === 'custom' && 'selected')}
        onClick={() => (hasImage ? updateAppearance({ theme: 'custom' }) : pickImage())}
      >
        {hasImage ? (
          <Swatch theme={base} image={appearance.customBackground} dim={appearance.appearance.customBackground?.dim} />
        ) : (
          <div className="swatch swatch-empty">{busy ? <Loader2 className="spin" size={20} /> : <ImagePlus size={22} />}</div>
        )}
        <span className="theme-name">
          {themes.custom.name}
          {current === 'custom' && <Check size={15} />}
        </span>
        <span className="tiny muted theme-desc">{hasImage ? 'Your image from this computer' : 'Upload an image from this computer'}</span>
      </button>
    </div>
  );
}

export function CustomBackgroundEditor() {
  const { appearance, updateAppearance, setAppearanceView, toast } = useStore();
  const [busy, setBusy] = useState(false);
  const stage = useRef<HTMLDivElement>(null);
  const dragging = useRef(false);
  if (!appearance) return null;
  const bg = appearance.appearance.customBackground;

  const choose = async () => {
    const path = await open({ multiple: false, filters: [{ name: 'Images', extensions: ['png', 'jpg', 'jpeg', 'webp'] }] });
    if (!path || Array.isArray(path)) return;
    setBusy(true);
    try {
      setAppearanceView(await api.appearanceSetBackground(path));
      toast('Background replaced. The Custom Background slot now uses the new image.', 'success');
    } catch (e) {
      toast(asError(e).message, 'error');
    } finally {
      setBusy(false);
    }
  };
  const reset = async () => {
    try {
      setAppearanceView(await api.appearanceResetBackground());
      toast('Custom background removed. Stabled Night is selected.', 'success');
    } catch (e) {
      toast(asError(e).message, 'error');
    }
  };

  const setFocus = (clientX: number, clientY: number, commit: boolean) => {
    const r = stage.current?.getBoundingClientRect();
    if (!r || !bg) return;
    const fx = Math.min(1, Math.max(0, (clientX - r.left) / r.width));
    const fy = Math.min(1, Math.max(0, (clientY - r.top) / r.height));
    const next = { ...appearance, appearance: { ...appearance.appearance, customBackground: { ...bg, focusX: fx, focusY: fy } } };
    setAppearanceView(next);
    if (commit) updateAppearance({ customBackground: { focusX: fx, focusY: fy } });
  };

  return (
    <div className="custom-bg panel-alt">
      <div className="custom-bg-head">
        <div>
          <div className="field-label">Custom Background</div>
          <p className="tiny muted">PNG, JPEG or WebP, at least 320 × 180. Stabled keeps its own copy — one slot, replaced when you choose a new image.</p>
        </div>
        <div style={{ display: 'flex', gap: 8 }}>
          <button type="button" className="btn btn-sm" onClick={choose} disabled={busy}>
            {busy ? <Loader2 size={15} className="spin" /> : <ImagePlus size={15} />} {bg ? 'Replace image' : 'Upload image'}
          </button>
          {bg && (
            <button type="button" className="btn btn-sm btn-ghost" onClick={reset}>
              <RotateCcw size={15} /> Reset
            </button>
          )}
        </div>
      </div>
      {appearance.customBackgroundError && (
        <Callout tone="danger">{appearance.customBackgroundError} The launcher is using Stabled Night colours until then.</Callout>
      )}
      {bg && appearance.customBackground && (
        <div className="custom-bg-edit">
          <div
            ref={stage}
            className="focus-stage"
            role="slider"
            tabIndex={0}
            aria-label="Focal point. Click or drag to choose which part of the image stays visible; arrow keys adjust."
            aria-valuetext={`${Math.round(bg.focusX * 100)}% across, ${Math.round(bg.focusY * 100)}% down`}
            onPointerDown={(e) => {
              dragging.current = true;
              (e.target as HTMLElement).setPointerCapture(e.pointerId);
              setFocus(e.clientX, e.clientY, false);
            }}
            onPointerMove={(e) => dragging.current && setFocus(e.clientX, e.clientY, false)}
            onPointerUp={(e) => {
              dragging.current = false;
              setFocus(e.clientX, e.clientY, true);
            }}
            onKeyDown={(e) => {
              const step = e.shiftKey ? 0.1 : 0.02;
              const d: Record<string, [number, number]> = { ArrowLeft: [-step, 0], ArrowRight: [step, 0], ArrowUp: [0, -step], ArrowDown: [0, step] };
              const m = d[e.key];
              if (!m) return;
              e.preventDefault();
              const fx = Math.min(1, Math.max(0, bg.focusX + m[0]));
              const fy = Math.min(1, Math.max(0, bg.focusY + m[1]));
              updateAppearance({ customBackground: { focusX: fx, focusY: fy } });
            }}
          >
            <img src={appearance.customBackground} alt="" draggable={false} />
            <span className="focus-dot" style={{ left: `${bg.focusX * 100}%`, top: `${bg.focusY * 100}%` }} />
          </div>
          <div className="field">
            <span className="field-label">Dimming {Math.round(bg.dim * 100)}%</span>
            <Range
              label="Background dimming"
              min={0}
              max={85}
              step={5}
              value={Math.round(bg.dim * 100)}
              onChange={(v) => setAppearanceView({ ...appearance, appearance: { ...appearance.appearance, customBackground: { ...bg, dim: v / 100 } } })}
              onCommit={(v) => updateAppearance({ customBackground: { dim: v / 100 } })}
            />
            <span className="hint">Darkens the image behind panels so text stays readable. The focal point keeps the important part in view at any window size.</span>
          </div>
        </div>
      )}
    </div>
  );
}

/** Simple vs Frosted Glass, the 10–90% Transparency slider and live samples directly beneath it. */
export function SurfaceStyle() {
  const { appearance, updateAppearance, setAppearanceView } = useStore();
  const [sampleOn, setSampleOn] = useState(true);
  if (!appearance) return null;
  const a = appearance.appearance;
  const glass = a.surface === 'glass';

  return (
    <div className="surface-style">
      <div className="surface-choices" role="radiogroup" aria-label="Surface style">
        {(['simple', 'glass'] as const).map((s) => (
          <button key={s} type="button" role="radio" aria-checked={a.surface === s} className={cx('surface-choice', a.surface === s && 'selected')} onClick={() => updateAppearance({ surface: s })}>
            <span className={cx('surface-thumb', s)} aria-hidden="true">
              <i />
            </span>
            <span>
              <strong>{s === 'simple' ? 'Simple' : 'Frosted Glass'}</strong>
              <span className="tiny muted" style={{ display: 'block' }}>
                {s === 'simple' ? 'Solid, opaque panels and buttons with minimal effects. Lightest on modest hardware.' : 'Translucent panels and buttons with a restrained blur and subtle outlines.'}
              </span>
            </span>
          </button>
        ))}
      </div>
      {glass ? (
        <div className="glass-controls">
          <div className="field">
            <div style={{ display: 'flex', justifyContent: 'space-between' }}>
              <label className="field-label" htmlFor="transparency">
                Transparency
              </label>
              <output htmlFor="transparency" className="pct-label" aria-live="polite">
                {a.glassTransparency}%
              </output>
            </div>
            <Range
              id="transparency"
              label="Transparency"
              min={10}
              max={90}
              step={1}
              value={a.glassTransparency}
              onChange={(v) => setAppearanceView({ ...appearance, appearance: { ...a, glassTransparency: v } })}
              onCommit={(v) => updateAppearance({ glassTransparency: v })}
            />
            <span className="hint">Higher transparency shows more of the background: 10% is nearly solid, 90% is nearly clear. Text and icons always stay fully opaque.</span>
          </div>
          <div className="live-preview" aria-label="Live preview">
            <div className="lp-panel panel">
              <div className="lp-title">Sample panel</div>
              <p className="tiny muted">Text stays crisp at any transparency.</p>
              <div className="lp-row">
                <button type="button" className="btn btn-primary btn-sm">
                  Primary
                </button>
                <button type="button" className="btn btn-sm">
                  Secondary
                </button>
                <span className="lp-toggle">
                  <Toggle label="Sample toggle" checked={sampleOn} onChange={setSampleOn} /> Toggle
                </span>
              </div>
            </div>
          </div>
        </div>
      ) : (
        <p className="tiny muted">Your Frosted Glass transparency ({a.glassTransparency}%) is kept for when you switch back.</p>
      )}
      <p className="tiny muted">Theme and surface style are global: they apply to this launcher and to the in-game Stabled menus of every Save.</p>
    </div>
  );
}
