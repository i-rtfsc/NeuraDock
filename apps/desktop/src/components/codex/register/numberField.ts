export function sanitizeNumericDraft(value: string): string {
  const digitsOnly = value.replace(/\D+/g, '');

  if (digitsOnly === '') {
    return '';
  }

  return digitsOnly.replace(/^0+(?=\d)/, '');
}

export function parseNumericDraft(value: string): number | null {
  if (value === '') {
    return null;
  }

  const parsedValue = Number.parseInt(value, 10);
  return Number.isNaN(parsedValue) ? null : parsedValue;
}

export function clampNumericValue(value: number, min: number, max?: number): number {
  const safeValue = Number.isFinite(value) ? value : min;
  const lowerBoundedValue = Math.max(min, safeValue);

  if (typeof max === 'number') {
    return Math.min(lowerBoundedValue, max);
  }

  return lowerBoundedValue;
}
