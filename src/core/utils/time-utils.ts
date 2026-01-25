/**
 * Time utility functions
 */

/**
 * Format a Date or timestamp into a relative time string (e.g., "5 minutes ago")
 * @param date Date object, timestamp number, or ISO string
 * @param locale Locale string (default: 'en')
 * @returns Formatted relative time string
 */
export function formatRelativeTime(
  date: Date | number | string,
  locale: string = 'en'
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

  const diffInSeconds = Math.floor((now.getTime() - utcDate.getTime()) / 1000);

  if (diffInSeconds < 0) return 'Just now'; // Future dates (clock skew)
  if (diffInSeconds < 60) return `${diffInSeconds}s ago`;

  const diffInMinutes = Math.floor(diffInSeconds / 60);
  if (diffInMinutes < 60) return `${diffInMinutes}m ago`;

  const diffInHours = Math.floor(diffInMinutes / 60);
  if (diffInHours < 24) return `${diffInHours}h ago`;

  const diffInDays = Math.floor(diffInHours / 24);
  if (diffInDays < 7) return `${diffInDays}d ago`;

  const diffInWeeks = Math.floor(diffInDays / 7);
  if (diffInWeeks < 4) return `${diffInWeeks}w ago`;

  // Fallback to absolute date
  return utcDate.toLocaleDateString(locale, {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
  });
}
