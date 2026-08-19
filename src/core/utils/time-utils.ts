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
 * Signature for an injectable translation function. When provided,
 * `formatRelativeTime` produces localized strings through it instead of the
 * hardcoded English fallbacks.
 */
export type RelativeTimeTranslator = (
  key: string,
  params?: Record<string, unknown>
) => string;

/**
 * Format a Date or timestamp into a relative time string (e.g., "5 minutes ago")
 * @param date Date object, timestamp number, or ISO string
 * @param locale Locale string (default: 'en'), used only for the absolute-date fallback
 * @param translate Optional i18n callback; when provided the relative strings are
 *   localized via its keys, otherwise hardcoded English is used (backward-compatible).
 * @returns Formatted relative time string
 */
export function formatRelativeTime(
  date: Date | number | string,
  locale: string = 'en',
  translate?: RelativeTimeTranslator
): string {
  if (!date) return '';

  const d =
    typeof date === 'string' || typeof date === 'number'
      ? new Date(date)
      : date;
  const now = new Date();

  // Convert UTC to local if needed (SQLite uses CURRENT_TIMESTAMP which is UTC)
  // CURRENT_TIMESTAMP in SQLite return format: "YYYY-MM-DD HH:mm:ss"
  let utcDate = d;
  if (typeof date === 'string' && !date.includes('Z') && !date.includes('+')) {
    // Replace space with 'T' and add 'Z' to make it a valid ISO 8601 string in UTC
    const isoString = date.replace(' ', 'T') + 'Z';
    const parsed = new Date(isoString);
    if (!isNaN(parsed.getTime())) {
      utcDate = parsed;
    }
  }

  const localized = (key: string, count: number) =>
    translate ? translate(key, { count }) : '';
  const countOf = (key: string, count: number, fallback: string) => {
    const localizedStr = localized(key, count);
    return localizedStr || fallback;
  };

  const diffInSeconds = Math.floor((now.getTime() - utcDate.getTime()) / 1000);

  if (diffInSeconds < 0) return countOf('time.justNow', 0, 'Just now'); // Future dates (clock skew)
  if (diffInSeconds < 60)
    return countOf('time.secondsAgo', diffInSeconds, `${diffInSeconds}s ago`);

  const diffInMinutes = Math.floor(diffInSeconds / 60);
  if (diffInMinutes < 60)
    return countOf('time.minutesAgo', diffInMinutes, `${diffInMinutes}m ago`);

  const diffInHours = Math.floor(diffInMinutes / 60);
  if (diffInHours < 24)
    return countOf('time.hoursAgo', diffInHours, `${diffInHours}h ago`);

  const diffInDays = Math.floor(diffInHours / 24);
  if (diffInDays < 7)
    return countOf('time.daysAgo', diffInDays, `${diffInDays}d ago`);

  const diffInWeeks = Math.floor(diffInDays / 7);
  if (diffInWeeks < 4)
    return countOf('time.weeksAgo', diffInWeeks, `${diffInWeeks}w ago`);

  // Fallback to absolute date
  return utcDate.toLocaleDateString(locale, {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
  });
}
