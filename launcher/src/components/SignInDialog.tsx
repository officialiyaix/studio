import { useState } from 'react';
import { ExternalLink, Loader2, ShieldCheck } from 'lucide-react';
import { api, asError } from '../lib/api';
import { useStore } from '../lib/store';
import { Callout, Modal } from './ui';

function MicrosoftLogo() {
  return (
    <svg width="18" height="18" viewBox="0 0 21 21" aria-hidden="true">
      <rect x="1" y="1" width="9" height="9" fill="#f25022" />
      <rect x="11" y="1" width="9" height="9" fill="#7fba00" />
      <rect x="1" y="11" width="9" height="9" fill="#00a4ef" />
      <rect x="11" y="11" width="9" height="9" fill="#ffb900" />
    </svg>
  );
}

/**
 * Centred sign-in popup. Sign-in happens on Microsoft's own page in the system browser; Stabled never asks for a
 * password. Closing it leaves the whole launcher usable except Launch.
 */
export function SignInDialog() {
  const { signIn, closeSignIn, auth, setAuth, toast } = useStore();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  if (!signIn.open) return null;

  const unconfigured = auth?.state === 'unconfigured';
  const noJava = auth?.state === 'no_java';

  const start = async () => {
    setBusy(true);
    setError(null);
    try {
      const s = await api.authSignIn();
      setAuth(s);
      if (s.state === 'signed_in') {
        toast(`Signed in as ${s.profile?.name}.`, 'success');
        closeSignIn();
      }
    } catch (e) {
      const err = asError(e);
      if (err.code !== 'AUTH_CANCELLED') setError(err.message);
    } finally {
      setBusy(false);
    }
  };

  const close = () => {
    if (busy) api.authCancel().catch(() => undefined);
    closeSignIn();
  };

  return (
    <Modal onClose={close} labelledBy="signin-title" title={undefined}>
      <div className="signin">
        <button type="button" className="icon-btn plain sm signin-close" aria-label="Close" onClick={close}>
          ×
        </button>
        <img src="/mark.svg" alt="" width={56} height={56} />
        <h2 id="signin-title">Sign in to Stabled</h2>
        <p className="muted">
          {signIn.reason ?? 'Sign in with your Microsoft account to play Minecraft: Java Edition.'} You can still browse Stabled, create and edit Saves, manage mods, change themes, read news and use the PC Optimiser without signing in.
        </p>
        {unconfigured && (
          <Callout tone="warning">
            Microsoft sign-in isn't configured in this build of Stabled (MICROSOFT_CLIENT_ID is missing), so launching isn't available. Everything else works.
          </Callout>
        )}
        {noJava && auth?.message && <Callout tone="warning">{auth.message}</Callout>}
        {error && <Callout tone="danger">{error}</Callout>}
        {busy ? (
          <div className="signin-wait" role="status">
            <Loader2 size={18} className="spin" />
            <span>Finish signing in on the Microsoft page in your browser…</span>
            <button type="button" className="btn btn-sm" onClick={() => api.authCancel()}>
              Cancel
            </button>
          </div>
        ) : (
          <button type="button" className="btn btn-lg ms-btn" onClick={start} disabled={unconfigured} data-autofocus>
            <MicrosoftLogo />
            {noJava ? 'Use a different Microsoft account' : 'Sign in with Microsoft'}
            <ExternalLink size={15} style={{ opacity: 0.6 }} />
          </button>
        )}
        <p className="tiny muted signin-note">
          <ShieldCheck size={14} /> Opens Microsoft's sign-in page in your browser. Stabled never sees your password
          {auth?.persistent === false ? '; on this platform your session is kept only until Stabled closes.' : ' and keeps your session encrypted with Windows.'}
        </p>
      </div>
    </Modal>
  );
}
