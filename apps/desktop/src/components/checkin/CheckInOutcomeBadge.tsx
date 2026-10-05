import { useTranslation } from 'react-i18next';
import { Badge, type BadgeProps } from '@/components/ui/badge';
import { cn } from '@/lib/utils';
import {
  checkInOutcomeLabelKey,
  checkInOutcomeTone,
  type CheckInTone,
} from '@/lib/checkInOutcome';

/**
 * Tailwind badge variant per tone, so an outcome reads the same everywhere it
 * is rendered (history, batch results, today's state).
 */
export const CHECK_IN_TONE_BADGE_VARIANT: Record<
  CheckInTone,
  NonNullable<BadgeProps['variant']>
> = {
  success: 'soft-success',
  info: 'soft-primary',
  warning: 'soft-warning',
  error: 'soft-danger',
};

/** Stable class name per tone; used by tests and for tone-specific styling. */
export function checkInToneClassName(tone: CheckInTone): string {
  return `check-in-tone-${tone}`;
}

interface CheckInOutcomeBadgeProps {
  /** Outcome token, or `null` while an attempt is still open. */
  outcome: string | null;
  className?: string;
}

/**
 * Renders an outcome token as translated text with the colour its tone
 * deserves. Unknown tokens fall back to the raw token in the error tone rather
 * than an untranslated i18n key.
 */
export function CheckInOutcomeBadge({ outcome, className }: CheckInOutcomeBadgeProps) {
  const { t } = useTranslation();
  // A missing outcome means the attempt has not finished yet; that is a wait,
  // not a failure.
  const tone: CheckInTone = outcome ? checkInOutcomeTone(outcome) : 'warning';
  const labelKey = checkInOutcomeLabelKey(outcome);
  const label = labelKey ? t(labelKey) : outcome ?? t('checkIn.outcomePending');

  return (
    <Badge
      variant={CHECK_IN_TONE_BADGE_VARIANT[tone]}
      data-tone={tone}
      className={cn('check-in-tone whitespace-nowrap', checkInToneClassName(tone), className)}
    >
      {label}
    </Badge>
  );
}
