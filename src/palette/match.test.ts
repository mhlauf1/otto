import { describe, expect, it } from 'vitest';
import { score } from './match';

describe('palette matching', () => {
  it('ranks prefix > word start > subsequence', () => {
    const prefix = score('new', 'New Claude session');
    const word = score('claude', 'New Claude session');
    const loose = score('ncs', 'New Claude session');
    expect(prefix).not.toBeNull();
    expect(word).not.toBeNull();
    expect(loose).not.toBeNull();
    expect(prefix ?? 0).toBeGreaterThan(word ?? 0);
    expect(word ?? 0).toBeGreaterThan(loose ?? 0);
  });

  it('rejects non-matches', () => {
    expect(score('xyz', 'Switch to Lauf')).toBeNull();
  });
});
