import { describe, expect, it, vi } from 'vitest';
import {
  CHECK_IN_ERROR_CODES,
  checkInFailureLabel,
  checkInOutcomeLabelKey,
  checkInOutcomeTone,
  checkInSourceLabelKey,
  checkInTriggerLabelKey,
  isCheckInErrorCode,
} from './checkInOutcome';

// i18n is mocked globally to return the key, so the assertions are on keys.
const t = vi.fn((key: string) => key);

describe('checkInOutcomeTone', () => {
  it('paints a real success and an already-confirmed day as success', () => {
    expect(checkInOutcomeTone('success')).toBe('success');
    expect(checkInOutcomeTone('already_confirmed')).toBe('success');
  });

  it('paints a guard skip as info, not as a failure', () => {
    expect(checkInOutcomeTone('skipped_guard')).toBe('info');
  });

  it('paints retryable failures, WAF blocks and in-flight attempts as warnings', () => {
    expect(checkInOutcomeTone('failed_retryable')).toBe('warning');
    expect(checkInOutcomeTone('waf_blocked')).toBe('warning');
    expect(checkInOutcomeTone('in_flight')).toBe('warning');
  });

  it('paints permanent failures, cancellations and everything unknown as errors', () => {
    expect(checkInOutcomeTone('failed_permanent')).toBe('error');
    expect(checkInOutcomeTone('cancelled')).toBe('error');
    expect(checkInOutcomeTone('something_new')).toBe('error');
    expect(checkInOutcomeTone(null)).toBe('error');
    expect(checkInOutcomeTone(undefined)).toBe('error');
  });
});

describe('checkInFailureLabel', () => {
  it.each([...CHECK_IN_ERROR_CODES])('maps the %s code onto its i18n key', (code) => {
    expect(checkInFailureLabel(code, t)).toBe(`checkIn.errorCode.${code}`);
  });

  it('returns null for an unknown code so the caller can fall back to the message', () => {
    expect(checkInFailureLabel('guard_blocked', t)).toBeNull();
    expect(checkInFailureLabel('', t)).toBeNull();
    expect(checkInFailureLabel(null, t)).toBeNull();
    expect(checkInFailureLabel(undefined, t)).toBeNull();
  });

  it('never asks i18n for a key it does not have', () => {
    t.mockClear();
    checkInFailureLabel('not_a_code', t);
    checkInFailureLabel(null, t);
    expect(t).not.toHaveBeenCalled();
    expect(checkInFailureLabel('network', t)).toBe('checkIn.errorCode.network');
    expect(t).toHaveBeenCalledWith('checkIn.errorCode.network');
  });

  it('knows exactly the codes the backend can emit', () => {
    for (const code of CHECK_IN_ERROR_CODES) {
      expect(isCheckInErrorCode(code)).toBe(true);
    }
    expect(isCheckInErrorCode('http_status')).toBe(false);
    expect(isCheckInErrorCode(null)).toBe(false);
  });
});

describe('label key helpers', () => {
  it('labels known outcome tokens and rejects unknown ones', () => {
    expect(checkInOutcomeLabelKey('already_confirmed')).toBe('checkIn.outcome.already_confirmed');
    expect(checkInOutcomeLabelKey('failed_permanent')).toBe('checkIn.outcome.failed_permanent');
    expect(checkInOutcomeLabelKey('in_flight')).toBeNull();
    expect(checkInOutcomeLabelKey(null)).toBeNull();
  });

  it('labels known trigger and source tokens and rejects unknown ones', () => {
    expect(checkInTriggerLabelKey('batch')).toBe('checkIn.trigger.batch');
    expect(checkInTriggerLabelKey('cron')).toBeNull();
    expect(checkInSourceLabelKey('balance_delta')).toBe('checkIn.source.balance_delta');
    expect(checkInSourceLabelKey('provider_rule')).toBe('checkIn.source.provider_rule');
    expect(checkInSourceLabelKey('guess')).toBeNull();
    expect(checkInSourceLabelKey(null)).toBeNull();
  });
});
