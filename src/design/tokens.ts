/**
 * Otto design tokens — the single source of truth for every visual value.
 *
 * `pnpm tokens` turns this file into `tokens.css` (CSS custom properties).
 * Components read only `var(--otto-*)`; Stylelint rejects raw colours and spacing.
 * Brand and reference notes live in docs/DESIGN.md.
 */

/** 4px grid. Keys are multiples of 4px: space[4] === 16px. */
export const space = {
  0: 0,
  0.5: 2,
  1: 4,
  1.5: 6,
  2: 8,
  3: 12,
  4: 16,
  5: 20,
  6: 24,
  8: 32,
  10: 40,
  12: 48,
  16: 64,
  20: 80,
} as const;

export const radius = {
  xs: 4,
  sm: 6,
  md: 8,
  lg: 12,
  xl: 16,
  full: 9999,
} as const;

export const font = {
  sans: '-apple-system, BlinkMacSystemFont, "SF Pro Text", system-ui, sans-serif',
  display: '-apple-system, BlinkMacSystemFont, "SF Pro Display", system-ui, sans-serif',
  mono: 'ui-monospace, "SF Mono", SFMono-Regular, Menlo, monospace',
} as const;

interface TypeStyle {
  size: number;
  line: number;
  weight: 400 | 500 | 590 | 600 | 700;
  /** Letter spacing in em. SF tightens as it grows. */
  tracking: number;
}

/** macOS type ramp: body is 13px, as in native Mac apps. */
export const type = {
  caption: { size: 11, line: 14, weight: 500, tracking: 0.006 },
  footnote: { size: 12, line: 16, weight: 400, tracking: 0 },
  body: { size: 13, line: 18, weight: 400, tracking: -0.003 },
  callout: { size: 14, line: 20, weight: 400, tracking: -0.006 },
  headline: { size: 13, line: 18, weight: 590, tracking: -0.003 },
  title3: { size: 15, line: 20, weight: 590, tracking: -0.009 },
  title2: { size: 17, line: 22, weight: 590, tracking: -0.012 },
  title1: { size: 22, line: 28, weight: 600, tracking: -0.018 },
  display: { size: 34, line: 40, weight: 600, tracking: -0.024 },
} as const satisfies Record<string, TypeStyle>;

/** Semantic colours. Both themes are first-class (SPEC §6.1). Neutrals are graphite, never warm. */
export interface Palette {
  window: string;
  sidebar: string;
  surface: string;
  surfaceRaised: string;
  fillHover: string;
  fillSelected: string;
  borderSubtle: string;
  borderStrong: string;
  textPrimary: string;
  textSecondary: string;
  textTertiary: string;
  textOnAccent: string;
  focusRing: string;
  success: string;
  warning: string;
  danger: string;
}

export const color: { light: Palette; dark: Palette } = {
  light: {
    window: '#F5F5F7',
    sidebar: 'rgba(236, 236, 240, 0.72)',
    surface: '#FFFFFF',
    surfaceRaised: '#FFFFFF',
    fillHover: 'rgba(17, 17, 20, 0.05)',
    fillSelected: 'rgba(17, 17, 20, 0.08)',
    borderSubtle: 'rgba(17, 17, 20, 0.08)',
    borderStrong: 'rgba(17, 17, 20, 0.16)',
    textPrimary: '#111114',
    textSecondary: 'rgba(17, 17, 20, 0.62)',
    textTertiary: 'rgba(17, 17, 20, 0.42)',
    textOnAccent: '#FFFFFF',
    focusRing: 'rgba(64, 98, 240, 0.5)',
    success: '#1E9E5A',
    warning: '#B88A00',
    danger: '#D93838',
  },
  dark: {
    window: '#161618',
    sidebar: 'rgba(30, 30, 34, 0.62)',
    surface: '#1C1C1F',
    surfaceRaised: '#242428',
    fillHover: 'rgba(242, 242, 244, 0.06)',
    fillSelected: 'rgba(242, 242, 244, 0.1)',
    borderSubtle: 'rgba(242, 242, 244, 0.08)',
    borderStrong: 'rgba(242, 242, 244, 0.16)',
    textPrimary: '#F2F2F4',
    textSecondary: 'rgba(242, 242, 244, 0.64)',
    textTertiary: 'rgba(242, 242, 244, 0.42)',
    textOnAccent: '#FFFFFF',
    focusRing: 'rgba(108, 134, 255, 0.55)',
    success: '#3CC27A',
    warning: '#E0B53A',
    danger: '#FF5F5A',
  },
};

/** One restrained accent per workspace (SPEC §6.1). Never orange or terracotta. */
export interface Accent {
  accent: string;
  accentSoft: string;
  accentText: string;
}

export const workspaceAccent: Record<
  'neutral' | 'cobalt' | 'jade',
  { light: Accent; dark: Accent }
> = {
  neutral: {
    light: { accent: '#111114', accentSoft: 'rgba(17, 17, 20, 0.08)', accentText: '#111114' },
    dark: { accent: '#F2F2F4', accentSoft: 'rgba(242, 242, 244, 0.1)', accentText: '#F2F2F4' },
  },
  /** Playbook. */
  cobalt: {
    light: { accent: '#4062F0', accentSoft: 'rgba(64, 98, 240, 0.1)', accentText: '#2F4FD6' },
    dark: { accent: '#6C86FF', accentSoft: 'rgba(108, 134, 255, 0.16)', accentText: '#9AADFF' },
  },
  /** Lauf. */
  jade: {
    light: { accent: '#13875F', accentSoft: 'rgba(19, 135, 95, 0.1)', accentText: '#0E7350' },
    dark: { accent: '#34C08F', accentSoft: 'rgba(52, 192, 143, 0.16)', accentText: '#6FD9B2' },
  },
};

export const shadow = {
  light: {
    sm: '0 1px 2px rgba(17, 17, 20, 0.06)',
    md: '0 1px 2px rgba(17, 17, 20, 0.04), 0 4px 16px rgba(17, 17, 20, 0.06)',
    lg: '0 2px 4px rgba(17, 17, 20, 0.04), 0 16px 48px rgba(17, 17, 20, 0.12)',
  },
  dark: {
    sm: '0 1px 2px rgba(0, 0, 0, 0.4)',
    md: '0 1px 2px rgba(0, 0, 0, 0.3), 0 4px 16px rgba(0, 0, 0, 0.35)',
    lg: '0 2px 4px rgba(0, 0, 0, 0.3), 0 16px 48px rgba(0, 0, 0, 0.5)',
  },
} as const;

export const motion = {
  duration: { instant: 80, fast: 140, base: 220, slow: 360 },
  easing: {
    standard: 'cubic-bezier(0.2, 0, 0, 1)',
    enter: 'cubic-bezier(0, 0, 0, 1)',
    exit: 'cubic-bezier(0.3, 0, 1, 1)',
  },
} as const;

export const layout = {
  sidebarWidth: 248,
  inspectorWidth: 320,
  titlebarHeight: 52,
} as const;
