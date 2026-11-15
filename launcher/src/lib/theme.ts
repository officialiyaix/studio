import type { AppearanceView, ThemeDef } from './api';

const VAR_NAMES: Record<string, string> = {
  bg: '--bg',
  bgAlt: '--bg-alt',
  surface: '--surface',
  surfaceAlt: '--surface-alt',
  surfaceHover: '--surface-hover',
  border: '--border',
  text: '--text',
  textMuted: '--text-muted',
  accent: '--accent',
  accentAlt: '--accent-alt',
  accentText: '--accent-text',
  cta: '--cta',
  ctaText: '--cta-text',
  success: '--success',
  warning: '--warning',
  danger: '--danger',
  info: '--info',
};

export function themeFor(view: AppearanceView, id = view.appearance.theme): ThemeDef {
  const { themes } = view;
  const wanted = id === 'custom' ? themes.custom.baseTheme : id;
  return themes.themes.find((t) => t.id === wanted) ?? themes.themes[0];
}

/** Applies the global appearance to the whole launcher. Text colours are never made translucent. */
export function applyAppearance(view: AppearanceView, reduceMotion: boolean): void {
  const root = document.documentElement;
  const theme = themeFor(view);
  for (const [key, cssVar] of Object.entries(VAR_NAMES)) {
    root.style.setProperty(cssVar, theme.colors[key as keyof typeof theme.colors]);
  }
  const glass = view.appearance.surface === 'glass';
  // Higher transparency = more background visible: 10% transparency → 90% surface opacity.
  const opacity = glass ? 100 - view.appearance.glassTransparency : 100;
  root.style.setProperty('--surface-pct', `${opacity}%`);
  root.style.setProperty('--glass-blur', glass ? '14px' : '0px');
  root.classList.toggle('glass', glass);
  root.dataset.light = String(theme.light && view.appearance.theme !== 'custom');
  root.dataset.customBg = String(view.appearance.theme === 'custom' && !!view.customBackground);
  root.dataset.reduceMotion = String(reduceMotion);
}
