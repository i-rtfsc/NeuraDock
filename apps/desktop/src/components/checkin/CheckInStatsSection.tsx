import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { BarChart3 } from 'lucide-react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Progress } from '@/components/ui/progress';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { formatDay } from '@/lib/checkInDate';
import { checkInFailureLabel } from '@/lib/checkInOutcome';
import { useCheckInStats } from '@/hooks/useCheckInHistory';

const PERIOD_OPTIONS = [7, 30, 90] as const;
const DEFAULT_PERIOD = 30;

function formatDuration(durationMs: number | null): string {
  if (durationMs === null || durationMs === undefined) return '—';
  if (durationMs < 1000) return `${durationMs} ms`;
  return `${(durationMs / 1000).toFixed(1)} s`;
}

interface CheckInStatsSectionProps {
  accountId: string;
}

function StatTile({ label, value, hint }: { label: string; value: string; hint?: string }) {
  return (
    <div className="rounded-[var(--radius-control)] border border-border/50 bg-muted/30 px-3 py-2">
      <div className="text-xs text-muted-foreground">{label}</div>
      <div className="mt-0.5 text-lg font-semibold tabular-nums">{value}</div>
      {hint && <div className="text-[11px] text-muted-foreground">{hint}</div>}
    </div>
  );
}

/**
 * Aggregate statistics for one account over a quota-day window.
 *
 * Confirmed and inferred days are shown side by side on purpose: an inferred
 * day is a quota increase that *might* have been a top-up, so it must not be
 * presented with the same confidence as a provider-confirmed check-in.
 */
export function CheckInStatsSection({ accountId }: CheckInStatsSectionProps) {
  const { t, i18n } = useTranslation();
  const [periodDays, setPeriodDays] = useState<number>(DEFAULT_PERIOD);
  const { data, isLoading } = useCheckInStats(accountId, periodDays, !!accountId);

  return (
    <Card className="card-vivid group">
      <CardHeader className="flex flex-row items-center justify-between gap-2 space-y-0">
        <CardTitle className="flex items-center gap-2">
          <BarChart3 className="h-5 w-5" aria-hidden />
          {t('checkIn.statsTitle')}
        </CardTitle>
        <Select
          value={String(periodDays)}
          onValueChange={(value) => setPeriodDays(Number(value))}
        >
          <SelectTrigger className="h-8 w-32 text-xs">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {PERIOD_OPTIONS.map((days) => (
              <SelectItem key={days} value={String(days)}>
                {t('checkIn.statsPeriodDays', { days })}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </CardHeader>
      <CardContent className="space-y-3">
        {isLoading || !data ? (
          <div className="flex h-40 items-center justify-center text-muted-foreground">
            {t('common.loading')}
          </div>
        ) : (
          <>
            <p className="text-xs text-muted-foreground">
              {t('checkIn.statsRange', {
                start: formatDay(data.start_date, i18n.language),
                end: formatDay(data.end_date, i18n.language),
              })}
            </p>

            <div className="space-y-1">
              <div className="flex items-center justify-between text-xs text-muted-foreground">
                <span>{t('checkIn.statsSuccessRate')}</span>
                <span className="font-medium tabular-nums text-foreground">
                  {data.success_rate.toFixed(0)}%
                </span>
              </div>
              <Progress value={data.success_rate} />
            </div>

            <div className="grid grid-cols-2 gap-2 sm:grid-cols-3">
              <StatTile label={t('checkIn.statsAttempts')} value={String(data.attempts)} />
              <StatTile label={t('checkIn.statsSucceeded')} value={String(data.succeeded)} />
              <StatTile label={t('checkIn.statsFailed')} value={String(data.failed)} />
              <StatTile label={t('checkIn.statsSkipped')} value={String(data.skipped)} />
              <StatTile
                label={t('checkIn.statsAverageDuration')}
                value={formatDuration(data.average_duration_ms)}
              />
              <StatTile
                label={t('checkIn.statsTotalReward')}
                value={`$${data.total_reward.toFixed(2)}`}
              />
              <StatTile
                label={t('checkIn.statsConfirmedDays')}
                value={String(data.confirmed_days)}
              />
              <StatTile
                label={t('checkIn.statsInferredDays')}
                value={String(data.inferred_days)}
                hint={t('checkIn.statsInferredHint')}
              />
            </div>

            <div className="space-y-1.5">
              <div className="text-xs font-medium text-muted-foreground">
                {t('checkIn.statsFailuresByCode')}
              </div>
              {data.failures_by_code.length === 0 ? (
                <p className="text-xs text-muted-foreground">{t('checkIn.statsNoFailures')}</p>
              ) : (
                <ul className="space-y-1">
                  {data.failures_by_code.map((failure) => (
                    <li
                      key={failure.error_code}
                      className="flex items-center justify-between gap-2 text-xs"
                    >
                      <span className="min-w-0 break-words text-muted-foreground">
                        {checkInFailureLabel(failure.error_code, t) ?? failure.error_code}
                      </span>
                      <span className="shrink-0 font-medium tabular-nums">{failure.count}</span>
                    </li>
                  ))}
                </ul>
              )}
            </div>
          </>
        )}
      </CardContent>
    </Card>
  );
}
