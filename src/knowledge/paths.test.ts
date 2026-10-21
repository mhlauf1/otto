import { describe, expect, it } from 'vitest';
import { resolveImage } from './paths';

describe('resolveImage', () => {
  it('resolves next to the note, or into the workspace’s assets', () => {
    expect(resolveImage('notes', 'Clients.md', 'logo.png')).toEqual({
      area: 'notes',
      path: 'logo.png',
    });
    expect(resolveImage('notes', 'clients/Stoc.md', '../../assets/stoc%20home.png')).toEqual({
      area: 'assets',
      path: 'stoc home.png',
    });
    expect(resolveImage('notes', 'Clients.md', '../assets/logo.png')).toEqual({
      area: 'assets',
      path: 'logo.png',
    });
  });

  it('refuses web URLs, absolute paths and anything leaving the workspace', () => {
    for (const src of [
      'https://example.com/a.png',
      'file:///etc/passwd',
      '/etc/x.png',
      '../../playbook/notes/x.png',
      '../../../x.png',
      '../assets',
    ]) {
      expect(resolveImage('notes', 'Clients.md', src)).toBeNull();
    }
    expect(resolveImage('global', 'About Mike.md', '../assets/x.png')).toBeNull();
  });
});
