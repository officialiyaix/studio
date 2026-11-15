import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { ArrowDown, ArrowUp, ChevronDown, Copy, FileText, FolderOpen, Lightbulb, Loader2, Radio, RefreshCw, Search, Skull, X } from 'lucide-react';
import { api, asError, type CrashReport, type LogContent, type LogFile, type Save } from '../lib/api';
import { useStore } from '../lib/store';
import { cx, formatBytes, relativeTime } from '../lib/format';
import { Callout, Segmented } from '../components/ui';
import { CrashFindings } from './CrashExplain';

const LINE_H = 20;
const OVERSCAN = 30;
type Level = 'error' | 'warn' | 'info' | 'debug';
type Filter = 'all' | 'warn' | 'error';

function label(f: LogFile): string {
  const date = f.modified ? new Date(f.modified).toLocaleString(undefined, { day: 'numeric', month: 'short', hour: '2-digit', minute: '2-digit' }) : '';
  switch (f.kind) {
    case 'latest':
      return 'Latest game log';
    case 'launch':
      return 'Console output (last launch)';
    case 'debug':
      return 'Debug log';
    case 'crash':
      return `Crash report · ${date}`;
    case 'jvm-crash':
      return `Java crash · ${date}`;
    default:
      return `Older log · ${f.name.replace(/^logs\//, '').replace(/\.log(\.gz)?$/, '')}`;
  }
}

/** Log level per line; stack-trace lines take the level of the line they belong to. */
function levels(lines: string[]): Level[] {
  const out: Level[] = new Array(lines.length);
  let prev: Level = 'info';
  for (let i = 0; i < lines.length; i++) {
    const l = lines[i];
    let lv: Level | null = null;
    if (/\/(ERROR|FATAL)\]|\[(ERROR|FATAL)\]|^#\s+(EXCEPTION|SIGSEGV)|^---- Minecraft Crash Report/.test(l)) lv = 'error';
    else if (/\/WARN\]|\[WARN(ING)?\]/.test(l)) lv = 'warn';
    else if (/\/DEBUG\]|\[DEBUG\]/.test(l)) lv = 'debug';
    else if (/\/INFO\]|\[INFO\]/.test(l)) lv = 'info';
    else if (/^\s+at |^\s*Caused by:|^\s*\.\.\. \d+ more|^\s*Suppressed:/.test(l) || /^[\w.$]+(Exception|Error)(:|$)/.test(l)) lv = prev === 'info' || prev === 'debug' ? 'error' : prev;
    out[i] = lv ?? prev;
    if (lv) prev = lv;
  }
  return out;
}

export function LogsViewer({ save }: { save: Save }) {
  const { toast } = useStore();
  const [files, setFiles] = useState<LogFile[] | null>(null);
  const [name, setName] = useState<string | null>(null);
  const [content, setContent] = useState<LogContent | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [filter, setFilter] = useState<Filter>('all');
  const [searchOpen, setSearchOpen] = useState(false);
  const [query, setQuery] = useState('');
  const [current, setCurrent] = useState(0);
  const [follow, setFollow] = useState(true);
  const [explain, setExplain] = useState<{ busy: boolean; report: CrashReport | null; error: string | null; open: boolean }>({ busy: false, report: null, error: null, open: false });
  const [scrollTop, setScrollTop] = useState(0);
  const [height, setHeight] = useState(480);
  const [freshFrom, setFreshFrom] = useState<number | null>(null);
  const viewRef = useRef<HTMLDivElement>(null);
  const searchRef = useRef<HTMLInputElement>(null);
  const prevCount = useRef(0);
  const contentRef = useRef<LogContent | null>(null);
  contentRef.current = content;
  const running = save.status === 'running' || save.status === 'launching';
  const file = files?.find((f) => f.name === name) ?? null;
  const live = running && (file?.kind === 'latest' || file?.kind === 'launch');

  const loadFiles = useCallback(async () => {
    try {
      const list = await api.saveLogs(save.id);
      setFiles(list);
      setName((cur) => (cur && list.some((f) => f.name === cur) ? cur : list[0]?.name ?? null));
    } catch (e) {
      setError(asError(e).message);
    }
  }, [save.id]);

  useEffect(() => {
    loadFiles();
  }, [loadFiles, save.status]);

  const loadContent = useCallback(
    async (quiet = false) => {
      if (!name) return;
      if (!quiet) setLoading(true);
      try {
        const c = await api.saveLogRead(save.id, name);
        const old = contentRef.current;
        if (quiet && old && old.name === c.name && c.text === old.text) return;
        if (quiet && old && old.name === c.name && c.text.length > old.text.length) setFreshFrom(prevCount.current);
        setContent(c);
        setError(null);
      } catch (e) {
        setError(asError(e).message);
      } finally {
        setLoading(false);
      }
    },
    [name, save.id],
  );

  useEffect(() => {
    setContent(null);
    setFreshFrom(null);
    setExplain({ busy: false, report: null, error: null, open: false });
    prevCount.current = 0;
    loadContent();
  }, [loadContent]);

  // Live tail while the game runs.
  useEffect(() => {
    if (!live) return;
    const t = window.setInterval(() => loadContent(true), 2000);
    return () => window.clearInterval(t);
  }, [live, loadContent]);

  // Crash files are explained straight away.
  useEffect(() => {
    if (file && (file.kind === 'crash' || file.kind === 'jvm-crash') && content?.name === file.name) runExplain();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [file?.name, content?.name]);

  // Ctrl+F (or Cmd+F) opens the search; Esc closes it.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'f') {
        e.preventDefault();
        setSearchOpen(true);
        window.setTimeout(() => searchRef.current?.select(), 0);
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, []);

  useEffect(() => {
    const el = viewRef.current;
    if (!el) return;
    const ro = new ResizeObserver(() => setHeight(el.clientHeight));
    ro.observe(el);
    setHeight(el.clientHeight);
    return () => ro.disconnect();
  }, [content !== null]);

  const lines = useMemo(() => (content ? content.text.replace(/\r/g, '').split('\n') : []), [content]);
  const lineLevels = useMemo(() => levels(lines), [lines]);
  const visible = useMemo(() => {
    const idx: number[] = [];
    for (let i = 0; i < lines.length; i++) {
      const lv = lineLevels[i];
      if (filter === 'all' || (filter === 'warn' && (lv === 'warn' || lv === 'error')) || (filter === 'error' && lv === 'error')) idx.push(i);
    }
    return idx;
  }, [lines, lineLevels, filter]);
  const counts = useMemo(() => ({ warn: lineLevels.filter((l) => l === 'warn').length, error: lineLevels.filter((l) => l === 'error').length }), [lineLevels]);

  // Search matches: positions in `visible`.
  const q = query.trim().toLowerCase();
  const matches = useMemo(() => {
    if (!q) return [] as number[];
    const m: number[] = [];
    visible.forEach((li, pos) => {
      if (lines[li].toLowerCase().includes(q)) m.push(pos);
    });
    return m;
  }, [q, visible, lines]);

  useEffect(() => setCurrent(0), [q, filter]);

  const scrollToPos = useCallback(
    (pos: number) => {
      const el = viewRef.current;
      if (!el) return;
      const top = pos * LINE_H - el.clientHeight / 2 + LINE_H;
      el.scrollTo({ top: Math.max(0, top), behavior: 'smooth' });
    },
    [],
  );

  useEffect(() => {
    if (matches.length) scrollToPos(matches[Math.min(current, matches.length - 1)]);
  }, [current, matches, scrollToPos]);

  // Follow new lines at the bottom while live.
  useEffect(() => {
    const el = viewRef.current;
    if (!el || !content) return;
    if (prevCount.current === 0 || (live && follow && !q)) el.scrollTop = el.scrollHeight;
    prevCount.current = lines.length;
  }, [content, lines.length, live, follow, q]);

  useEffect(() => {
    if (freshFrom == null) return;
    const t = window.setTimeout(() => setFreshFrom(null), 900);
    return () => window.clearTimeout(t);
  }, [freshFrom]);

  async function runExplain() {
    if (!name) return;
    setExplain({ busy: true, report: null, error: null, open: true });
    try {
      setExplain({ busy: false, report: await api.crashExplain(save.id, name), error: null, open: true });
    } catch (e) {
      setExplain({ busy: false, report: null, error: asError(e).message, open: true });
    }
  }

  const step = (d: number) => {
    if (!matches.length) return;
    setCurrent((c) => (c + d + matches.length) % matches.length);
  };

  const start = Math.max(0, Math.floor(scrollTop / LINE_H) - OVERSCAN);
  const end = Math.min(visible.length, Math.ceil((scrollTop + height) / LINE_H) + OVERSCAN);
  const currentPos = matches.length ? matches[Math.min(current, matches.length - 1)] : -1;
  const gutter = String(lines.length).length;

  const highlight = (text: string, isCurrent: boolean) => {
    if (!q) return text || ' ';
    const lower = text.toLowerCase();
    const parts: React.ReactNode[] = [];
    let i = 0;
    let k = 0;
    while (true) {
      const j = lower.indexOf(q, i);
      if (j < 0) break;
      parts.push(text.slice(i, j));
      parts.push(
        <mark key={k++} className={cx(isCurrent && 'current')}>
          {text.slice(j, j + q.length)}
        </mark>,
      );
      i = j + q.length;
    }
    parts.push(text.slice(i));
    return parts;
  };

  return (
    <div className="logs">
      <div className="logs-toolbar">
        <div className="logs-file">
          <FileText size={15} />
          <select className="input" aria-label="Log file" value={name ?? ''} onChange={(e) => setName(e.target.value)} disabled={!files?.length}>
            {files?.map((f) => (
              <option key={f.name} value={f.name}>
                {label(f)}
              </option>
            ))}
          </select>
          <ChevronDown size={14} className="logs-file-chev" />
        </div>
        <Segmented
          label="Show"
          value={filter}
          onChange={setFilter}
          options={[
            { value: 'all', label: 'All' },
            { value: 'warn', label: `Warnings${counts.warn ? ` · ${counts.warn}` : ''}` },
            { value: 'error', label: `Errors${counts.error ? ` · ${counts.error}` : ''}` },
          ]}
        />
        <div className="logs-actions">
          {live && (
            <button type="button" className={cx('live-pill', follow && 'on')} onClick={() => setFollow((f) => !f)} title={follow ? 'Following new lines — click to stop' : 'Click to follow new lines'}>
              <Radio size={13} /> Live
            </button>
          )}
          <button
            type="button"
            className={cx('icon-btn', searchOpen && 'active')}
            aria-label="Search this log (Ctrl+F)"
            title="Search (Ctrl+F)"
            aria-pressed={searchOpen}
            onClick={() => {
              setSearchOpen((o) => !o);
              window.setTimeout(() => searchRef.current?.focus(), 0);
            }}
          >
            <Search size={16} />
          </button>
          <button type="button" className="btn btn-sm" onClick={runExplain} disabled={!content || explain.busy} title="Explain errors in this file">
            {explain.busy ? <Loader2 size={14} className="spin" /> : <Lightbulb size={14} />} Explain
          </button>
          <button
            type="button"
            className="icon-btn"
            aria-label="Copy log"
            title="Copy"
            disabled={!content}
            onClick={() => content && navigator.clipboard.writeText(content.text).then(() => toast('Copied the log.', 'success'), () => toast("Couldn't copy.", 'error'))}
          >
            <Copy size={16} />
          </button>
          <button type="button" className="icon-btn" aria-label="Open logs folder" title="Open folder" onClick={() => api.saveOpenFolder(save.id, 'logs').catch((e) => toast(asError(e).message, 'error'))}>
            <FolderOpen size={16} />
          </button>
          <button
            type="button"
            className="icon-btn"
            aria-label="Refresh"
            title="Refresh"
            onClick={() => {
              loadFiles();
              loadContent();
            }}
          >
            <RefreshCw size={16} className={cx(loading && 'spin')} />
          </button>
        </div>
      </div>

      <div className={cx('logs-search', searchOpen && 'open')} aria-hidden={!searchOpen}>
        <Search size={15} />
        <input
          ref={searchRef}
          className="input"
          placeholder="Find in log"
          aria-label="Find in log"
          value={query}
          tabIndex={searchOpen ? 0 : -1}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Enter') {
              e.preventDefault();
              step(e.shiftKey ? -1 : 1);
            }
            if (e.key === 'Escape') {
              setSearchOpen(false);
              setQuery('');
            }
          }}
        />
        <span className="tiny muted logs-count" aria-live="polite">
          {q ? (matches.length ? `${Math.min(current, matches.length - 1) + 1} of ${matches.length}` : 'No matches') : ''}
        </span>
        <button type="button" className="icon-btn plain sm" aria-label="Previous match" disabled={!matches.length} tabIndex={searchOpen ? 0 : -1} onClick={() => step(-1)}>
          <ArrowUp size={15} />
        </button>
        <button type="button" className="icon-btn plain sm" aria-label="Next match" disabled={!matches.length} tabIndex={searchOpen ? 0 : -1} onClick={() => step(1)}>
          <ArrowDown size={15} />
        </button>
        <button
          type="button"
          className="icon-btn plain sm"
          aria-label="Close search"
          tabIndex={searchOpen ? 0 : -1}
          onClick={() => {
            setSearchOpen(false);
            setQuery('');
          }}
        >
          <X size={15} />
        </button>
      </div>

      {explain.open && (
        <div className="logs-explain">
          <div className="logs-explain-head">
            <Skull size={15} />
            <strong>Explanation</strong>
            <button type="button" className="icon-btn plain sm" aria-label="Hide explanation" onClick={() => setExplain((x) => ({ ...x, open: false }))} style={{ marginLeft: 'auto' }}>
              <X size={14} />
            </button>
          </div>
          {explain.busy && <div className="skeleton" style={{ height: 90 }} />}
          {explain.error && <Callout tone="danger">{explain.error}</Callout>}
          {explain.report && <CrashFindings report={explain.report} />}
        </div>
      )}

      {error && <Callout tone="danger">{error}</Callout>}
      {files && files.length === 0 && (
        <div className="logs-empty">
          <FileText size={28} />
          <p>No logs yet. They appear here after this Save's first launch.</p>
        </div>
      )}
      {content && (
        <>
          <div className="log-view" ref={viewRef} onScroll={(e) => setScrollTop(e.currentTarget.scrollTop)} tabIndex={0} aria-label={`Contents of ${file ? label(file) : 'log'}`} key={content.name}>
            <div className="log-spacer" style={{ height: Math.max(visible.length, 1) * LINE_H }}>
              <div className="log-lines" style={{ transform: `translateY(${start * LINE_H}px)` }}>
                {visible.slice(start, end).map((li, k) => {
                  const pos = start + k;
                  return (
                    <div key={li} className={cx('log-line', `lv-${lineLevels[li]}`, pos === currentPos && 'match', freshFrom != null && li >= freshFrom && 'fresh')}>
                      <span className="ln" style={{ width: `${gutter + 1}ch` }}>
                        {li + 1}
                      </span>
                      <span className="lt">{highlight(lines[li], pos === currentPos)}</span>
                    </div>
                  );
                })}
              </div>
            </div>
            {visible.length === 0 && <p className="muted small log-none">{filter === 'error' ? 'No errors in this file.' : 'No warnings in this file.'}</p>}
          </div>
          <p className="tiny muted logs-meta">
            {lines.length.toLocaleString()} lines · {formatBytes(content.size)}
            {content.modified && ` · updated ${relativeTime(content.modified)}`}
            {content.truncated && ' · showing the end of a large file'}
            {live && ' · refreshing every 2 seconds'}
          </p>
        </>
      )}
      {!content && loading && <div className="skeleton" style={{ height: 420 }} />}
    </div>
  );
}
