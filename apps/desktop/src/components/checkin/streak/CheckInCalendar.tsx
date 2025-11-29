import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { ChevronLeft, ChevronRight } from 'lucide-react';
import { CheckInDayDto } from '@/hooks/useCheckInStreak';
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
    <div className="flex items-center justify-between">
      {variant === 'standalone' ? (
        <CardTitle>{t('streaks.calendarTitle')}</CardTitle>
      ) : (
        <p className="text-base font-semibold">{t('streaks.calendarTitle')}</p>
      )}
      <div className="flex items-center gap-2">
        <Button variant="ghost" size="icon" onClick={handlePrevMonth}>
          <ChevronLeft className="h-4 w-4" />
        </Button>
        <span className="text-sm font-medium min-w-[120px] text-center">
          {monthFormatter.format(new Date(year, month - 1, 1))}
        </span>
        <Button variant="ghost" size="icon" onClick={handleNextMonth}>
          <ChevronRight className="h-4 w-4" />
        </Button>
      </div>
    </div>
  );

  const weekdayHeader = (
    <div className="grid grid-cols-7 gap-2 text-center mb-2">
      {(weekdayLabels.length ? weekdayLabels : DEFAULT_WEEKDAYS).map((day) => (
        <div key={day} className="text-xs font-medium text-muted-foreground py-2">
          {day}
        </div>
      ))}
    </div>
  );

  const calendarGrid = (
    <div className="grid grid-cols-7 gap-2 text-center">
      {Array.from({ length: firstDayOfWeek }).map((_, i) => (
        <div key={`empty-${i}`} />
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
              'aspect-square p-2 rounded-lg text-sm font-medium transition-colors',
              'hover:bg-accent hover:text-accent-foreground',
              'disabled:opacity-50 disabled:cursor-not-allowed',
              dayData?.is_checked_in
                ? 'bg-green-500 text-white hover:bg-green-600 dark:bg-green-600 dark:hover:bg-green-700'
                : 'bg-muted text-muted-foreground',
              today && 'ring-2 ring-blue-500 ring-offset-2 dark:ring-offset-background'
            )}
            disabled={!dayData}
          >
            {day}
          </button>
        );
      })}
    </div>
  );

  const legend = (
    <div className="flex items-center justify-center gap-4 mt-4 text-xs text-muted-foreground">
      <div className="flex items-center gap-1">
        <div className="w-4 h-4 rounded bg-green-500 dark:bg-green-600" />
        <span>{t('streaks.legendChecked')}</span>
      </div>
      <div className="flex items-center gap-1">
        <div className="w-4 h-4 rounded bg-muted" />
        <span>{t('streaks.legendUnchecked')}</span>
      </div>
      <div className="flex items-center gap-1">
        <div className="w-4 h-4 rounded border-2 border-blue-500" />
        <span>{t('streaks.legendToday')}</span>
      </div>
    </div>
  );

  if (variant === 'inline') {
    return (
      <div className={cn('space-y-4', className)}>
        {header}
        {weekdayHeader}
        {calendarGrid}
        {legend}
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
