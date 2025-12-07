import { useState, useMemo } from 'react';
import { useSearchParams } from 'react-router-dom';
import { Plus, Upload, Search, DollarSign, RefreshCw } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Badge } from '@/components/ui/badge';
import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs';

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

  return (
    <PageContainer className="space-y-6 max-w-[1600px]">
      {/* Header Section */}
      <div className="flex flex-col gap-6 md:flex-row md:items-start md:justify-between">
        <div className="space-y-1.5">
          <h1 className="text-3xl font-bold tracking-tight">{t('accounts.title')}</h1>
          <p className="text-muted-foreground">
            {t('accounts.description')}
          </p>
        </div>
        <div className="flex items-center gap-2">
          <Button variant="outline" size="sm" onClick={() => setBatchUpdateDialogOpen(true)}>
            <RefreshCw className="mr-2 h-4 w-4" />
            {t('accounts.batchUpdate')}
          </Button>
          <Button variant="outline" size="sm" onClick={() => setJsonImportDialogOpen(true)}>
            <Upload className="mr-2 h-4 w-4" />
            {t('accounts.importJSON')}
          </Button>
          <Button size="sm" onClick={handleCreate} className="shadow-sm">
            <Plus className="mr-2 h-4 w-4" />
            {t('accounts.addAccount')}
          </Button>
        </div>
      </div>

      {/* Statistics Cards */}
      {filteredStatistics && (
        <div className="grid gap-4 md:grid-cols-3">
          <Card className="bg-gradient-to-br from-blue-50 to-transparent dark:from-blue-950/20 border-blue-100 dark:border-blue-900/50">
            <CardContent className="p-6 flex items-center justify-between">
              <div className="space-y-1">
                <p className="text-sm font-medium text-muted-foreground">{t('dashboard.stats.totalIncome')}</p>
                <p className="text-2xl font-bold text-blue-600 dark:text-blue-400">
                  ${filteredStatistics.total_income.toFixed(2)}
                </p>
              </div>
              <div className="h-10 w-10 rounded-full bg-blue-100 dark:bg-blue-900/50 flex items-center justify-center">
                <DollarSign className="h-5 w-5 text-blue-600 dark:text-blue-400" />
              </div>
            </CardContent>
          </Card>
          <Card className="bg-gradient-to-br from-orange-50 to-transparent dark:from-orange-950/20 border-orange-100 dark:border-orange-900/50">
            <CardContent className="p-6 flex items-center justify-between">
              <div className="space-y-1">
                <p className="text-sm font-medium text-muted-foreground">{t('dashboard.stats.historicalConsumption')}</p>
                <p className="text-2xl font-bold text-orange-600 dark:text-orange-400">
                  ${filteredStatistics.total_consumed.toFixed(2)}
                </p>
              </div>
              <div className="h-10 w-10 rounded-full bg-orange-100 dark:bg-orange-900/50 flex items-center justify-center">
                <RefreshCw className="h-5 w-5 text-orange-600 dark:text-orange-400" />
              </div>
            </CardContent>
          </Card>
          <Card className="bg-gradient-to-br from-green-50 to-transparent dark:from-green-950/20 border-green-100 dark:border-green-900/50">
            <CardContent className="p-6 flex items-center justify-between">
              <div className="space-y-1">
                <p className="text-sm font-medium text-muted-foreground">{t('dashboard.stats.currentBalance')}</p>
                <p className="text-2xl font-bold text-green-600 dark:text-green-400">
                  ${filteredStatistics.total_current_balance.toFixed(2)}
                </p>
              </div>
              <div className="h-10 w-10 rounded-full bg-green-100 dark:bg-green-900/50 flex items-center justify-center">
                <DollarSign className="h-5 w-5 text-green-600 dark:text-green-400" />
              </div>
            </CardContent>
          </Card>
        </div>
      )}

      {/* Toolbar & Filters */}
      <div className="flex flex-col gap-4 md:flex-row md:items-center md:justify-between py-2">
        <div className="flex items-center gap-4 flex-1 w-full">
          {/* Provider Tabs - Now on the Left */}
          {allProviders.length > 0 && (
            <Tabs value={selectedProvider} onValueChange={setSelectedProvider} className="w-full md:w-auto">
              <TabsList className="bg-muted/50 w-full md:w-auto overflow-x-auto justify-start">
                <TabsTrigger value="all">All</TabsTrigger>
                {allProviders.map(p => (
                  <TabsTrigger key={p.id} value={p.id}>{p.name}</TabsTrigger>
                ))}
              </TabsList>
            </Tabs>
          )}

          <div className="flex-1 hidden md:block" />

          {/* Search Bar - Now on the Right */}
          <div className="relative w-full md:max-w-xs">
            <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
            <Input
              placeholder={t('accounts.searchPlaceholder')}
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="pl-9 bg-muted/50 border-muted-foreground/20"
            />
          </div>
        </div>
      </div>

      {/* Accounts List */}
      {isLoading ? (
        <AccountListSkeleton />
      ) : filteredAccounts && filteredAccounts.length > 0 ? (
        <div className="space-y-8">
          {Object.entries(accountsByProvider).map(([providerId, providerAccounts]) => {
            const providerInfo = providers?.find(p => p.id === providerId);
            const providerName = providerInfo?.name || providerAccounts[0]?.provider_name || 'Unknown';
            const enabledCount = providerAccounts.filter(a => a.enabled).length;
            
            return (
              <div key={providerId} className="space-y-4 animate-in fade-in slide-in-from-bottom-4 duration-500">
                {/* Provider Header */}
                <div className="flex items-center justify-between border-b pb-2">
                  <div className="flex items-center gap-3">
                    <h2 className="text-xl font-semibold tracking-tight">{providerName}</h2>
                    <Badge variant="secondary" className="rounded-full px-2.5">
                      {providerAccounts.length}
                    </Badge>
                    {enabledCount > 0 && enabledCount !== providerAccounts.length && (
                      <Badge variant="outline" className="text-xs">
                        {enabledCount} {t('accounts.enabled')}
                      </Badge>
                    )}
                  </div>
                  <div className="flex items-center gap-2">
                    {enabledCount > 0 && (
                      <>
                        <Button
                          variant="ghost"
                          size="sm"
                          className="h-8 text-muted-foreground hover:text-foreground"
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

                {/* Accounts Grid */}
                <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4">
                  {providerAccounts.map((account) => (
                    <AccountCard
                      key={account.id}
                      account={account}
                      onEdit={handleEdit}
                    />
                  ))}
                </div>
              </div>
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
