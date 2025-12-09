import { useMemo, useState } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { CheckInTrendChart } from '@/components/checkin/streak/CheckInTrendChart';
import { CheckInCalendar } from '@/components/checkin/streak/CheckInCalendar';
import { CheckInDayDetailDialog } from '@/components/checkin/streak/CheckInDayDetailDialog';
import { Badge } from '@/components/ui/badge';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Input } from '@/components/ui/input';
import { Search, Layers, Box, Calendar, TrendingUp, DollarSign, Percent, Flame, Trophy, CalendarCheck } from 'lucide-react';
import { cn } from '@/lib/utils';
import {
  CheckInStreakDto,
  useAllCheckInStreaks,
  useCheckInStreak,
  useCheckInCalendar,
  useCheckInTrend,
} from '@/hooks/useCheckInStreak';
import { useTranslation } from 'react-i18next';

import { PageContainer } from '@/components/layout/PageContainer';
import { SidebarPageLayout } from '@/components/layout/SidebarPageLayout';

export function CheckInStreaksPage() {
  const { t } = useTranslation();
  const [selectedAccountId, setSelectedAccountId] = useState<string>('all');
  const { data: allStreaks, isLoading: isLoadingStreaks } = useAllCheckInStreaks();
  const [calendarDate, setCalendarDate] = useState(() => {
    const now = new Date();
    return { year: now.getFullYear(), month: now.getMonth() + 1 };
  });
  const [selectedDate, setSelectedDate] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState('');

  const selectedAccount = useMemo<CheckInStreakDto | undefined>(() => {
    if (!allStreaks || selectedAccountId === 'all') {
      return undefined;
    }

    return allStreaks.find((streak) => streak.account_id === selectedAccountId);
  }, [allStreaks, selectedAccountId]);

  const accountIdForDetails =
    selectedAccountId === 'all' ? selectedAccount?.account_id ?? null : selectedAccountId;
  const detailsEnabled = !!accountIdForDetails;

  const { data: streak } = useCheckInStreak(accountIdForDetails ?? '', detailsEnabled);
  const { data: calendar } = useCheckInCalendar(
    accountIdForDetails ?? '',
    calendarDate.year,
    calendarDate.month,
    detailsEnabled
  );
  const { data: trend } = useCheckInTrend(accountIdForDetails ?? '', 90, detailsEnabled);

  // Filter accounts for sidebar
  const filteredStreaks = useMemo(() => {
    if (!allStreaks) return [];
    if (!searchQuery) return allStreaks;
    const query = searchQuery.toLowerCase();
    return allStreaks.filter(
      (s) =>
        s.account_name.toLowerCase().includes(query) ||
        s.provider_name.toLowerCase().includes(query)
    );
  }, [allStreaks, searchQuery]);

  const providerGroups = useMemo(
    () => {
      if (!filteredStreaks) {
        return [] as {
          providerId: string;
          providerName: string;
          accounts: CheckInStreakDto[];
        }[];
      }

      const groupsMap = new Map<
        string,
        { providerId: string; providerName: string; accounts: CheckInStreakDto[] }
      >();

      filteredStreaks.forEach((streakItem) => {
        if (!groupsMap.has(streakItem.provider_id)) {
          groupsMap.set(streakItem.provider_id, {
            providerId: streakItem.provider_id,
            providerName: streakItem.provider_name,
            accounts: [],
          });
        }
        groupsMap.get(streakItem.provider_id)!.accounts.push(streakItem);
      });

      return Array.from(groupsMap.values());
    },
    [filteredStreaks]
  );

  const handleDateClick = (date: string) => {
    setSelectedDate(date);
  };

  const handleMonthChange = (year: number, month: number) => {
    setCalendarDate({ year, month });
  };

  const selectedDayData = useMemo(() => {
    if (!calendar || !selectedDate) {
      return null;
    }

    return calendar.days.find((day) => day.date === selectedDate) ?? null;
  }, [calendar, selectedDate]);

  const sidebarContent = (
    <>
      <div className="relative">
        <Search className="absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-muted-foreground" />
        <Input
          placeholder={t('accounts.searchPlaceholder')}
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          className="pl-8 h-9 bg-background shadow-sm border-border/50 text-sm"
        />
      </div>
      
      <Card className="flex-1 border-border/50 shadow-sm bg-background/50 backdrop-blur-sm overflow-hidden">
        <ScrollArea className="h-full">
          <div className="p-2 space-y-1">
            <button
              onClick={() => setSelectedAccountId('all')}
              className={cn(
                "w-full flex items-center justify-between px-3 py-2 rounded-lg text-sm font-medium transition-colors",
                selectedAccountId === 'all' 
                  ? "bg-primary text-primary-foreground shadow-sm" 
                  : "text-muted-foreground hover:bg-muted hover:text-foreground"
              )}
            >
              <div className="flex items-center gap-2">
                <Layers className="h-4 w-4" />
                <span>{t('streaks.selectAll')}</span>
              </div>
              <span className={cn("text-xs", selectedAccountId === 'all' ? "opacity-90" : "opacity-70")}>
                {allStreaks?.length || 0}
              </span>
            </button>
            
            {providerGroups.map((group) => (
              <div key={group.providerId} className="mt-4 first:mt-2">
                <div className="px-3 mb-1 text-xs font-semibold text-muted-foreground/50 tracking-wider flex items-center gap-2">
                  <Box className="h-3 w-3" />
                  {group.providerName}
                </div>
                <div className="space-y-1">
                  {group.accounts.map((account) => {
                    const isActive = selectedAccountId === account.account_id;
                    return (
                      <button
                        key={account.account_id}
                        onClick={() => setSelectedAccountId(account.account_id)}
                        title={account.account_name}
                        className={cn(
                          "w-full flex items-center justify-between px-3 py-2 rounded-lg text-sm font-medium transition-colors",
                          isActive 
                            ? "bg-primary text-primary-foreground shadow-sm" 
                            : "text-muted-foreground hover:bg-muted hover:text-foreground"
                        )}
                      >
                        <span className="truncate">{account.account_name}</span>
                        {account.current_streak > 0 && (
                          <Badge variant="secondary" className={cn("text-[10px] h-5 px-1.5", isActive ? "bg-primary-foreground/20 text-primary-foreground" : "")}>
                            {account.current_streak}
                          </Badge>
                        )}
                      </button>
                    );
                  })}
                </div>
              </div>
            ))}
          </div>
        </ScrollArea>
      </Card>
    </>
  );

  if (isLoadingStreaks) {
    return (
      <PageContainer title={t('streaks.pageTitle')}>
        <div className="flex items-center justify-center h-64">
          <p className="text-muted-foreground">{t('common.loading')}</p>
        </div>
      </PageContainer>
    );
  }

  return (
    <PageContainer 
      className="h-full overflow-hidden"
      title={
        <div className="flex flex-col gap-1">
          <h1 className="text-2xl font-bold tracking-tight">
            {selectedAccountId === 'all' ? t('streaks.pageTitle') : selectedAccount?.account_name}
          </h1>
          {selectedAccount && (
            <div className="flex items-center gap-2 text-sm text-muted-foreground">
              <Box className="h-3.5 w-3.5" />
              <span>{selectedAccount.provider_name}</span>
            </div>
          )}
        </div>
      }
    >
      <SidebarPageLayout sidebar={sidebarContent}>
        {selectedAccountId === 'all' ? (
          /* All Accounts Overview */
          <div className="space-y-8 animate-in fade-in slide-in-from-bottom-4 duration-500">
            {allStreaks && allStreaks.length === 0 ? (
              <Card className="border-dashed bg-muted/30">
                <div className="flex flex-col items-center justify-center h-64 gap-4">
                  <p className="text-muted-foreground">{t('streaks.emptyTitle')}</p>
                  <p className="text-sm text-muted-foreground">{t('streaks.emptyDescription')}</p>
                </div>
              </Card>
            ) : (
              <div className="space-y-8">
                {providerGroups.map((group) => (
                  <Card key={group.providerId} className="border-border/50 shadow-sm overflow-hidden">
                    <div className="flex items-center justify-between px-4 py-3 bg-muted/30 border-b border-border/50">
                      <div className="flex items-center gap-3">
                        <h2 className="text-base font-semibold tracking-tight">{group.providerName}</h2>
                        <Badge variant="secondary" className="rounded-full px-2.5 text-xs bg-background/50">
                          {group.accounts.length}
                        </Badge>
                      </div>
                    </div>
                    <div className="p-4 grid gap-4 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4">
                      {group.accounts.map((accountStreak) => (
                        <Card
                          key={accountStreak.account_id}
                          role="button"
                          tabIndex={0}
                          onClick={() => setSelectedAccountId(accountStreak.account_id)}
                          className="group relative overflow-hidden rounded-xl border-border/50 shadow-sm bg-background transition-all hover:shadow-md hover:border-primary/50 cursor-pointer active:scale-[0.98]"
                        >
                          <CardContent className="p-5 flex flex-col h-full justify-between gap-4">
                            <div className="space-y-2">
                              <div className="flex items-start justify-between gap-2">
                                <div className="space-y-1 min-w-0 flex-1">
                                  <p className="font-semibold truncate text-base" title={accountStreak.account_name}>
                                    {accountStreak.account_name}
                                  </p>
                                  <p className="text-xs text-muted-foreground truncate">
                                    {t('streaks.lastCheckIn', {
                                      date: accountStreak.last_check_in_date ?? t('streaks.noData'),
                                    })}
                                  </p>
                                </div>
                                {accountStreak.current_streak > 0 && (
                                  <Badge 
                                    variant="default" 
                                    className="rounded-full text-[10px] px-2 h-5 shrink-0 bg-orange-500 hover:bg-orange-600 border-none"
                                  >
                                    <TrendingUp className="w-3 h-3 mr-1" />
                                    {accountStreak.current_streak}
                                  </Badge>
                                )}
                              </div>
                            </div>
                            
                            <div className="flex items-center justify-between pt-4 mt-auto">
                              <div className="flex flex-col gap-1">
                                <p className="text-[10px] text-muted-foreground uppercase tracking-wider font-medium">{t('streaks.summaryCurrentLabel')}</p>
                                <p className="text-lg font-bold text-orange-600 dark:text-orange-400 leading-none">{accountStreak.current_streak}</p>
                              </div>
                              <div className="flex flex-col gap-1 text-center">
                                <p className="text-[10px] text-muted-foreground uppercase tracking-wider font-medium">{t('streaks.summaryLongestLabel')}</p>
                                <p className="text-lg font-bold text-yellow-600 dark:text-yellow-400 leading-none">{accountStreak.longest_streak}</p>
                              </div>
                              <div className="flex flex-col gap-1 text-right">
                                <p className="text-[10px] text-muted-foreground uppercase tracking-wider font-medium">{t('streaks.summaryTotalLabel')}</p>
                                <p className="text-lg font-bold text-blue-600 dark:text-blue-400 leading-none">{accountStreak.total_check_in_days}</p>
                              </div>
                            </div>
                          </CardContent>
                        </Card>
                      ))}
                    </div>
                  </Card>
                ))}
              </div>
            )}
          </div>
        ) : (
          /* Single Account Details */
          <div className="space-y-8 animate-in fade-in slide-in-from-bottom-4 duration-500">
            {streak && (
              <div className="relative overflow-hidden rounded-3xl bg-gradient-to-br from-orange-500 via-orange-600 to-red-600 p-8 text-white shadow-xl ring-1 ring-black/5">
                <div className="absolute -right-12 -top-12 opacity-10 rotate-12">
                  <Flame className="h-80 w-80" />
                </div>
                
                <div className="relative z-10 grid gap-8 md:grid-cols-2 items-end">
                  <div>
                    <div className="flex items-center gap-2 opacity-90 mb-2">
                      <div className="p-1.5 rounded-lg bg-white/20 backdrop-blur-sm">
                        <Flame className="h-5 w-5" />
                      </div>
                      <span className="text-sm font-bold uppercase tracking-wider">{t('streaks.currentStreak')}</span>
                    </div>
                    <div className="flex items-baseline gap-3">
                      <span className="text-8xl font-black tracking-tighter drop-shadow-sm">
                        {streak.current_streak}
                      </span>
                      <span className="text-2xl font-medium opacity-80">{t('streaks.daysUnit')}</span>
                    </div>
                  </div>

                  <div className="flex flex-wrap gap-4 md:justify-end">
                    <div className="rounded-2xl bg-white/10 backdrop-blur-md border border-white/10 p-5 min-w-[160px] flex-1 md:flex-none transition-transform hover:scale-105">
                      <div className="flex items-center gap-2 opacity-80 mb-2">
                        <Trophy className="h-4 w-4" />
                        <span className="text-xs font-bold uppercase tracking-wide">{t('streaks.longestStreak')}</span>
                      </div>
                      <div className="text-3xl font-bold">
                        {streak.longest_streak} <span className="text-sm font-normal opacity-70">{t('streaks.daysUnit')}</span>
                      </div>
                    </div>
                    
                    <div className="rounded-2xl bg-white/10 backdrop-blur-md border border-white/10 p-5 min-w-[160px] flex-1 md:flex-none transition-transform hover:scale-105">
                      <div className="flex items-center gap-2 opacity-80 mb-2">
                        <CalendarCheck className="h-4 w-4" />
                        <span className="text-xs font-bold uppercase tracking-wide">{t('streaks.totalDays')}</span>
                      </div>
                      <div className="text-3xl font-bold">
                        {streak.total_check_in_days} <span className="text-sm font-normal opacity-70">{t('streaks.daysUnit')}</span>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            )}

            <div className="grid gap-6 lg:grid-cols-3">
              {/* Calendar Card */}
              <Card className="lg:col-span-2 border-none shadow-lg bg-background/60 backdrop-blur-xl ring-1 ring-border/50">
                <CardHeader className="pb-4">
                  <div className="flex items-center gap-3">
                    <div className="p-2 rounded-xl bg-primary/10 text-primary">
                      <Calendar className="h-5 w-5" />
                    </div>
                    <CardTitle className="text-xl">{t('streaks.calendarTitle')}</CardTitle>
                  </div>
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
                    <div className="h-[300px] flex items-center justify-center text-muted-foreground">
                      {t('common.loading')}
                    </div>
                  )}
                </CardContent>
              </Card>

              {/* Monthly Stats Card */}
              <Card className="border-none shadow-lg bg-background/60 backdrop-blur-xl ring-1 ring-border/50 h-fit">
                <CardHeader className="pb-4">
                  <div className="flex items-center gap-3">
                    <div className="p-2 rounded-xl bg-primary/10 text-primary">
                      <TrendingUp className="h-5 w-5" />
                    </div>
                    <CardTitle className="text-xl">{t('streaks.monthlyStats')}</CardTitle>
                  </div>
                </CardHeader>
                <CardContent className="space-y-4">
                  {calendar ? (
                    <div className="grid gap-4">
                      <div className="group relative overflow-hidden p-5 rounded-2xl bg-gradient-to-br from-blue-50 to-blue-100/50 dark:from-blue-950/40 dark:to-blue-900/20 border border-blue-100 dark:border-blue-800/30 transition-all hover:shadow-md">
                        <div className="flex items-center justify-between mb-2">
                          <span className="text-sm font-semibold text-blue-700 dark:text-blue-300 uppercase tracking-wide">{t('streaks.monthCheckedInDays')}</span>
                          <Calendar className="h-4 w-4 text-blue-500" />
                        </div>
                        <div className="flex items-baseline gap-1">
                          <span className="text-3xl font-bold text-blue-700 dark:text-blue-400">
                            {calendar.month_stats.checked_in_days}
                          </span>
                          <span className="text-sm font-medium text-blue-600/60 dark:text-blue-400/60">/ {calendar.month_stats.total_days}</span>
                        </div>
                      </div>

                      <div className="group relative overflow-hidden p-5 rounded-2xl bg-gradient-to-br from-green-50 to-green-100/50 dark:from-green-950/40 dark:to-green-900/20 border border-green-100 dark:border-green-800/30 transition-all hover:shadow-md">
                        <div className="flex items-center justify-between mb-2">
                          <span className="text-sm font-semibold text-green-700 dark:text-green-300 uppercase tracking-wide">{t('streaks.checkInRate')}</span>
                          <Percent className="h-4 w-4 text-green-500" />
                        </div>
                        <div className="text-3xl font-bold text-green-700 dark:text-green-400">
                          {calendar.month_stats.check_in_rate.toFixed(1)}<span className="text-lg">%</span>
                        </div>
                      </div>

                      <div className="group relative overflow-hidden p-5 rounded-2xl bg-gradient-to-br from-orange-50 to-orange-100/50 dark:from-orange-950/40 dark:to-orange-900/20 border border-orange-100 dark:border-orange-800/30 transition-all hover:shadow-md">
                        <div className="flex items-center justify-between mb-2">
                          <span className="text-sm font-semibold text-orange-700 dark:text-orange-300 uppercase tracking-wide">{t('streaks.incomeIncrement')}</span>
                          <DollarSign className="h-4 w-4 text-orange-500" />
                        </div>
                        <div className="text-3xl font-bold text-orange-700 dark:text-orange-400">
                          ${calendar.month_stats.total_income_increment.toFixed(2)}
                        </div>
                      </div>
                    </div>
                  ) : (
                    <div className="h-[200px] flex items-center justify-center text-muted-foreground">
                      {t('common.loading')}
                    </div>
                  )}
                </CardContent>
              </Card>
            </div>

            {/* Trend Chart */}
            {trend && trend.data_points.length > 0 && (
              <Card className="border-none shadow-lg bg-background/60 backdrop-blur-xl ring-1 ring-border/50">
                <CardHeader className="pb-4">
                  <div className="flex items-center gap-3">
                    <div className="p-2 rounded-xl bg-primary/10 text-primary">
                      <TrendingUp className="h-5 w-5" />
                    </div>
                    <CardTitle className="text-xl">{t('streaks.trendTitle') || "Check-in Trend (90 Days)"}</CardTitle>
                  </div>
                </CardHeader>
                <CardContent>
                  <CheckInTrendChart data={trend.data_points} />
                </CardContent>
              </Card>
            )}
          </div>
        )}
      </SidebarPageLayout>

      {/* Day Detail Dialog */}
      <CheckInDayDetailDialog
        open={!!selectedDate}
        onOpenChange={(open) => !open && setSelectedDate(null)}
        dayData={selectedDayData}
      />
    </PageContainer>
  );
}
