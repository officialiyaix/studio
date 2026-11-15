import { useEffect, useRef, useState } from 'react';
import { AlertCircle, Copy, FolderOpen, ImageIcon, Loader2, MoreHorizontal, Pencil, Trash2 } from 'lucide-react';
import { api, asError, type Save, type SaveSummary } from '../lib/api';
import { useStore } from '../lib/store';
import { formatBytes } from '../lib/format';
import { Callout, ConfirmDialog, Modal } from '../components/ui';
import { IconEditor, type IconEditorHandle } from './IconEditor';

export function RenameDialog({ save, onClose }: { save: Save; onClose: () => void }) {
  const { upsertSave, toast } = useStore();
  const [name, setName] = useState(save.name);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const submit = async () => {
    setBusy(true);
    try {
      upsertSave(await api.saveRename(save.id, name));
      toast('Renamed. Worlds and files are unchanged.', 'success');
      onClose();
    } catch (e) {
      setError(asError(e).message);
    } finally {
      setBusy(false);
    }
  };
  return (
    <Modal
      title="Rename Save"
      onClose={onClose}
      footer={
        <>
          <button className="btn" onClick={onClose}>
            Cancel
          </button>
          <button className="btn btn-primary" onClick={submit} disabled={busy}>
            Rename
          </button>
        </>
      }
    >
      <form
        className="field"
        onSubmit={(e) => {
          e.preventDefault();
          submit();
        }}
      >
        <label htmlFor="rename">Name</label>
        <input id="rename" className="input" data-autofocus value={name} maxLength={40} aria-invalid={!!error} onChange={(e) => setName(e.target.value)} />
        {error ? (
          <span className="error">
            <AlertCircle size={14} /> {error}
          </span>
        ) : (
          <span className="hint">Only the display name changes. The Save's folder, worlds and settings stay where they are.</span>
        )}
      </form>
    </Modal>
  );
}

export function IconDialog({ save, onClose }: { save: Save; onClose: () => void }) {
  const { upsertSave, toast } = useStore();
  const ref = useRef<IconEditorHandle>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const submit = async () => {
    setBusy(true);
    try {
      upsertSave(await api.saveSetIcon(save.id, ref.current?.exportPng() ?? null));
      toast('Icon updated.', 'success');
      onClose();
    } catch (e) {
      setError(asError(e).message);
    } finally {
      setBusy(false);
    }
  };
  return (
    <Modal
      title="Change icon"
      onClose={onClose}
      footer={
        <>
          <button className="btn" onClick={onClose}>
            Cancel
          </button>
          <button className="btn btn-primary" onClick={submit} disabled={busy || !!error}>
            Save icon
          </button>
        </>
      }
    >
      <IconEditor handle={ref} onError={setError} initial={save.icon} />
      {error && <Callout tone="danger">{error}</Callout>}
    </Modal>
  );
}

export function DeleteDialog({ save, onClose }: { save: Save; onClose: () => void }) {
  const { removeSave, toast } = useStore();
  const [summary, setSummary] = useState<SaveSummary | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    api.saveSummary(save.id).then(setSummary).catch(() => undefined);
  }, [save.id]);
  const confirm = async () => {
    setBusy(true);
    try {
      await api.saveDelete(save.id);
      removeSave(save.id);
      toast(`"${save.name}" was moved to the Recycle Bin.`, 'success');
      onClose();
    } catch (e) {
      setError(asError(e).message);
      setBusy(false);
    }
  };
  return (
    <ConfirmDialog title={`Delete "${save.name}"?`} confirmLabel="Move to Recycle Bin" danger busy={busy} onConfirm={confirm} onCancel={onClose}>
      <p>This removes the whole Save from Stabled and moves its folder to the Windows Recycle Bin, including:</p>
      <ul className="delete-list">
        <li>
          <strong>{summary ? summary.worlds : '…'}</strong> world{summary?.worlds === 1 ? '' : 's'} and their progress
        </li>
        <li>
          <strong>{summary ? summary.screenshots : '…'}</strong> screenshot{summary?.screenshots === 1 ? '' : 's'}
        </li>
        <li>
          <strong>{summary ? summary.mods : '…'}</strong> mod file{summary?.mods === 1 ? '' : 's'}, plus its settings, resource packs and shader packs
        </li>
      </ul>
      <p className="muted small">
        {summary ? `${formatBytes(summary.sizeBytes)} in total. ` : ''}You can restore it from the Recycle Bin until it's emptied. Other Saves and shared game downloads are not affected.
      </p>
      {save.status === 'running' && <Callout tone="warning">Close Minecraft first — a running Save can't be deleted.</Callout>}
      {error && <Callout tone="danger">{error}</Callout>}
    </ConfirmDialog>
  );
}

/** "More" menu with rename, icon, duplicate, folder and delete. */
export function SaveMenu({ save }: { save: Save }) {
  const { upsertSave, select, toast } = useStore();
  const [open, setOpen] = useState(false);
  const [dialog, setDialog] = useState<'rename' | 'icon' | 'delete' | null>(null);
  const [dupBusy, setDupBusy] = useState(false);
  const wrap = useRef<HTMLDivElement>(null);
  const busy = save.status === 'running' || save.status === 'downloading' || save.status === 'launching';

  useEffect(() => {
    if (!open) return;
    const close = (e: MouseEvent) => !wrap.current?.contains(e.target as Node) && setOpen(false);
    const esc = (e: KeyboardEvent) => e.key === 'Escape' && setOpen(false);
    window.addEventListener('mousedown', close);
    window.addEventListener('keydown', esc);
    return () => {
      window.removeEventListener('mousedown', close);
      window.removeEventListener('keydown', esc);
    };
  }, [open]);

  const duplicate = async () => {
    setOpen(false);
    setDupBusy(true);
    try {
      const copy = await api.saveDuplicate(save.id);
      upsertSave(copy);
      select(copy.id);
      toast(`Duplicated as "${copy.name}" — worlds, mods and settings were copied.`, 'success');
    } catch (e) {
      toast(asError(e).message, 'error');
    } finally {
      setDupBusy(false);
    }
  };

  return (
    <div ref={wrap} style={{ position: 'relative' }}>
      <button type="button" className="icon-btn" aria-label={`More actions for ${save.name}`} aria-haspopup="menu" aria-expanded={open} onClick={() => setOpen((o) => !o)}>
        {dupBusy ? <Loader2 size={18} className="spin" /> : <MoreHorizontal size={18} />}
      </button>
      {open && (
        <div className="menu" role="menu" style={{ right: 0, top: 44 }}>
          <button role="menuitem" onClick={() => (setOpen(false), setDialog('rename'))}>
            <Pencil size={15} /> Rename
          </button>
          <button role="menuitem" onClick={() => (setOpen(false), setDialog('icon'))}>
            <ImageIcon size={15} /> Change icon
          </button>
          <button role="menuitem" onClick={duplicate} disabled={busy} title={busy ? 'Not available while running' : undefined}>
            <Copy size={15} /> Duplicate
          </button>
          <button role="menuitem" onClick={() => (setOpen(false), api.saveOpenFolder(save.id).catch((e) => toast(asError(e).message, 'error')))}>
            <FolderOpen size={15} /> Open Save folder
          </button>
          <hr />
          <button role="menuitem" className="danger" onClick={() => (setOpen(false), setDialog('delete'))} disabled={busy}>
            <Trash2 size={15} /> Delete…
          </button>
        </div>
      )}
      {dialog === 'rename' && <RenameDialog save={save} onClose={() => setDialog(null)} />}
      {dialog === 'icon' && <IconDialog save={save} onClose={() => setDialog(null)} />}
      {dialog === 'delete' && <DeleteDialog save={save} onClose={() => setDialog(null)} />}
    </div>
  );
}
