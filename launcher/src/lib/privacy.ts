// Streamer mode: one place that decides what personal details the launcher may show.
// Features that display something personal ask here, so they all follow the same switches.
import { useMemo } from 'react';
import { useStore } from './store';

/** Replaces the user-name folder in paths: C:\Users\Alex\… → C:\Users\•••\…, /home/alex/… → /home/•••/… */
export function redactPath(text: string): string {
  return text.replace(/([\\/](?:Users|home|Documents and Settings)[\\/])[^\\/\s"']+/gi, '$1•••');
}

export interface Privacy {
  /** Streamer mode is on. */
  on: boolean;
  /** The name to show for the signed-in player. */
  name: (real: string | null | undefined) => string | null;
  hideName: boolean;
  hideAvatar: boolean;
  hideSpecs: boolean;
  redactLogs: boolean;
  /** A path (or text containing paths) safe to show. */
  path: (text: string) => string;
}

export function usePrivacy(): Privacy {
  const { settings } = useStore();
  const s = settings?.streamer;
  return useMemo(() => {
    const on = !!s?.enabled;
    const hideName = on && !!s?.hideUsername;
    const hidePaths = on && !!s?.hidePaths;
    return {
      on,
      hideName,
      hideAvatar: on && !!s?.hideAvatar,
      hideSpecs: on && !!s?.hideSpecs,
      redactLogs: on && !!s?.redactLogs,
      name: (real) => (real ? (hideName ? 'Player' : real) : null),
      path: (text) => (hidePaths ? redactPath(text) : text),
    };
  }, [s]);
}
