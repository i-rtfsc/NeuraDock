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
import type { CheckInDayDto } from '@/lib/tauri-commands';
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

        <div className="grid gap-4">
          {/* Check-in Status */}
          <Card className="p-4 flex items-center justify-between shadow-sm border-border/50">
            <span className="text-sm font-medium text-muted-foreground">{t('streaks.checkInStatus')}</span>
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
          </Card>

          {/* Balance Changes */}
          <Card className="shadow-sm border-border/50">
            <CardHeader className="pb-3">
              <CardTitle className="text-base">{t('streaks.dailyBalanceChanges')}</CardTitle>
            </CardHeader>
            <CardContent className="space-y-3">
              <div className="flex justify-between items-center">
                <span className="text-sm text-muted-foreground">{t('streaks.totalIncome')}</span>
                <div className="flex items-center gap-2">
                  <span className="font-medium">${dayData.total_income.toFixed(2)}</span>
                  {dayData.income_increment !== null && dayData.income_increment > 0 && (
                    <span className="text-xs text-green-600 dark:text-green-400 flex items-center bg-green-100 dark:bg-green-900/30 px-1.5 py-0.5 rounded">
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

              <div className="flex justify-between items-center pt-2 border-t border-border/50">
                <span className="text-sm font-medium">{t('streaks.currentBalance')}</span>
                <span className="font-bold text-blue-600 dark:text-blue-400">
                  ${dayData.current_balance.toFixed(2)}
                </span>
              </div>
            </CardContent>
          </Card>

          {/* Income Increment Info */}
          {dayData.income_increment !== null && (
            <Card className="p-4 bg-muted/30 border-none shadow-none">
              {dayData.income_increment > 0 ? (
                <p className="text-sm text-muted-foreground text-center">
                  {t('streaks.incrementPositivePrefix')}{' '}
                  <span className="font-semibold text-green-600 dark:text-green-400">
                    ${dayData.income_increment.toFixed(2)}
                  </span>
                  {t('streaks.incrementPositiveSuffix')}
                </p>
              ) : (
                <p className="text-sm text-muted-foreground text-center">{t('streaks.incrementZero')}</p>
              )}
            </Card>
          )}
        </div>

        <DialogFooter>
          <Button onClick={() => onOpenChange(false)}>{t('common.close')}</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
