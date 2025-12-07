import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Flame, Trophy, CalendarCheck } from 'lucide-react';
import { useTranslation } from 'react-i18next';

interface StreakStatsCardsProps {
  currentStreak: number;
  longestStreak: number;
  totalDays: number;
}

export function StreakStatsCards({
  currentStreak,
  longestStreak,
  totalDays,
}: StreakStatsCardsProps) {
  const { t } = useTranslation();
  return (
    <div className="grid gap-4 md:grid-cols-3">
      {/* Current Streak */}
      <Card className="border-border/50 shadow-sm bg-gradient-to-br from-orange-50/50 to-transparent dark:from-orange-950/20">
        <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
          <CardTitle className="text-sm font-medium">{t('streaks.currentStreak')}</CardTitle>
          <div className="p-1.5 rounded-md bg-orange-100/50 dark:bg-orange-900/20 text-orange-600 dark:text-orange-400">
            <Flame className="h-4 w-4" />
          </div>
        </CardHeader>
        <CardContent>
          <div className="text-3xl font-bold text-foreground">
            {currentStreak}
          </div>
          <p className="text-xs text-muted-foreground mt-1">{t('streaks.daysUnit')}</p>
        </CardContent>
      </Card>

      {/* Longest Streak */}
      <Card className="border-border/50 shadow-sm bg-gradient-to-br from-yellow-50/50 to-transparent dark:from-yellow-950/20">
        <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
          <CardTitle className="text-sm font-medium">{t('streaks.longestStreak')}</CardTitle>
          <div className="p-1.5 rounded-md bg-yellow-100/50 dark:bg-yellow-900/20 text-yellow-600 dark:text-yellow-400">
            <Trophy className="h-4 w-4" />
          </div>
        </CardHeader>
        <CardContent>
          <div className="text-3xl font-bold text-foreground">
            {longestStreak}
          </div>
          <p className="text-xs text-muted-foreground mt-1">{t('streaks.daysUnit')}</p>
        </CardContent>
      </Card>

      {/* Total Days */}
      <Card className="border-border/50 shadow-sm bg-gradient-to-br from-blue-50/50 to-transparent dark:from-blue-950/20">
        <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
          <CardTitle className="text-sm font-medium">{t('streaks.totalDays')}</CardTitle>
          <div className="p-1.5 rounded-md bg-blue-100/50 dark:bg-blue-900/20 text-blue-600 dark:text-blue-400">
            <CalendarCheck className="h-4 w-4" />
          </div>
        </CardHeader>
        <CardContent>
          <div className="text-3xl font-bold text-foreground">
            {totalDays}
          </div>
          <p className="text-xs text-muted-foreground mt-1">{t('streaks.daysUnit')}</p>
        </CardContent>
      </Card>
    </div>
  );
}
