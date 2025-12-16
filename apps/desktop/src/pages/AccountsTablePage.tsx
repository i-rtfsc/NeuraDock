import { useState, useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { 
  Plus, 
  Upload, 
  Search, 
  Download, 
  Wallet, 
  TrendingUp, 
  History, 
  Layers, 
  Box, 
  Calendar, 
  RefreshCw
} from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Badge } from '@/components/ui/badge';
import { Card } from '@/components/ui/card';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from '@/components/ui/tooltip';
import { toast } from 'sonner';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';

import { useAccounts } from '@/hooks/useAccounts';
import { useProviders } from '@/hooks/useProviders';
import { useAccountActions } from '@/hooks/useAccountActions';
import { AccountsTable } from '@/components/account/AccountsTable';
import { AccountDialog } from '@/components/account/AccountDialog';
import { JsonImportDialog } from '@/components/account/JsonImportDialog';
import { BatchUpdateDialog } from '@/components/account/BatchUpdateDialog';
import { PageContainer } from '@/components/layout/PageContainer';
import { Account } from '@/lib/tauri-commands';
import { cn } from '@/lib/utils';

export function AccountsTablePage() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const { data: accounts = [], isLoading } = useAccounts();
  const { data: providers } = useProviders();
  const {
    editingAccount,
    dialogOpen: accountDialogOpen,
    handleEdit,
    handleCreate,
    handleDialogClose,
  } = useAccountActions();

  const [searchQuery, setSearchQuery] = useState('');
  const [providerFilter, setProviderFilter] = useState<string>('all');
  const [jsonImportDialogOpen, setJsonImportDialogOpen] = useState(false);
  const [batchUpdateDialogOpen, setBatchUpdateDialogOpen] = useState(false);
  const [checkingInIds, setCheckingInIds] = useState<Set<string>>(new Set());
  const [sortConfig, setSortConfig] = useState<{ key: keyof Account; direction: 'asc' | 'desc' } | null>(null);

  // Get unique providers from accounts
  const allProviders = useMemo(() => {
    if (providers && providers.length > 0) {
      return providers.map((provider) => ({
        id: provider.id,
        name: provider.name,
      }));
    }

    if (!accounts) return [];
    const uniqueIds = new Set(accounts.map((a) => a.provider_id));
    return Array.from(uniqueIds).map((id) => {
      const account = accounts.find((a) => a.provider_id === id);
      return {
        id,
        name: account?.provider_name || 'Unknown',
      };
    });
  }, [accounts, providers]);

  // Filter accounts
  const filteredAccounts = useMemo(() => {
    if (!accounts) return [];
    let result = accounts;

    // Filter by search query
    if (searchQuery) {
      const query = searchQuery.toLowerCase();
      result = result.filter(
        (account) =>
          account.name.toLowerCase().includes(query) ||
          account.provider_name.toLowerCase().includes(query)
      );
    }

    // Filter by provider
    if (providerFilter !== 'all') {
      result = result.filter((a) => a.provider_id === providerFilter);
    }

    // Sort
    if (sortConfig) {
      result = [...result].sort((a, b) => {
        const aValue = a[sortConfig.key];
        const bValue = b[sortConfig.key];

        if (aValue === bValue) return 0;
        if (aValue === undefined || aValue === null) return 1;
        if (bValue === undefined || bValue === null) return -1;

        if (aValue < bValue) {
          return sortConfig.direction === 'asc' ? -1 : 1;
        }
        if (aValue > bValue) {
          return sortConfig.direction === 'asc' ? 1 : -1;
        }
        return 0;
      });
    }

    return result;
  }, [accounts, searchQuery, providerFilter, sortConfig]);

  // Calculate filtered statistics based on actual filtered accounts
  const filteredStatistics = useMemo(() => {
    if (!filteredAccounts || filteredAccounts.length === 0) return null;

    // Calculate statistics from filtered accounts
    const total_current_balance = filteredAccounts.reduce(
      (sum, acc) => sum + (acc.current_balance || 0),
      0
    );
    const total_income = filteredAccounts.reduce(
      (sum, acc) => sum + (acc.total_income || 0),
      0
    );
    const total_consumed = filteredAccounts.reduce(
      (sum, acc) => sum + (acc.total_consumed || 0),
      0
    );

    return {
      total_current_balance,
      total_income,
      total_consumed,
    };
  }, [filteredAccounts]);

  // Batch check-in mutation
  const batchCheckInMutation = useMutation({
    mutationFn: async (accountIds: string[]) => {
      const results = await Promise.allSettled(
        accountIds.map((id) => invoke('execute_check_in', { accountId: id }))
      );
      const succeeded = results.filter((r) => r.status === 'fulfilled').length;
      const failed = results.filter((r) => r.status === 'rejected').length;
      return { succeeded, failed, total: accountIds.length };
    },
    onSuccess: ({ succeeded, failed, total }) => {
      if (failed === 0) {
        toast.success(t('checkIn.batchSuccess', `成功签到 ${succeeded} 个账号`));
      } else {
        toast.warning(
          t('checkIn.batchPartial', `成功 ${succeeded} 个，失败 ${failed} 个，共 ${total} 个`)
        );
      }
      queryClient.invalidateQueries({ queryKey: ['accounts'] });
    },
    onError: (error: Error) => {
      toast.error(error.message);
    },
  });

  // Single check-in mutation
  const checkInMutation = useMutation({
    mutationFn: (accountId: string) =>
      invoke('execute_check_in', { accountId }),
    onMutate: (accountId) => {
      setCheckingInIds((prev) => new Set(prev).add(accountId));
    },
    onSettled: (_, __, accountId) => {
      setCheckingInIds((prev) => {
        const next = new Set(prev);
        next.delete(accountId);
        return next;
      });
    },
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

  // Batch refresh balances
  const batchRefreshMutation = useMutation({
    mutationFn: async (accountIds: string[]) => {
      await Promise.all(
        accountIds.map((id) =>
          invoke('fetch_account_balance', {
            accountId: id,
            forceRefresh: true,
          })
        )
      );
    },
    onSuccess: () => {
      toast.success(t('accounts.balancesRefreshed'));
      queryClient.invalidateQueries({ queryKey: ['accounts'] });
    },
    onError: (error: Error) => {
      toast.error(error.message);
    },
  });

  // Toggle account enabled
  const toggleMutation = useMutation({
    mutationFn: (account: Account) =>
      invoke('toggle_account_enabled', {
        accountId: account.id,
        enabled: !account.enabled,
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['accounts'] });
    },
    onError: (error: Error) => {
      toast.error(error.message);
    },
  });

  // Delete account
  const deleteMutation = useMutation({
    mutationFn: (accountId: string) =>
      invoke('delete_account', { accountId }),
    onSuccess: () => {
      toast.success(t('accountCard.deleted', '账号已删除'));
      queryClient.invalidateQueries({ queryKey: ['accounts'] });
    },
    onError: (error: Error) => {
      toast.error(error.message);
    },
  });

  const handleAccountClick = (account: Account) => {
    navigate(`/accounts/${account.id}`);
  };

  const handleBatchCheckIn = () => {
    const enabledIds = filteredAccounts
      .filter((a) => a.enabled)
      .map((a) => a.id);

    if (enabledIds.length === 0) {
      toast.error(t('accounts.noEnabledAccounts', '没有启用的账号'));
      return;
    }

    batchCheckInMutation.mutate(enabledIds);
  };

  const handleBatchRefresh = () => {
    const enabledIds = filteredAccounts
      .filter((a) => a.enabled)
      .map((a) => a.id);

    if (enabledIds.length === 0) {
      toast.error(t('accounts.noEnabledAccounts', '没有启用的账号'));
      return;
    }

    batchRefreshMutation.mutate(enabledIds);
  };

  const hasEnabledAccounts = filteredAccounts.filter(a => a.enabled).length > 0;

  return (
    <PageContainer
      className="h-full flex flex-col"
      title={
        <div className="flex items-center gap-3">
          <span className="text-2xl font-bold tracking-tight">{t('accounts.title')}</span>
          {accounts && accounts.length > 0 && (
            <Badge variant="secondary" className="text-sm font-normal rounded-full px-2.5">
              {accounts.length}
            </Badge>
          )}
        </div>
      }
      actions={
        <div className="flex items-center gap-3">
          {/* Search */}
          <div className="relative w-64">
            <Search className="absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-muted-foreground" />
            <Input
              placeholder={t('accounts.searchPlaceholder')}
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="pl-8 h-9 bg-background shadow-sm border-border/50 text-sm"
            />
          </div>

          {/* Provider Filter */}
          <Select value={providerFilter} onValueChange={setProviderFilter}>
            <SelectTrigger className="w-40 h-9 shadow-sm border-border/50">
              <SelectValue placeholder={t('accounts.allProviders')} />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">
                <div className="flex items-center gap-2">
                  <Layers className="h-4 w-4" />
                  <span>{t('accounts.allProviders')}</span>
                </div>
              </SelectItem>
              {allProviders.map(p => (
                <SelectItem key={p.id} value={p.id}>
                  <div className="flex items-center gap-2">
                    <Box className="h-4 w-4" />
                    <span>{p.name}</span>
                  </div>
                </SelectItem>
              ))}
            </SelectContent>
          </Select>

          <div className="w-px h-6 bg-border" />

          {/* Batch Check-in Button */}
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                variant="ghost"
                size="icon"
                onClick={handleBatchCheckIn}
                disabled={batchCheckInMutation.isPending || !hasEnabledAccounts}
                title={t('checkIn.batchCheckIn')}
              >
                <Calendar className="h-4 w-4" />
              </Button>
            </TooltipTrigger>
            <TooltipContent>
              <p>{t('checkIn.batchCheckIn', '批量签到')}</p>
            </TooltipContent>
          </Tooltip>

          {/* Batch Refresh Button */}
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                variant="ghost"
                size="icon"
                onClick={handleBatchRefresh}
                disabled={batchRefreshMutation.isPending || !hasEnabledAccounts}
                title={t('accounts.refreshBalances')}
              >
                <RefreshCw className={cn("h-4 w-4", batchRefreshMutation.isPending && "animate-spin")} />
              </Button>
            </TooltipTrigger>
            <TooltipContent>
              <p>{t('accounts.refreshBalances', '批量刷新余额')}</p>
            </TooltipContent>
          </Tooltip>

          <div className="w-px h-6 bg-border" />

          {/* Add/Manage Dropdown */}
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button size="sm" className="shadow-sm">
                <Plus className="mr-2 h-4 w-4" />
                {t('accounts.addAndManage', '添加/管理')}
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end" className="w-48">
              <DropdownMenuLabel>{t('accounts.accountOperations', '账号操作')}</DropdownMenuLabel>
              <DropdownMenuItem onClick={handleCreate}>
                <Plus className="mr-2 h-4 w-4" />
                <span>{t('accounts.addAccount')}</span>
              </DropdownMenuItem>
              <DropdownMenuSeparator />
              <DropdownMenuLabel>{t('accounts.dataManagement', '数据管理')}</DropdownMenuLabel>
              <DropdownMenuItem onClick={() => setBatchUpdateDialogOpen(true)}>
                <Upload className="mr-2 h-4 w-4" />
                <span>{t('accounts.batchUpdate')}</span>
              </DropdownMenuItem>
              <DropdownMenuItem onClick={() => setJsonImportDialogOpen(true)}>
                <Download className="mr-2 h-4 w-4" />
                <span>{t('accounts.importJSON')}</span>
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
      }
    >
      <div className="flex-1 flex flex-col gap-4 overflow-hidden">
        {/* Statistics Cards */}
        {filteredStatistics && (
          <Card className="p-5 border-border/50 shadow-md bg-gradient-to-br from-blue-50 via-white to-green-50 dark:from-blue-950/30 dark:via-background dark:to-green-950/30">
            <div className="flex items-center justify-between gap-6">
              <div className="flex-1 grid grid-cols-3 gap-6">
                {/* Current Balance */}
                <div className="flex items-center gap-3">
                  <div className="p-2.5 rounded-xl bg-green-100 dark:bg-green-900/30 text-green-600 dark:text-green-400 shadow-sm">
                    <Wallet className="h-5 w-5" />
                  </div>
                  <div>
                    <p className="text-xs text-muted-foreground font-medium">{t('dashboard.stats.currentBalance')}</p>
                    <p className="text-2xl font-bold text-green-600 dark:text-green-400 font-mono">
                      ${filteredStatistics.total_current_balance.toFixed(2)}
                    </p>
                  </div>
                </div>

                {/* Total Income */}
                <div className="flex items-center gap-3">
                  <div className="p-2.5 rounded-xl bg-blue-100 dark:bg-blue-900/30 text-blue-600 dark:text-blue-400 shadow-sm">
                    <TrendingUp className="h-5 w-5" />
                  </div>
                  <div>
                    <p className="text-xs text-muted-foreground font-medium">{t('dashboard.stats.totalIncome')}</p>
                    <p className="text-2xl font-bold text-blue-600 dark:text-blue-400 font-mono">
                      ${filteredStatistics.total_income.toFixed(2)}
                    </p>
                  </div>
                </div>

                {/* Historical Consumption */}
                <div className="flex items-center gap-3">
                  <div className="p-2.5 rounded-xl bg-orange-100 dark:bg-orange-900/30 text-orange-600 dark:text-orange-400 shadow-sm">
                    <History className="h-5 w-5" />
                  </div>
                  <div>
                    <p className="text-xs text-muted-foreground font-medium">{t('dashboard.stats.historicalConsumption')}</p>
                    <p className="text-2xl font-bold text-orange-600 dark:text-orange-400 font-mono">
                      ${filteredStatistics.total_consumed.toFixed(2)}
                    </p>
                  </div>
                </div>
              </div>
            </div>
          </Card>
        )}

        {/* Table */}
        <div className="flex-1 overflow-auto">
          {isLoading ? (
            <div className="flex items-center justify-center h-64">
              <div className="text-muted-foreground">{t('accounts.loading')}</div>
            </div>
          ) : filteredAccounts.length === 0 ? (
            <div className="flex flex-col items-center justify-center h-64 text-center">
              {searchQuery || providerFilter !== 'all' ? (
                <>
                  <p className="text-lg font-semibold">{t('accounts.noResults')}</p>
                  <p className="text-muted-foreground mt-1">
                    {t('accounts.tryDifferentSearch', '尝试其他搜索条件')}
                  </p>
                  <Button
                    variant="link"
                    onClick={() => {
                      setSearchQuery('');
                      setProviderFilter('all');
                    }}
                    className="mt-2"
                  >
                    {t('accounts.clearFilters', '清除筛选')}
                  </Button>
                </>
              ) : (
                <>
                  <p className="text-lg font-semibold">{t('accounts.noAccounts')}</p>
                  <p className="text-muted-foreground mt-1">
                    {t('accounts.noAccountsDescription')}
                  </p>
                  <div className="flex gap-2 mt-4">
                    <Button onClick={handleCreate}>
                      <Plus className="mr-2 h-4 w-4" />
                      {t('accounts.addAccount')}
                    </Button>
                    <Button variant="outline" onClick={() => setJsonImportDialogOpen(true)}>
                      <Upload className="mr-2 h-4 w-4" />
                      {t('accounts.importJSON')}
                    </Button>
                  </div>
                </>
              )}
            </div>
          ) : (
            <AccountsTable
              accounts={filteredAccounts}
              onAccountClick={handleAccountClick}
              onCheckIn={(id) => checkInMutation.mutate(id)}
              onEdit={handleEdit}
              onToggle={(account) => toggleMutation.mutate(account)}
              onDelete={(account) => {
                if (window.confirm(t('accountCard.deleteWarning'))) {
                  deleteMutation.mutate(account.id);
                }
              }}
              onRefreshBalance={(id) => refreshBalanceMutation.mutate(id)}
              checkingInIds={checkingInIds}
              sortConfig={sortConfig}
              onSortChange={setSortConfig}
            />
          )}
        </div>
      </div>

      {/* Dialogs */}
      <AccountDialog
        open={accountDialogOpen}
        onOpenChange={handleDialogClose}
        mode={editingAccount ? 'edit' : 'create'}
        accountId={editingAccount?.id}
        defaultValues={
          editingAccount
            ? {
                name: editingAccount.name,
                provider_id: editingAccount.provider_id,
                cookies: editingAccount.cookies,
                api_user: editingAccount.api_user,
                auto_checkin_enabled: editingAccount.auto_checkin_enabled,
                auto_checkin_hour: editingAccount.auto_checkin_hour,
                auto_checkin_minute: editingAccount.auto_checkin_minute,
              }
            : undefined
        }
      />

      <JsonImportDialog open={jsonImportDialogOpen} onOpenChange={setJsonImportDialogOpen} />

      <BatchUpdateDialog open={batchUpdateDialogOpen} onOpenChange={setBatchUpdateDialogOpen} />
    </PageContainer>
  );
}
