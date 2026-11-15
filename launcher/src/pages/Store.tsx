import { useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from 'react';
import { BookOpen, Bug, Check, Download, ExternalLink, Heart, Image as ImageIcon, Loader2, Package, Palette, Search, Sparkles, SunMedium, WifiOff } from 'lucide-react';
import { api, asError, type StoreHit, type StoreKind, type StoreProject } from '../lib/api';
import { useStore } from '../lib/store';
import { cx, relativeTime } from '../lib/format';
import { Callout, Modal } from '../components/ui';

const KINDS: Array<{ id: StoreKind; label: string; icon: typeof Package; blurb: string }> = [
  { id: 'mod', label: 'Mods', icon: Package, blurb: 'Fabric mods made for your Save' },
  { id: 'resourcepack', label: 'Resource packs', icon: Palette, blurb: 'Textures, sounds and fonts' },
  { id: 'shader', label: 'Shaders', icon: SunMedium, blurb: 'Lighting and shadows (adds Iris)' },
];

const CATEGORIES: Record<StoreKind, string[]> = {
  mod: ['optimization', 'utility', 'decoration', 'adventure', 'technology', 'magic', 'storage', 'worldgen', 'equipment', 'mobs', 'social', 'transportation', 'game-mechanics', 'management', 'food', 'library'],
  resourcepack: ['16x', '32x', '64x', 'realistic', 'simplistic', 'themed', 'vanilla-like', 'combat', 'decoration', 'audio', 'gui', 'fonts', 'models'],
  shader: ['vanilla-like', 'semi-realistic', 'realistic', 'cartoon', 'fantasy', 'potato', 'low', 'medium', 'high', 'screenshot'],
};

const SORTS = [
  { id: 'relevance', label: 'Best match' },
  { id: 'downloads', label: 'Most downloaded' },
  { id: 'follows', label: 'Most followed' },
  { id: 'updated', label: 'Recently updated' },
  { id: 'newest', label: 'Newest' },
];

const nice = (c: string) => c.replace(/-/g, ' ').replace(/^\w/, (x) => x.toUpperCase());
const compact = (n: number) => new Intl.NumberFormat(undefined, { notation: 'compact', maximumFractionDigits: 1 }).format(n);
const tint = (color: number | null) => (color != null ? `#${color.toString(16).padStart(6, '0')}` : 'var(--accent)');

type InstallState = 'idle' | 'busy' | 'done';

export function StorePage() {
  const { saves, selectedId, toast } = useStore();
  const [kind, setKind] = useState<StoreKind>('mod');
  const [input, setInput] = useState('');
  const [query, setQuery] = useState('');
  const [sort, setSort] = useState('relevance');
  const [category, setCategory] = useState<string | null>(null);
  const [saveId, setSaveId] = useState<string | null>(selectedId ?? saves[0]?.id ?? null);
  const [hits, setHits] = useState<StoreHit[]>([]);
  const [total, setTotal] = useState(0);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [installed, setInstalled] = useState<Set<string>>(new Set());
  const [installing, setInstalling] = useState<Record<string, InstallState>>({});
  const [detail, setDetail] = useState<StoreHit | null>(null);
  const req = useRef(0);
  const sentinel = useRef<HTMLDivElement>(null);
  const save = saves.find((s) => s.id === saveId) ?? null;
  const busySave = save && (save.status === 'running' || save.status === 'downloading' || save.status === 'launching');

  useEffect(() => {
    if (!saveId && saves.length) setSaveId(selectedId ?? saves[0].id);
  }, [saves, saveId, selectedId]);

  // Search as you type, after a short pause.
  useEffect(() => {
    const t = window.setTimeout(() => setQuery(input), 300);
    return () => window.clearTimeout(t);
  }, [input]);

  const fetchPage = useCallback(
    async (offset: number) => {
      const id = ++req.current;
      setLoading(true);
      try {
        const r = await api.storeSearch(kind, query, query && sort === 'relevance' ? 'relevance' : sort === 'relevance' ? 'downloads' : sort, category, saveId, offset);
        if (id !== req.current) return;
        setHits((h) => (offset === 0 ? r.hits : [...h, ...r.hits.filter((x) => !h.some((y) => y.projectId === x.projectId))]));
        setTotal(r.total);
        setError(null);
      } catch (e) {
        if (id === req.current) setError(asError(e).message);
      } finally {
        if (id === req.current) setLoading(false);
      }
    },
    [kind, query, sort, category, saveId],
  );

  useEffect(() => {
    setHits([]);
    fetchPage(0);
  }, [fetchPage]);

  useEffect(() => {
    if (!saveId) return setInstalled(new Set());
    api
      .storeInstalled(saveId, kind)
      .then((ids) => setInstalled(new Set(ids)))
      .catch(() => setInstalled(new Set()));
  }, [saveId, kind]);

  // Infinite scroll.
  useEffect(() => {
    const el = sentinel.current;
    if (!el) return;
    const io = new IntersectionObserver((entries) => {
      if (entries[0].isIntersecting && !loading && hits.length < total && !error) fetchPage(hits.length);
    }, { rootMargin: '400px' });
    io.observe(el);
    return () => io.disconnect();
  }, [fetchPage, hits.length, total, loading, error]);

  const install = async (hit: { projectId: string; title: string }) => {
    if (!saveId || !save) return;
    setInstalling((m) => ({ ...m, [hit.projectId]: 'busy' }));
    try {
      const r = await api.storeInstall(saveId, kind, hit.projectId);
      setInstalling((m) => ({ ...m, [hit.projectId]: 'done' }));
      setInstalled((s) => new Set(s).add(hit.projectId));
      toast(`${r.message} → ${save.name}`, 'success');
    } catch (e) {
      setInstalling((m) => ({ ...m, [hit.projectId]: 'idle' }));
      toast(asError(e).message, 'error');
    }
  };

  const k = KINDS.find((x) => x.id === kind)!;
  const kindIndex = KINDS.findIndex((x) => x.id === kind);

  return (
    <div className="page store scroll">
      <section className="store-hero">
        <div className="store-hero-glow" aria-hidden="true" />
        <div className="store-hero-text">
          <h2>
            <Sparkles size={20} /> Find something new
          </h2>
          <p className="muted small">Thousands of mods, resource packs and shaders from Modrinth. Stabled picks the version made for your Save, checks every download and adds anything it needs.</p>
        </div>
        <label className="store-target">
          <span className="tiny muted">Install to</span>
          <select className="input" value={saveId ?? ''} onChange={(e) => setSaveId(e.target.value || null)} disabled={!saves.length}>
            {!saves.length && <option value="">Create a Save first</option>}
            {saves.map((s) => (
              <option key={s.id} value={s.id}>
                {s.name} · {s.minecraftVersion}
              </option>
            ))}
          </select>
        </label>
      </section>

      <div className="store-kinds" role="tablist" aria-label="What to browse" style={{ ['--k' as string]: kindIndex }}>
        <span className="store-kind-indicator" aria-hidden="true" />
        {KINDS.map((x) => {
          const Icon = x.icon;
          return (
            <button
              key={x.id}
              type="button"
              role="tab"
              aria-selected={kind === x.id}
              className={cx('store-kind', kind === x.id && 'active')}
              onClick={() => {
                setKind(x.id);
                setCategory(null);
              }}
            >
              <Icon size={16} /> {x.label}
            </button>
          );
        })}
      </div>

      <div className="store-controls">
        <div className="store-search">
          <Search size={16} />
          <input className="input" placeholder={`Search ${k.label.toLowerCase()}`} aria-label={`Search ${k.label.toLowerCase()}`} value={input} onChange={(e) => setInput(e.target.value)} />
          {loading && <Loader2 size={15} className="spin" />}
        </div>
        <select className="input store-sort" aria-label="Sort" value={sort} onChange={(e) => setSort(e.target.value)}>
          {SORTS.map((s) => (
            <option key={s.id} value={s.id}>
              {s.label}
            </option>
          ))}
        </select>
      </div>
      <div className="store-cats" role="group" aria-label="Categories">
        <button type="button" className={cx('chip-btn', !category && 'active')} onClick={() => setCategory(null)}>
          All
        </button>
        {CATEGORIES[kind].map((c) => (
          <button key={c} type="button" className={cx('chip-btn', category === c && 'active')} onClick={() => setCategory(category === c ? null : c)}>
            {nice(c)}
          </button>
        ))}
      </div>

      <p className="tiny muted store-count">
        {save ? `${k.blurb} for Minecraft ${save.minecraftVersion}` : k.blurb}
        {total > 0 && ` · ${total.toLocaleString()} found`}
        {busySave && ' · close the game to install into this Save'}
      </p>

      {error && (
        <div className="store-error">
          <WifiOff size={26} />
          <p>{error}</p>
          <button type="button" className="btn btn-sm" onClick={() => fetchPage(0)}>
            Try again
          </button>
        </div>
      )}

      <div className="store-grid" key={`${kind}-${query}-${sort}-${category}-${saveId}`}>
        {hits.map((h, i) => (
          <StoreCard
            key={h.projectId}
            hit={h}
            index={i % 24}
            state={installed.has(h.projectId) ? 'done' : installing[h.projectId] ?? 'idle'}
            canInstall={!!save && !busySave}
            onOpen={() => setDetail(h)}
            onInstall={() => install(h)}
          />
        ))}
        {loading && hits.length === 0 && Array.from({ length: 8 }, (_, i) => <div key={i} className="store-card skeleton-card" style={{ ['--i' as string]: i }} />)}
      </div>
      {!loading && !error && hits.length === 0 && (
        <div className="store-error">
          <Search size={26} />
          <p>Nothing matches. Try other words or another category.</p>
        </div>
      )}
      <div ref={sentinel} className="store-sentinel">
        {loading && hits.length > 0 && <Loader2 size={18} className="spin" />}
      </div>

      {detail && (
        <ProjectDialog
          hit={detail}
          state={installed.has(detail.projectId) ? 'done' : installing[detail.projectId] ?? 'idle'}
          canInstall={!!save && !busySave}
          saveName={save?.name ?? null}
          onInstall={() => install(detail)}
          onClose={() => setDetail(null)}
        />
      )}
    </div>
  );
}

function InstallButton({ state, canInstall, onInstall, big }: { state: InstallState; canInstall: boolean; onInstall: () => void; big?: boolean }) {
  return (
    <button
      type="button"
      className={cx('btn install-btn', big ? 'btn-lg' : 'btn-sm', state === 'done' ? 'installed' : 'btn-cta', state === 'busy' && 'busy')}
      disabled={state !== 'idle' || !canInstall}
      onClick={(e) => {
        e.stopPropagation();
        onInstall();
      }}
    >
      {state === 'busy' ? <Loader2 size={14} className="spin" /> : state === 'done' ? <Check size={14} className="pop" /> : <Download size={14} />}
      {state === 'busy' ? 'Installing' : state === 'done' ? 'Installed' : 'Install'}
    </button>
  );
}

function StoreCard({ hit, index, state, canInstall, onOpen, onInstall }: { hit: StoreHit; index: number; state: InstallState; canInstall: boolean; onOpen: () => void; onInstall: () => void }) {
  return (
    <article
      className="store-card"
      style={{ ['--i' as string]: index, ['--tint' as string]: tint(hit.color) }}
      tabIndex={0}
      role="button"
      aria-label={`${hit.title} by ${hit.author}`}
      onClick={onOpen}
      onKeyDown={(e) => (e.key === 'Enter' || e.key === ' ') && (e.preventDefault(), onOpen())}
    >
      <div className="store-card-banner">
        {hit.imageUrl ? <img src={hit.imageUrl} alt="" loading="lazy" /> : <div className="store-card-banner-fill" />}
      </div>
      <div className="store-card-body">
        <div className="store-card-head">
          <span className="store-icon">{hit.iconUrl ? <img src={hit.iconUrl} alt="" loading="lazy" /> : <Package size={22} />}</span>
          <div style={{ minWidth: 0 }}>
            <h3 className="store-title">{hit.title}</h3>
            <div className="tiny muted">by {hit.author}</div>
          </div>
        </div>
        <p className="small muted store-desc">{hit.description}</p>
        <div className="store-tags">
          {hit.categories.slice(0, 3).map((c) => (
            <span key={c} className="store-tag">
              {nice(c)}
            </span>
          ))}
        </div>
        <div className="store-card-foot">
          <span className="store-stat" title={`${hit.downloads.toLocaleString()} downloads`}>
            <Download size={13} /> {compact(hit.downloads)}
          </span>
          <span className="store-stat" title={`${hit.follows.toLocaleString()} followers`}>
            <Heart size={13} /> {compact(hit.follows)}
          </span>
          <InstallButton state={state} canInstall={canInstall} onInstall={onInstall} />
        </div>
      </div>
    </article>
  );
}

function ProjectDialog({ hit, state, canInstall, saveName, onInstall, onClose }: { hit: StoreHit; state: InstallState; canInstall: boolean; saveName: string | null; onInstall: () => void; onClose: () => void }) {
  const { toast } = useStore();
  const [p, setP] = useState<StoreProject | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [shot, setShot] = useState(0);
  useEffect(() => {
    api.storeProject(hit.projectId).then(setP, (e) => setError(asError(e).message));
  }, [hit.projectId]);
  const open = (url: string | null) => url && api.openExternal(url).catch((e) => toast(asError(e).message, 'error'));
  const images = p?.gallery ?? [];
  return (
    <Modal
      title={hit.title}
      onClose={onClose}
      wide
      footer={
        <>
          <button type="button" className="btn" onClick={() => open(p?.pageUrl ?? null)} disabled={!p}>
            <ExternalLink size={14} /> View on Modrinth
          </button>
          <InstallButton state={state} canInstall={canInstall} onInstall={onInstall} big />
        </>
      }
    >
      <div className="project" style={{ ['--tint' as string]: tint(hit.color) }}>
        <div className="project-head">
          <span className="store-icon lg">{hit.iconUrl ? <img src={hit.iconUrl} alt="" /> : <Package size={30} />}</span>
          <div style={{ minWidth: 0 }}>
            <p className="small">{hit.description}</p>
            <div className="project-stats tiny muted">
              <span>
                <Download size={12} /> {hit.downloads.toLocaleString()} downloads
              </span>
              <span>
                <Heart size={12} /> {hit.follows.toLocaleString()} followers
              </span>
              <span>by {hit.author}</span>
              {p?.updated && <span>updated {relativeTime(p.updated)}</span>}
              {p?.license && <span>{p.license}</span>}
            </div>
            {saveName && <div className="tiny muted">Installs the version made for "{saveName}".</div>}
          </div>
        </div>
        {images.length > 0 && (
          <div className="project-gallery">
            <img key={images[shot].url} src={images[shot].url} alt={images[shot].title ?? ''} className="project-shot" />
            {images.length > 1 && (
              <div className="project-thumbs">
                {images.slice(0, 8).map((g, i) => (
                  <button key={g.url} type="button" className={cx('project-thumb', i === shot && 'active')} onClick={() => setShot(i)} aria-label={`Image ${i + 1}`}>
                    <img src={g.url} alt="" loading="lazy" />
                  </button>
                ))}
              </div>
            )}
          </div>
        )}
        <div className="project-links">
          {p?.sourceUrl && (
            <button type="button" className="chip-btn" onClick={() => open(p.sourceUrl)}>
              <BookOpen size={13} /> Source
            </button>
          )}
          {p?.issuesUrl && (
            <button type="button" className="chip-btn" onClick={() => open(p.issuesUrl)}>
              <Bug size={13} /> Issues
            </button>
          )}
          {p?.wikiUrl && (
            <button type="button" className="chip-btn" onClick={() => open(p.wikiUrl)}>
              <BookOpen size={13} /> Wiki
            </button>
          )}
          {images.length === 0 && p && (
            <span className="tiny muted">
              <ImageIcon size={12} /> No screenshots
            </span>
          )}
        </div>
        {error && <Callout tone="danger">{error}</Callout>}
        {!p && !error && <div className="skeleton" style={{ height: 200 }} />}
        {p && <Markdown text={p.body} />}
      </div>
    </Modal>
  );
}

/** A small, safe Markdown renderer for project descriptions: headings, lists, paragraphs, bold, italics and code.
 *  Images and HTML are dropped; links show their text. */
function Markdown({ text }: { text: string }) {
  const blocks = useMemo(() => {
    const clean = text
      .replace(/<!--[\s\S]*?-->/g, '')
      .replace(/!\[[^\]]*]\([^)]*\)/g, '')
      .replace(/<[^>]+>/g, '')
      .replace(/\r/g, '');
    const out: Array<{ t: 'h' | 'p' | 'li' | 'code'; level?: number; text: string }> = [];
    let para: string[] = [];
    let code: string[] | null = null;
    const flush = () => {
      if (para.join(' ').trim()) out.push({ t: 'p', text: para.join(' ').trim() });
      para = [];
    };
    for (const raw of clean.split('\n')) {
      const line = raw.trimEnd();
      if (line.trim().startsWith('```')) {
        if (code) {
          out.push({ t: 'code', text: code.join('\n') });
          code = null;
        } else {
          flush();
          code = [];
        }
        continue;
      }
      if (code) {
        code.push(raw);
        continue;
      }
      const h = /^(#{1,6})\s+(.*)$/.exec(line.trim());
      const li = /^\s*(?:[-*+]|\d+[.)])\s+(.*)$/.exec(line);
      if (h) {
        flush();
        out.push({ t: 'h', level: h[1].length, text: h[2] });
      } else if (li) {
        flush();
        out.push({ t: 'li', text: li[1] });
      } else if (!line.trim()) {
        flush();
      } else if (!/^\s*(-{3,}|\*{3,}|_{3,})\s*$/.test(line)) {
        para.push(line.trim());
      }
    }
    flush();
    return out.slice(0, 400);
  }, [text]);

  const inline = (s: string): ReactNode[] => {
    const parts: ReactNode[] = [];
    const re = /(\*\*[^*]+\*\*|__[^_]+__|`[^`]+`|\*[^*]+\*|_[^_]+_|\[[^\]]+]\([^)]+\))/g;
    let last = 0;
    let m: RegExpExecArray | null;
    let k = 0;
    while ((m = re.exec(s))) {
      parts.push(s.slice(last, m.index));
      const tok = m[0];
      if (tok.startsWith('**') || tok.startsWith('__')) parts.push(<strong key={k++}>{tok.slice(2, -2)}</strong>);
      else if (tok.startsWith('`')) parts.push(<code key={k++}>{tok.slice(1, -1)}</code>);
      else if (tok.startsWith('[')) parts.push(<span key={k++} className="md-link">{tok.slice(1, tok.indexOf(']'))}</span>);
      else parts.push(<em key={k++}>{tok.slice(1, -1)}</em>);
      last = m.index + tok.length;
    }
    parts.push(s.slice(last));
    return parts;
  };

  return (
    <div className="markdown">
      {blocks.map((b, i) =>
        b.t === 'h' ? (
          <div key={i} className={`md-h md-h${Math.min(b.level ?? 2, 4)}`}>
            {inline(b.text)}
          </div>
        ) : b.t === 'li' ? (
          <div key={i} className="md-li">
            {inline(b.text)}
          </div>
        ) : b.t === 'code' ? (
          <pre key={i} className="md-code">
            {b.text}
          </pre>
        ) : (
          <p key={i}>{inline(b.text)}</p>
        ),
      )}
    </div>
  );
}
