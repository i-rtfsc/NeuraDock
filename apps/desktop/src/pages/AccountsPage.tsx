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
  RefreshCw,
  Users
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
import { useAccounts, useDeleteAccount, useToggleAccount } from '@/hooks/useAccounts';
import { useProviders } from '@/hooks/useProviders';
import type { ProviderDto } from '@/hooks/useProviders';
import { useAccountActions } from '@/hooks/useAccountActions';
import { AccountsTable } from '@/components/account/AccountsTable';
import { AccountDialog } from '@/components/account/AccountDialog';
import { JsonImportDialog } from '@/components/account/JsonImportDialog';
import { BatchUpdateDialog } from '@/components/account/BatchUpdateDialog';
import { PageContainer } from '@/components/layout/PageContainer';
import { HeaderActions, HeaderActionsSeparator } from '@/components/layout/HeaderActions';
import { Account } from '@/lib/tauri-commands';
import { cn } from '@/lib/utils';
import { useCheckIn, useBatchCheckIn } from '@/hooks/useCheckIn';
import { useRefreshAccountBalance, useRefreshAllBalances } from '@/hooks/useBalance';
import { usePersistedState } from '@/hooks/usePersistedState';
import { motion, type Variants } from 'framer-motion';
import { createFadeUpItem, createStaggerContainer } from '@/lib/motion';

export function AccountsPage() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { data: accounts = [], isLoading } = useAccounts();
  const { data: providers } = useProviders();
  const providersById = useMemo(() => {
    if (!providers) {
      return {} as Record<string, ProviderDto>;
    }
    return providers.reduce((acc, provider) => {
      acc[provider.id] = provider;
      return acc;
    }, {} as Record<string, ProviderDto>);
  }, [providers]);
  const {
    editingAccount,
    dialogOpen: accountDialogOpen,
    handleEdit,
    handleCreate,
    handleDialogClose,
  } = useAccountActions();

  const checkInMutation = useCheckIn();
  const batchCheckInMutation = useBatchCheckIn();
  const refreshBalanceMutation = useRefreshAccountBalance();
  const refreshAllBalancesMutation = useRefreshAllBalances();
  const toggleMutation = useToggleAccount();
  const deleteMutation = useDeleteAccount();

  const [searchQuery, setSearchQuery] = usePersistedState<string>(
    'neuradock.accounts.searchQuery',
    ''
  );
  const [providerFilter, setProviderFilter] = usePersistedState<string>(
    'neuradock.accounts.providerFilter',
    'all'
  );
  const [jsonImportDialogOpen, setJsonImportDialogOpen] = useState(false);
  const [batchUpdateDialogOpen, setBatchUpdateDialogOpen] = useState(false);
  const [checkingInIds, setCheckingInIds] = useState<Set<string>>(new Set());
  const [sortConfig, setSortConfig] = usePersistedState<{
    key: keyof Account;
    direction: 'asc' | 'desc';
  } | null>('neuradock.accounts.sortConfig', null);

  const containerVariants: Variants = createStaggerContainer({ staggerChildren: 0.05 });
  const itemVariants: Variants = createFadeUpItem({ y: 10, scale: 1 });

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
    const total_quota = filteredAccounts.reduce(
      (sum, acc) => sum + (acc.total_quota || 0),
      0
    );
    const total_consumed = filteredAccounts.reduce(
      (sum, acc) => sum + (acc.total_consumed || 0),
      0
    );

    return {
      total_current_balance,
      total_quota,
      total_consumed,
    };
  }, [filteredAccounts]);

  const handleAccountClick = (account: Account) => {
    navigate(`/accounts/${account.id}`);
  };

  const handleAccountCheckIn = (accountId: string) => {
    setCheckingInIds((prev) => new Set(prev).add(accountId));
    checkInMutation.mutate(accountId, {
      onSettled: () => {
        setCheckingInIds((prev) => {
          const next = new Set(prev);
          next.delete(accountId);
          return next;
        });
      },
    });
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

    refreshAllBalancesMutation.mutate(enabledIds);
  };

  const hasEnabledAccounts = filteredAccounts.filter(a => a.enabled).length > 0;

  return (
    <PageContainer
      className="h-full flex flex-col"
      title={
        <div className="flex items-center gap-3">
          <span className="text-2xl">{t('accounts.title')}</span>
          {accounts && accounts.length > 0 && (
            <Badge variant="soft-primary" className="text-sm rounded-full px-2.5">
              {accounts.length}
            </Badge>
          )}
        </div>
      }
      actions={
        <HeaderActions>
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

          <HeaderActionsSeparator />

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
                disabled={refreshAllBalancesMutation.isPending || !hasEnabledAccounts}
                title={t('accounts.refreshBalances')}
              >
                <RefreshCw className={cn(
                  "h-4 w-4",
                  refreshAllBalancesMutation.isPending && "animate-spin"
                )} />
              </Button>
            </TooltipTrigger>
            <TooltipContent>
              <p>{t('accounts.refreshBalances', '批量刷新余额')}</p>
            </TooltipContent>
          </Tooltip>

          <HeaderActionsSeparator />

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
        </HeaderActions>
      }
    >
      <div className="flex-1 flex flex-col gap-6 overflow-visible pt-2">
        {/* Statistics Cards */}
        {filteredStatistics && (
          <motion.div
            variants={containerVariants}
            initial="hidden"
            animate="show"
            className="grid gap-4 grid-cols-1 md:grid-cols-3"
          >
            {/* Current Balance */}
            <motion.div variants={itemVariants}>
              <Card className="card-vivid group">
                <div className="p-6 flex items-center justify-between">
                  <div>
                    <p className="text-sm font-medium text-muted-foreground">{t('dashboard.stats.currentBalance')}</p>
                    <p className="text-2xl font-bold text-success font-mono mt-1">
                      ${filteredStatistics.total_current_balance.toFixed(2)}
                    </p>
                  </div>
                  <div className="p-3 rounded-full bg-success-soft text-success">
                    <Wallet className="h-5 w-5" />
                  </div>
                </div>
              </Card>
            </motion.div>

            {/* Total Quota */}
            <motion.div variants={itemVariants}>
              <Card className="card-vivid group">
                <div className="p-6 flex items-center justify-between">
                  <div>
                    <p className="text-sm font-medium text-muted-foreground">{t('dashboard.stats.totalQuota')}</p>
                    <p className="text-2xl font-bold text-info font-mono mt-1">
                      ${filteredStatistics.total_quota.toFixed(2)}
                    </p>
                  </div>
                  <div className="p-3 rounded-full bg-info-soft text-info">
                    <TrendingUp className="h-5 w-5" />
                  </div>
                </div>
              </Card>
            </motion.div>

            {/* Historical Consumption */}
            <motion.div variants={itemVariants}>
              <Card className="card-vivid group">
                <div className="p-6 flex items-center justify-between">
                  <div>
                    <p className="text-sm font-medium text-muted-foreground">{t('dashboard.stats.historicalConsumption')}</p>
                    <p className="text-2xl font-bold text-warning font-mono mt-1">
                      ${filteredStatistics.total_consumed.toFixed(2)}
                    </p>
                  </div>
                  <div className="p-3 rounded-full bg-warning-soft text-warning">
                    <History className="h-5 w-5" />
                  </div>
                </div>
              </Card>
            </motion.div>
          </motion.div>
        )}

        {/* Table */}
        <div className="flex-1 overflow-auto">
          {isLoading ? (
            <div className="flex items-center justify-center h-64">
              <div className="text-muted-foreground">{t('accounts.loading')}</div>
            </div>
          ) : filteredAccounts.length === 0 ? (
            <div className="text-center py-12">
              {searchQuery || providerFilter !== 'all' ? (
                <>
                  <Search className="h-12 w-12 mx-auto text-muted-foreground/50 mb-4" />
                  <h3 className="text-lg font-medium mb-2">{t('accounts.noResults')}</h3>
                  <p className="text-sm text-muted-foreground mb-4">
                    {t('accounts.tryDifferentSearch', '尝试其他搜索条件')}
                  </p>
                  <Button
                    variant="outline"
                    onClick={() => {
                      setSearchQuery('');
                      setProviderFilter('all');
                    }}
                  >
                    {t('accounts.clearFilters', '清除筛选')}
                  </Button>
                </>
              ) : (
                <>
                  <Users className="h-12 w-12 mx-auto text-muted-foreground/50 mb-4" />
                  <h3 className="text-lg font-medium mb-2">{t('accounts.noAccounts')}</h3>
                  <p className="text-sm text-muted-foreground mb-4">
                    {t('accounts.noAccountsDescription')}
                  </p>
                  <div className="flex justify-center gap-2">
                    <Button onClick={handleCreate} variant="outline">
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
              onCheckIn={handleAccountCheckIn}
              onEdit={handleEdit}
              onToggle={(account) =>
                toggleMutation.mutate({ accountId: account.id, enabled: !account.enabled })
              }
              onDelete={(account) => {
                if (window.confirm(t('accountCard.deleteWarning'))) {
                  deleteMutation.mutate(account.id);
                }
              }}
              onRefreshBalance={(id) => refreshBalanceMutation.mutate(id)}
              checkingInIds={checkingInIds}
              sortConfig={sortConfig}
              onSortChange={setSortConfig}
              providersById={providersById}
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
