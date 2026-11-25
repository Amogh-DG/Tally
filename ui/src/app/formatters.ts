/**
 * Formats a duration in seconds to a concise string e.g. "1h 30m", "45m", "12s"
 */
export function formatDuration(seconds: number): string {
  if (!seconds || seconds <= 0) return '0s';
  const total = Math.round(seconds);
  const hours = Math.floor(total / 3600);
  const minutes = Math.floor((total % 3600) / 60);
  const secs = total % 60;

  const parts: string[] = [];
  if (hours > 0) parts.push(`${hours}h`);
  if (minutes > 0) parts.push(`${minutes}m`);
  if (secs > 0 && hours === 0) parts.push(`${secs}s`);

  return parts.length > 0 ? parts.join(' ') : '0s';
}

/**
 * Formats duration in full words e.g. "1 hr 30 mins 15 secs"
 */
export function formatDurationLong(seconds: number): string {
  if (!seconds || seconds <= 0) return '0 seconds';
  const total = Math.round(seconds);
  const hours = Math.floor(total / 3600);
  const minutes = Math.floor((total % 3600) / 60);
  const secs = total % 60;

  const parts: string[] = [];
  if (hours > 0) parts.push(`${hours} ${hours === 1 ? 'hr' : 'hrs'}`);
  if (minutes > 0) parts.push(`${minutes} ${minutes === 1 ? 'min' : 'mins'}`);
  if (secs > 0 || parts.length === 0) parts.push(`${secs} ${secs === 1 ? 'sec' : 'secs'}`);

  return parts.join(' ');
}

/**
 * Formats percentage with one decimal place or integer
 */
export function formatPercent(value: number, total: number): number {
  if (!total || total <= 0) return 0;
  const pct = (value / total) * 100;
  return Math.min(100, Math.max(0, Math.round(pct * 10) / 10));
}
