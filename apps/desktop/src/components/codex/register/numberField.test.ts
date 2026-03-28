import { describe, expect, it } from 'vitest';
import { clampNumericValue, parseNumericDraft, sanitizeNumericDraft } from './numberField';

describe('numberField helpers', () => {
  it('sanitizes numeric drafts and removes forced leading zeroes', () => {
    expect(sanitizeNumericDraft('')).toBe('');
    expect(sanitizeNumericDraft('0010')).toBe('10');
    expect(sanitizeNumericDraft('1a2b')).toBe('12');
    expect(sanitizeNumericDraft('0')).toBe('0');
  });

  it('parses numeric drafts safely', () => {
    expect(parseNumericDraft('')).toBeNull();
    expect(parseNumericDraft('15')).toBe(15);
    expect(parseNumericDraft('0015')).toBe(15);
  });

  it('clamps numeric values into the expected bounds', () => {
    expect(clampNumericValue(0, 1, 100)).toBe(1);
    expect(clampNumericValue(15, 1, 100)).toBe(15);
    expect(clampNumericValue(101, 1, 100)).toBe(100);
    expect(clampNumericValue(Number.NaN, 1, 100)).toBe(1);
  });
});
