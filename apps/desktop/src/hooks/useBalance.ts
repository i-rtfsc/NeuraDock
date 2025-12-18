import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import { toast } from 'sonner';
import { useTranslation } from 'react-i18next';
import { cacheInvalidators } from '@/lib/cacheInvalidators';
import { extractErrorMessage } from '@/lib/errorHandling';

export interface BalanceDto {
  current_balance: number;
  total_consumed: number;
  total_income: number;
}

export interface ProviderBalanceDto {
  provider_id: string;
  provider_name: string;
  current_balance: number;
  total_consumed: number;
  total_income: number;
  account_count: number;
}

export interface BalanceStatisticsDto {
  providers: ProviderBalanceDto[];
  total_current_balance: number;
  total_consumed: number;
  total_income: number;
}

// Query: Fetch account balance
export function useFetchAccountBalance(accountId: string, enabled = true) {
  return useQuery({
    queryKey: ['balance', accountId],
    queryFn: () => invoke<BalanceDto>('fetch_account_balance', { accountId, forceRefresh: false }),
    enabled: enabled && !!accountId,
    staleTime: 5 * 60 * 1000, // Consider data fresh for 5 minutes
    gcTime: 10 * 60 * 1000, // Keep in cache for 10 minutes
    refetchOnMount: false, // Don't refetch on component mount if data is fresh
    refetchOnWindowFocus: false, // Don't refetch on window focus
    retry: 1, // Only retry once on failure
  });
}

// Mutation: Refresh account balance (force refresh)
export function useRefreshAccountBalance() {
  const queryClient = useQueryClient();
  const { t } = useTranslation();

  return useMutation({
    mutationFn: (accountId: string) =>
      invoke<BalanceDto>('fetch_account_balance', { accountId, forceRefresh: true }),
    onSuccess: (data, accountId) => {
      queryClient.setQueryData(['balance', accountId], data);
      cacheInvalidators.invalidateAccount(queryClient, accountId);
      cacheInvalidators.invalidateAllAccounts(queryClient);
      toast.success(t('accountCard.balanceRefreshed', '余额已刷新'));
    },
    onError: (error: unknown) => {
      const message = extractErrorMessage(error);
      toast.error(
        t('accountCard.balanceRefreshFailed', {
          defaultValue: '刷新余额失败：{{message}}',
          message,
        })
      );
    },
  });
}

// Mutation: Refresh all account balances (force refresh)
export function useRefreshAllBalances() {
  const queryClient = useQueryClient();
  const { t } = useTranslation();

  return useMutation({
    mutationFn: (accountIds: string[]) =>
      invoke<Record<string, BalanceDto | null>>('fetch_accounts_balances', {
        accountIds,
        forceRefresh: true,
      }),
    onSuccess: (data) => {
      Object.entries(data).forEach(([accountId, balance]) => {
        if (balance) {
          queryClient.setQueryData(['balance', accountId], balance);
        }
      });
      cacheInvalidators.invalidateAllAccounts(queryClient);
      toast.success(t('accounts.balancesRefreshed', '所有余额已刷新'));
    },
    onError: (error: unknown) => {
      const message = extractErrorMessage(error);
      toast.error(
        t('accounts.refreshFailed', {
          defaultValue: '批量刷新失败：{{message}}',
          message,
        })
      );
    },
  });
}

// Query: Fetch multiple account balances
export function useFetchAccountsBalances(accountIds: string[]) {
  return useQuery({
    queryKey: ['balances', accountIds],
    queryFn: () => invoke<Record<string, BalanceDto | null>>('fetch_accounts_balances', { accountIds, forceRefresh: false }),
    enabled: accountIds.length > 0,
  });
}

// Query: Get balance statistics
export function useBalanceStatistics() {
  return useQuery({
    queryKey: ['balance-statistics'],
    queryFn: () => invoke<BalanceStatisticsDto>('get_balance_statistics'),
  });
}
