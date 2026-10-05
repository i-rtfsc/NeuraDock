// Centralized Query Keys for TanStack Query

export const accountKeys = {
  all: ['accounts'] as const,
  lists: () => [...accountKeys.all, 'list'] as const,
  list: (enabledOnly: boolean) => [...accountKeys.lists(), { enabledOnly }] as const,
  details: () => [...accountKeys.all, 'detail'] as const,
  detail: (id: string) => [...accountKeys.details(), id] as const,
  stats: () => [...accountKeys.all, 'stats'] as const,
  balance: (accountId: string) => [...accountKeys.detail(accountId), 'balance'] as const,
  tokens: (accountId: string) => [...accountKeys.detail(accountId), 'tokens'] as const,
  balanceStatistics: () => ['balance-statistics'] as const,
};

export const providerKeys = {
  all: ['providers'] as const,
  lists: () => [...providerKeys.all, 'list'] as const,
  list: () => [...providerKeys.lists()] as const,
  detail: (id: string) => [...providerKeys.all, 'detail', id] as const,
};

export const checkInKeys = {
  all: ['check-in'] as const,
  streak: (accountId: string) => [...checkInKeys.all, 'streak', accountId] as const,
  streaks: () => [...checkInKeys.all, 'streaks'] as const,
  // Account-scoped prefixes, so cache invalidation can target one account's
  // calendar/trend/day-detail queries without knowing their parameters.
  calendars: (accountId: string) => [...checkInKeys.all, 'calendar', accountId] as const,
  calendar: (accountId: string, year: number, month: number) => 
    [...checkInKeys.calendars(accountId), { year, month }] as const,
  trends: (accountId: string) => [...checkInKeys.all, 'trend', accountId] as const,
  trend: (accountId: string, days: number) => 
    [...checkInKeys.trends(accountId), { days }] as const,
  dayDetails: (accountId: string) => [...checkInKeys.all, 'day-detail', accountId] as const,
  dayDetail: (accountId: string, date: string) =>
    [...checkInKeys.dayDetails(accountId), date] as const,
};

export const notificationKeys = {
  all: ['notifications'] as const,
  channels: () => [...notificationKeys.all, 'channels'] as const,
};
