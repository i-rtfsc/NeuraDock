import { useMemo, useState } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { StreakStatsCards } from '@/components/checkin/streak/StreakStatsCards';
import { CheckInTrendChart } from '@/components/checkin/streak/CheckInTrendChart';
import { CheckInCalendar } from '@/components/checkin/streak/CheckInCalendar';
import { CheckInDayDetailDialog } from '@/components/checkin/streak/CheckInDayDetailDialog';
import { AccountStreakSelector } from '@/components/checkin/streak/AccountStreakSelector';
import { Badge } from '@/components/ui/badge';
import {
  CheckInStreakDto,
  useAllCheckInStreaks,
  useCheckInStreak,
  useCheckInCalendar,
  useCheckInTrend,
} from '@/hooks/useCheckInStreak';
import { useTranslation } from 'react-i18next';

export function CheckInStreaksPage() {
  const { t } = useTranslation();
  const [selectedAccountId, setSelectedAccountId] = useState<string>('all');
  const [selectedDate, setSelectedDate] = useState<string | null>(null);
  const [calendarDate, setCalendarDate] = useState({
    year: new Date().getFullYear(),
    month: new Date().getMonth() + 1,
  });

  const { data: allStreaks, isLoading: isLoadingStreaks } = useAllCheckInStreaks();
  const { data: streak } = useCheckInStreak(
    selectedAccountId !== 'all' ? selectedAccountId : '',
    selectedAccountId !== 'all'
  );
  const { data: calendar } = useCheckInCalendar(
    selectedAccountId !== 'all' ? selectedAccountId : '',
    calendarDate.year,
    calendarDate.month,
    selectedAccountId !== 'all'
  );
  const { data: trend } = useCheckInTrend(
    selectedAccountId !== 'all' ? selectedAccountId : '',
    30,
    selectedAccountId !== 'all'
  );

  const selectedAccount =
    selectedAccountId !== 'all'
      ? allStreaks?.find((account) => account.account_id === selectedAccountId) ?? null
      : null;

  const providerGroups = useMemo(() => {
    if (!allStreaks) return [] as Array<{
      providerId: string;
      providerName: string;
      accounts: CheckInStreakDto[];
    }>;

    const map = new Map<string, { providerId: string; providerName: string; accounts: CheckInStreakDto[] }>();

    for (const account of allStreaks) {
      if (!map.has(account.provider_id)) {
        map.set(account.provider_id, {
          providerId: account.provider_id,
          providerName: account.provider_name,
          accounts: [],
        });
      }
      map.get(account.provider_id)!.accounts.push(account);
    }

    return Array.from(map.values());
  }, [allStreaks]);

  const handleDateClick = (date: string) => {
    setSelectedDate(date);
  };

  const handleMonthChange = (year: number, month: number) => {
    setCalendarDate({ year, month });
  };

  const selectedDayData = calendar?.days.find((d) => d.date === selectedDate) || null;

  if (isLoadingStreaks) {
    return (
      <div className="container mx-auto p-6">
        <div className="flex items-center justify-center h-64">
          <p className="text-muted-foreground">{t('common.loading')}</p>
        </div>
      </div>
    );
  }

  // If no accounts have data, show empty state
  if (!allStreaks || allStreaks.length === 0) {
    return (
      <div className="container mx-auto p-6">
        <div className="flex flex-col items-center justify-center h-64 gap-4">
          <p className="text-muted-foreground">{t('streaks.emptyTitle')}</p>
          <p className="text-sm text-muted-foreground">{t('streaks.emptyDescription')}</p>
        </div>
      </div>
    );
  }

  return (
    <div className="container mx-auto p-6 space-y-6">
      {/* Header */}
      <div className="flex flex-col gap-2 md:flex-row md:items-center md:justify-between">
        <div>
          <h1 className="text-3xl font-bold">{t('streaks.pageTitle')}</h1>
          {selectedAccount && (
            <p className="text-sm text-muted-foreground">
              {selectedAccount.account_name} · {selectedAccount.provider_name}
            </p>
          )}
        </div>
        <AccountStreakSelector
          accounts={allStreaks || []}
          selectedAccountId={selectedAccountId}
          onAccountChange={setSelectedAccountId}
        />
      </div>

      {selectedAccountId === 'all' ? (
        /* Show all accounts overview */
        <div className="space-y-6">
          <p className="text-sm text-muted-foreground">{t('streaks.allAccountsNotice')}</p>
          {/* Group accounts by provider similar to Accounts page */}
          <div className="space-y-6">
            {providerGroups.map((group) => (
              <Card key={group.providerId} className="border-2 rounded-2xl">
                <CardHeader className="pb-3">
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-3">
                      <CardTitle className="text-xl font-semibold">{group.providerName}</CardTitle>
                      <Badge variant="outline" className="rounded-full">
                        {t('streaks.accountCount', { count: group.accounts.length })}
                      </Badge>
                    </div>
                  </div>
                </CardHeader>
                <CardContent className="pt-2">
                  <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
                    {group.accounts.map((accountStreak) => (
                      <Card
                        key={accountStreak.account_id}
                        role="button"
                        tabIndex={0}
                        onClick={() => setSelectedAccountId(accountStreak.account_id)}
                        onKeyDown={(event) => {
                          if (event.key === 'Enter' || event.key === ' ') {
                            event.preventDefault();
                            setSelectedAccountId(accountStreak.account_id);
                          }
                        }}
                        className="rounded-2xl border transition-all hover:border-primary/50 cursor-pointer"
                      >
                        <CardContent className="p-4 space-y-3">
                          <div className="flex items-start justify-between">
                            <div>
                              <p className="font-semibold">{accountStreak.account_name}</p>
                              <p className="text-xs text-muted-foreground">
                                {t('streaks.lastCheckIn', {
                                  date: accountStreak.last_check_in_date ?? t('streaks.noData'),
                                })}
                              </p>
                            </div>
                            <Badge variant="secondary" className="rounded-full text-xs px-2">
                              {t('streaks.daysWithUnit', { count: accountStreak.current_streak })}
                            </Badge>
                          </div>
                          <div className="grid grid-cols-3 gap-2 text-xs">
                            <div className="space-y-1">
                              <p className="text-muted-foreground">{t('streaks.summaryCurrentLabel')}</p>
                              <p className="font-semibold text-orange-600">
                                {accountStreak.current_streak}
                              </p>
                            </div>
                            <div className="space-y-1">
                              <p className="text-muted-foreground">{t('streaks.summaryLongestLabel')}</p>
                              <p className="font-semibold text-yellow-600">
                                {accountStreak.longest_streak}
                              </p>
                            </div>
                            <div className="space-y-1">
                              <p className="text-muted-foreground">{t('streaks.summaryTotalLabel')}</p>
                              <p className="font-semibold text-blue-600">
                                {accountStreak.total_check_in_days}
                              </p>
                            </div>
                          </div>
                        </CardContent>
                      </Card>
                    ))}
                  </div>
                </CardContent>
              </Card>
            ))}
          </div>
        </div>
      ) : (
        /* Show single account details */
        <div className="space-y-6">
          {streak && calendar && (
            <Card className="overflow-hidden">
              <CardContent className="pt-6">
                <div className="grid gap-6 lg:grid-cols-[360px,1fr]">
                  <div className="rounded-2xl border bg-muted/30 p-5 space-y-4">
                    <div className="space-y-1">
                      <p className="text-xs uppercase tracking-wide text-muted-foreground">
                        {t('streaks.accountOverviewLabel')}
                      </p>
                      <p className="text-2xl font-semibold">
                        {selectedAccount?.account_name ?? t('streaks.currentAccountFallback')}
                      </p>
                      {streak.last_check_in_date && (
                        <p className="text-xs text-muted-foreground">
                          {t('streaks.lastCheckIn', { date: streak.last_check_in_date })}
                        </p>
                      )}
                    </div>

                    <div className="space-y-3">
                      {[
                        {
                          label: t('streaks.currentStreak'),
                          value: t('streaks.daysWithUnit', { count: streak.current_streak }),
                          accent: 'text-orange-600 dark:text-orange-400',
                        },
                        {
                          label: t('streaks.longestStreak'),
                          value: t('streaks.daysWithUnit', { count: streak.longest_streak }),
                          accent: 'text-yellow-600 dark:text-yellow-400',
                        },
                        {
                          label: t('streaks.totalDays'),
                          value: t('streaks.daysWithUnit', { count: streak.total_check_in_days }),
                          accent: 'text-blue-600 dark:text-blue-400',
                        },
                      ].map((stat) => (
                        <div
                          key={stat.label}
                          className="flex items-center justify-between rounded-xl bg-background/80 border px-3 py-3"
                        >
                          <div>
                            <p className="text-xs text-muted-foreground">{stat.label}</p>
                            <p className="text-base font-semibold">{stat.value}</p>
                          </div>
                          <span className={`text-lg font-semibold ${stat.accent}`}>{stat.value}</span>
                        </div>
                      ))}
                    </div>

                    <div className="space-y-3">
                      <p className="text-xs uppercase tracking-wide text-muted-foreground">
                        {t('streaks.monthlyStats')}
                      </p>
                      {[{
                        label: t('streaks.monthCheckedInDays'),
                        value: `${calendar.month_stats.checked_in_days}/${calendar.month_stats.total_days}`,
                        accent: 'text-blue-600 dark:text-blue-400'
                      }, {
                        label: t('streaks.checkInRate'),
                        value: `${calendar.month_stats.check_in_rate.toFixed(1)}%`,
                        accent: 'text-green-600 dark:text-green-400'
                      }, {
                        label: t('streaks.incomeIncrement'),
                        value: `$${calendar.month_stats.total_income_increment.toFixed(2)}`,
                        accent: 'text-orange-600 dark:text-orange-400'
                      }].map((stat) => (
                        <div
                          key={stat.label}
                          className="flex items-center justify-between rounded-xl bg-background/80 border px-3 py-3"
                        >
                          <p className="text-sm text-muted-foreground">{stat.label}</p>
                          <span className={`text-base font-semibold ${stat.accent}`}>{stat.value}</span>
                        </div>
                      ))}
                    </div>
                  </div>
                  <CheckInCalendar
                    year={calendarDate.year}
                    month={calendarDate.month}
                    days={calendar.days}
                    onDateClick={handleDateClick}
                    onMonthChange={handleMonthChange}
                    variant="inline"
                    className="rounded-2xl border bg-background/80 p-4"
                  />
                </div>
              </CardContent>
            </Card>
          )}

          {/* Fallback if only stats available */}
          {streak && !calendar && (
            <StreakStatsCards
              currentStreak={streak.current_streak}
              longestStreak={streak.longest_streak}
              totalDays={streak.total_check_in_days}
            />
          )}

          {/* Trend Chart */}
          {trend && trend.data_points.length > 0 && (
            <CheckInTrendChart data={trend.data_points} />
          )}

          {/* Calendar */}
          {calendar && !streak && (
            <CheckInCalendar
              year={calendarDate.year}
              month={calendarDate.month}
              days={calendar.days}
              onDateClick={handleDateClick}
              onMonthChange={handleMonthChange}
            />
          )}

          {/* Month stats fallback when calendar view not rendered inline */}
          {calendar && (!streak || selectedAccountId === 'all') && (
            <Card>
              <CardContent className="pt-6">
                <div className="flex items-center justify-around text-sm">
                  <div className="text-center">
                    <div className="text-2xl font-bold text-blue-600 dark:text-blue-400">
                      {calendar.month_stats.checked_in_days}/{calendar.month_stats.total_days}
                    </div>
                    <div className="text-muted-foreground">{t('streaks.monthDaysLabel')}</div>
                  </div>
                  <div className="text-center">
                    <div className="text-2xl font-bold text-green-600 dark:text-green-400">
                      {calendar.month_stats.check_in_rate.toFixed(1)}%
                    </div>
                    <div className="text-muted-foreground">{t('streaks.monthRateLabel')}</div>
                  </div>
                  <div className="text-center">
                    <div className="text-2xl font-bold text-orange-600 dark:text-orange-400">
                      ${calendar.month_stats.total_income_increment.toFixed(2)}
                    </div>
                    <div className="text-muted-foreground">{t('streaks.monthIncomeLabel')}</div>
                  </div>
                </div>
              </CardContent>
            </Card>
          )}
        </div>
      )}

      {/* Day Detail Dialog */}
      <CheckInDayDetailDialog
        open={!!selectedDate}
        onOpenChange={(open) => !open && setSelectedDate(null)}
        dayData={selectedDayData}
      />
    </div>
  );
}
