import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { AlertCircle, ChevronDown, Loader2, Search } from 'lucide-react';
import { api, asError, type Preset, type VersionList } from '../lib/api';
import { useStore } from '../lib/store';
import { cx, formatDate } from '../lib/format';
import { Callout, Modal, Range, Segmented } from '../components/ui';
import { IconEditor, type IconEditorHandle } from './IconEditor';

const PRESETS: Array<{ value: Preset; label: string }> = [
  { value: 'performance', label: 'Performance' },
  { value: 'balanced', label: 'Balanced' },
  { value: 'quality', label: 'Quality' },
  { value: 'custom', label: 'Custom' },
];

/** Create Save: Name → Minecraft Version → Icon, then optional advanced options. */
export function CreateSaveDialog({ onClose }: { onClose: () => void }) {
  const { settings, upsertSave, select, toast } = useStore();
  const [name, setName] = useState('');
  const [nameError, setNameError] = useState<string | null>(null);
  const [touched, setTouched] = useState(false);
  const [versions, setVersions] = useState<VersionList | null>(null);
  const [version, setVersion] = useState<string | null>(null);
  const [versionError, setVersionError] = useState<string | null>(null);
  const [query, setQuery] = useState('');
  const [showUnsupported, setShowUnsupported] = useState(false);
  const [iconError, setIconError] = useState<string | null>(null);
  const [advanced, setAdvanced] = useState(false);
  const [memory, setMemory] = useState(settings?.saveDefaults.memoryMb ?? 4096);
  const [preset, setPreset] = useState<Preset>(settings?.saveDefaults.performancePreset ?? 'balanced');
  const [busy, setBusy] = useState(false);
  const [formError, setFormError] = useState<string | null>(null);
  const icon = useRef<IconEditorHandle>(null);

  useEffect(() => {
    api.versionsList().then((v) => {
      setVersions(v);
      const preferred = settings?.saveDefaults.minecraftVersion;
      const first = v.versions.find((x) => x.supported && x.id === preferred) ?? v.versions.find((x) => x.supported);
      setVersion(first?.id ?? null);
    });
  }, [settings?.saveDefaults.minecraftVersion]);

  const validateName = useCallback(async (value: string) => {
    try {
      await api.saveValidateName(value);
      setNameError(null);
      return true;
    } catch (e) {
      setNameError(asError(e).message);
      return false;
    }
  }, []);

  useEffect(() => {
    if (!touched) return;
    const t = window.setTimeout(() => validateName(name), 250);
    return () => window.clearTimeout(t);
  }, [name, touched, validateName]);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    const list = versions?.versions ?? [];
    return list.filter((v) => (!q || v.id.toLowerCase().includes(q)) && (v.supported || showUnsupported || q));
  }, [versions, query, showUnsupported]);
  const unsupportedCount = (versions?.versions ?? []).filter((v) => !v.supported).length;
  const selected = versions?.versions.find((v) => v.id === version);

  const submit = async () => {
    setTouched(true);
    setFormError(null);
    const okName = await validateName(name);
    const okVersion = !!version && !!selected?.supported;
    setVersionError(okVersion ? null : 'Choose a Minecraft version supported by the Stabled client.');
    if (!okName || !okVersion || iconError) return;
    setBusy(true);
    try {
      const save = await api.saveCreate({
        name,
        minecraftVersion: version!,
        iconPngBase64: icon.current?.exportPng() ?? null,
        memoryMb: advanced ? memory : null,
        performancePreset: advanced ? preset : null,
      });
      upsertSave(save);
      select(save.id);
      toast(`Created "${save.name}". Launch it to install Minecraft ${save.minecraftVersion}.`, 'success');
      onClose();
    } catch (e) {
      const err = asError(e);
      if (err.code === 'NAME') setNameError(err.message);
      else setFormError(err.message);
    } finally {
      setBusy(false);
    }
  };

  return (
    <Modal
      wide
      title="Create Save"
      onClose={onClose}
      footer={
        <>
          <button type="button" className="btn" onClick={onClose}>
            Cancel
          </button>
          <button type="button" className="btn btn-cta" onClick={submit} disabled={busy}>
            {busy && <Loader2 size={15} className="spin" />} Create Save
          </button>
        </>
      }
    >
      <p className="muted small">
        A Save is its own Minecraft installation — separate mods, settings, worlds, resource packs and screenshots. It isn't a single world: each Save can hold many.
      </p>
      <form
        className="create-form"
        onSubmit={(e) => {
          e.preventDefault();
          submit();
        }}
      >
        {/* 1. Name */}
        <div className="field">
          <label htmlFor="save-name">Name</label>
          <input
            id="save-name"
            className="input"
            data-autofocus
            value={name}
            maxLength={40}
            placeholder="e.g. Survival with friends"
            aria-invalid={!!nameError}
            aria-describedby={nameError ? 'save-name-error' : 'save-name-hint'}
            onChange={(e) => {
              setName(e.target.value);
              setTouched(true);
            }}
            onBlur={() => {
              setTouched(true);
              validateName(name);
            }}
          />
          {nameError ? (
            <span className="error" id="save-name-error">
              <AlertCircle size={14} /> {nameError}
            </span>
          ) : (
            <span className="hint" id="save-name-hint">
              Shown in Stabled and on the in-game menu. You can rename it later without affecting its files.
            </span>
          )}
        </div>

        {/* 2. Minecraft Version */}
        <div className="field">
          <span className="field-label" id="version-label">
            Minecraft Version
          </span>
          <div className="version-picker panel-alt">
            <div className="version-search">
              <Search size={15} />
              <input
                className="input"
                placeholder="Search versions"
                value={query}
                aria-label="Search Minecraft versions"
                onChange={(e) => setQuery(e.target.value)}
              />
            </div>
            <div className="version-list" role="listbox" aria-labelledby="version-label">
              {!versions && <div className="skeleton" style={{ height: 44, margin: 8 }} />}
              {versions && filtered.length === 0 && <p className="muted small" style={{ padding: 12 }}>No versions match "{query}".</p>}
              {filtered.map((v) => (
                <button
                  type="button"
                  key={v.id}
                  role="option"
                  aria-selected={v.id === version}
                  aria-disabled={!v.supported}
                  className={cx('version-option', v.id === version && 'selected', !v.supported && 'unsupported')}
                  onClick={() => {
                    if (!v.supported) return;
                    setVersion(v.id);
                    setVersionError(null);
                  }}
                >
                  <span className="vid">{v.id}</span>
                  {v.supported ? <span className="badge success">Stabled client</span> : <span className="badge">Not supported yet</span>}
                  {v.hasBetaComponents && <span className="badge warning">Beta component</span>}
                  <span className="tiny muted vdate">{formatDate(v.releaseTime) ?? ''}</span>
                </button>
              ))}
            </div>
            {unsupportedCount > 0 && !query && (
              <button type="button" className="btn btn-sm btn-ghost" style={{ margin: 6 }} onClick={() => setShowUnsupported((s) => !s)}>
                {showUnsupported ? 'Hide' : 'Show'} {unsupportedCount} other releases (not supported by the Stabled client yet)
              </button>
            )}
          </div>
          {versionError ? (
            <span className="error">
              <AlertCircle size={14} /> {versionError}
            </span>
          ) : (
            selected && <span className="hint">{selected.loader} · {selected.note}</span>
          )}
          {versions?.source === 'offline' && <span className="hint">Mojang's version list couldn't be reached, so only versions known to this build are shown.</span>}
        </div>

        {/* 3. Icon (optional) */}
        <div className="field">
          <span className="field-label">
            Icon <span className="muted" style={{ fontWeight: 500 }}>(optional)</span>
          </span>
          <IconEditor handle={icon} onError={setIconError} />
          {iconError && (
            <span className="error">
              <AlertCircle size={14} /> {iconError}
            </span>
          )}
        </div>

        <div className="advanced">
          <button type="button" className="btn btn-sm btn-ghost" aria-expanded={advanced} onClick={() => setAdvanced((a) => !a)}>
            <ChevronDown size={15} style={{ transform: advanced ? 'rotate(180deg)' : undefined, transition: 'transform var(--speed)' }} />
            Advanced options
          </button>
          {advanced && (
            <div className="advanced-body panel-alt">
              <div className="field">
                <label htmlFor="mem">Memory for Minecraft: {(memory / 1024).toFixed(1)} GB</label>
                <Range id="mem" label="Memory" min={1024} max={16384} step={512} value={memory} onChange={setMemory} />
                <span className="hint">4 GB suits most Saves. More only helps very large modpacks.</span>
              </div>
              <div className="field">
                <span className="field-label">Performance preset</span>
                <Segmented label="Performance preset" value={preset} onChange={setPreset} options={PRESETS} />
                <span className="hint">Applied on first launch. You can change it in game under Stabled Settings.</span>
              </div>
            </div>
          )}
        </div>
        {formError && <Callout tone="danger">{formError}</Callout>}
      </form>
    </Modal>
  );
}
