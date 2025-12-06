import { useState, useMemo } from 'react';
import { Plus, Upload, Search, DollarSign, RefreshCw } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Card } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Badge } from '@/components/ui/badge';
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
  const [searchQuery, setSearchQuery] = useState('');
  const [jsonImportDialogOpen, setJsonImportDialogOpen] = useState(false);
  const [batchUpdateDialogOpen, setBatchUpdateDialogOpen] = useState(false);

  // Filter accounts by search query
  const filteredAccounts = accounts?.filter((account) =>
    account.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
    account.provider_name.toLowerCase().includes(searchQuery.toLowerCase())
  );

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



  const getProviderStats = (providerId: string) => {
    return statistics?.providers.find(p => p.provider_id === providerId);
  };

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
    <div className="space-y-6 p-6">
      {/* Header with Statistics */}
      <div className="space-y-4">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <h1 className="text-3xl font-bold tracking-tight bg-gradient-to-r from-blue-600 to-purple-600 bg-clip-text text-transparent">
              {t('accounts.title')}
            </h1>
            {accounts && accounts.length > 0 && (
              <Badge variant="secondary">
                {accounts.length} {accounts.length === 1 ? t('accounts.account') : t('accounts.accounts_plural')}
              </Badge>
            )}
          </div>
          <div className="flex gap-2">
            <Button variant="secondary" size="sm" onClick={() => setBatchUpdateDialogOpen(true)}>
              <RefreshCw className="mr-2 h-4 w-4" />
              {t('accounts.batchUpdate')}
            </Button>
            <Button variant="secondary" size="sm" onClick={() => setJsonImportDialogOpen(true)}>
              <Upload className="mr-2 h-4 w-4" />
              {t('accounts.importJSON')}
            </Button>
            <Button variant="default" size="sm" onClick={handleCreate}>
              <Plus className="mr-2 h-4 w-4" />
              {t('accounts.addAccount')}
            </Button>
          </div>
        </div>

        {/* Total Statistics at Top */}
        {statistics && (
          <Card  className="border-2 border-blue-200 dark:border-blue-800">
            <div className="p-6">
              <div className="flex items-center justify-around text-center">
                <div className="flex flex-col gap-1">
                  <span className="text-sm text-muted-foreground font-medium">{t('dashboard.stats.totalIncome')}</span>
                  <span className="font-bold text-3xl bg-gradient-to-r from-blue-600 to-blue-400 bg-clip-text text-transparent">
                    ${statistics.total_income.toFixed(2)}
                  </span>
                </div>
                <div className="h-12 w-px bg-gradient-to-b from-transparent via-border to-transparent" />
                <div className="flex flex-col gap-1">
                  <span className="text-sm text-muted-foreground font-medium">{t('dashboard.stats.historicalConsumption')}</span>
                  <span className="font-bold text-3xl bg-gradient-to-r from-orange-600 to-orange-400 bg-clip-text text-transparent">
                    ${statistics.total_consumed.toFixed(2)}
                  </span>
                </div>
                <div className="h-12 w-px bg-gradient-to-b from-transparent via-border to-transparent" />
                <div className="flex flex-col gap-1">
                  <span className="text-sm text-muted-foreground font-medium">{t('dashboard.stats.currentBalance')}</span>
                  <span className="font-bold text-3xl bg-gradient-to-r from-green-600 to-green-400 bg-clip-text text-transparent">
                    ${statistics.total_current_balance.toFixed(2)}
                  </span>
                </div>
              </div>
            </div>
          </Card>
        )}
      </div>

      {/* Search Bar */}
      {accounts && accounts.length > 0 && (
        <div className="relative">
          <Search className="absolute left-4 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground z-10" />
          <Input
            placeholder={t('accounts.searchPlaceholder')}
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="pl-11"
          />
        </div>
      )}

      {/* Accounts List - Grouped by Provider */}
      {isLoading ? (
        <Card>
          <div className="p-8">
            <div className="flex flex-col items-center justify-center gap-4">
              <div className="h-12 w-12 animate-spin rounded-full border-4 border-blue-200 border-t-blue-600" />
              <p className="text-center text-muted-foreground font-medium">{t('accounts.loading')}</p>
            </div>
          </div>
        </Card>
      ) : filteredAccounts && filteredAccounts.length > 0 ? (
        <>
          {/* Group by Provider */}
          <div className="space-y-6">
            {Object.entries(accountsByProvider).map(([providerId, providerAccounts]) => {
              const providerInfo = providers?.find(p => p.id === providerId);
              const providerStats = getProviderStats(providerId);
              const providerName = providerInfo?.name || providerAccounts[0]?.provider_name || 'Unknown';
              const enabledCount = providerAccounts.filter(a => a.enabled).length;
              
              return (
                <Card key={providerId} >
                  <div className="p-6 space-y-4">
                    <div className="flex items-center justify-between">
                      <div className="flex items-center gap-3">
                        <h2 className="text-2xl font-bold">{providerName}</h2>
                        <Badge variant="outline">
                          {providerAccounts.length} {providerAccounts.length === 1 ? t('accounts.account') : t('accounts.accounts_plural')}
                        </Badge>
                        {enabledCount > 0 && (
                          <Badge variant="default">
                            {enabledCount} {t('accounts.enabled')}
                          </Badge>
                        )}
                      </div>
                      <div className="flex gap-2">
                        {enabledCount > 0 && (
                          <>
                            <Button
                              variant="secondary"
                              size="sm"
                              onClick={() => handleRefreshProviderBalances(providerAccounts)}
                              disabled={refreshAllBalancesMutation.isPending}
                              title={t('accounts.refreshAllBalances') || 'Refresh all balances'}
                            >
                              <RefreshCw className={`mr-2 h-4 w-4 ${refreshAllBalancesMutation.isPending ? 'animate-spin' : ''}`} />
                              {t('accounts.refreshBalances') || 'Refresh Balances'}
                            </Button>
                            <BatchCheckInButton
                              accountIds={providerAccounts.filter(a => a.enabled).map(a => a.id)}
                              onComplete={() => {}}
                            />
                          </>
                        )}
                      </div>
                    </div>
                    {providerStats && (
                      <div className="flex items-center gap-6 text-sm bg-muted/30 rounded-2xl p-4">
                        <div className="flex items-center gap-2">
                          <DollarSign className="h-4 w-4 text-blue-600" />
                          <span className="text-muted-foreground font-medium">{t('dashboard.stats.totalIncome')}:</span>
                          <span className="font-bold text-blue-600">${providerStats.total_income.toFixed(2)}</span>
                        </div>
                        <div className="flex items-center gap-2">
                          <span className="text-muted-foreground font-medium">{t('dashboard.stats.historicalConsumption')}:</span>
                          <span className="font-bold text-orange-600">${providerStats.total_consumed.toFixed(2)}</span>
                        </div>
                        <div className="flex items-center gap-2">
                          <span className="text-muted-foreground font-medium">{t('dashboard.stats.currentBalance')}:</span>
                          <span className="font-bold text-green-600">${providerStats.current_balance.toFixed(2)}</span>
                        </div>
                      </div>
                    )}
                    <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
                      {providerAccounts.map((account) => (
                        <AccountCard
                          key={account.id}
                          account={account}
                          onEdit={handleEdit}
                        />
                      ))}
                    </div>
                  </div>
                </Card>
              );
            })}
          </div>
        </>
      ) : accounts && accounts.length > 0 && searchQuery ? (
        <Card>
          <div className="p-8">
            <p className="text-center text-muted-foreground">
              {t('accounts.noResultsFor')} <span className="font-semibold">"{searchQuery}"</span>
            </p>
          </div>
        </Card>
      ) : (
        <Card  className="border-dashed">
          <div className="p-12 text-center space-y-6">
            <div className="flex flex-col items-center gap-3">
              <div className="h-16 w-16 rounded-full bg-primary/10 flex items-center justify-center">
                <Plus className="h-8 w-8 text-primary" />
              </div>
              <h3 className="text-2xl font-bold">{t('accounts.noAccounts')}</h3>
              <p className="text-muted-foreground max-w-md">
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
    </div>
  );
}
