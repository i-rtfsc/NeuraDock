import React, { useState } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';
import { motion } from 'framer-motion';
import {
  ArrowLeft,
  RefreshCw,
  Edit,
  Trash2,
  Wallet,
  TrendingUp,
  History,
  KeyRound,
  Settings2,
  Key,
  Clock,
  Award,
  Flame,
  CalendarCheck,
  Activity
} from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Card } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Progress } from '@/components/ui/progress';
import { PageContainer } from '@/components/layout/PageContainer';
import { AccountDialog } from '@/components/account/AccountDialog';
import { CheckInCalendar } from '@/components/checkin/streak/CheckInCalendar';
import { CheckInTrendChart } from '@/components/checkin/streak/CheckInTrendChart';
import { CheckInDayDetailDialog } from '@/components/checkin/streak/CheckInDayDetailDialog';
import { ConfigDialog } from '@/components/token/ConfigDialog';
import type { Account } from '@/lib/tauri-commands';
import type { TokenDto } from '@/types/token';
import { useAccountActions } from '@/hooks/useAccountActions';
import { useCheckInCalendar, useCheckInTrend, useCheckInStreak, useCheckInDayDetail } from '@/hooks/useCheckInStreak';
import { cn } from '@/lib/utils';

export function AccountOverviewPage() {
  const { accountId } = useParams<{ accountId: string }>();
  const navigate = useNavigate();
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const { editingAccount, dialogOpen, handleEdit, handleDialogClose } = useAccountActions();

  // Token configuration state
  const [configDialogOpen, setConfigDialogOpen] = useState(false);
  const [selectedToken, setSelectedToken] = useState<TokenDto | null>(null);

  // Day detail dialog state
  const [dayDetailDialogOpen, setDayDetailDialogOpen] = useState(false);
  const [selectedDate, setSelectedDate] = useState<string>('');

  // Token refresh state
  const [tokensRefreshing, setTokensRefreshing] = useState(false);
  const tokensRefreshingRef = React.useRef(false);

  // Calendar state
  const now = new Date();
  const [calendarDate, setCalendarDate] = React.useState({
    year: now.getFullYear(),
    month: now.getMonth() + 1,
  });

  const autoRefreshedAccountIdRef = React.useRef<string | null>(null);

  // Fetch account details
  const { data: account, isLoading } = useQuery<Account>({
    queryKey: ['account', accountId],
    queryFn: async () => {
      const accounts = await invoke<Account[]>('get_all_accounts', { enabledOnly: false });
      const found = accounts.find(a => a.id === accountId);
      if (!found) throw new Error('Account not found');
      return found;
    },
    enabled: !!accountId,
  });

  // Fetch tokens
  const { data: tokens = [], isLoading: tokensLoading } = useQuery<TokenDto[]>({
    queryKey: ['tokens', accountId],
    queryFn: () =>
      invoke<TokenDto[]>('fetch_account_tokens', {
        accountId: accountId!,
        forceRefresh: false,
      }),
    enabled: !!accountId,
  });

  const refreshTokens = React.useCallback(
    async (showToast: boolean) => {
      if (!accountId || tokensRefreshingRef.current) return;
      tokensRefreshingRef.current = true;
      setTokensRefreshing(true);
      try {
        const data = await invoke<TokenDto[]>('fetch_account_tokens', {
          accountId,
          forceRefresh: true,
        });
        queryClient.setQueryData(['tokens', accountId], data);
        if (showToast) {
          toast.success(t('token.refreshSuccess', 'API keys refreshed successfully'));
        }
      } catch (error) {
        const message = error instanceof Error && error.message ? `: ${error.message}` : '';
        toast.error(`${t('token.refreshError', 'Failed to refresh API keys')}${message}`);
      } finally {
        tokensRefreshingRef.current = false;
        setTokensRefreshing(false);
      }
    },
    [accountId, queryClient, t]
  );

  React.useEffect(() => {
    if (!accountId) return;
    if (autoRefreshedAccountIdRef.current === accountId) return;
    autoRefreshedAccountIdRef.current = accountId;
    refreshTokens(false);
  }, [accountId, refreshTokens]);

  // Fetch check-in calendar and trend
  const { data: calendar } = useCheckInCalendar(
    accountId ?? '',
    calendarDate.year,
    calendarDate.month,
    !!accountId
  );
  const { data: trend } = useCheckInTrend(accountId ?? '', 30, !!accountId);
  const { data: streak } = useCheckInStreak(accountId ?? '', !!accountId);
  const { data: dayDetail } = useCheckInDayDetail(accountId ?? '', selectedDate, dayDetailDialogOpen && !!selectedDate);

  const showTokensLoading = tokensLoading || (tokensRefreshing && tokens.length === 0);
  const sortedTokens = React.useMemo(() => {
    if (!tokens.length) return tokens;
    return [...tokens].sort((a, b) => {
      const nameA = (a.name || a.masked_key || '').toLowerCase();
      const nameB = (b.name || b.masked_key || '').toLowerCase();
      return nameA.localeCompare(nameB);
    });
  }, [tokens]);

  // Refresh balance mutation
  const refreshBalanceMutation = useMutation({
    mutationFn: () =>
      invoke('fetch_account_balance', {
        accountId: accountId!,
        forceRefresh: true,
      }),
    onSuccess: () => {
      toast.success(t('accountCard.balanceRefreshed'));
      queryClient.invalidateQueries({ queryKey: ['account', accountId] });
    },
    onError: (error: Error) => {
      toast.error(error.message);
    },
  });

  // Delete mutation
  const deleteMutation = useMutation({
    mutationFn: () => invoke('delete_account', { accountId: accountId! }),
    onSuccess: () => {
      toast.success(t('accountCard.deleted', '账号已删除'));
      navigate('/accounts');
    },
    onError: (error: Error) => {
      toast.error(error.message);
    },
  });

  if (isLoading) {
    return (
      <PageContainer>
        <div className="flex items-center justify-center h-64">
          <RefreshCw className="h-8 w-8 animate-spin text-muted-foreground" />
        </div>
      </PageContainer>
    );
  }

  if (!account) {
    return (
      <PageContainer>
        <div className="flex flex-col items-center justify-center h-64 gap-4">
          <p className="text-lg font-semibold">{t('accounts.accountNotFound', '账号不存在')}</p>
          <Button onClick={() => navigate('/accounts')}>
            {t('common.back')}
          </Button>
        </div>
      </PageContainer>
    );
  }

  const handleConfigureToken = (token: TokenDto) => {
    setSelectedToken(token);
    setConfigDialogOpen(true);
  };

  const handleDayClick = (date: string) => {
    setSelectedDate(date);
    setDayDetailDialogOpen(true);
  };

  // Format quota as USD currency
  const formatQuotaUSD = (quota: number): string => {
    const dollars = quota / 500000;
    return `$${dollars.toFixed(2)}`;
  };

  // Shared card classes
  const cardClass = "card-vivid group p-6";

  return (
    <PageContainer
      className="h-full flex flex-col overflow-hidden"
      title={
        <div className="flex items-center gap-3">
          <Button
            variant="ghost"
            size="icon"
            onClick={() => navigate('/accounts')}
            className="shrink-0 h-9 w-9 rounded-full hover:bg-muted"
          >
            <ArrowLeft className="h-5 w-5" />
          </Button>
          <div className="flex items-center gap-3">
            <span className="text-xl font-bold tracking-tight">{account.name}</span>
            <Badge variant="outline" className="text-muted-foreground">{account.provider_name}</Badge>
            {account.auto_checkin_enabled && (
              <Badge variant="secondary" className="gap-1.5 text-xs bg-success-soft text-success border-0">
                <span className="relative flex h-1.5 w-1.5">
                  <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-success/40 opacity-75"></span>
                  <span className="relative inline-flex rounded-full h-1.5 w-1.5 bg-success"></span>
                </span>
                {t('accountOverview.auto', 'Auto')} {String(account.auto_checkin_hour).padStart(2, '0')}:
                {String(account.auto_checkin_minute).padStart(2, '0')}
              </Badge>
            )}
          </div>
        </div>
      }
      actions={
        <div className="flex items-center gap-2">
          <Button
            variant="outline"
            size="sm"
            onClick={() => refreshBalanceMutation.mutate()}
            disabled={refreshBalanceMutation.isPending}
            className="shadow-sm"
          >
            <RefreshCw className={cn("mr-2 h-4 w-4", refreshBalanceMutation.isPending && "animate-spin")} />
            {t('accountCard.refreshBalance')}
          </Button>
          <Button
            variant="outline"
            size="sm"
            onClick={() => handleEdit(account)}
            className="shadow-sm"
          >
            <Edit className="mr-2 h-4 w-4" />
            {t('accountCard.edit')}
          </Button>
          <Button
            variant="destructive"
            size="sm"
            onClick={() => {
              if (window.confirm(t('accountCard.deleteWarning'))) {
                deleteMutation.mutate();
              }
            }}
            disabled={deleteMutation.isPending}
            className="shadow-sm"
          >
            <Trash2 className="mr-2 h-4 w-4" />
            {t('accountCard.delete')}
          </Button>
        </div>
      }
    >
      <motion.div 
        className="flex-1 overflow-auto space-y-8 pb-8 pt-2 px-2 auto-hide-scrollbar"
        initial={{ opacity: 0, x: 20 }}
        animate={{ opacity: 1, x: 0 }}
        transition={{ duration: 0.3, ease: "easeOut" }}
      >
        {/* API Key Configuration Section */}
        <section className="space-y-4">
          <div className="flex items-center justify-between px-1">
            <h2 className="text-xl font-bold flex items-center gap-3 tracking-tight">
              <div className="p-2.5 rounded-2xl bg-primary/10 text-primary shadow-sm border border-primary/5">
                <KeyRound className="h-5 w-5" />
              </div>
              {t('accountOverview.apiKeyConfig', 'API Key Configuration')}
            </h2>
            <Button
              variant="outline"
              size="sm"
              onClick={() => refreshTokens(true)}
              disabled={tokensRefreshing || !accountId}
              className="shadow-sm rounded-xl h-9 hover:bg-primary/5 hover:text-primary hover:border-primary/30 transition-all"
            >
              <RefreshCw className={cn("mr-2 h-4 w-4", tokensRefreshing && "animate-spin")} />
              {tokensRefreshing
                ? t('token.refreshing', 'Refreshing...')
                : t('token.refreshTokens', 'Refresh API Keys')}
            </Button>
          </div>

          {showTokensLoading ? (
            <div className="flex items-center justify-center py-20 bg-muted/5 rounded-[var(--radius-panel)] border border-dashed border-border/60">
              <RefreshCw className="h-8 w-8 animate-spin text-primary/40" />
            </div>
          ) : tokens.length === 0 ? (
            <Card className="py-20 text-center border-dashed bg-muted/5 shadow-none rounded-[var(--radius-panel)]">
              <KeyRound className="h-12 w-12 mx-auto text-muted-foreground/20 mb-4" />
              <p className="text-lg font-medium text-muted-foreground/80">{t('token.noTokens')}</p>
              <p className="text-sm text-muted-foreground/50 max-w-sm mx-auto mt-1">
                {t('accountOverview.tokenHint', 'API keys are refreshed automatically when you open this page. You can also refresh manually.')}
              </p>
            </Card>
          ) : (
            <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4">
              {sortedTokens.map((token) => (
                <Card
                  key={token.id}
                  className={cn(
                    "card-vivid group flex flex-col p-5 border-border/40 shadow-sm",
                    !token.is_active && "opacity-60 grayscale"
                  )}
                >
                  <div className="mb-4">
                    <div className="flex items-start justify-between gap-2 mb-2">
                      <h3 className="text-base font-extrabold truncate flex-1 group-hover:text-primary transition-colors" title={token.name}>
                        {token.name}
                      </h3>
                      <Badge
                        variant={token.is_active ? 'soft-primary' : 'secondary'}
                        className="flex-shrink-0 rounded-full px-2 py-0.5 text-[10px] h-5 font-bold uppercase tracking-wider"
                      >
                        {token.status_text}
                      </Badge>
                    </div>
                    <div className="flex items-center gap-1.5 text-xs font-mono text-muted-foreground bg-muted/40 w-fit px-2.5 py-1.5 rounded-xl border border-border/30 group-hover:border-primary/20 group-hover:bg-primary/5 transition-all">
                      <Key className="h-3 w-3 text-primary/60" />
                      {token.masked_key}
                    </div>
                  </div>

                  <div className="flex-1 flex flex-col gap-5">
                    {/* Quota Usage Section */}
                    <div className="flex-1 space-y-4">
                      {token.unlimited_quota ? (
                        <div className="p-4 rounded-2xl bg-success-soft/50 border border-success-border/30">
                          <div className="flex items-center justify-between mb-1.5">
                            <span className="text-xs font-bold text-success-soft-foreground/80">{t('token.quotaUnlimited', 'Unlimited')}</span>
                            <Badge variant="outline" className="text-[10px] h-4 px-1.5 text-success border-success/30 bg-success/5">
                              ∞
                            </Badge>
                          </div>
                          <div className="text-xs text-success-soft-foreground/60">
                            <span>{t('token.usedQuota', 'Used')}: </span>
                            <span className="font-mono font-bold text-success">{formatQuotaUSD(token.used_quota)}</span>
                          </div>
                        </div>
                      ) : (
                        <div className="space-y-2.5">
                          <div className="flex justify-between text-xs px-0.5">
                            <span className="text-muted-foreground font-medium">{t('token.quotaUsage', 'Quota Usage')}</span>
                            <span className={cn(
                              "font-black tracking-tighter",
                              token.usage_percentage > 90 ? "text-danger" : "text-primary"
                            )}>{token.usage_percentage.toFixed(1)}%</span>
                          </div>
                          <Progress
                            value={token.usage_percentage}
                            className="h-2 rounded-full bg-muted/50"
                            indicatorClassName={cn(
                              "transition-all duration-1000",
                              token.usage_percentage > 90 ? "bg-danger" :
                              token.usage_percentage > 75 ? "bg-warning" : "bg-primary"
                            )}
                          />
                          <div className="flex justify-between text-[10px] text-muted-foreground/70 font-medium">
                            <div className="flex items-center gap-1">
                              <span>{t('token.usedQuota', 'Used')}:</span>
                              <span className="font-mono font-bold text-foreground/80">{formatQuotaUSD(token.used_quota)}</span>
                            </div>
                            <div className="flex items-center gap-1">
                              <span>{t('token.remainQuota', 'Remain')}:</span>
                              <span className="font-mono font-bold text-foreground/80">{formatQuotaUSD(token.remain_quota)}</span>
                            </div>
                          </div>
                        </div>
                      )}

                      <div className="grid grid-cols-2 gap-2.5">
                        {/* Expiration */}
                        {token.expired_at && (
                          <div className="flex flex-col gap-1 p-2.5 rounded-xl bg-muted/20 border border-border/20">
                            <span className="text-[9px] text-muted-foreground/60 uppercase font-black tracking-widest">{t('token.expiresAt', 'Expires')}</span>
                            <div className="flex items-center gap-1.5 text-[11px] font-bold">
                              <Clock className="h-3 w-3 text-primary/60" />
                              {new Date(token.expired_at).toLocaleDateString()}
                            </div>
                          </div>
                        )}

                        {/* Model Limits */}
                        <div className={cn("flex flex-col gap-1 p-2.5 rounded-xl bg-muted/20 border border-border/20", !token.expired_at && "col-span-2")}>
                          <span className="text-[9px] text-muted-foreground/60 uppercase font-black tracking-widest">{t('token.supportedModels', 'Models')}</span>
                          <div className="text-[11px] font-bold truncate">
                            {!token.model_limits_enabled ? (
                              <span className="text-success-soft-foreground">
                                {t('token.noLimits', 'Unrestricted')}
                              </span>
                            ) : token.model_limits_allowed.length > 0 ? (
                              <span title={token.model_limits_allowed.join(', ')}>
                                {token.model_limits_allowed.slice(0, 2).join(', ')}
                                {token.model_limits_allowed.length > 2 && ` +${token.model_limits_allowed.length - 2}`}
                              </span>
                            ) : (
                              <span className="text-muted-foreground/50">
                                {t('token.noModelsConfigured', 'None')}
                              </span>
                            )}
                          </div>
                        </div>
                      </div>
                    </div>

                    {/* Configure Button */}
                    <Button
                      variant="outline"
                      size="sm"
                      className="w-full mt-auto text-xs font-black rounded-xl h-10 border-primary/20 hover:border-primary hover:bg-primary/5 hover:text-primary hover:scale-[1.02] shadow-sm active:scale-95 transition-all uppercase tracking-tight"
                      onClick={() => handleConfigureToken(token)}
                      disabled={!token.is_active}
                    >
                      <Settings2 className="mr-2 h-4 w-4" />
                      {t('token.configureAI', 'Configure AI Tool')}
                    </Button>
                  </div>
                </Card>
              ))}
            </div>
          )}
        </section>

        {/* Statistics & Calendar */}
        <div className="space-y-6">
          <div className="grid grid-cols-1 lg:grid-cols-4 gap-6 items-stretch">
            {/* Account Statistics Card */}
            <Card className={cn("lg:col-span-1 p-6 h-full flex flex-col", cardClass)}>
              <h3 className="text-lg font-bold mb-4 flex items-center gap-2">
                <div className="p-1.5 rounded-md bg-info/10 text-info">
                  <Activity className="h-4 w-4" />
                </div>
                {t('accountOverview.accountStatistics', 'Account Statistics')}
              </h3>
              <div className="space-y-4">
                {/* Current Balance */}
                <div className="space-y-2">
                  <div className="flex items-center gap-2 text-sm text-muted-foreground">
                    <div className="p-1.5 rounded-md bg-success-soft text-success">
                      <Wallet className="h-4 w-4" />
                    </div>
                    <span>{t('accountCard.currentBalance')}</span>
                  </div>
                  <p className="text-2xl font-bold font-mono text-success">
                    ${account.current_balance?.toFixed(2) ?? '0.00'}
                  </p>
                </div>

                <div className="h-px bg-border" />

                {/* Total Quota */}
                <div className="space-y-2">
                  <div className="flex items-center gap-2 text-sm text-muted-foreground">
                    <div className="p-1.5 rounded-md bg-info-soft text-info">
                      <TrendingUp className="h-4 w-4" />
                    </div>
                    <span>{t('accountCard.totalQuota')}</span>
                  </div>
                  <p className="text-2xl font-bold font-mono text-info">
                    ${account.total_quota?.toFixed(2) ?? '0.00'}
                  </p>
                </div>

                <div className="h-px bg-border" />

                {/* Historical Consumption */}
                <div className="space-y-2">
                  <div className="flex items-center gap-2 text-sm text-muted-foreground">
                    <div className="p-1.5 rounded-md bg-warning-soft text-warning">
                      <History className="h-4 w-4" />
                    </div>
                    <span>{t('accountCard.historicalConsumption')}</span>
                  </div>
                  <p className="text-2xl font-bold font-mono text-warning">
                    ${account.total_consumed?.toFixed(2) ?? '0.00'}
                  </p>
                </div>
              </div>
            </Card>

            {/* Check-in Statistics Card */}
            <Card className={cn("lg:col-span-1 p-6 h-full flex flex-col", cardClass)}>
              <h3 className="text-lg font-bold mb-4 flex items-center gap-2">
                <div className="p-1.5 rounded-md bg-warning/10 text-warning">
                  <Flame className="h-4 w-4" />
                </div>
                {t('accountOverview.checkInStatistics', 'Check-in Statistics')}
              </h3>
              <div className="space-y-4">
                {/* Current Streak */}
                <div className="space-y-2">
                  <div className="flex items-center gap-2 text-sm text-muted-foreground">
                    <div className="p-1.5 rounded-md bg-warning-soft text-warning">
                      <Flame className="h-4 w-4" />
                    </div>
                    <span>{t('streaks.currentStreak')}</span>
                  </div>
                  <p className="text-2xl font-bold font-mono text-warning">
                    {streak?.current_streak ?? 0} {t('streaks.daysUnit')}
                  </p>
                </div>

                <div className="h-px bg-border" />

                {/* Longest Streak */}
                <div className="space-y-2">
                  <div className="flex items-center gap-2 text-sm text-muted-foreground">
                    <div className="p-1.5 rounded-md bg-accent-2-soft text-accent-2">
                      <Award className="h-4 w-4" />
                    </div>
                    <span>{t('streaks.longestStreak')}</span>
                  </div>
                  <p className="text-2xl font-bold font-mono text-accent-2">
                    {streak?.longest_streak ?? 0} {t('streaks.daysUnit')}
                  </p>
                </div>

                <div className="h-px bg-border" />

                {/* Total Days */}
                <div className="space-y-2">
                  <div className="flex items-center gap-2 text-sm text-muted-foreground">
                    <div className="p-1.5 rounded-md bg-info-soft text-info">
                      <CalendarCheck className="h-4 w-4" />
                    </div>
                    <span>{t('streaks.totalDays')}</span>
                  </div>
                  <p className="text-2xl font-bold font-mono text-info">
                    {streak?.total_check_in_days ?? 0} {t('streaks.daysUnit')}
                  </p>
                </div>

                {calendar?.month_stats && (
                  <>
                    <div className="h-px bg-border" />

                    {/* Month Check-in Rate */}
                    <div className="space-y-2">
                      <div className="flex items-center justify-between text-sm">
                        <span className="text-muted-foreground">{t('streaks.checkInRate')}</span>
                        <span className="font-medium">{calendar.month_stats.check_in_rate.toFixed(1)}%</span>
                      </div>
                      <Progress
                        value={calendar.month_stats.check_in_rate}
                        className="h-2"
                      />
                      <p className="text-xs text-muted-foreground">
                        {calendar.month_stats.checked_in_days} / {calendar.month_stats.total_days} {t('streaks.daysUnit')}
                      </p>
                    </div>
                  </>
                )}
              </div>
            </Card>

            {/* Calendar */}
            <div className="lg:col-span-2">
              {calendar && (
                <Card className={cn("p-6 h-full", cardClass)}>
                  <CheckInCalendar
                    year={calendar.year}
                    month={calendar.month}
                    days={calendar.days}
                    onDateClick={handleDayClick}
                    onMonthChange={(year, month) => setCalendarDate({ year, month })}
                    variant="inline"
                  />
                </Card>
              )}
            </div>
          </div>

          {/* Check-in Trends */}
          {trend && (
            <div>
              <h2 className="text-xl font-bold mb-4 flex items-center gap-2">
                <TrendingUp className="h-5 w-5 text-info" />
                {t('accountOverview.checkInTrends', 'Check-in Trends')}
              </h2>
              <Card className={cn("p-6", cardClass)}>
                <CheckInTrendChart data={trend.data_points} />
              </Card>
            </div>
          )}
        </div>
      </motion.div>

      {/* Edit Dialog */}
      <AccountDialog
        open={dialogOpen}
        onOpenChange={handleDialogClose}
        mode="edit"
        accountId={editingAccount?.id ?? account.id}
        defaultValues={{
          name: (editingAccount ?? account).name,
          provider_id: (editingAccount ?? account).provider_id,
          cookies: editingAccount?.cookies,
          api_user: editingAccount?.api_user,
          auto_checkin_enabled: (editingAccount ?? account).auto_checkin_enabled,
          auto_checkin_hour: (editingAccount ?? account).auto_checkin_hour,
          auto_checkin_minute: (editingAccount ?? account).auto_checkin_minute,
          check_in_interval_hours: (editingAccount ?? account).check_in_interval_hours,
        }}
      />

      {/* Config Dialog for Tokens */}
      {selectedToken && (
        <ConfigDialog
          open={configDialogOpen}
          onOpenChange={setConfigDialogOpen}
          token={selectedToken}
          account={account}
        />
      )}

      {/* Day Detail Dialog */}
      <CheckInDayDetailDialog
        open={dayDetailDialogOpen}
        onOpenChange={setDayDetailDialogOpen}
        dayData={dayDetail ?? null}
      />
    </PageContainer>
  );
}
