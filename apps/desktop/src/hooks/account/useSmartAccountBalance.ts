import { useFetchAccountBalance } from '@/hooks/useBalance';
import { Account } from '@/lib/tauri-commands';

/**
 * Hook to smartly fetch account balance based on cache age.
 * It encapsulates the logic of deciding whether to fetch fresh data or use cached data.
 */
export function useSmartAccountBalance(account: Account) {
  // Get cache age from settings (default to 1 hour)
  // Reading from localStorage directly in render loop is not ideal for performance,
  // but acceptable for low frequency. Ideally this should come from a settings context/hook.
  const maxCacheAgeHours = parseInt(localStorage.getItem('maxCacheAgeHours') || '1', 10);
  const maxCacheAgeMs = maxCacheAgeHours * 60 * 60 * 1000;

  // Only fetch fresh balance if account is enabled AND balance is stale/missing
  const shouldFetchBalance = account.enabled && (
    !account.current_balance || 
    !account.total_consumed || 
    account.total_income == null ||
    !account.last_balance_check_at ||
    (new Date().getTime() - new Date(account.last_balance_check_at).getTime()) > maxCacheAgeMs
  );

  // Fetch fresh balance in background (with smart caching from TanStack Query)
  const queryResult = useFetchAccountBalance(
    account.id,
    shouldFetchBalance
  );

  // Determine the balance to display: prefer fresh data, fallback to account properties
  const displayBalance = queryResult.data || (
    account.current_balance != null && account.total_consumed != null && account.total_income != null ? {
      current_balance: account.current_balance,
      total_consumed: account.total_consumed,
      total_income: account.total_income,
    } : null
  );

  return {
    balance: displayBalance,
    isLoading: queryResult.isLoading,
    isFetching: queryResult.isFetching,
    error: queryResult.error,
    shouldFetch: shouldFetchBalance,
  };
}
