import { useTranslation } from 'react-i18next';
import { CalendarClock, CheckCircle2, Loader2, TimerReset } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { formatDay, localDayString } from '@/lib/checkInDate';
import { useCheckIn } from '@/hooks/useCheckIn';
import { useCheckInState } from '@/hooks/useCheckInHistory';

/**
 * Render an RFC3339 instant as "local day + local time".
 *
 * The day part goes through `@/lib/checkInDate` so a late-evening timestamp
 * never rolls over to the wrong calendar day for the user.
 */
function formatUntil(until: string, locale: string): string {
  const date = new Date(until);
  if (Number.isNaN(date.getTime())) {
    return until;
  }
  const day = formatDay(localDayString(date), locale);
  const time = date.toLocaleTimeString(locale, { hour: '2-digit', minute: '2-digit' });
  return `${day} ${time}`;
}

const STATE_BADGE_VARIANT = {
  due: 'soft-primary',
  done: 'soft-success',
  in_flight: 'soft-warning',
  backoff: 'soft-warning',
} as const;

interface CheckInTodayStateCardProps {
  accountId: string;
}

/**
 * "Today" card: the read-model answer to "should this account check in now?".
 *
 * The four states are distinct on purpose — `due` (act), `done` (already
 * satisfied, with how we know), `in_flight` (wait for the open attempt) and
 * `backoff` (wait until a clock time) — instead of guessing from a timestamp.
 */
export function CheckInTodayStateCard({ accountId }: CheckInTodayStateCardProps) {
  const { t, i18n } = useTranslation();
  const checkIn = useCheckIn();
  const { data: state, isLoading, isError } = useCheckInState(accountId);

  const renderBody = () => {
    if (isLoading) {
      return <p className="text-sm text-muted-foreground">{t('common.loading')}</p>;
    }
    if (isError || !state) {
      return <p className="text-sm text-muted-foreground">{t('checkIn.stateLoadFailed')}</p>;
    }

    switch (state.state) {
      case 'due':
        return (
          <div className="flex flex-wrap items-center gap-3">
            <span className="flex items-center gap-2 text-sm text-muted-foreground">
              <CalendarClock className="h-4 w-4 text-info" aria-hidden />
              {t('checkIn.state.due')}
            </span>
            <Button
              size="sm"
              className="shrink-0"
              disabled={checkIn.isPending}
              onClick={() => checkIn.mutate(accountId)}
            >
              {checkIn.isPending ? (
                <Loader2 className="h-4 w-4 animate-spin" aria-hidden />
              ) : (
                <CalendarClock className="h-4 w-4" aria-hidden />
              )}
              {checkIn.isPending ? t('checkIn.checking') : t('checkIn.checkInNow')}
            </Button>
          </div>
        );
      case 'done':
        return (
          <div className="flex flex-wrap items-center gap-3">
            <span className="flex items-center gap-2 text-sm text-muted-foreground">
              <CheckCircle2 className="h-4 w-4 text-success" aria-hidden />
              {t('checkIn.state.done')}
            </span>
            {state.confidence === 'inferred' ? (
              <Badge
                variant="soft-warning"
                title={t('checkIn.state.inferredHint')}
                className="check-in-tone check-in-tone-warning"
              >
                {t('checkIn.state.inferred')}
              </Badge>
            ) : (
              <Badge
                variant={state.confidence === 'manual' ? 'soft-primary' : 'soft-success'}
                className="check-in-tone check-in-tone-success"
              >
                {state.confidence === 'manual'
                  ? t('checkIn.state.manual')
                  : t('checkIn.state.confirmed')}
              </Badge>
            )}
          </div>
        );
      case 'in_flight':
        return (
          <div className="flex items-center gap-2 text-sm text-muted-foreground">
            <Loader2 className="h-4 w-4 animate-spin text-warning" aria-hidden />
            {t('checkIn.state.in_flight')}
          </div>
        );
      case 'backoff':
        return (
          <div className="flex flex-wrap items-center gap-x-3 gap-y-1 text-sm text-muted-foreground">
            <span className="flex items-center gap-2">
              <TimerReset className="h-4 w-4 text-warning" aria-hidden />
              {t('checkIn.state.backoff')}
            </span>
            {state.until && (
              <span>{t('checkIn.state.backoffUntil', { time: formatUntil(state.until, i18n.language) })}</span>
            )}
            {state.retryable_failures > 0 && (
              <span>{t('checkIn.state.retryableFailures', { count: state.retryable_failures })}</span>
            )}
          </div>
        );
      default:
        return (
          <p className="text-sm text-muted-foreground">
            {t('checkIn.stateLoadFailed')}
          </p>
        );
    }
  };

  const stateKey =
    state && state.state in STATE_BADGE_VARIANT
      ? (state.state as keyof typeof STATE_BADGE_VARIANT)
      : null;

  return (
    <Card className="card-vivid group">
      <CardHeader className="pb-2">
        <CardTitle className="flex items-center justify-between gap-2 text-sm font-medium">
          <span className="flex items-center gap-2">
            <CalendarClock className="h-4 w-4 text-primary" aria-hidden />
            {t('checkIn.todayStateTitle')}
          </span>
          {stateKey && (
            <Badge variant={STATE_BADGE_VARIANT[stateKey]} className="whitespace-nowrap">
              {t(`checkIn.state.${stateKey}`)}
            </Badge>
          )}
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-2">
        {renderBody()}
        {state && (
          <p className="text-xs text-muted-foreground">
            {t('checkIn.quotaDay', {
              day: formatDay(state.quota_day, i18n.language),
            })}
          </p>
        )}
      </CardContent>
    </Card>
  );
}
