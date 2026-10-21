import { describe, expect, it } from 'vitest';
import { ago, sessionLabels } from './labels';
import type { LiveSession } from './registry';

const session = (id: string, title: string) =>
  ({
    info: {
      id,
      title,
      workspaceId: 'lauf',
      agent: 'claude',
      signIn: false,
      projectPath: null,
      branch: null,
    },
  }) as unknown as LiveSession;

describe('sessionLabels', () => {
  it('numbers repeated titles so siblings can be told apart', () => {
    const labels = sessionLabels([
      session('a', 'Claude Code'),
      session('b', 'Shell'),
      session('c', 'Claude Code'),
    ]);
    expect([...labels.values()]).toEqual(['Claude Code', 'Shell', 'Claude Code 2']);
  });
});

describe('ago', () => {
  const now = Date.UTC(2026, 9, 20, 12, 0);
  it('reads like a sidebar timestamp', () => {
    expect(ago(now - 10_000, now)).toBe('now');
    expect(ago(now - 12 * 60_000, now)).toBe('12m');
    expect(ago(now - 3 * 3_600_000, now)).toBe('3h');
    expect(ago(now - 2 * 86_400_000, now)).toBe('2d');
  });
});
