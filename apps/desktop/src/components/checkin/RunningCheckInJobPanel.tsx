import { useTranslation } from 'react-i18next';
import { Loader2, XCircle } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Card } from '@/components/ui/card';
import { Progress } from '@/components/ui/progress';
import type { RunningJobDto } from '@/lib/tauri';

interface RunningCheckInJobPanelProps {
  /** The job matched to this batch, or `null` when we can only say "running". */
  job: RunningJobDto | null;
  onCancel: (jobId: string) => void;
  isCancelling: boolean;
}

/**
 * Live progress of a batch check-in this page started.
 *
 * When the job cannot be matched (the registry is polled, so there is a window
 * where no job is visible yet) this degrades to a plain "in progress" line
 * instead of inventing a percentage.
 */
export function RunningCheckInJobPanel({
  job,
  onCancel,
  isCancelling,
}: RunningCheckInJobPanelProps) {
  const { t } = useTranslation();
  const percent = job && job.total > 0 ? Math.min(100, (job.processed / job.total) * 100) : 0;

  return (
    <Card className="card-vivid group">
      <div className="flex flex-wrap items-center gap-3 p-4">
        <Loader2 className="h-4 w-4 shrink-0 animate-spin text-primary" aria-hidden />
        <div className="min-w-0 flex-1">
          <div className="flex items-center justify-between gap-2 text-sm">
            <span className="font-medium">{t('checkIn.runningTitle')}</span>
            <span className="text-xs text-muted-foreground tabular-nums">
              {job
                ? t('checkIn.runningProgress', { processed: job.processed, total: job.total })
                : t('checkIn.runningUnknown')}
            </span>
          </div>
          {job && <Progress className="mt-2" value={percent} />}
          {job?.cancelled && (
            <p className="mt-1 text-xs text-warning">{t('checkIn.runCancelled')}</p>
          )}
        </div>
        {job && (
          <Button
            variant="outline"
            size="sm"
            className="shrink-0"
            disabled={isCancelling || job.cancelled}
            onClick={() => onCancel(job.id)}
          >
            <XCircle className="h-3.5 w-3.5" aria-hidden />
            {t('common.cancel')}
          </Button>
        )}
      </div>
    </Card>
  );
}
