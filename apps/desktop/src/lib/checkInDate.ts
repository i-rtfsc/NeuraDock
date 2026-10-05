/**
 * Helpers for check-in day strings.
 *
 * A check-in day is a plain calendar day (`'YYYY-MM-DD'`) with no time zone
 * attached. Parsing it with `new Date('2026-01-02')` treats it as UTC midnight,
 * which shifts the day for users west of UTC; these helpers always work in the
 * user's local time zone instead.
 */

/**
 * Format a Date as the local calendar day, e.g. `'2026-01-02'`.
 * Never uses `toISOString()`, which would convert to UTC first.
 */
export function localDayString(date: Date = new Date()): string {
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, '0');
  const day = String(date.getDate()).padStart(2, '0');
  return `${year}-${month}-${day}`;
}

/**
 * Parse a `'YYYY-MM-DD'` day as local noon, so that no time-zone offset
 * (or DST transition) can push the value into an adjacent day.
 */
export function parseDayString(day: string): Date {
  const [year, month, dayOfMonth] = day.split('-').map(Number);
  return new Date(year, month - 1, dayOfMonth, 12);
}

/** Format a `'YYYY-MM-DD'` day for display using the given locale. */
export function formatDay(day: string, locale: string): string {
  return new Intl.DateTimeFormat(locale, {
    year: 'numeric',
    month: 'long',
    day: 'numeric',
  }).format(parseDayString(day));
}
