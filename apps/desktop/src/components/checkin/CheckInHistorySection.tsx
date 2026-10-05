import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { ChevronLeft, ChevronRight, History } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import { CheckInOutcomeBadge } from './CheckInOutcomeBadge';
import { formatDay, localDayString } from '@/lib/checkInDate';
import {
  checkInFailureLabel,
  checkInSourceLabelKey,
  checkInTriggerLabelKey,
} from '@/lib/checkInOutcome';
import { useCheckInHistory } from '@/hooks/useCheckInHistory';

const PAGE_SIZE = 10;

/** Attempts carry millisecond durations; keep the unit readable at any scale. */
function formatDuration(durationMs: number | null): string {
  if (durationMs === null || durationMs === undefined) return '—';
  if (durationMs < 1000) return `${durationMs} ms`;
  return `${(durationMs / 1000).toFixed(1)} s`;
}

function formatStartedAt(startedAt: string, locale: string): string {
  const date = new Date(startedAt);
  if (Number.isNaN(date.getTime())) return startedAt;
  const day = formatDay(localDayString(date), locale);
  const time = date.toLocaleTimeString(locale, { hour: '2-digit', minute: '2-digit' });
  return `${day} ${time}`;
}

interface CheckInHistorySectionProps {
  accountId: string;
}

/**
 * Attempt history, newest first.
 *
 * Rows are operational records, so a failed or skipped run shows up here even
 * though it produced no evidence. The reason is rendered from `error_code`
 * when we can translate it, and from the code itself otherwise.
 */
export function CheckInHistorySection({ accountId }: CheckInHistorySectionProps) {
  const { t, i18n } = useTranslation();
  const [page, setPage] = useState(1);
  const { data, isLoading, isFetching } = useCheckInHistory(
    accountId,
    page,
    PAGE_SIZE,
    !!accountId
  );

  const totalPages = data ? Math.max(1, Math.ceil(data.total / (data.page_size || PAGE_SIZE))) : 1;
  const attempts = data?.attempts ?? [];

  return (
    <Card className="card-vivid group">
      <CardHeader className="flex flex-row items-center justify-between gap-2 space-y-0">
        <CardTitle className="flex items-center gap-2">
          <History className="h-5 w-5" aria-hidden />
          {t('checkIn.historyTitle')}
        </CardTitle>
        {data && (
          <span className="text-xs text-muted-foreground">
            {t('checkIn.historyTotal', { total: data.total })}
          </span>
        )}
      </CardHeader>
      <CardContent className="space-y-3">
        {isLoading ? (
          <div className="flex h-40 items-center justify-center text-muted-foreground">
            {t('common.loading')}
          </div>
        ) : attempts.length === 0 ? (
          <div className="flex h-40 flex-col items-center justify-center gap-1 text-muted-foreground">
            <History className="h-8 w-8 opacity-40" aria-hidden />
            <p className="text-sm">{t('checkIn.historyEmpty')}</p>
          </div>
        ) : (
          <div className={isFetching ? 'opacity-60 transition-opacity' : undefined}>
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>{t('checkIn.columnTime')}</TableHead>
                  <TableHead>{t('checkIn.columnDay')}</TableHead>
                  <TableHead>{t('checkIn.columnTrigger')}</TableHead>
                  <TableHead>{t('checkIn.columnOutcome')}</TableHead>
                  <TableHead className="text-right">{t('checkIn.columnDuration')}</TableHead>
                  <TableHead className="text-right">{t('checkIn.columnReward')}</TableHead>
                  <TableHead>{t('checkIn.columnSource')}</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {attempts.map((attempt) => {
                  const triggerKey = checkInTriggerLabelKey(attempt.trigger);
                  const sourceKey = checkInSourceLabelKey(attempt.source);
                  const failureLabel = checkInFailureLabel(attempt.error_code, t);
                  return (
                    <TableRow key={attempt.id}>
                      <TableCell className="whitespace-nowrap text-xs text-muted-foreground">
                        {formatStartedAt(attempt.started_at, i18n.language)}
                      </TableCell>
                      <TableCell className="whitespace-nowrap text-xs">
                        {formatDay(attempt.quota_day, i18n.language)}
                      </TableCell>
                      <TableCell className="text-xs">
                        {triggerKey ? t(triggerKey) : attempt.trigger}
                      </TableCell>
                      <TableCell>
                        <div className="flex flex-col gap-0.5">
                          <CheckInOutcomeBadge outcome={attempt.outcome} />
                          {(failureLabel || attempt.error_code) && (
                            <span className="text-xs text-muted-foreground">
                              {failureLabel ?? attempt.error_code}
                              {attempt.http_status !== null && ` (HTTP ${attempt.http_status})`}
                            </span>
                          )}
                        </div>
                      </TableCell>
                      <TableCell className="whitespace-nowrap text-right text-xs tabular-nums">
                        {formatDuration(attempt.duration_ms)}
                      </TableCell>
                      <TableCell className="whitespace-nowrap text-right text-xs tabular-nums">
                        {attempt.reward === null || attempt.reward === undefined
                          ? '—'
                          : `$${attempt.reward.toFixed(2)}`}
                      </TableCell>
                      <TableCell className="text-xs text-muted-foreground">
                        {sourceKey ? t(sourceKey) : (attempt.source ?? '—')}
                      </TableCell>
                    </TableRow>
                  );
                })}
              </TableBody>
            </Table>
          </div>
        )}

        <div className="flex items-center justify-end gap-2">
          <span className="text-xs text-muted-foreground">
            {t('checkIn.historyPage', { page: data?.page ?? page, pages: totalPages })}
          </span>
          <Button
            variant="outline"
            size="sm"
            disabled={page <= 1 || isFetching}
            onClick={() => setPage((current) => Math.max(1, current - 1))}
          >
            <ChevronLeft className="h-3.5 w-3.5" aria-hidden />
            {t('checkIn.previousPage')}
          </Button>
          <Button
            variant="outline"
            size="sm"
            disabled={page >= totalPages || isFetching}
            onClick={() => setPage((current) => Math.min(totalPages, current + 1))}
          >
            {t('checkIn.nextPage')}
            <ChevronRight className="h-3.5 w-3.5" aria-hidden />
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}
