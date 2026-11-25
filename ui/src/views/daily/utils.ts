import { get_day, get_range_hours, type DayReport, type RangeHours } from '../../app/api';

/**
 * Format seconds into human readable duration, e.g. "8h 0m", "1h 30m", "45s".
 */
export function formatDuration(secs: number): string {
  if (!secs || secs <= 0) return '0m';
  const totalSeconds = Math.round(secs);
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = totalSeconds % 60;

  if (hours > 0) {
    return `${hours}h ${minutes}m`;
  }
  if (minutes > 0) {
    return `${minutes}m`;
  }
  return `${seconds}s`;
}

/**
 * Format seconds into fractional hours string, e.g. "2.5h" or "8.0h".
 */
export function formatHoursDecimal(secs: number): string {
  const hours = secs / 3600;
  return `${hours.toFixed(1)}h`;
}

/**
 * Format YYYY-MM-DD date string to a friendly human readable date, e.g. "Monday, Nov 23, 2026".
 */
export function formatDateDisplay(dateStr: string): string {
  if (!dateStr) return '';
  const [year, month, day] = dateStr.split('-').map(Number);
  if (!year || !month || !day) return dateStr;
  const d = new Date(year, month - 1, day);
  return d.toLocaleDateString(undefined, {
    weekday: 'short',
    year: 'numeric',
    month: 'short',
    day: 'numeric'
  });
}

/**
 * Shift a YYYY-MM-DD date string by a number of days (+1 or -1).
 */
export function shiftDate(dateStr: string, days: number): string {
  const [year, month, day] = dateStr.split('-').map(Number);
  const d = new Date(year, month - 1, day);
  d.setDate(d.getDate() + days);
  return formatDateISO(d);
}

/**
 * Format a Date object to YYYY-MM-DD string.
 */
export function formatDateISO(d: Date): string {
  const year = d.getFullYear();
  const month = String(d.getMonth() + 1).padStart(2, '0');
  const day = String(d.getDate()).padStart(2, '0');
  return `${year}-${month}-${day}`;
}

/**
 * Parse an ISO timestamp to seconds from local midnight (0 to 86400).
 * In mock mode, fixtures use UTC timestamps directly as display times.
 */
export function parseSecondsInDay(isoString: string, reportDate: string): number {
  if (!isoString) return 0;
  if (isMockMode()) {
    const match = isoString.match(/^(\d{4}-\d{2}-\d{2})T(\d{2}):(\d{2}):(\d{2})/);
    if (match) {
      const [, datePart, h, m, s] = match;
      if (datePart === reportDate) {
        return parseInt(h, 10) * 3600 + parseInt(m, 10) * 60 + parseInt(s, 10);
      }
    }
    const d = new Date(isoString);
    const dayStart = new Date(reportDate + 'T00:00:00Z');
    const diff = (d.getTime() - dayStart.getTime()) / 1000;
    return Math.max(0, Math.min(86400, diff));
  }

  const d = new Date(isoString);
  const [y, mo, da] = reportDate.split('-').map(Number);
  const localMidnight = new Date(y, mo - 1, da, 0, 0, 0, 0);
  const diff = (d.getTime() - localMidnight.getTime()) / 1000;
  return Math.max(0, Math.min(86400, diff));
}

/**
 * Format the time portion of an ISO timestamp as HH:MM (local time in live mode).
 */
export function formatTimeOfDay(isoString: string): string {
  if (!isoString) return '';
  if (isMockMode()) {
    const match = isoString.match(/T(\d{2}:\d{2})/);
    if (match) {
      return match[1];
    }
    const d = new Date(isoString);
    const h = String(d.getUTCHours()).padStart(2, '0');
    const m = String(d.getUTCMinutes()).padStart(2, '0');
    return `${h}:${m}`;
  }
  const d = new Date(isoString);
  const h = String(d.getHours()).padStart(2, '0');
  const m = String(d.getMinutes()).padStart(2, '0');
  return `${h}:${m}`;
}

/**
 * Deterministically hash a string to generate a pleasant, accessible color.
 */
export function stringToColor(str: string, isSite: boolean = false): string {
  let hash = 0;
  for (let i = 0; i < str.length; i++) {
    hash = str.charCodeAt(i) + ((hash << 5) - hash);
  }

  if (isSite) {
    // Green / teal / cyan hues (140 - 200)
    const hue = Math.abs(hash % 60) + 140;
    return `hsl(${hue}, 65%, 45%)`;
  } else {
    // Blue / purple / violet hues (210 - 270)
    const hue = Math.abs(hash % 60) + 210;
    return `hsl(${hue}, 70%, 50%)`;
  }
}

/**
 * Checks if mock mode is active.
 */
export function isMockMode(): boolean {
  return (
    import.meta.env.VITE_MOCK === '1' ||
    import.meta.env.VITE_MOCK === 'true' ||
    typeof window === 'undefined' ||
    !(window as any).__TAURI_INTERNALS__
  );
}

/**
 * Fetch day report with support for mock mode empty states.
 */
export async function loadDayReport(date: string): Promise<DayReport> {
  const report = await get_day(date);
  // In mock mode, only 2026-11-23 has fixture activity. Return empty day report for other dates.
  if (isMockMode() && date !== '2026-11-23') {
    return {
      date,
      active_secs: 0,
      timeline: [],
      top_apps: [],
      top_sites: []
    };
  }
  return report;
}

/**
 * Fetch range hours for a given time window (week or month ending on selectedDate).
 */
export async function loadRangeHours(selectedDate: string, daysBack: number): Promise<RangeHours> {
  const to = selectedDate;
  const from = shiftDate(selectedDate, -(daysBack - 1));
  const items = await get_range_hours(from, to);
  return items;
}
