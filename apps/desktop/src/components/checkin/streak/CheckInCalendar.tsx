import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { ChevronLeft, ChevronRight, CalendarCheck } from 'lucide-react';
import type { CheckInDayDto } from '@/lib/tauri-commands';
import { cn } from '@/lib/utils';
import { useTranslation } from 'react-i18next';

interface CheckInCalendarProps {
  year: number;
  month: number;
  days: CheckInDayDto[];
  onDateClick: (date: string) => void;
  onMonthChange: (year: number, month: number) => void;
  variant?: 'standalone' | 'inline';
  className?: string;
}

const DEFAULT_WEEKDAYS = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];

export function CheckInCalendar({
  year,
  month,
  days,
  onDateClick,
  onMonthChange,
  variant = 'standalone',
  className,
}: CheckInCalendarProps) {
  const { t, i18n } = useTranslation();
  const weekdayLabelsRaw = t('streaks.weekdays', { returnObjects: true }) as string[] | string;
  const weekdayLabels = Array.isArray(weekdayLabelsRaw) ? weekdayLabelsRaw : DEFAULT_WEEKDAYS;
  const monthFormatter = new Intl.DateTimeFormat(i18n.language, {
    year: 'numeric',
    month: 'long',
  });
  const handlePrevMonth = () => {
    if (month === 1) {
      onMonthChange(year - 1, 12);
    } else {
      onMonthChange(year, month - 1);
    }
  };

  const handleNextMonth = () => {
    if (month === 12) {
      onMonthChange(year + 1, 1);
    } else {
      onMonthChange(year, month + 1);
    }
  };

  // Get first day of the week (0 = Sunday, 6 = Saturday)
  const firstDayOfWeek = new Date(year, month - 1, 1).getDay();

  // Get total days in month
  const daysInMonth = new Date(year, month, 0).getDate();

  // Create a map for quick lookup
  const dayMap = new Map<string, CheckInDayDto>();
  days.forEach((day) => {
    dayMap.set(day.date, day);
  });

  const getDayData = (day: number): CheckInDayDto | null => {
    const dateStr = `${year}-${String(month).padStart(2, '0')}-${String(day).padStart(2, '0')}`;
    return dayMap.get(dateStr) || null;
  };

  const isToday = (day: number): boolean => {
    const today = new Date();
    return (
      today.getFullYear() === year &&
      today.getMonth() + 1 === month &&
      today.getDate() === day
    );
  };

  const header = (
    <div className="flex items-center justify-between mb-6">
      {variant === 'standalone' ? (
        <CardTitle className="text-xl font-black tracking-tight">{t('streaks.calendarTitle')}</CardTitle>
      ) : (
        <p className="text-lg font-black tracking-tight flex items-center gap-2">
          <div className="p-1.5 rounded-lg bg-primary/10 text-primary shadow-sm">
            <CalendarCheck className="h-4 w-4" />
          </div>
          {t('streaks.calendarTitle')}
        </p>
      )}
      <div className="flex items-center gap-1 bg-muted/50 p-1 rounded-xl border border-border/50">
        <Button variant="ghost" size="icon" onClick={handlePrevMonth} className="h-8 w-8 rounded-lg hover:bg-background shadow-none">
          <ChevronLeft className="h-4 w-4" />
        </Button>
        <span className="text-xs font-black uppercase tracking-widest min-w-[100px] text-center">
          {monthFormatter.format(new Date(year, month - 1, 1))}
        </span>
        <Button variant="ghost" size="icon" onClick={handleNextMonth} className="h-8 w-8 rounded-lg hover:bg-background shadow-none">
          <ChevronRight className="h-4 w-4" />
        </Button>
      </div>
    </div>
  );

  const weekdayHeader = (
    <div className="grid grid-cols-7 gap-2 text-center mb-4">
      {(weekdayLabels.length ? weekdayLabels : DEFAULT_WEEKDAYS).map((day) => (
        <div key={day} className="text-[10px] font-black uppercase tracking-tighter text-muted-foreground/60">
          {day}
        </div>
      ))}
    </div>
  );

  const calendarGrid = (
    <div className="grid grid-cols-7 gap-2 justify-items-center">
      {Array.from({ length: firstDayOfWeek }).map((_, i) => (
        <div key={`empty-${i}`} className="w-10 h-10" />
      ))}
      {Array.from({ length: daysInMonth }).map((_, i) => {
        const day = i + 1;
        const dayData = getDayData(day);
        const today = isToday(day);

        return (
          <button
            key={day}
            onClick={() => dayData && onDateClick(dayData.date)}
            className={cn(
              'w-10 h-10 rounded-xl text-xs font-black transition-all duration-base flex flex-col items-center justify-center relative group',
              'hover:scale-110 active:scale-95',
              'disabled:opacity-20 disabled:cursor-not-allowed disabled:hover:scale-100',
              dayData?.is_checked_in
                ? 'bg-success text-success-foreground shadow-sm shadow-success/20'
                : 'bg-muted/40 text-muted-foreground/60 hover:bg-muted',
              today && 'ring-2 ring-primary ring-offset-2 dark:ring-offset-background z-10'
            )}
            disabled={!dayData}
          >
            {day}
            {dayData?.income_increment && dayData.income_increment > 0 && (
              <span className="absolute -top-1 -right-1 flex h-3 w-3 items-center justify-center">
                <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-primary opacity-75"></span>
                <span className="relative inline-flex rounded-full h-1.5 w-1.5 bg-primary shadow-[0_0_8px_rgba(var(--primary),0.8)]"></span>
              </span>
            )}
          </button>
        );
      })}
    </div>
  );

  const legend = (
    <div className="flex items-center justify-center gap-6 text-[10px] font-bold uppercase tracking-widest text-muted-foreground/70 mt-8">
      <div className="flex items-center gap-2">
        <div className="w-3 h-3 rounded-md bg-success shadow-sm shadow-success/30" />
        <span>{t('streaks.legendChecked')}</span>
      </div>
      <div className="flex items-center gap-2">
        <div className="w-3 h-3 rounded-md bg-muted/40" />
        <span>{t('streaks.legendUnchecked')}</span>
      </div>
      <div className="flex items-center gap-2">
        <div className="w-3 h-3 rounded-md border-2 border-primary" />
        <span>{t('streaks.legendToday')}</span>
      </div>
    </div>
  );

  if (variant === 'inline') {
    return (
      <div className={cn(className)}>
        {header}
        <div className="mt-4">{weekdayHeader}</div>
        <div className="mt-2">{calendarGrid}</div>
        <div className="mt-8">{legend}</div>
      </div>
    );
  }

  return (
    <Card className={className}>
      <CardHeader>{header}</CardHeader>
      <CardContent>
        {weekdayHeader}
        {calendarGrid}
        {legend}
      </CardContent>
    </Card>
  );
}
