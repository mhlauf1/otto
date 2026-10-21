import type { LiveSession } from './registry';

/** Numbers repeats ("Claude Code", "Claude Code 2") so sibling sessions can be told apart. */
export function sessionLabels(sessions: LiveSession[]): Map<string, string> {
  const seen = new Map<string, number>();
  const labels = new Map<string, string>();
  for (const s of sessions) {
    const n = (seen.get(s.info.title) ?? 0) + 1;
    seen.set(s.info.title, n);
    labels.set(s.info.id, n === 1 ? s.info.title : `${s.info.title} ${String(n)}`);
  }
  return labels;
}

/** "now", "12m", "3h", "2d", then a short date. */
export function ago(ms: number, now = Date.now()): string {
  const minutes = Math.floor((now - ms) / 60_000);
  if (minutes < 1) return 'now';
  if (minutes < 60) return `${String(minutes)}m`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${String(hours)}h`;
  const days = Math.floor(hours / 24);
  if (days < 7) return `${String(days)}d`;
  return new Date(ms).toLocaleDateString(undefined, { month: 'short', day: 'numeric' });
}
