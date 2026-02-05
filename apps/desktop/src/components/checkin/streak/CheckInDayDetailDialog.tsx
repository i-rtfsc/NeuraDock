import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { TrendingUp } from 'lucide-react';
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
      <DialogContent className="sm:max-w-[400px] rounded-[var(--radius-panel)] border-none p-0 overflow-hidden shadow-2xl">
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
        </DialogHeader>

        <div className="px-6 pb-6 space-y-6">
          {/* Main Stats Grid */}
          <div className="grid grid-cols-2 gap-4">
            <div className="p-4 rounded-2xl bg-primary/5 border border-primary/10 flex flex-col gap-1">
              <span className="text-[9px] font-black text-primary/60 uppercase tracking-widest">{t('streaks.totalQuota')}</span>
              <div className="flex items-baseline gap-1.5">
                <span className="text-xl font-black tracking-tighter text-primary">${dayData.total_quota.toFixed(2)}</span>
                {dayData.income_increment !== null && dayData.income_increment > 0 && (
                  <span className="text-[10px] font-black text-success">+{dayData.income_increment.toFixed(2)}</span>
                )}
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