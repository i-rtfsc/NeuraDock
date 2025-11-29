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
      <Card>
        <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
          <CardTitle className="text-sm font-medium">{t('streaks.currentStreak')}</CardTitle>
          <Flame className="h-4 w-4 text-orange-500" />
        </CardHeader>
        <CardContent>
          <div className="text-3xl font-bold text-orange-600 dark:text-orange-400">
            {currentStreak}
          </div>
          <p className="text-xs text-muted-foreground mt-1">{t('streaks.daysUnit')}</p>
        </CardContent>
      </Card>

      {/* Longest Streak */}
      <Card>
        <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
          <CardTitle className="text-sm font-medium">{t('streaks.longestStreak')}</CardTitle>
          <Trophy className="h-4 w-4 text-yellow-500" />
        </CardHeader>
        <CardContent>
          <div className="text-3xl font-bold text-yellow-600 dark:text-yellow-400">
            {longestStreak}
          </div>
          <p className="text-xs text-muted-foreground mt-1">{t('streaks.daysUnit')}</p>
        </CardContent>
      </Card>

      {/* Total Days */}
      <Card>
        <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
          <CardTitle className="text-sm font-medium">{t('streaks.totalDays')}</CardTitle>
          <CalendarCheck className="h-4 w-4 text-blue-500" />
        </CardHeader>
        <CardContent>
          <div className="text-3xl font-bold text-blue-600 dark:text-blue-400">
            {totalDays}
          </div>
          <p className="text-xs text-muted-foreground mt-1">{t('streaks.daysUnit')}</p>
        </CardContent>
      </Card>
    </div>
  );
}
