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
  // Today's read-model state for one account. One key per account, so the key
  // itself is also the invalidation prefix.
  state: (accountId: string) => [...checkInKeys.all, 'state', accountId] as const,
  // Attempt history, optionally across all accounts (`null`).
  histories: (accountId: string | null) =>
    [...checkInKeys.all, 'history', accountId ?? 'all'] as const,
  history: (accountId: string | null, page: number, pageSize: number) =>
    [...checkInKeys.histories(accountId), { page, pageSize }] as const,
  // Aggregate stats for one account. With a period it is one query key; with
  // just the account it is the prefix that invalidates every period — the same
  // plural/singular trick as `calendars`/`calendar`.
  stats: (accountId: string, periodDays?: number) =>
    periodDays === undefined
      ? ([...checkInKeys.all, 'stats', accountId] as const)
      : ([...checkInKeys.all, 'stats', accountId, { periodDays }] as const),
  // Not account-scoped: a running job is a global, in-memory fact.
  runningJobs: () => [...checkInKeys.all, 'running-jobs'] as const,
};

export const notificationKeys = {
  all: ['notifications'] as const,
  channels: () => [...notificationKeys.all, 'channels'] as const,
};
