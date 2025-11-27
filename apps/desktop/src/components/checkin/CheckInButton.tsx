import { useState } from 'react';
import { Button } from '@/components/ui/button';
import { PlayCircle, Loader2 } from 'lucide-react';
import { useCheckIn, CheckInResult } from '@/hooks/useCheckIn';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { useTranslation } from 'react-i18next';

interface CheckInButtonProps {
  accountId: string;
  accountName: string;
  variant?: 'default' | 'outline' | 'ghost';
  size?: 'default' | 'sm' | 'lg';
  disabled?: boolean;
  className?: string;
}

export function CheckInButton({
  accountId,
  accountName,
  variant = 'default',
  size = 'default',
  disabled = false,
  className = '',
}: CheckInButtonProps) {
  const [showResult, setShowResult] = useState(false);
  const [result, setResult] = useState<CheckInResult | null>(null);
  const checkInMutation = useCheckIn();
  const { t } = useTranslation();

  const handleCheckIn = async () => {
    try {
      const data = await checkInMutation.mutateAsync(accountId);
      setResult(data);
      setShowResult(true);
    } catch (error) {
      // Error is handled by the hook
      console.error('Check-in error:', error);
    }
  };

  return (
    <>
      <Button
        variant={variant}
        size={size}
        onClick={handleCheckIn}
        disabled={disabled || checkInMutation.isPending}
        className={className}
      >
        {checkInMutation.isPending ? (
          <>
            <Loader2 className="mr-2 h-4 w-4 animate-spin" />
            {t('checkIn.checking')}
          </>
        ) : (
          <>
            <PlayCircle className="mr-2 h-4 w-4" />
            {t('checkIn.checkIn')}
          </>
        )}
      </Button>

      {/* Result Dialog */}
      {result && (
        <Dialog open={showResult} onOpenChange={setShowResult}>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>
                {result.success ? `✅ ${t('checkIn.success')}` : `❌ ${t('checkIn.failed')}`}
              </DialogTitle>
              <DialogDescription>{accountName}</DialogDescription>
            </DialogHeader>

            <div className="space-y-4 py-4">
              {result.success ? (
                <>
                  <div className="rounded-lg border border-green-500/50 bg-green-500/10 p-4">
                    <p className="text-sm font-medium text-green-700 dark:text-green-300">
                      {t('checkIn.success')}
                    </p>
                  </div>

                  {result.balance && (
                    <div className="space-y-2">
                      <h4 className="text-sm font-medium">{t('checkIn.balance')}</h4>
                      <div className="grid grid-cols-3 gap-4 rounded-lg bg-muted p-4">
                        <div>
                          <p className="text-xs text-muted-foreground">{t('dashboard.quota')}</p>
                          <p className="text-lg font-semibold">
                            ${result.balance.quota.toFixed(2)}
                          </p>
                        </div>
                        <div>
                          <p className="text-xs text-muted-foreground">{t('dashboard.used')}</p>
                          <p className="text-lg font-semibold">
                            ${result.balance.used.toFixed(2)}
                          </p>
                        </div>
                        <div>
                          <p className="text-xs text-muted-foreground">{t('dashboard.remaining')}</p>
                          <p className="text-lg font-semibold text-green-600 dark:text-green-400">
                            ${result.balance.remaining.toFixed(2)}
                          </p>
                        </div>
                      </div>
                    </div>
                  )}
                </>
              ) : (
                <div className="rounded-lg border border-red-500/50 bg-red-500/10 p-4">
                  <p className="text-sm font-medium text-red-700 dark:text-red-300">
                    {result.error || t('checkIn.failed')}
                  </p>
                </div>
              )}
            </div>

            <div className="flex justify-end">
              <Button onClick={() => setShowResult(false)}>{t('checkIn.close')}</Button>
            </div>
          </DialogContent>
        </Dialog>
      )}
    </>
  );
}
