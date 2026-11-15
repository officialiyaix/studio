import { useCallback, useEffect, useImperativeHandle, useRef, useState, type Ref } from 'react';
import { ImagePlus, RotateCcw } from 'lucide-react';
import { open } from '@tauri-apps/plugin-dialog';
import { api, asError } from '../lib/api';
import { Range, Segmented } from '../components/ui';

export interface IconEditorHandle {
  /** PNG (base64, no prefix) of the framed icon, or null when the default Stabled icon is used. */
  exportPng: () => string | null;
}

const OUT = 256;
const VIEW = 132;

/** Upload → validate (backend) → frame with fill/fit, zoom and drag → 256×256 PNG. */
export function IconEditor({ handle, onError, initial }: { handle: Ref<IconEditorHandle>; onError: (m: string | null) => void; initial?: string | null }) {
  const [src, setSrc] = useState<string | null>(initial ?? null);
  const [img, setImg] = useState<HTMLImageElement | null>(null);
  const [mode, setMode] = useState<'fill' | 'fit'>('fill');
  const [zoom, setZoom] = useState(100);
  const [offset, setOffset] = useState({ x: 0, y: 0 });
  const drag = useRef<{ x: number; y: number; ox: number; oy: number } | null>(null);
  const preview = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    if (!src) {
      setImg(null);
      return;
    }
    const i = new Image();
    i.onload = () => setImg(i);
    i.onerror = () => onError('That image could not be displayed.');
    i.src = src;
  }, [src, onError]);

  const draw = useCallback(
    (canvas: HTMLCanvasElement, size: number) => {
      const ctx = canvas.getContext('2d');
      if (!ctx || !img) return;
      ctx.clearRect(0, 0, size, size);
      const base = mode === 'fill' ? Math.max(size / img.width, size / img.height) : Math.min(size / img.width, size / img.height);
      const scale = base * (mode === 'fill' ? zoom / 100 : 1);
      const w = img.width * scale;
      const h = img.height * scale;
      const k = size / VIEW;
      const x = (size - w) / 2 + (mode === 'fill' ? offset.x * k : 0);
      const y = (size - h) / 2 + (mode === 'fill' ? offset.y * k : 0);
      ctx.imageSmoothingQuality = 'high';
      ctx.drawImage(img, x, y, w, h);
    },
    [img, mode, zoom, offset],
  );

  useEffect(() => {
    if (preview.current) {
      preview.current.width = VIEW * 2;
      preview.current.height = VIEW * 2;
      draw(preview.current, VIEW * 2);
    }
  }, [draw]);

  useImperativeHandle(handle, () => ({
    exportPng: () => {
      if (!img) return null;
      const c = document.createElement('canvas');
      c.width = OUT;
      c.height = OUT;
      draw(c, OUT);
      return c.toDataURL('image/png').split(',')[1] ?? null;
    },
  }));

  const choose = async () => {
    onError(null);
    const path = await open({ multiple: false, filters: [{ name: 'Images', extensions: ['png', 'jpg', 'jpeg', 'webp'] }] });
    if (!path || Array.isArray(path)) return;
    try {
      const info = await api.imageInspect(path);
      setSrc(info.dataUrl);
      setZoom(100);
      setOffset({ x: 0, y: 0 });
      setMode('fill');
    } catch (e) {
      onError(asError(e).message);
    }
  };

  return (
    <div className="icon-editor">
      <div
        className="icon-stage"
        onPointerDown={(e) => {
          if (!img || mode !== 'fill') return;
          (e.target as HTMLElement).setPointerCapture(e.pointerId);
          drag.current = { x: e.clientX, y: e.clientY, ox: offset.x, oy: offset.y };
        }}
        onPointerMove={(e) => {
          if (!drag.current) return;
          setOffset({ x: drag.current.ox + e.clientX - drag.current.x, y: drag.current.oy + e.clientY - drag.current.y });
        }}
        onPointerUp={() => (drag.current = null)}
        onKeyDown={(e) => {
          if (!img || mode !== 'fill') return;
          const d = e.shiftKey ? 10 : 2;
          const map: Record<string, [number, number]> = { ArrowLeft: [-d, 0], ArrowRight: [d, 0], ArrowUp: [0, -d], ArrowDown: [0, d] };
          const m = map[e.key];
          if (m) {
            e.preventDefault();
            setOffset((o) => ({ x: o.x + m[0], y: o.y + m[1] }));
          }
        }}
        tabIndex={img && mode === 'fill' ? 0 : -1}
        aria-label={img ? 'Icon preview. Drag or use the arrow keys to reposition.' : 'Default Stabled icon'}
        style={{ cursor: img && mode === 'fill' ? 'grab' : 'default' }}
      >
        {img ? <canvas ref={preview} style={{ width: VIEW, height: VIEW }} /> : <img src="/mark.svg" alt="" width={VIEW} height={VIEW} />}
      </div>
      <div className="icon-controls">
        <div style={{ display: 'flex', gap: 8, flexWrap: 'wrap' }}>
          <button type="button" className="btn btn-sm" onClick={choose}>
            <ImagePlus size={15} /> {img ? 'Choose another image' : 'Upload image'}
          </button>
          {img && (
            <button
              type="button"
              className="btn btn-sm btn-ghost"
              onClick={() => {
                setSrc(null);
                onError(null);
              }}
            >
              <RotateCcw size={15} /> Use default icon
            </button>
          )}
        </div>
        {img ? (
          <>
            <Segmented
              label="Framing"
              value={mode}
              onChange={setMode}
              options={[
                { value: 'fill', label: 'Fill (crop)' },
                { value: 'fit', label: 'Fit whole image' },
              ]}
            />
            {mode === 'fill' && (
              <label className="field" style={{ gap: 2 }}>
                <span className="tiny muted">Zoom {zoom}%</span>
                <Range label="Zoom" min={100} max={300} step={5} value={zoom} onChange={setZoom} />
              </label>
            )}
            <span className="tiny muted">{mode === 'fill' ? 'Drag the preview to choose what stays in frame.' : 'The whole image is shown with transparent edges.'}</span>
          </>
        ) : (
          <span className="tiny muted">PNG, JPEG or WebP. Without an image, the Stabled icon is used. Stabled keeps its own copy, so moving the original later is fine.</span>
        )}
      </div>
    </div>
  );
}
