/** Icons drawn for Stabled (lucide covers everything else). */

/** Discord's mark, used only to link to the Stabled Discord server. */
export function DiscordGlyph({ size = 20 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" aria-hidden="true" fill="currentColor">
      <path d="M20.32 4.37a19.8 19.8 0 0 0-4.89-1.52.07.07 0 0 0-.08.04c-.21.38-.44.87-.61 1.25a18.27 18.27 0 0 0-5.49 0 12.64 12.64 0 0 0-.62-1.25.08.08 0 0 0-.08-.04 19.74 19.74 0 0 0-4.89 1.52.07.07 0 0 0-.03.03C.53 9.05-.32 13.58.1 18.06a.08.08 0 0 0 .03.06 19.9 19.9 0 0 0 5.99 3.03.08.08 0 0 0 .08-.03c.46-.63.87-1.3 1.23-1.99a.08.08 0 0 0-.04-.11 13.1 13.1 0 0 1-1.87-.89.08.08 0 0 1-.01-.13c.13-.09.25-.19.37-.29a.07.07 0 0 1 .08-.01c3.93 1.79 8.18 1.79 12.06 0a.07.07 0 0 1 .08.01c.12.1.24.2.37.29a.08.08 0 0 1-.01.13c-.6.35-1.22.65-1.87.89a.08.08 0 0 0-.04.11c.36.7.77 1.36 1.23 1.99a.08.08 0 0 0 .08.03 19.84 19.84 0 0 0 6-3.03.08.08 0 0 0 .03-.06c.5-5.18-.84-9.67-3.55-13.66a.06.06 0 0 0-.03-.03ZM8.02 15.33c-1.18 0-2.16-1.08-2.16-2.42 0-1.33.96-2.42 2.16-2.42 1.21 0 2.18 1.1 2.16 2.42 0 1.34-.96 2.42-2.16 2.42Zm7.97 0c-1.18 0-2.15-1.08-2.15-2.42 0-1.33.95-2.42 2.15-2.42 1.21 0 2.18 1.1 2.16 2.42 0 1.34-.94 2.42-2.16 2.42Z" />
    </svg>
  );
}

/**
 * Stabled's "about" glyph: an info mark drawn in the logo's language. The "i" is two slanted bars like the
 * strokes of the Stabled S, inside an open ring whose gap sits where the S's corners point. On hover the
 * ring sweeps closed and the bars lift (see .info-glyph in layout.css).
 */
export function InfoGlyph({ size = 22 }: { size?: number }) {
  return (
    <svg className="info-glyph" width={size} height={size} viewBox="0 0 24 24" aria-hidden="true" fill="none">
      <circle className="info-glyph-ring" cx="12" cy="12" r="9.6" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" pathLength="100" />
      <path className="info-glyph-dot" d="M12.35 6.35h2.55l-0.75 2.2h-2.55Z" fill="currentColor" />
      <path className="info-glyph-stem" d="M11.55 10.05h2.6l-1.75 7.6h-2.6Z" fill="currentColor" />
    </svg>
  );
}
