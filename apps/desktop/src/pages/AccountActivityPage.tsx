import { useState, useEffect } from 'react';
import { useParams, useNavigate, Link } from 'react-router-dom';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { CheckInTrendChart } from '@/components/checkin/streak/CheckInTrendChart';
import { CheckInCalendar } from '@/components/checkin/streak/CheckInCalendar';
import { CheckInDayDetailDialog } from '@/components/checkin/streak/CheckInDayDetailDialog';
import { ChevronLeft, Calendar, TrendingUp, Percent, Flame, Trophy, CalendarCheck } from 'lucide-react';
import {
  useCheckInStreak,
  useCheckInCalendar,
  useCheckInTrend,
} from '@/hooks/useCheckInStreak';
import { useAccounts } from '@/hooks/useAccounts';
import { useTranslation } from 'react-i18next';
import { PageContainer } from '@/components/layout/PageContainer';
import { BentoGrid } from '@/components/layout/CardGrid';

export function AccountActivityPage() {
  const { accountId } = useParams<{ accountId: string }>();
  const navigate = useNavigate();
  const { t } = useTranslation();
  const { data: accounts } = useAccounts();
  
  const [calendarDate, setCalendarDate] = useState(() => {
    const now = new Date();
    return { year: now.getFullYear(), month: now.getMonth() + 1 };
  });
  const [selectedDate, setSelectedDate] = useState<string | null>(null);

  const account = accounts?.find(a => a.id === accountId);
  const { data: streak } = useCheckInStreak(accountId ?? '', !!accountId);
  const { data: calendar } = useCheckInCalendar(
    accountId ?? '',
    calendarDate.year,
    calendarDate.month,
    !!accountId
  );
  const { data: trend } = useCheckInTrend(accountId ?? '', 90, !!accountId);

  // 获取选中日期的详细数据
  const selectedDayData = selectedDate && calendar
    ? calendar.days.find(day => day.date === selectedDate)
    : null;

  useEffect(() => {
    if (!accountId || (accounts && !account)) {
      navigate('/accounts');
    }
  }, [accountId, account, accounts, navigate]);

  if (!account) {
    return null;
  }

  const handleDateClick = (date: string) => {
    setSelectedDate(date);
  };

  const handleMonthChange = (year: number, month: number) => {
    setCalendarDate({ year, month });
  };

  return (
    <PageContainer
      headerClassName="py-2"
      title={
        <div className="flex items-center gap-3 min-w-0">
          <Link
            to="/accounts"
            className="inline-flex items-center justify-center h-btn-icon-sm w-btn-icon-sm rounded-full text-muted-foreground hover:text-foreground hover:bg-muted transition-colors"
            aria-label={t('accounts.title')}
          >
            <ChevronLeft className="h-4 w-4" />
          </Link>
          <div className="min-w-0">
            <div className="text-lg font-semibold truncate">{account.name}</div>
            <div className="text-xs text-muted-foreground truncate">{account.provider_name}</div>
          </div>
        </div>
      }
    >

      {/* 统计概览卡片 */}
      <BentoGrid className="mb-section-gap-sm">
        <Card className="bg-gradient-to-br from-info/10 to-info/5 border-info-border interactive-scale">
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-info flex items-center gap-2">
              <Flame className="h-4 w-4" />
              {t('streaks.currentStreak')}
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-3xl font-bold tabular-nums">
              {streak?.current_streak ?? 0}
            </div>
            <p className="text-xs text-muted-foreground mt-1">
              {t('streaks.daysUnit')}
            </p>
          </CardContent>
        </Card>

        <Card className="bg-gradient-to-br from-accent-2/10 to-accent-2/5 border-accent-2-border interactive-scale">
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-accent-2 flex items-center gap-2">
              <Trophy className="h-4 w-4" />
              {t('streaks.longestStreak')}
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-3xl font-bold tabular-nums">
              {streak?.longest_streak ?? 0}
            </div>
            <p className="text-xs text-muted-foreground mt-1">
              {t('streaks.daysUnit')}
            </p>
          </CardContent>
        </Card>

        <Card className="bg-gradient-to-br from-success/10 to-success/5 border-success-border interactive-scale">
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-success flex items-center gap-2">
              <CalendarCheck className="h-4 w-4" />
              {t('streaks.totalDays')}
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-3xl font-bold tabular-nums">
              {streak?.total_check_in_days ?? 0}
            </div>
            <p className="text-xs text-muted-foreground mt-1">
              {t('streaks.daysUnit')}
            </p>
          </CardContent>
        </Card>

        <Card className="bg-gradient-to-br from-warning/10 to-warning/5 border-warning-border interactive-scale">
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-warning flex items-center gap-2">
              <Percent className="h-4 w-4" />
              {t('streaks.checkInRate')}
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-3xl font-bold tabular-nums">
              {calendar?.month_stats.check_in_rate
                ? `${calendar.month_stats.check_in_rate.toFixed(0)}%`
                : '0%'
              }
            </div>
            <p className="text-xs text-muted-foreground mt-1">
              {t('streaks.monthlyStats')}
            </p>
          </CardContent>
        </Card>
      </BentoGrid>

      {/* 签到日历和趋势图 */}
      <div className="grid gap-card-gap lg:grid-cols-2">
        <Card className="border-border/50 interactive-scale">
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Calendar className="h-5 w-5" />
              {t('streaks.calendarTitle')}
            </CardTitle>
          </CardHeader>
          <CardContent>
            {calendar ? (
              <CheckInCalendar
                year={calendarDate.year}
                month={calendarDate.month}
                days={calendar.days}
                onDateClick={handleDateClick}
                onMonthChange={handleMonthChange}
                variant="inline"
              />
            ) : (
              <div className="h-64 flex items-center justify-center text-muted-foreground">
                {t('streaks.noData')}
              </div>
            )}
          </CardContent>
        </Card>

        <Card className="border-border/50 interactive-scale">
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <TrendingUp className="h-5 w-5" />
                  <h3 className="text-sm font-medium text-muted-foreground uppercase tracking-wider">{t('streaks.trendTitle')}</h3>
            </CardTitle>
          </CardHeader>
          <CardContent>
            {trend && trend.data_points && trend.data_points.length > 0 ? (
              <CheckInTrendChart data={trend.data_points} />
            ) : (
              <div className="h-64 flex items-center justify-center text-muted-foreground">
                {t('streaks.noData')}
              </div>
            )}
          </CardContent>
        </Card>
      </div>

      {/* 日期详情弹窗 */}
      <CheckInDayDetailDialog
        open={!!selectedDate}
        onOpenChange={(open) => !open && setSelectedDate(null)}
        dayData={selectedDayData ?? null}
      />
    </PageContainer>
  );
}
