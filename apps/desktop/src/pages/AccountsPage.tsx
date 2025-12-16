import { useState, useMemo } from 'react';
import { useSearchParams } from 'react-router-dom';
import { Plus, Upload, RefreshCw, Search, Wallet, TrendingUp, History, Layers, Box } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Card } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Badge } from '@/components/ui/badge';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { cn } from '@/lib/utils';

import { useAccounts } from '@/hooks/useAccounts';
import { useProviders } from '@/hooks/useProviders';
import { useBalanceStatistics, useRefreshAllBalances } from '@/hooks/useBalance';
import { useAccountActions } from '@/hooks/useAccountActions';
import { AccountCard } from '@/components/account/AccountCard';
import { AccountDialog } from '@/components/account/AccountDialog';
import { JsonImportDialog } from '@/components/account/JsonImportDialog';
import { BatchUpdateDialog } from '@/components/account/BatchUpdateDialog';
import { BatchCheckInButton } from '@/components/checkin/BatchCheckInButton';
import { Account } from '@/lib/tauri-commands';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';

import { PageContainer } from '@/components/layout/PageContainer';
import { SidebarPageLayout } from '@/components/layout/SidebarPageLayout';
import { CardGrid } from '@/components/layout/CardGrid';
import { AccountListSkeleton } from '@/components/skeletons/AccountListSkeleton';

export function AccountsPage() {
  const { data: accounts, isLoading } = useAccounts();
  const { data: providers } = useProviders();
  const { data: statistics } = useBalanceStatistics();
  const refreshAllBalancesMutation = useRefreshAllBalances();
  const { 
    editingAccount, 
    dialogOpen: accountDialogOpen, 
    handleEdit, 
    handleCreate, 
    handleDialogClose 
  } = useAccountActions();
  const { t } = useTranslation();
  
  const [searchParams, setSearchParams] = useSearchParams();
  const selectedProvider = searchParams.get('provider') || 'all';
  
  const [searchQuery, setSearchQuery] = useState('');
  const [jsonImportDialogOpen, setJsonImportDialogOpen] = useState(false);
  const [batchUpdateDialogOpen, setBatchUpdateDialogOpen] = useState(false);

  const setSelectedProvider = (value: string) => {
    setSearchParams(prev => {
      prev.set('provider', value);
      return prev;
    });
  };

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

    // Filter by selected provider tab
    if (selectedProvider !== 'all') {
      result = result.filter(a => a.provider_id === selectedProvider);
    }
    
    return result;
  }, [accounts, searchQuery, selectedProvider]);

  // Get all unique providers from the *original* accounts list for the tabs
  const allProviders = useMemo(() => {
    if (!accounts) return [];
    const uniqueIds = new Set(accounts.map(a => a.provider_id));
    return Array.from(uniqueIds).map(id => {
      const account = accounts.find(a => a.provider_id === id);
      const provider = providers?.find(p => p.id === id);
      return {
        id,
        name: provider?.name || account?.provider_name || 'Unknown'
      };
    });
  }, [accounts, providers]);

  // Group accounts by provider
  const accountsByProvider = useMemo(() => {
    if (!filteredAccounts) return {};
    
    return filteredAccounts.reduce((acc, account) => {
      const providerId = account.provider_id;
      if (!acc[providerId]) {
        acc[providerId] = [];
      }
      acc[providerId].push(account);
      return acc;
    }, {} as Record<string, Account[]>);
  }, [filteredAccounts]);

  // Calculate filtered statistics based on selected provider
  const filteredStatistics = useMemo(() => {
    if (!statistics) return null;
    
    if (selectedProvider === 'all') {
      return statistics;
    }

    const providerStats = statistics.providers.find(p => p.provider_id === selectedProvider);
    if (!providerStats) return null;

    return {
      ...statistics,
      total_income: providerStats.total_income,
      total_consumed: providerStats.total_consumed,
      total_current_balance: providerStats.current_balance,
    };
  }, [statistics, selectedProvider]);

  const handleRefreshProviderBalances = async (providerAccounts: Account[]) => {
    const enabledAccountIds = providerAccounts.filter(a => a.enabled).map(a => a.id);
    if (enabledAccountIds.length === 0) {
      toast.error(t('accounts.noEnabledAccounts') || 'No enabled accounts');
      return;
    }

    try {
      await refreshAllBalancesMutation.mutateAsync(enabledAccountIds);
      toast.success(t('accounts.balancesRefreshed') || 'Balances refreshed');
    } catch (error) {
      console.error('Failed to refresh balances:', error);
      toast.error(t('common.error'));
    }
  };

  const sidebarContent = (
    <>
      {/* Statistics Summary Card */}
      {filteredStatistics && (
        <Card className="p-5 border-border/50 shadow-md bg-gradient-to-br from-blue-50 via-white to-green-50 dark:from-blue-950/30 dark:via-background dark:to-green-950/30">
          <div className="space-y-4">
            {/* Current Balance */}
            <div className="flex items-center justify-between">
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
            </div>

            <div className="h-px bg-gradient-to-r from-transparent via-border to-transparent" />

            {/* Total Income */}
            <div className="flex items-center justify-between">
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
            </div>

            <div className="h-px bg-gradient-to-r from-transparent via-border to-transparent" />

            {/* Historical Consumption */}
            <div className="flex items-center justify-between">
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

      <Card className="flex-1 border-border/50 shadow-sm bg-background/50 backdrop-blur-sm overflow-hidden transition-all duration-base ease-smooth hover:shadow-hover-md hover:border-border">
        <ScrollArea className="h-full">
          <div className="p-2 space-y-1">
            <button
              onClick={() => setSelectedProvider('all')}
              className={cn(
                "w-full flex items-center justify-between px-3 py-2 rounded-lg text-sm font-medium transition-colors",
                selectedProvider === 'all'
                  ? "bg-primary text-primary-foreground shadow-sm"
                  : "text-muted-foreground hover:bg-muted hover:text-foreground"
              )}
            >
              <div className="flex items-center gap-2">
                <Layers className="h-4 w-4" />
                <span>{t('accounts.allProviders')}</span>
              </div>
              <span className={cn("text-xs", selectedProvider === 'all' ? "opacity-90" : "opacity-70")}>
                {accounts?.length || 0}
              </span>
            </button>

            <div className="my-2 px-3 text-xs font-semibold text-muted-foreground/50 uppercase tracking-wider">
              {t('accounts.providersLabel')}
            </div>

            {allProviders.map(p => {
              const count = accounts?.filter(a => a.provider_id === p.id).length || 0;
              const isActive = selectedProvider === p.id;
              return (
                <button
                  key={p.id}
                  onClick={() => setSelectedProvider(p.id)}
                  className={cn(
                    "w-full flex items-center justify-between px-3 py-2 rounded-lg text-sm font-medium transition-colors",
                    isActive
                      ? "bg-primary text-primary-foreground shadow-sm"
                      : "text-muted-foreground hover:bg-muted hover:text-foreground"
                  )}
                >
                  <div className="flex items-center gap-2 truncate">
                    <Box className="h-4 w-4 shrink-0" />
                    <span className="truncate">{p.name}</span>
                  </div>
                  <span className={cn("text-xs", isActive ? "opacity-90" : "opacity-70")}>
                    {count}
                  </span>
                </button>
              );
            })}
          </div>
        </ScrollArea>
      </Card>
    </>
  );

  return (
    <PageContainer 
      className="h-full overflow-hidden"
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
          <Select value={selectedProvider} onValueChange={setSelectedProvider}>
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

          <Button variant="ghost" size="icon" onClick={() => setBatchUpdateDialogOpen(true)} title={t('accounts.batchUpdate')}>
            <RefreshCw className="h-4 w-4" />
          </Button>
          <Button variant="ghost" size="icon" onClick={() => setJsonImportDialogOpen(true)} title={t('accounts.importJSON')}>
            <Upload className="h-4 w-4" />
          </Button>
          <Button size="sm" onClick={handleCreate} className="shadow-sm">
            <Plus className="mr-2 h-4 w-4" />
            {t('accounts.addAccount')}
          </Button>
        </div>
      }
    >
      <SidebarPageLayout sidebar={sidebarContent}>
        <div className="space-y-section-gap">
          {/* Accounts List */}
          {isLoading ? (
            <AccountListSkeleton />
          ) : filteredAccounts && filteredAccounts.length > 0 ? (
            <div className="space-y-section-gap">
              {Object.entries(accountsByProvider).map(([providerId, providerAccounts]) => {
                const providerInfo = providers?.find(p => p.id === providerId);
                const providerName = providerInfo?.name || providerAccounts[0]?.provider_name || 'Unknown';
                const enabledCount = providerAccounts.filter(a => a.enabled).length;

                return (
                  <Card key={providerId} className="animate-in fade-in slide-in-from-bottom-4 duration-500 border-border/50 shadow-sm overflow-hidden">
                    {/* Provider Header */}
                    <div className="flex items-center justify-between px-[var(--layout-page-content-padding)] py-3 bg-muted/30">
                      <div className="flex items-center gap-element-gap">
                        <h2 className="text-base font-semibold tracking-tight">{providerName}</h2>
                        <Badge variant="secondary" className="rounded-full px-2.5 text-xs bg-background/50">
                          {providerAccounts.length}
                        </Badge>
                      </div>
                      <div className="flex items-center gap-2">
                        {enabledCount > 0 && (
                          <>
                            <Button
                              variant="ghost"
                              size="sm"
                              className="text-muted-foreground hover:text-foreground hover:bg-background/50"
                              onClick={() => handleRefreshProviderBalances(providerAccounts)}
                              disabled={refreshAllBalancesMutation.isPending}
                            >
                              <RefreshCw className={`mr-2 h-3.5 w-3.5 ${refreshAllBalancesMutation.isPending ? 'animate-spin' : ''}`} />
                              <span className="text-xs">{t('accounts.refreshBalances')}</span>
                            </Button>
                            <BatchCheckInButton
                              accountIds={providerAccounts.filter(a => a.enabled).map(a => a.id)}
                              onComplete={() => {}}
                            />
                          </>
                        )}
                      </div>
                    </div>

                    {/* Accounts Grid - 使用统一的CardGrid组件 */}
                    <div className="p-[var(--layout-page-content-padding)]">
                      <CardGrid variant="cards">
                        {providerAccounts.map((account) => (
                          <AccountCard
                            key={account.id}
                            account={account}
                            onEdit={handleEdit}
                          />
                        ))}
                      </CardGrid>
                    </div>
                  </Card>
                );
              })}
            </div>
          ) : accounts && accounts.length > 0 && searchQuery ? (
            <div className="flex flex-col items-center justify-center py-12 text-center">
              <div className="h-12 w-12 rounded-full bg-muted flex items-center justify-center mb-4">
                <Search className="h-6 w-6 text-muted-foreground" />
              </div>
              <h3 className="text-lg font-semibold">{t('accounts.noResultsFor')} "{searchQuery}"</h3>
              <p className="text-muted-foreground mt-1">{t('accounts.tryDifferentSearch')}</p>
              <Button variant="link" onClick={() => setSearchQuery('')} className="mt-2">
                {t('accounts.clearSearch')}
              </Button>
            </div>
          ) : (
            <Card className="border-dashed bg-muted/30">
              <div className="p-12 text-center space-y-6">
                <div className="flex flex-col items-center gap-3">
                  <div className="h-16 w-16 rounded-full bg-primary/10 flex items-center justify-center">
                    <Plus className="h-8 w-8 text-primary" />
                  </div>
                  <h3 className="text-2xl font-bold">{t('accounts.noAccounts')}</h3>
                  <p className="text-muted-foreground max-w-md mx-auto">
                    {t('accounts.noAccountsDescription')}
                  </p>
                </div>
                <div className="flex gap-3 justify-center">
                  <Button variant="default" onClick={handleCreate}>
                    <Plus className="mr-2 h-4 w-4" />
                    {t('accounts.addAccount')}
                  </Button>
                  <Button variant="secondary" onClick={() => setJsonImportDialogOpen(true)}>
                    <Upload className="mr-2 h-4 w-4" />
                    {t('accounts.importJSON')}
                  </Button>
                </div>
              </div>
            </Card>
          )}
        </div>
      </SidebarPageLayout>

      {/* Dialogs */}
      <AccountDialog
        open={accountDialogOpen}
        onOpenChange={handleDialogClose}
        mode={editingAccount ? 'edit' : 'create'}
        accountId={editingAccount?.id}
        defaultValues={editingAccount ? {
          name: editingAccount.name,
          provider_id: editingAccount.provider_id,
          cookies: editingAccount.cookies,
          api_user: editingAccount.api_user,
          auto_checkin_enabled: editingAccount.auto_checkin_enabled,
          auto_checkin_hour: editingAccount.auto_checkin_hour,
          auto_checkin_minute: editingAccount.auto_checkin_minute,
        } : undefined}
      />

      <JsonImportDialog
        open={jsonImportDialogOpen}
        onOpenChange={setJsonImportDialogOpen}
      />

      <BatchUpdateDialog
        open={batchUpdateDialogOpen}
        onOpenChange={setBatchUpdateDialogOpen}
      />
    </PageContainer>
  );
}
