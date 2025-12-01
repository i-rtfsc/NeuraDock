import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';

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
  });
}

// Mutation: Refresh account balance (force refresh)
export function useRefreshAccountBalance() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (accountId: string) =>
      invoke<BalanceDto>('fetch_account_balance', { accountId, forceRefresh: true }),
    onSuccess: (data, accountId) => {
      // Update the cache with the new balance
      queryClient.setQueryData(['balance', accountId], data);
      // Invalidate related queries
      queryClient.invalidateQueries({ queryKey: ['accounts'] });
      queryClient.invalidateQueries({ queryKey: ['balance-statistics'] });
    },
  });
}

// Mutation: Refresh all account balances (force refresh)
export function useRefreshAllBalances() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (accountIds: string[]) =>
      invoke<Record<string, BalanceDto | null>>('fetch_accounts_balances', {
        accountIds,
        forceRefresh: true
      }),
    onSuccess: (data) => {
      // Update cache for each account
      Object.entries(data).forEach(([accountId, balance]) => {
        if (balance) {
          queryClient.setQueryData(['balance', accountId], balance);
        }
      });
      // Invalidate related queries
      queryClient.invalidateQueries({ queryKey: ['accounts'] });
      queryClient.invalidateQueries({ queryKey: ['balance-statistics'] });
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
