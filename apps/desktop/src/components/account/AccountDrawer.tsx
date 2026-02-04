import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
} from '@/components/ui/sheet';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Button } from '@/components/ui/button';
import { Card } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { ScrollArea } from '@/components/ui/scroll-area';
import {
  Activity,
  Wallet,
  TrendingUp,
  History,
  CheckCircle2,
  XCircle,
  AlertCircle,
  RefreshCw,
  Edit,
  Settings as SettingsIcon,
  KeyRound,
  Calendar,
} from 'lucide-react';
import { Account } from '@/lib/tauri-commands';
import type { TokenDto } from '@/types/token';
import { toast } from 'sonner';

interface AccountDrawerProps {
  account: Account | null;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onEdit?: (account: Account) => void;
  onCheckIn?: (accountId: string) => void;
  onRefreshBalance?: (accountId: string) => void;
}

export function AccountDrawer({
  account,
  open,
  onOpenChange,
  onEdit,
  onCheckIn,
  onRefreshBalance,
}: AccountDrawerProps) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [activeTab, setActiveTab] = useState('overview');

  // Fetch tokens when account is selected
  const { data: tokens = [], isLoading: tokensLoading } = useQuery<TokenDto[]>({
    queryKey: ['tokens', account?.id],
    queryFn: () =>
      invoke<TokenDto[]>('fetch_account_tokens', {
        accountId: account!.id,
        forceRefresh: false,
      }),
    enabled: !!account && open && activeTab === 'tokens',
    staleTime: 0,
  });

  // Check-in mutation
  const checkInMutation = useMutation({
    mutationFn: (accountId: string) =>
      invoke('execute_check_in', { accountId }),
    onSuccess: () => {
      toast.success(t('checkIn.success'));
      queryClient.invalidateQueries({ queryKey: ['accounts'] });
    },
    onError: (error: Error) => {
      toast.error(error.message || t('checkIn.failed'));
    },
  });

  // Refresh balance mutation
  const refreshBalanceMutation = useMutation({
    mutationFn: (accountId: string) =>
      invoke('fetch_account_balance', {
        accountId,
        forceRefresh: true,
      }),
    onSuccess: () => {
      toast.success(t('accountCard.balanceRefreshed'));
      queryClient.invalidateQueries({ queryKey: ['accounts'] });
    },
    onError: (error: Error) => {
      toast.error(error.message);
    },
  });

  if (!account) return null;

  const getStatusIcon = () => {
    if (!account.enabled) {
      return <XCircle className="h-5 w-5 text-muted-foreground" />;
    }

    if (account.session_expires_at) {
      const daysUntilExpiry =
        account.session_days_remaining ??
        (() => {
          const ms = Date.parse(account.session_expires_at ?? '');
          if (Number.isNaN(ms)) return null;
          return (ms - Date.now()) / (1000 * 60 * 60 * 24);
        })();

      if (daysUntilExpiry !== null && daysUntilExpiry < 0) {
        return <AlertCircle className="h-5 w-5 text-destructive" />;
      } else if (daysUntilExpiry !== null && daysUntilExpiry < 3) {
        return <AlertCircle className="h-5 w-5 text-warning" />;
      }
    }

    return <CheckCircle2 className="h-5 w-5 text-success" />;
  };

  const getStatusText = () => {
    if (!account.enabled) {
      return t('accountCard.disabled');
    }

    if (account.session_expires_at) {
      const daysUntilExpiry =
        account.session_days_remaining ??
        (() => {
          const ms = Date.parse(account.session_expires_at ?? '');
          if (Number.isNaN(ms)) return null;
          return (ms - Date.now()) / (1000 * 60 * 60 * 24);
        })();

      if (daysUntilExpiry !== null && daysUntilExpiry < 0) {
        return t('accountCard.sessionExpired');
      } else if (daysUntilExpiry !== null && daysUntilExpiry < 3) {
        return t('accountCard.sessionExpiresSoon', { days: Math.floor(daysUntilExpiry) });
      }
      if (daysUntilExpiry !== null) {
        return t('accountCard.sessionValidDays', { days: Math.floor(daysUntilExpiry) });
      }
    }

    return t('accountCard.active');
  };

  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent className="w-full sm:max-w-[600px] p-0 flex flex-col">
        <SheetHeader className="px-6 pt-6 pb-4 border-b">
          <div className="flex items-start justify-between gap-4">
            <div className="flex-1 min-w-0">
              <SheetTitle className="text-xl font-bold truncate">{account.name}</SheetTitle>
              <SheetDescription className="flex items-center gap-2 mt-1">
                <Badge variant="outline">{account.provider_name}</Badge>
                {account.auto_checkin_enabled && (
                  <span className="text-xs text-muted-foreground">
                    🔄 {String(account.auto_checkin_hour).padStart(2, '0')}:
                    {String(account.auto_checkin_minute).padStart(2, '0')}
                  </span>
                )}
              </SheetDescription>
            </div>
          </div>
        </SheetHeader>

        <Tabs value={activeTab} onValueChange={setActiveTab} className="flex-1 flex flex-col">
          <div className="px-6 pt-4">
            <TabsList className="grid w-full grid-cols-4">
              <TabsTrigger value="overview" className="text-xs sm:text-sm">
                <Activity className="h-4 w-4 sm:mr-2" />
                <span className="hidden sm:inline">{t('management.overview', '概览')}</span>
              </TabsTrigger>
              <TabsTrigger value="tokens" className="text-xs sm:text-sm">
                <KeyRound className="h-4 w-4 sm:mr-2" />
                <span className="hidden sm:inline">{t('management.tokens', 'Token')}</span>
              </TabsTrigger>
              <TabsTrigger value="records" className="text-xs sm:text-sm">
                <Calendar className="h-4 w-4 sm:mr-2" />
                <span className="hidden sm:inline">{t('management.records', '记录')}</span>
              </TabsTrigger>
              <TabsTrigger value="settings" className="text-xs sm:text-sm">
                <SettingsIcon className="h-4 w-4 sm:mr-2" />
                <span className="hidden sm:inline">{t('management.settings', '设置')}</span>
              </TabsTrigger>
            </TabsList>
          </div>

          <ScrollArea className="flex-1 px-6">
            {/* Overview Tab */}
            <TabsContent value="overview" className="mt-4 space-y-4">
              {/* Status Card */}
              <Card className="p-4 border-border/50">
                <h3 className="text-sm font-semibold mb-3 flex items-center gap-2">
                  <Activity className="h-4 w-4" />
                  {t('management.accountStatus', '账号状态')}
                </h3>
                <div className="flex items-center gap-3">
                  {getStatusIcon()}
                  <div>
                    <p className="text-sm font-medium">{getStatusText()}</p>
                    {account.auto_checkin_enabled && (
                      <p className="text-xs text-muted-foreground">
                        {t('accountCard.autoAt')} {String(account.auto_checkin_hour).padStart(2, '0')}:
                        {String(account.auto_checkin_minute).padStart(2, '0')}
                      </p>
                    )}
                  </div>
                </div>
              </Card>

              {/* Balance Statistics */}
              <div className="grid grid-cols-3 gap-3">
                <Card className="p-4 border-border/50 bg-gradient-to-br from-success-soft to-transparent">
                  <div className="flex items-center gap-2 text-muted-foreground mb-2">
                    <Wallet className="h-4 w-4 text-success" />
                    <span className="text-xs font-medium">{t('accountCard.currentBalance')}</span>
                  </div>
                  <p className="text-xl font-bold font-mono">
                    ${(account.current_balance ?? 0).toFixed(2)}
                  </p>
                </Card>

                <Card className="p-4 border-border/50 bg-gradient-to-br from-info-soft to-transparent">
                  <div className="flex items-center gap-2 text-muted-foreground mb-2">
                    <TrendingUp className="h-4 w-4 text-info" />
                    <span className="text-xs font-medium">{t('accountCard.totalQuota')}</span>
                  </div>
                  <p className="text-xl font-bold font-mono">
                    ${account.total_quota?.toFixed(2) ?? '0.00'}
                  </p>
                </Card>

                <Card className="p-4 border-border/50 bg-gradient-to-br from-warning-soft to-transparent">
                  <div className="flex items-center gap-2 text-muted-foreground mb-2">
                    <History className="h-4 w-4 text-warning" />
                    <span className="text-xs font-medium">{t('accountCard.historicalConsumption')}</span>
                  </div>
                  <p className="text-xl font-bold font-mono">
                    ${account.total_consumed?.toFixed(2) ?? '0.00'}
                  </p>
                </Card>
              </div>

              {/* Quick Actions */}
              <Card className="p-4 border-border/50">
                <h3 className="text-sm font-semibold mb-3">{t('management.quickActions', '快速操作')}</h3>
                <div className="flex flex-wrap gap-2">
                  <Button
                    onClick={() => {
                      if (onCheckIn) {
                        onCheckIn(account.id);
                        checkInMutation.mutate(account.id);
                      }
                    }}
                    disabled={!account.enabled || checkInMutation.isPending}
                    className="flex-1"
                  >
                    {checkInMutation.isPending ? (
                      <RefreshCw className="mr-2 h-4 w-4 animate-spin" />
                    ) : null}
                    {t('checkIn.checkIn')}
                  </Button>
                  <Button
                    variant="outline"
                    onClick={() => {
                      if (onRefreshBalance) {
                        onRefreshBalance(account.id);
                        refreshBalanceMutation.mutate(account.id);
                      }
                    }}
                    disabled={refreshBalanceMutation.isPending}
                    className="flex-1"
                  >
                    {refreshBalanceMutation.isPending ? (
                      <RefreshCw className="mr-2 h-4 w-4 animate-spin" />
                    ) : (
                      <RefreshCw className="mr-2 h-4 w-4" />
                    )}
                    {t('accountCard.balance')}
                  </Button>
                  <Button
                    variant="outline"
                    onClick={() => onEdit?.(account)}
                  >
                    <Edit className="mr-2 h-4 w-4" />
                    {t('accountCard.edit')}
                  </Button>
                </div>
              </Card>
            </TabsContent>

            {/* Tokens Tab */}
            <TabsContent value="tokens" className="mt-4 space-y-4">
              <Card className="p-4 border-border/50">
                <div className="flex items-center justify-between mb-4">
                  <h3 className="text-sm font-semibold flex items-center gap-2">
                    <KeyRound className="h-4 w-4" />
                    {t('management.tokenManagement', 'Token 管理')}
                  </h3>
                  {!tokensLoading && (
                    <Badge variant="secondary">{tokens.length} {t('management.available', '可用')}</Badge>
                  )}
                </div>

                {tokensLoading ? (
                  <div className="flex items-center justify-center py-8">
                    <RefreshCw className="h-6 w-6 animate-spin text-muted-foreground" />
                  </div>
                ) : tokens.length === 0 ? (
                  <div className="text-center py-8 text-sm text-muted-foreground">
                    {t('token.noTokens')}
                  </div>
                ) : (
                  <div className="space-y-3">
                    {tokens.map((token) => (
                      <Card key={token.id} className="p-3 border-border/50 bg-muted/30">
                        <div className="flex items-start justify-between gap-2">
                          <div className="flex-1 min-w-0">
                            <p className="font-mono text-sm truncate">{token.name || token.id}</p>
                            <div className="flex items-center gap-2 mt-1">
                              <span className="text-xs text-muted-foreground">
                                $
                                {token.unlimited_quota
                                  ? '∞'
                                  : token.remain_quota.toFixed(2)}{' '}
                                {t('token.remaining')}
                              </span>
                              {token.status === 1 ? (
                                <Badge variant="outline" className="text-xs">
                                  {t('token.active')}
                                </Badge>
                              ) : (
                                <Badge variant="secondary" className="text-xs">
                                  {t('token.disabled')}
                                </Badge>
                              )}
                            </div>
                          </div>
                          <Button size="sm" variant="outline" className="shrink-0">
                            {t('token.configure')}
                          </Button>
                        </div>
                      </Card>
                    ))}
                  </div>
                )}
              </Card>
            </TabsContent>

            {/* Records Tab */}
            <TabsContent value="records" className="mt-4 space-y-4">
              <Card className="p-4 border-border/50">
                <h3 className="text-sm font-semibold mb-3">{t('management.checkInRecords', '签到记录')}</h3>
                <div className="text-center py-8 text-sm text-muted-foreground">
                  {t('management.viewFullRecords', '点击查看完整签到日历')}
                </div>
              </Card>
            </TabsContent>

            {/* Settings Tab */}
            <TabsContent value="settings" className="mt-4 space-y-4">
              <Card className="p-4 border-border/50">
                <h3 className="text-sm font-semibold mb-3">{t('management.accountSettings', '账号设置')}</h3>
                <div className="text-center py-8 text-sm text-muted-foreground">
                  {t('management.editInDialog', '请使用编辑按钮修改账号设置')}
                </div>
              </Card>
            </TabsContent>
          </ScrollArea>
        </Tabs>
      </SheetContent>
    </Sheet>
  );
}
