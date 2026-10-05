import { QueryClient } from '@tanstack/react-query';
import { accountKeys, providerKeys, checkInKeys, notificationKeys } from './query-keys';

/**
 * Centralized cache invalidation utilities to ensure consistent
 * query cache updates across the application.
 */
export const cacheInvalidators = {
  /**
   * Invalidate caches for a specific account
   */
  invalidateAccount: (queryClient: QueryClient, accountId: string) => {
    // This will also invalidate sub-keys like balance and tokens
    queryClient.invalidateQueries({ queryKey: accountKeys.detail(accountId) });
  },

  /**
   * Invalidate all accounts list and related aggregate data
   */
  invalidateAllAccounts: (queryClient: QueryClient) => {
    queryClient.invalidateQueries({ queryKey: accountKeys.all });
    // We might want to refetch the list immediately
    queryClient.refetchQueries({ queryKey: accountKeys.lists(), type: 'active' });
    
    queryClient.invalidateQueries({ queryKey: accountKeys.balanceStatistics() });
    queryClient.refetchQueries({ queryKey: accountKeys.balanceStatistics(), type: 'active' });
  },

  /**
   * Invalidate every cache affected by a check-in for one account:
   * its streak / calendar / trend / day-detail plus the aggregate streak list
   * and the account's own record, balance and balance statistics.
   */
  invalidateCheckIn: (queryClient: QueryClient, accountId: string) => {
    queryClient.invalidateQueries({ queryKey: checkInKeys.streak(accountId) });
    queryClient.invalidateQueries({ queryKey: checkInKeys.calendars(accountId) });
    queryClient.invalidateQueries({ queryKey: checkInKeys.trends(accountId) });
    queryClient.invalidateQueries({ queryKey: checkInKeys.dayDetails(accountId) });
    queryClient.invalidateQueries({ queryKey: checkInKeys.streaks() });

    queryClient.invalidateQueries({ queryKey: accountKeys.detail(accountId) });
    queryClient.invalidateQueries({ queryKey: accountKeys.lists() });
    queryClient.invalidateQueries({ queryKey: accountKeys.balance(accountId) });
    queryClient.invalidateQueries({ queryKey: accountKeys.balanceStatistics() });
  },

  /**
   * Invalidate caches after check-in operation
   */
  invalidateAfterCheckIn: (queryClient: QueryClient, accountId: string) => {
    cacheInvalidators.invalidateCheckIn(queryClient, accountId);
  },

  /**
   * Invalidate provider-related caches
   */
  invalidateProvider: (queryClient: QueryClient, providerId?: string) => {
    queryClient.invalidateQueries({ queryKey: providerKeys.all });
    if (providerId) {
      queryClient.invalidateQueries({ queryKey: providerKeys.detail(providerId) });
    }
  },

  /**
   * Invalidate notification channels
   */
  invalidateNotificationChannels: (queryClient: QueryClient) => {
    queryClient.invalidateQueries({ queryKey: notificationKeys.channels() });
  },
};
