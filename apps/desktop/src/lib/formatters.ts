/**
 * Format a number as a currency string (USD).
 * @param amount The amount to format.
 * @returns Formatted currency string (e.g., "$1,234.56").
 */
export function formatCurrency(amount: number): string {
  return new Intl.NumberFormat('en-US', {
    style: 'currency',
    currency: 'USD',
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  }).format(amount);
}

/** Relay token quotas are stored in 1/500000 USD units. */
export function formatQuotaUSD(quota: number): string {
  return formatCurrency(quota / 500000);
}

/**
 * Format a date string or Date object to a locale string.
 * @param date The date to format.
 * @returns Formatted date string.
 */
export function formatDate(date: Date | string): string {
  const d = typeof date === 'string' ? new Date(date) : date;
  return d.toLocaleString();
}
