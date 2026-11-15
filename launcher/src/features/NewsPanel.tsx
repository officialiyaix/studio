import { useCallback, useEffect, useState } from 'react';
import DOMPurify from 'dompurify';
import { BookOpen, ExternalLink, Loader2, Newspaper, RefreshCw, WifiOff } from 'lucide-react';
import { api, asError, type NewsItem, type NewsResult } from '../lib/api';
import { useStore } from '../lib/store';
import { cx, formatDate, relativeTime } from '../lib/format';
import { Callout, Modal } from '../components/ui';

/** Independently scrolling news column for Home. */
export function NewsPanel() {
  const { settings } = useStore();
  const [news, setNews] = useState<NewsResult | null>(null);
  const [loading, setLoading] = useState(true);
  const [reading, setReading] = useState<NewsItem | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      setNews(await api.newsGet());
    } catch (e) {
      setNews({ items: [], state: 'failed', fetchedAt: null, error: asError(e).message, feedConfigured: false });
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    load();
  }, [load, settings?.launcher.showMinecraftNews]);

  const remote = news?.items.filter((i) => i.source !== 'guide') ?? [];
  const guides = news?.items.filter((i) => i.source === 'guide') ?? [];

  return (
    <aside className="news panel" aria-label="News">
      <div className="news-head">
        <h2>
          <Newspaper size={17} /> News
        </h2>
        <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
          {news?.state === 'live' && <span className="badge success">Live</span>}
          {news?.state === 'cached' && (
            <span className="badge warning" title={news.error ?? undefined}>
              Cached · {relativeTime(news.fetchedAt)}
            </span>
          )}
          {(news?.state === 'local' || news?.state === 'failed') && (
            <span className="badge" title={news?.error ?? undefined}>
              <WifiOff size={11} /> Offline
            </span>
          )}
          <button type="button" className="icon-btn plain sm" aria-label="Refresh news" onClick={load} disabled={loading}>
            {loading ? <Loader2 size={15} className="spin" /> : <RefreshCw size={15} />}
          </button>
        </div>
      </div>
      <div className="news-list scroll" tabIndex={0} aria-label="News articles">
        {loading && !news && [0, 1, 2].map((i) => <div key={i} className="skeleton news-skel" />)}
        {news?.state === 'failed' && <Callout tone="warning">News couldn't be loaded right now. Showing Stabled guides instead.</Callout>}
        {news && !news.feedConfigured && remote.length === 0 && news.state !== 'failed' && (
          <p className="tiny muted" style={{ padding: '0 2px' }}>
            {settings?.launcher.showMinecraftNews ? 'No live news available.' : 'Minecraft release notes are turned off in Settings.'} Stabled guides are below.
          </p>
        )}
        {remote.map((item) => (
          <NewsCard key={item.id} item={item} onOpen={() => setReading(item)} />
        ))}
        {guides.length > 0 && (
          <div className="section-title" style={{ margin: '6px 2px 0' }}>
            <BookOpen size={13} /> Stabled guides
          </div>
        )}
        {guides.map((item) => (
          <NewsCard key={item.id} item={item} onOpen={() => setReading(item)} />
        ))}
      </div>
      {reading && <NewsReader item={reading} onClose={() => setReading(null)} />}
    </aside>
  );
}

function NewsCard({ item, onOpen }: { item: NewsItem; onOpen: () => void }) {
  const date = formatDate(item.date);
  return (
    <button type="button" className={cx('news-card', item.image && 'has-image')} onClick={onOpen}>
      {item.image && <img src={item.image} alt="" loading="lazy" referrerPolicy="no-referrer" onError={(e) => ((e.target as HTMLImageElement).style.display = 'none')} />}
      <div className="news-card-body">
        <div className="news-meta">
          <span className={cx('badge', item.source === 'minecraft' ? 'success' : item.source === 'guide' ? 'info' : 'accent')}>{item.category}</span>
          {date && <time className="tiny muted" dateTime={item.date ?? undefined}>{date}</time>}
        </div>
        <h3>{item.title}</h3>
        <p className="small muted">{item.summary}</p>
      </div>
    </button>
  );
}

function NewsReader({ item, onClose }: { item: NewsItem; onClose: () => void }) {
  const [html, setHtml] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    api
      .newsDetail(item.id)
      .then((body) => {
        if (item.source === 'minecraft') {
          setHtml(DOMPurify.sanitize(body, { FORBID_TAGS: ['style', 'script', 'iframe', 'form'], FORBID_ATTR: ['style', 'onerror', 'onclick'] }));
        } else {
          const esc = (s: string) => s.replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' })[c]!);
          setHtml(body.split(/\n{2,}/).map((p) => `<p>${esc(p)}</p>`).join(''));
        }
      })
      .catch((e) => setError(asError(e).message));
  }, [item]);

  // Links inside articles open in the system browser, never inside the launcher.
  const onClick = (e: React.MouseEvent) => {
    const a = (e.target as HTMLElement).closest('a');
    if (a) {
      e.preventDefault();
      if (a.href.startsWith('https://')) api.openExternal(a.href).catch(() => undefined);
    }
  };

  return (
    <Modal
      wide
      title={item.title}
      onClose={onClose}
      footer={
        <>
          {item.url && (
            <button type="button" className="btn" onClick={() => api.openExternal(item.url!)}>
              <ExternalLink size={15} /> Open article
            </button>
          )}
          <button type="button" className="btn btn-primary" onClick={onClose}>
            Done
          </button>
        </>
      }
    >
      <div className="news-meta">
        <span className="badge">{item.category}</span>
        {formatDate(item.date) && <span className="tiny muted">{formatDate(item.date)}</span>}
        {item.source === 'minecraft' && <span className="tiny muted">Release notes from Mojang</span>}
      </div>
      {item.image && <img className="reader-image" src={item.image} alt="" referrerPolicy="no-referrer" />}
      {error && <Callout tone="danger">{error}</Callout>}
      {!html && !error && <div className="skeleton" style={{ height: 160 }} />}
      {html && <div className="reader selectable" onClick={onClick} dangerouslySetInnerHTML={{ __html: html }} />}
    </Modal>
  );
}
