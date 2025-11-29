import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { CheckCircle, XCircle, TrendingUp } from 'lucide-react';
import { CheckInDayDto } from '@/hooks/useCheckInStreak';
import { useTranslation } from 'react-i18next';

interface CheckInDayDetailDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  dayData: CheckInDayDto | null;
}

export function CheckInDayDetailDialog({
  open,
  onOpenChange,
  dayData,
}: CheckInDayDetailDialogProps) {
  const { t, i18n } = useTranslation();
  if (!dayData) return null;

  const formatDate = (dateStr: string): string => {
    const date = new Date(dateStr);
    return date.toLocaleDateString(i18n.language, {
      year: 'numeric',
      month: 'long',
      day: 'numeric',
    });
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[500px]">
        <DialogHeader>
          <DialogTitle>{t('streaks.dayDetailTitle', { date: formatDate(dayData.date) })}</DialogTitle>
        </DialogHeader>

        <div className="space-y-4">
          {/* Check-in Status */}
          <div className="flex items-center gap-2">
            <span className="text-sm text-muted-foreground">{t('streaks.checkInStatus')}:</span>
            {dayData.is_checked_in ? (
              <Badge className="bg-green-500 hover:bg-green-600">
                <CheckCircle className="w-3 h-3 mr-1" />
                {t('streaks.checkedIn')}
              </Badge>
            ) : (
              <Badge variant="secondary">
                <XCircle className="w-3 h-3 mr-1" />
                {t('streaks.notCheckedIn')}
              </Badge>
            )}
          </div>

          {/* Balance Changes */}
          <Card>
            <CardHeader>
              <CardTitle className="text-base">{t('streaks.dailyBalanceChanges')}</CardTitle>
            </CardHeader>
            <CardContent className="space-y-3">
              <div className="flex justify-between items-center">
                <span className="text-sm text-muted-foreground">{t('streaks.totalIncome')}</span>
                <div className="flex items-center gap-2">
                  <span className="font-medium">${dayData.total_income.toFixed(2)}</span>
                  {dayData.income_increment !== null && dayData.income_increment > 0 && (
                    <span className="text-xs text-green-600 dark:text-green-400 flex items-center">
                      <TrendingUp className="w-3 h-3 mr-0.5" />
                      +${dayData.income_increment.toFixed(2)}
                    </span>
                  )}
                </div>
              </div>

              <div className="flex justify-between items-center">
                <span className="text-sm text-muted-foreground">{t('streaks.historicalConsumption')}</span>
                <span className="font-medium">${dayData.total_consumed.toFixed(2)}</span>
              </div>

              <div className="flex justify-between items-center">
                <span className="text-sm text-muted-foreground">{t('streaks.currentBalance')}</span>
                <span className="font-medium text-blue-600 dark:text-blue-400">
                  ${dayData.current_balance.toFixed(2)}
                </span>
              </div>
            </CardContent>
          </Card>

          {/* Income Increment Info */}
          {dayData.income_increment !== null && (
            <div className="text-sm text-muted-foreground bg-muted p-3 rounded-lg">
              {dayData.income_increment > 0 ? (
                <p>
                  {t('streaks.incrementPositivePrefix')}{' '}
                  <span className="font-semibold text-green-600 dark:text-green-400">
                    ${dayData.income_increment.toFixed(2)}
                  </span>
                  {t('streaks.incrementPositiveSuffix')}
                </p>
              ) : (
                <p>{t('streaks.incrementZero')}</p>
              )}
            </div>
          )}
        </div>

        <DialogFooter>
          <Button onClick={() => onOpenChange(false)}>{t('common.close')}</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
