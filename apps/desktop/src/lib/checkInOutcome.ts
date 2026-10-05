/**
 * Presentation helpers for check-in outcomes and failure reason codes.
 *
 * The backend reports what happened with stable tokens (`outcome`) and stable
 * reason codes (`error_code`) instead of formatted prose, so the UI can
 * translate them. A provider sentence is a fallback, never the primary text.
 */

/** Semantic colour bucket a check-in outcome should be rendered with. */
export type CheckInTone = 'success' | 'info' | 'warning' | 'error';

/** Minimal translation signature, so these helpers stay testable. */
export type TranslateFn = (key: string) => string;

/**
 * Every reason code the check-in pipeline can attach to a failed attempt.
 * Mirrors `check_in_codes` in the Rust HTTP client.
 */
export const CHECK_IN_ERROR_CODES = [
  'not_supported',
  'already_checked_in',
  'invalid_response',
  'WAF_CHALLENGE',
  'http_status_client',
  'http_status_server',
  'network',
  'timeout',
  'cancelled',
] as const;

export type CheckInErrorCode = (typeof CHECK_IN_ERROR_CODES)[number];

/** Every outcome token the backend can report. */
export const CHECK_IN_OUTCOMES = [
  'success',
  'already_confirmed',
  'failed_retryable',
  'failed_permanent',
  'waf_blocked',
  'skipped_guard',
  'cancelled',
] as const;

export type CheckInOutcomeToken = (typeof CHECK_IN_OUTCOMES)[number];

/**
 * Outcome -> tone.
 *
 * `in_flight` is not a persisted outcome; it is the state the read model
 * reports while an attempt is open, and it maps onto the same "wait" bucket as
 * a retryable failure.
 */
const OUTCOME_TONES: Record<string, CheckInTone> = {
  success: 'success',
  already_confirmed: 'success',
  skipped_guard: 'info',
  failed_retryable: 'warning',
  waf_blocked: 'warning',
  in_flight: 'warning',
};

/**
 * Tone for an outcome token. Anything unknown — including a missing outcome —
 * is treated as an error, so an unrecognised token is never painted green.
 */
export function checkInOutcomeTone(outcome: string | null | undefined): CheckInTone {
  if (!outcome) {
    return 'error';
  }
  return OUTCOME_TONES[outcome] ?? 'error';
}

const ERROR_CODE_SET = new Set<string>(CHECK_IN_ERROR_CODES);
const OUTCOME_SET = new Set<string>(CHECK_IN_OUTCOMES);

const TRIGGER_LABEL_KEYS: Record<string, string> = {
  manual: 'checkIn.trigger.manual',
  batch: 'checkIn.trigger.batch',
  scheduler: 'checkIn.trigger.scheduler',
  observe: 'checkIn.trigger.observe',
};

const SOURCE_LABEL_KEYS: Record<string, string> = {
  provider_response: 'checkIn.source.provider_response',
  provider_rule: 'checkIn.source.provider_rule',
  confirmed_unknown: 'checkIn.source.confirmed_unknown',
  legacy_estimate: 'checkIn.source.legacy_estimate',
  manual: 'checkIn.source.manual',
  balance_delta: 'checkIn.source.balance_delta',
};

export function isCheckInErrorCode(errorCode: string | null | undefined): boolean {
  return !!errorCode && ERROR_CODE_SET.has(errorCode);
}

/**
 * Translate a failure reason code.
 *
 * Returns `null` for a missing or unknown code so the caller can fall back to
 * the backend message instead of rendering an i18n key that does not exist.
 */
export function checkInFailureLabel(
  errorCode: string | null | undefined,
  t: TranslateFn
): string | null {
  if (!isCheckInErrorCode(errorCode)) {
    return null;
  }
  return t(`checkIn.errorCode.${errorCode}`);
}

/** i18n key for an outcome token, or `null` when the token is unknown. */
export function checkInOutcomeLabelKey(outcome: string | null | undefined): string | null {
  if (!outcome || !OUTCOME_SET.has(outcome)) {
    return null;
  }
  return `checkIn.outcome.${outcome}`;
}

/** i18n key for a trigger token (`manual`, `batch`, `scheduler`, `observe`). */
export function checkInTriggerLabelKey(trigger: string | null | undefined): string | null {
  if (!trigger) {
    return null;
  }
  return TRIGGER_LABEL_KEYS[trigger] ?? null;
}

/** i18n key for an evidence source token, or `null` when it is unknown. */
export function checkInSourceLabelKey(source: string | null | undefined): string | null {
  if (!source) {
    return null;
  }
  return SOURCE_LABEL_KEYS[source] ?? null;
}
