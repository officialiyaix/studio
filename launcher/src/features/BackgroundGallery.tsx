import { useEffect, useRef, useState } from 'react';
import { ArrowLeft, Check, Images, Loader2 } from 'lucide-react';
import { api, asError } from '../lib/api';
import { useStore } from '../lib/store';
import { cx } from '../lib/format';
import gallery from '../lib/backgrounds.json';

interface GalleryImage {
  id: string;
  width: number;
  height: number;
}
interface GalleryCategory {
  id: string;
  name: string;
  images: GalleryImage[];
}

const CATEGORIES = (gallery as { categories: GalleryCategory[] }).categories;
const thumb = (cat: string, img: string) => `/backgrounds/${cat}--${img}.jpg`;
const plural = (n: number) => `${n} background${n === 1 ? '' : 's'}`;

/**
 * Background themes bundled with Stabled, by category. Each category card shows a blurred peek of two of its
 * images; opening it lists every image. Choosing one makes it the Custom Background (launcher and in game).
 */
export function BackgroundGallery() {
  const { appearance, setAppearanceView, toast } = useStore();
  const [openCat, setOpenCat] = useState<string | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const root = useRef<HTMLDivElement>(null);
  const first = useRef(true);

  // Opening or leaving a category changes the gallery's height; keep its header in view.
  useEffect(() => {
    if (first.current) {
      first.current = false;
      return;
    }
    root.current?.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
  }, [openCat]);

  if (!appearance || CATEGORIES.length === 0) return null;

  const source = appearance.appearance.theme === 'custom' ? appearance.appearance.customBackground?.source : undefined;
  const selectedCat = source?.startsWith('gallery:') ? source.slice(8).split('/')[0] : null;
  const category = CATEGORIES.find((c) => c.id === openCat) ?? null;

  const apply = async (cat: GalleryCategory, img: GalleryImage) => {
    const id = `${cat.id}/${img.id}`;
    setBusy(id);
    try {
      setAppearanceView(await api.appearanceSetGalleryBackground(id));
      toast(`${cat.name} background applied to the launcher and your in-game Stabled menus.`, 'success');
    } catch (e) {
      toast(asError(e).message, 'error');
    } finally {
      setBusy(null);
    }
  };

  return (
    <div className="bg-gallery" ref={root}>
      <div className="bg-gallery-head">
        <div>
          <div className="field-label">Background themes</div>
          <p className="tiny muted">
            {category ? `${category.name} · ${plural(category.images.length)}. Pick one to use it as your background.` : 'Pick a category to see its backgrounds.'}
          </p>
        </div>
        {category && (
          <button type="button" className="btn btn-sm btn-ghost" onClick={() => setOpenCat(null)}>
            <ArrowLeft size={15} /> All categories
          </button>
        )}
      </div>

      {!category ? (
        <div className="bg-cats" key="cats">
          {CATEGORIES.map((c, i) => {
            const peek = c.images.slice(0, 2);
            return (
              <button
                key={c.id}
                type="button"
                className={cx('bg-cat', selectedCat === c.id && 'selected', peek.length === 1 && 'single')}
                style={{ '--i': i } as React.CSSProperties}
                onClick={() => setOpenCat(c.id)}
                aria-label={`${c.name}, ${plural(c.images.length)}`}
              >
                <span className="bg-peek" aria-hidden="true">
                  {peek.map((img, k) => (
                    <img key={img.id} className={`peek-${k}`} src={thumb(c.id, img.id)} alt="" draggable={false} loading="lazy" />
                  ))}
                  <span className="bg-peek-shade" />
                </span>
                <span className="bg-cat-meta">
                  <span className="bg-cat-name">
                    {c.name}
                    {selectedCat === c.id && <Check size={14} />}
                  </span>
                  <span className="bg-cat-count">
                    <Images size={12} /> {c.images.length}
                  </span>
                </span>
              </button>
            );
          })}
        </div>
      ) : (
        <div className="bg-images" key={category.id} role="radiogroup" aria-label={`${category.name} backgrounds`}>
          {category.images.map((img, i) => {
            const id = `${category.id}/${img.id}`;
            const selected = source === `gallery:${id}`;
            return (
              <button
                key={id}
                type="button"
                role="radio"
                aria-checked={selected}
                aria-label={`${category.name} background ${i + 1}`}
                className={cx('bg-image', selected && 'selected')}
                style={{ '--i': i } as React.CSSProperties}
                disabled={busy !== null}
                onClick={() => apply(category, img)}
              >
                <img src={thumb(category.id, img.id)} alt="" draggable={false} loading="lazy" />
                {busy === id && (
                  <span className="bg-image-busy">
                    <Loader2 size={22} className="spin" />
                  </span>
                )}
                {selected && (
                  <span className="bg-image-check">
                    <Check size={14} />
                  </span>
                )}
              </button>
            );
          })}
        </div>
      )}
    </div>
  );
}
