import { useState, useMemo } from 'react';
import { Plus, Upload, Search, DollarSign, RefreshCw } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { useAccounts } from '@/hooks/useAccounts';
import { useProviders } from '@/hooks/useProviders';
import { useBalanceStatistics, useRefreshAllBalances } from '@/hooks/useBalance';
import { AccountCard } from '@/components/account/AccountCard';
import { AccountDialog } from '@/components/account/AccountDialog';
import { JsonImportDialog } from '@/components/account/JsonImportDialog';
import { BatchUpdateDialog } from '@/components/account/BatchUpdateDialog';
import { BatchCheckInButton } from '@/components/checkin/BatchCheckInButton';
import { Account, AccountDetail } from '@/lib/tauri-commands';
import { Badge } from '@/components/ui/badge';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import { toast } from 'sonner';

export function AccountsPage() {
  const { data: accounts, isLoading } = useAccounts();
  const { data: providers } = useProviders();
  const { data: statistics } = useBalanceStatistics();
  const refreshAllBalancesMutation = useRefreshAllBalances();
  const { t } = useTranslation();
  const [searchQuery, setSearchQuery] = useState('');
  const [accountDialogOpen, setAccountDialogOpen] = useState(false);
  const [jsonImportDialogOpen, setJsonImportDialogOpen] = useState(false);
  const [batchUpdateDialogOpen, setBatchUpdateDialogOpen] = useState(false);
  const [editingAccount, setEditingAccount] = useState<AccountDetail | null>(null);

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

  const handleEdit = async (account: Account) => {
    try {
      // Fetch full account details including credentials
      const accountDetail = await invoke<AccountDetail>('get_account_detail', { accountId: account.id });
      setEditingAccount(accountDetail);
      setAccountDialogOpen(true);
    } catch (error) {
      console.error('Failed to fetch account details:', error);
      toast.error(t('common.error'));
    }
  };

  const handleCreate = () => {
    setEditingAccount(null);
    setAccountDialogOpen(true);
  };

  const handleDialogClose = () => {
    setAccountDialogOpen(false);
    setEditingAccount(null);
  };

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
    <div className="space-y-6">
      {/* Header with Statistics */}
      <div className="space-y-4">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <h1 className="text-3xl font-bold tracking-tight">{t('accounts.title')}</h1>
            {accounts && accounts.length > 0 && (
              <Badge variant="secondary" className="rounded-full text-base px-3 py-1">
                {accounts.length} {accounts.length === 1 ? t('accounts.account') : t('accounts.accounts_plural')}
              </Badge>
            )}
          </div>
          <div className="flex gap-2">
            <Button variant="outline" size="sm" onClick={() => setBatchUpdateDialogOpen(true)} className="rounded-full">
              <RefreshCw className="mr-2 h-4 w-4" />
              {t('accounts.batchUpdate')}
            </Button>
            <Button variant="outline" size="sm" onClick={() => setJsonImportDialogOpen(true)} className="rounded-full">
              <Upload className="mr-2 h-4 w-4" />
              {t('accounts.importJSON')}
            </Button>
            <Button variant="outline" size="sm" onClick={handleCreate} className="rounded-full">
              <Plus className="mr-2 h-4 w-4" />
              {t('accounts.addAccount')}
            </Button>
          </div>
        </div>

        {/* Total Statistics at Top */}
        {statistics && (
          <Card className="bg-primary/5 rounded-2xl border-2">
            <CardContent className="pt-6">
              <div className="flex items-center justify-around text-center">
                <div className="flex flex-col gap-1">
                  <span className="text-sm text-muted-foreground">{t('dashboard.stats.totalIncome')}</span>
                  <span className="font-bold text-2xl text-blue-600">${statistics.total_income.toFixed(2)}</span>
                </div>
                <div className="h-12 w-px bg-border" />
                <div className="flex flex-col gap-1">
                  <span className="text-sm text-muted-foreground">{t('dashboard.stats.historicalConsumption')}</span>
                  <span className="font-bold text-2xl text-orange-600">${statistics.total_consumed.toFixed(2)}</span>
                </div>
                <div className="h-12 w-px bg-border" />
                <div className="flex flex-col gap-1">
                  <span className="text-sm text-muted-foreground">{t('dashboard.stats.currentBalance')}</span>
                  <span className="font-bold text-2xl text-green-600">${statistics.total_current_balance.toFixed(2)}</span>
                </div>
              </div>
            </CardContent>
          </Card>
        )}
      </div>

      {/* Search Bar */}
      {accounts && accounts.length > 0 && (
        <div className="relative">
          <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
          <Input
            placeholder={t('accounts.searchPlaceholder')}
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="pl-9 rounded-full"
          />
        </div>
      )}

      {/* Accounts List - Grouped by Provider */}
      {isLoading ? (
        <Card className="rounded-2xl">
          <CardContent className="pt-6">
            <p className="text-center text-muted-foreground">{t('accounts.loading')}</p>
          </CardContent>
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
                <Card key={providerId} className="border-2 rounded-2xl">
                  <CardHeader className="pb-3">
                    <div className="flex items-center justify-between">
                      <div className="flex items-center gap-3">
                        <CardTitle className="text-xl">{providerName}</CardTitle>
                        <Badge variant="outline" className="rounded-full">
                          {providerAccounts.length} {providerAccounts.length === 1 ? t('accounts.account') : t('accounts.accounts_plural')}
                        </Badge>
                        {enabledCount > 0 && (
                          <Badge variant="secondary" className="rounded-full">
                            {enabledCount} {t('accounts.enabled')}
                          </Badge>
                        )}
                      </div>
                      <div className="flex gap-2">
                        {enabledCount > 0 && (
                          <>
                            <Button
                              variant="outline"
                              size="sm"
                              onClick={() => handleRefreshProviderBalances(providerAccounts)}
                              disabled={refreshAllBalancesMutation.isPending}
                              className="rounded-full"
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
                      <div className="flex items-center gap-6 pt-2 text-sm">
                        <div className="flex items-center gap-2">
                          <DollarSign className="h-4 w-4 text-blue-600" />
                          <span className="text-muted-foreground">{t('dashboard.stats.totalIncome')}:</span>
                          <span className="font-semibold text-blue-600">${providerStats.total_income.toFixed(2)}</span>
                        </div>
                        <div className="flex items-center gap-2">
                          <span className="text-muted-foreground">{t('dashboard.stats.historicalConsumption')}:</span>
                          <span className="font-semibold text-orange-600">${providerStats.total_consumed.toFixed(2)}</span>
                        </div>
                        <div className="flex items-center gap-2">
                          <span className="text-muted-foreground">{t('dashboard.stats.currentBalance')}:</span>
                          <span className="font-semibold text-green-600">${providerStats.current_balance.toFixed(2)}</span>
                        </div>
                      </div>
                    )}
                  </CardHeader>
                  <CardContent>
                    <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
                      {providerAccounts.map((account) => (
                        <AccountCard
                          key={account.id}
                          account={account}
                          onEdit={handleEdit}
                        />
                      ))}
                    </div>
                  </CardContent>
                </Card>
              );
            })}
          </div>
        </>
      ) : accounts && accounts.length > 0 && searchQuery ? (
        <Card>
          <CardContent className="pt-6">
            <p className="text-center text-muted-foreground">
              {t('accounts.noResultsFor')} "{searchQuery}"
            </p>
          </CardContent>
        </Card>
      ) : (
        <Card>
          <CardHeader>
            <CardTitle>{t('accounts.noAccounts')}</CardTitle>
            <CardDescription>
              {t('accounts.noAccountsDescription')}
            </CardDescription>
          </CardHeader>
          <CardContent className="flex gap-2">
            <Button onClick={handleCreate}>
              <Plus className="mr-2 h-4 w-4" />
              {t('accounts.addAccount')}
            </Button>
            <Button variant="outline" onClick={() => setJsonImportDialogOpen(true)}>
              <Upload className="mr-2 h-4 w-4" />
              {t('accounts.importJSON')}
            </Button>
          </CardContent>
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
