// @vitest-environment node
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { renderTokensCss } from './renderTokensCss';
import { color, space, workspaceAccent } from './tokens';

describe('design tokens', () => {
  it('tokens.css is up to date with tokens.ts (run `pnpm tokens`)', () => {
    const committed = readFileSync(fileURLToPath(new URL('./tokens.css', import.meta.url)), 'utf8');
    expect(committed).toBe(renderTokensCss());
  });

  it('sits on a 4px grid (half steps allowed for hairline gaps)', () => {
    for (const value of Object.values(space)) {
      expect(value % 2).toBe(0);
    }
  });

  it('light and dark palettes define the same roles', () => {
    expect(Object.keys(color.dark).sort()).toEqual(Object.keys(color.light).sort());
  });

  it('never uses orange or terracotta as an accent (SPEC §6.1)', () => {
    const hue = (hex: string): number => {
      const [r, g, b] = [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16) / 255) as [
        number,
        number,
        number,
      ];
      const max = Math.max(r, g, b);
      const min = Math.min(r, g, b);
      if (max === min) return 0;
      const d = max - min;
      const h = max === r ? ((g - b) / d) % 6 : max === g ? (b - r) / d + 2 : (r - g) / d + 4;
      return (h * 60 + 360) % 360;
    };
    for (const ws of Object.values(workspaceAccent)) {
      for (const theme of [ws.light, ws.dark]) {
        if (theme.accent.startsWith('#')) {
          const h = hue(theme.accent);
          const isNeutral = theme.accent === '#111114' || theme.accent === '#F2F2F4';
          if (!isNeutral) expect(h < 10 || h > 50).toBe(true);
        }
      }
    }
  });
});
