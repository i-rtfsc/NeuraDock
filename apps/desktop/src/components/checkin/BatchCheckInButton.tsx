import { useState } from 'react';
import { Button } from '@/components/ui/button';
import { PlayCircle, Loader2 } from 'lucide-react';
import { useBatchCheckIn, BatchCheckInResult } from '@/hooks/useCheckIn';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Badge } from '@/components/ui/badge';
import { useTranslation } from 'react-i18next';

interface BatchCheckInButtonProps {
  accountIds: string[];
  disabled?: boolean;
  onComplete?: (result: BatchCheckInResult) => void;
}

export function BatchCheckInButton({
  accountIds,
  disabled = false,
  onComplete,
}: BatchCheckInButtonProps) {
  const [showResult, setShowResult] = useState(false);
  const [result, setResult] = useState<BatchCheckInResult | null>(null);
  const batchCheckInMutation = useBatchCheckIn();
  const { t } = useTranslation();

  const handleBatchCheckIn = async () => {
    if (accountIds.length === 0) {
      return;
    }

    try {
      const data = await batchCheckInMutation.mutateAsync(accountIds);
      setResult(data);
      setShowResult(true);
      onComplete?.(data);
    } catch (error) {
      console.error('Batch check-in error:', error);
    }
  };

  return (
    <>
      <Button
        variant="outline"
        onClick={handleBatchCheckIn}
        disabled={disabled || accountIds.length === 0 || batchCheckInMutation.isPending}
        size="sm"
        className="rounded-full"
      >
        {batchCheckInMutation.isPending ? (
          <>
            <Loader2 className="mr-2 h-4 w-4 animate-spin" />
            {t('checkIn.checking')}
          </>
        ) : (
          <>
            <PlayCircle className="mr-2 h-4 w-4" />
            {t('checkIn.batchCheckIn')} ({accountIds.length})
          </>
        )}
      </Button>

      {/* Results Dialog */}
      {result && (
        <Dialog open={showResult} onOpenChange={setShowResult}>
          <DialogContent className="max-w-2xl">
            <DialogHeader>
              <DialogTitle>{t('checkIn.batchTitle')}</DialogTitle>
              <DialogDescription>
                {result.succeeded} {t('checkIn.succeeded')}, {result.failed} {t('checkIn.failedCount')} / {result.total} {t('checkIn.total')}
              </DialogDescription>
            </DialogHeader>

            <div className="space-y-4 py-4">
              {/* Summary */}
              <div className="grid grid-cols-3 gap-4">
                <div className="rounded-lg border border-border bg-muted/50 p-4 text-center">
                  <p className="text-2xl font-bold">{result.total}</p>
                  <p className="text-xs text-muted-foreground">{t('checkIn.total')}</p>
                </div>
                <div className="rounded-lg border border-green-500/50 bg-green-500/10 p-4 text-center">
                  <p className="text-2xl font-bold text-green-600 dark:text-green-400">
                    {result.succeeded}
                  </p>
                  <p className="text-xs text-muted-foreground">{t('checkIn.succeeded')}</p>
                </div>
                <div className="rounded-lg border border-red-500/50 bg-red-500/10 p-4 text-center">
                  <p className="text-2xl font-bold text-red-600 dark:text-red-400">
                    {result.failed}
                  </p>
                  <p className="text-xs text-muted-foreground">{t('checkIn.failedCount')}</p>
                </div>
              </div>

              {/* Detailed Results */}
              <div className="space-y-2">
                <h4 className="text-sm font-medium">{t('checkIn.batchDescription')}</h4>
                <div className="max-h-[400px] overflow-y-auto space-y-2 rounded-lg border border-border p-2">
                  {result.results.map((item) => (
                    <div
                      key={item.account_id}
                      className={`rounded-md border p-3 ${
                        item.success
                          ? 'border-green-500/50 bg-green-500/5'
                          : 'border-red-500/50 bg-red-500/5'
                      }`}
                    >
                      <div className="flex items-start justify-between gap-2">
                        <div className="flex-1 min-w-0">
                          <div className="flex items-center gap-2">
                            <Badge variant={item.success ? 'default' : 'destructive'}>
                              {item.success ? t('checkIn.succeeded') : t('checkIn.failedCount')}
                            </Badge>
                            <span className="text-sm font-medium truncate">
                              {item.account_name || item.account_id}
                            </span>
                          </div>

                          {item.error && (
                            <p className="text-xs text-red-600 dark:text-red-400 mt-1">
                              {item.error}
                            </p>
                          )}

                          {item.balance && (
                            <div className="flex gap-4 mt-2 text-xs text-muted-foreground">
                              <span>{t('dashboard.current_balance')}: ${item.balance.current_balance.toFixed(2)}</span>
                              <span>{t('dashboard.consumed')}: ${item.balance.total_consumed.toFixed(2)}</span>
                              <span className="text-green-600 dark:text-green-400">
                                {t('dashboard.total_income')}: ${item.balance.total_income.toFixed(2)}
                              </span>
                            </div>
                          )}
                        </div>
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            </div>

            <div className="flex justify-end gap-2">
              <Button onClick={() => setShowResult(false)}>{t('checkIn.close')}</Button>
            </div>
          </DialogContent>
        </Dialog>
      )}
    </>
  );
}
