import { describe, expect, it } from 'vitest';
import { toPastedPrompt } from './paste';

describe('composer', () => {
  it('sends multi-line prompts as a single bracketed paste', () => {
    expect(toPastedPrompt('line one\r\nline two')).toBe('\x1b[200~line one\nline two\x1b[201~');
  });
});
