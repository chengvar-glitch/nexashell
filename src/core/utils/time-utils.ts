/**
 * Time utility functions
 */

/**
 * Parse a SQLite CURRENT_TIMESTAMP ("YYYY-MM-DD HH:mm:ss", space-separated
 * UTC) — or any Date/ISO string — into a numeric epoch, returning 0 on
 * failure. The space→'T' + 'Z' normalization keeps the parse valid in WebKit
 * (WKWebView), where the raw space-separated value fails to parse.
 */
export function parseDbTimestamp(value?: string | null): number {
  if (!value) return 0;
  const iso =
    value.includes('T') || value.includes('Z') || value.includes('+')
      ? value
      : value.replace(' ', 'T') + 'Z';
  const parsed = new Date(iso);
  return isNaN(parsed.getTime()) ? 0 : parsed.getTime();
}

/**
 * Sort saved sessions: pinned first (most recently pinned on top), then by
 * `updated_at` descending (most recently updated first). `pinned_at` /
 * `updated_at` are SQLite timestamps; missing values sort last.
 */
export function sortSessions<
  T extends {
    updated_at?: string | null;
    is_pinned?: boolean;
    pinned_at?: string | null;
  },
>(sessions: T[]): T[] {
  return [...sessions].sort((a, b) => {
    const pinDiff =
      (b.is_pinned ? parseDbTimestamp(b.pinned_at) : 0) -
      (a.is_pinned ? parseDbTimestamp(a.pinned_at) : 0);
    if (pinDiff !== 0) return pinDiff;
    return parseDbTimestamp(b.updated_at) - parseDbTimestamp(a.updated_at);
  });
}

/**
 * Format a Date or timestamp into a locale-aware relative time string
 * (e.g. "5 minutes ago"), via the platform's Intl.RelativeTimeFormat.
 * Falls back to an absolute date once the diff exceeds ~4 weeks.
 *
 * @param date Date object, timestamp number, or SQLite/ISO string
 * @param locale BCP-47 locale tag (default: 'en')
 */
export function formatRelativeTime(
  date: Date | number | string,
  locale: string = 'en'
): string {
  if (!date) return '';
  const time =
    typeof date === 'string'
      ? parseDbTimestamp(date)
      : date instanceof Date
        ? date.getTime()
        : date;
  if (!time) return '';

  const diffSeconds = Math.floor((Date.now() - time) / 1000);
  const rtf = new Intl.RelativeTimeFormat(locale, { numeric: 'auto' });

  // Future dates (clock skew) read as "now".
  if (diffSeconds < 0) return rtf.format(0, 'second');
  if (diffSeconds < 60) return rtf.format(-diffSeconds, 'second');

  const diffMinutes = Math.floor(diffSeconds / 60);
  if (diffMinutes < 60) return rtf.format(-diffMinutes, 'minute');

  const diffHours = Math.floor(diffMinutes / 60);
  if (diffHours < 24) return rtf.format(-diffHours, 'hour');

  const diffDays = Math.floor(diffHours / 24);
  if (diffDays < 7) return rtf.format(-diffDays, 'day');

  const diffWeeks = Math.floor(diffDays / 7);
  if (diffWeeks < 4) return rtf.format(-diffWeeks, 'week');

  return new Date(time).toLocaleDateString(locale, {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
  });
}
