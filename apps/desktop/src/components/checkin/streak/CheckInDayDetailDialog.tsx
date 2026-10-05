import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { TrendingUp } from 'lucide-react';
import type { CheckInDayDto } from '@/lib/tauri-commands';
import { useTranslation } from 'react-i18next';
import { useEffect, useState } from 'react';
import { Checkbox } from '@/components/ui/checkbox';

interface CheckInDayDetailDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  dayData: CheckInDayDto | null;
  onConfirmReward?: () => void;
  isSavingReward?: boolean;
  rewardError?: string;
}

export function CheckInDayDetailDialog({
  open,
  onOpenChange,
  dayData,
  onConfirmReward,
  isSavingReward = false,
  rewardError,
}: CheckInDayDetailDialogProps) {
  const { t, i18n } = useTranslation();
  const [confirmed, setConfirmed] = useState(false);
  useEffect(() => setConfirmed(false), [open, dayData?.date]);
  if (!dayData) return null;
  const canConfirm = onConfirmReward && dayData.date <= new Date().toISOString().slice(0, 10)
    && (!dayData.is_checked_in || dayData.reward_is_estimate || dayData.income_increment === null);

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
      <DialogContent className="sm:max-w-[400px] max-h-[90vh] rounded-[var(--radius-panel)] border-none p-0 overflow-y-auto shadow-2xl">
        <DialogHeader className="p-6 pb-4">
          <div className="flex items-center justify-between mb-1">
            <span className="text-[10px] font-black uppercase tracking-[0.2em] text-muted-foreground/50">
              {formatDate(dayData.date)}
            </span>
            {dayData.is_checked_in ? (
              <Badge className="bg-success-soft text-success border-0 font-black px-2.5 py-0.5 rounded-lg text-[10px] uppercase tracking-wider">
                {t('streaks.checkedIn')}
              </Badge>
            ) : (
              <Badge variant="secondary" className="bg-muted text-muted-foreground border-0 font-black px-2.5 py-0.5 rounded-lg text-[10px] uppercase tracking-wider">
                {t('streaks.notCheckedIn')}
              </Badge>
            )}
          </div>
          <DialogTitle className="text-2xl font-black tracking-tighter text-foreground">
            {t('streaks.dailyBalanceChanges')}
          </DialogTitle>
          <DialogDescription className="sr-only">
            {t('streaks.rewardTitle')} · {t('streaks.currentBalance')}
          </DialogDescription>
        </DialogHeader>

        <div className="px-6 pb-6 space-y-6">
          <div className="p-4 rounded-2xl bg-success-soft border border-success/20 space-y-2">
            <div className="flex items-center justify-between">
              <span className="text-sm font-medium">{t('streaks.rewardTitle')}</span>
              {dayData.reward_is_estimate && <Badge variant="secondary">{t('streaks.rewardEstimate')}</Badge>}
            </div>
            <p className="text-2xl font-bold text-success">
              {dayData.income_increment !== null
                ? `$${dayData.income_increment.toFixed(2)}`
                : t(dayData.is_checked_in ? 'streaks.rewardUnknown' : 'streaks.notCheckedIn')}
            </p>
            {dayData.reward_is_estimate && <p className="text-xs text-muted-foreground">{t('streaks.rewardEstimateHint')}</p>}
          </div>
          {/* Main Stats Grid */}
          <div className="grid grid-cols-2 gap-4">
            <div className="p-4 rounded-2xl bg-primary/5 border border-primary/10 flex flex-col gap-1">
              <span className="text-[9px] font-black text-primary/60 uppercase tracking-widest">{t('streaks.totalQuota')}</span>
              <div className="flex items-baseline gap-1.5">
                <span className="text-xl font-black tracking-tighter text-primary">${dayData.total_quota.toFixed(2)}</span>
              </div>
            </div>
            
            <div className="p-4 rounded-2xl bg-warning/5 border border-warning/10 flex flex-col gap-1">
              <span className="text-[9px] font-black text-warning/60 uppercase tracking-widest">{t('streaks.dailyConsumption')}</span>
              <span className="text-xl font-black tracking-tighter text-warning">${dayData.daily_consumed.toFixed(2)}</span>
            </div>
          </div>

          {/* Current Balance - Large Highlight */}
          <div className="relative overflow-hidden p-6 rounded-[2rem] bg-card border border-border/40 shadow-sm group">
            <div className="absolute top-0 right-0 p-4 opacity-5 group-hover:opacity-10 transition-opacity">
              <TrendingUp className="w-16 h-16" />
            </div>
            <div className="relative z-10">
              <span className="text-[10px] font-black text-muted-foreground/60 uppercase tracking-[0.2em]">{t('streaks.currentBalance')}</span>
              <div className="text-4xl font-black tracking-tighter text-vivid bg-clip-text text-transparent bg-gradient-to-r from-primary via-accent-2 to-primary animate-gradient-x mt-1">
                ${dayData.current_balance.toFixed(2)}
              </div>
            </div>
          </div>

          {/* Bottom Reference Info */}
          <div className="flex items-center justify-between px-2 text-[11px] font-bold text-muted-foreground uppercase tracking-widest border-t border-border/20 pt-4">
            <div className="flex items-center gap-2">
              <div className="w-1.5 h-1.5 rounded-full bg-primary/20" />
              <span>{t('streaks.historicalConsumption')}</span>
            </div>
            <span className="font-mono text-foreground/80 tracking-tight">${dayData.total_consumed.toFixed(2)}</span>
          </div>
          {canConfirm && (
            <div className="space-y-3 border-t border-border pt-4">
              <p className="text-xs text-muted-foreground">{t('streaks.manualRecordHint')}</p>
              <label className="flex items-start gap-2 text-sm cursor-pointer">
                <Checkbox checked={confirmed} onCheckedChange={value => setConfirmed(value === true)} disabled={isSavingReward} />
                <span>{t('streaks.manualConfirm')}</span>
              </label>
              <Button className="w-full" disabled={!confirmed || isSavingReward} onClick={onConfirmReward}>
                {t('streaks.manualRecord')}
              </Button>
              {rewardError && <p role="alert" className="text-sm text-destructive">{rewardError}</p>}
            </div>
          )}
        </div>

        <DialogFooter className="px-6 pb-6 pt-0">
          <Button 
            onClick={() => onOpenChange(false)}
            className="w-full font-black rounded-2xl h-12 bg-muted hover:bg-muted/80 text-foreground border-0 shadow-none transition-all active:scale-95"
          >
            {t('common.close')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
