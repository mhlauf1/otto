import type { Area } from '../bindings';

/**
 * Which knowledge file an image in a note points at. `src` is relative to the note, so
 * `../assets/logo.png` from a note reaches the workspace's assets. Anything that would
 * leave the workspace's knowledge, or a web URL, resolves to null (the core refuses
 * escapes too; this just avoids asking).
 */
export function resolveImage(
  area: Area,
  notePath: string,
  src: string,
): { area: Area; path: string } | null {
  if (/^[a-z][a-z0-9+.-]*:/i.test(src) || src.startsWith('/')) return null;
  const parts = notePath.split('/').slice(0, -1);
  for (const part of decodeURIComponent(src).split('/')) {
    if (part === '' || part === '.') continue;
    if (part === '..') {
      if (parts.length === 0) {
        // Stepping out of this area: only into a sibling area of the same workspace.
        parts.push('..');
        continue;
      }
      if (parts.at(-1) === '..') return null;
      parts.pop();
      continue;
    }
    parts.push(part);
  }
  if (parts[0] === '..') {
    const target = parts[1];
    const rest = parts.slice(2).join('/');
    if (area === 'global' || !rest) return null;
    if (target === 'assets' || target === 'notes' || target === 'private') {
      return { area: target, path: rest };
    }
    return null;
  }
  return parts.length > 0 ? { area, path: parts.join('/') } : null;
}
