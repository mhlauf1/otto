import {
  color,
  font,
  layout,
  motion,
  radius,
  shadow,
  space,
  type,
  workspaceAccent,
  type Accent,
  type Palette,
} from './tokens';

const kebab = (key: string): string =>
  key
    .replace(/\./g, '-')
    .replace(/([a-z0-9])([A-Z])/g, '$1-$2')
    .toLowerCase();

const px = (n: number): string => (n === 0 ? '0' : `${String(n)}px`);

const block = (selector: string, decls: string[], indent = ''): string =>
  `${indent}${selector} {\n${decls.map((d) => `${indent}  ${d}`).join('\n')}\n${indent}}`;

const paletteDecls = (p: Readonly<Record<keyof Palette, string>>): string[] =>
  Object.entries(p).map(([k, v]) => `--otto-color-${kebab(k)}: ${v};`);

const accentDecls = (a: Readonly<Record<keyof Accent, string>>): string[] =>
  Object.entries(a).map(([k, v]) => `--otto-${kebab(k)}: ${v};`);

const shadowDecls = (s: Record<string, string>): string[] =>
  Object.entries(s).map(([k, v]) => `--otto-shadow-${k}: ${v};`);

/** Render every token as CSS custom properties. Pure, so it can be snapshot-tested. */
export function renderTokensCss(): string {
  const root: string[] = [
    ...Object.entries(space).map(([k, v]) => `--otto-space-${kebab(k)}: ${px(v)};`),
    ...Object.entries(radius).map(([k, v]) => `--otto-radius-${k}: ${px(v)};`),
    ...Object.entries(font).map(([k, v]) => `--otto-font-${k}: ${v};`),
    ...Object.entries(type).flatMap(([k, t]) => [
      `--otto-type-${kebab(k)}-size: ${px(t.size)};`,
      `--otto-type-${kebab(k)}-line: ${px(t.line)};`,
      `--otto-type-${kebab(k)}-weight: ${String(t.weight)};`,
      `--otto-type-${kebab(k)}-tracking: ${String(t.tracking)}em;`,
    ]),
    ...Object.entries(motion.duration).map(([k, v]) => `--otto-duration-${k}: ${String(v)}ms;`),
    ...Object.entries(motion.easing).map(([k, v]) => `--otto-easing-${k}: ${v};`),
    ...Object.entries(layout).map(([k, v]) => `--otto-${kebab(k)}: ${px(v)};`),
    'color-scheme: light dark;',
    ...paletteDecls(color.light),
    ...shadowDecls(shadow.light),
  ];

  const accents = (theme: 'light' | 'dark', indent = ''): string[] =>
    Object.entries(workspaceAccent).map(([ws, a]) => {
      const selector = ws === 'neutral' ? ':root' : `[data-workspace='${ws}']`;
      return block(selector, accentDecls(a[theme]), indent);
    });

  return [
    '/* Generated from src/design/tokens.ts by `pnpm tokens`. Do not edit by hand. */',
    '',
    block(':root', root),
    '',
    ...accents('light'),
    '',
    '@media (prefers-color-scheme: dark) {',
    block(':root', [...paletteDecls(color.dark), ...shadowDecls(shadow.dark)], '  '),
    '',
    ...accents('dark', '  '),
    '}',
    '',
    '@media (prefers-reduced-motion: reduce) {',
    block(
      ':root',
      Object.keys(motion.duration).map((k) => `--otto-duration-${k}: 0ms;`),
      '  ',
    ),
    '}',
    '',
  ].join('\n');
}
