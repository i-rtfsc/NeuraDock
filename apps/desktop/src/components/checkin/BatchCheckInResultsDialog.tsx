import { useTranslation } from 'react-i18next';
import { AlertTriangle, CheckCircle2, MinusCircle, XCircle } from 'lucide-react';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { CheckInOutcomeBadge } from './CheckInOutcomeBadge';
import { checkInFailureLabel } from '@/lib/checkInOutcome';
import type { CheckInResult } from '@/hooks/useCheckIn';

interface BatchCheckInResultsDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** Per-account results of the finished batch, in batch order. */
  results: CheckInResult[];
}

/** How one account's outcome reads as a one-line status. */
function resultStatusKey(outcome: string | null): string {
  switch (outcome) {
    case 'success':
    case 'already_confirmed':
      return 'checkIn.resultCheckedIn';
    case 'skipped_guard':
      return 'checkIn.resultSkipped';
    case 'cancelled':
      return 'checkIn.resultCancelled';
    default:
      return 'checkIn.resultFailed';
  }
}

function StatusIcon({ outcome }: { outcome: string | null }) {
  const className = 'h-4 w-4 shrink-0';
  switch (outcome) {
    case 'success':
    case 'already_confirmed':
      return <CheckCircle2 className={`${className} text-success`} aria-hidden />;
    case 'skipped_guard':
      return <MinusCircle className={`${className} text-info`} aria-hidden />;
    case 'cancelled':
      return <XCircle className={`${className} text-muted-foreground`} aria-hidden />;
    default:
      return <AlertTriangle className={`${className} text-warning`} aria-hidden />;
  }
}

/**
 * Per-account breakdown of a finished batch check-in.
 *
 * The batch toast carries only an aggregate count, so this dialog is where a
 * user finds out *which* accounts failed and *why*: the translated reason code
 * when we know it, the backend sentence otherwise.
 */
export function BatchCheckInResultsDialog({
  open,
  onOpenChange,
  results,
}: BatchCheckInResultsDialogProps) {
  const { t } = useTranslation();

  const succeeded = results.filter(
    (result) => result.outcome === 'success' || result.outcome === 'already_confirmed'
  ).length;
  const skipped = results.filter((result) => result.outcome === 'skipped_guard').length;
  const failed = results.length - succeeded - skipped;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent size="lg" data-testid="batch-check-in-results">
        <DialogHeader>
          <DialogTitle>{t('checkIn.batchDetailsTitle')}</DialogTitle>
          <DialogDescription>{t('checkIn.batchDetailsDescription')}</DialogDescription>
        </DialogHeader>

        <div className="flex flex-wrap items-center gap-x-4 gap-y-1 text-xs text-muted-foreground">
          <span>{t('checkIn.total')}: {results.length}</span>
          <span className="text-success">
            {t('checkIn.succeeded')}: {succeeded}
          </span>
          <span className="text-warning">
            {t('checkIn.failedCount')}: {failed}
          </span>
          {skipped > 0 && (
            <span>
              {t('checkIn.statsSkipped')}: {skipped}
            </span>
          )}
        </div>

        {results.length === 0 ? (
          <p className="py-6 text-center text-sm text-muted-foreground">
            {t('checkIn.batchDetailsEmpty')}
          </p>
        ) : (
          <ul className="max-h-[420px] divide-y divide-border/60 overflow-y-auto pr-1">
            {results.map((result) => {
              const reason =
                checkInFailureLabel(result.error_code, t) ?? result.error ?? null;
              return (
                <li
                  key={result.account_id}
                  className="flex items-start justify-between gap-3 py-3"
                >
                  <div className="min-w-0">
                    <div className="flex items-center gap-2 text-sm font-medium">
                      <StatusIcon outcome={result.outcome} />
                      <span className="truncate">{result.account_name}</span>
                    </div>
                    <div className="mt-0.5 pl-6 text-xs text-muted-foreground">
                      <span>{t(resultStatusKey(result.outcome))}</span>
                      {reason && <span className="ml-2 break-words">{reason}</span>}
                    </div>
                  </div>
                  <CheckInOutcomeBadge outcome={result.outcome} />
                </li>
              );
            })}
          </ul>
        )}

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            {t('common.close')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
