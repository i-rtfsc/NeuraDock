import React from 'react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { act, renderHook, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { commands } from '@/lib/tauri';
import type { ExecuteCheckInResult } from '@/lib/tauri';
import {
  checkInCommands,
  type CheckInCalendarDto,
  type CheckInDayDto,
  type CheckInStreakDto,
  type CheckInTrendDto,
} from '@/lib/tauri-commands';
import { accountKeys, checkInKeys } from '@/lib/query-keys';
import { useBatchCheckIn, useCheckIn } from './useCheckIn';
import {
  useCheckInCalendar,
  useCheckInDayDetail,
  useCheckInStreak,
  useCheckInTrend,
} from './useCheckInStreak';

vi.mock('@/lib/tauri', () => ({
  commands: {
    executeCheckIn: vi.fn(),
    executeBatchCheckIn: vi.fn(),
  },
}));

vi.mock('sonner', () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
    warning: vi.fn(),
  },
}));

// Mock the read commands so the check-in hooks can be observed refetching.
vi.mock('@/lib/tauri-commands', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/lib/tauri-commands')>();
  return {
    ...actual,
    checkInCommands: {
      ...actual.checkInCommands,
      getStreak: vi.fn(),
      getCalendar: vi.fn(),
      getTrend: vi.fn(),
      getDayDetail: vi.fn(),
    },
  };
});

const DAY = '2026-01-02';
const YEAR = 2026;
const MONTH = 0;
const TREND_DAYS = 30;

function executeResult(accountId: string): ExecuteCheckInResult {
  return {
    account_id: accountId,
    account_name: `account ${accountId}`,
    provider_id: 'provider-1',
    success: true,
    balance: null,
    error: null,
    outcome: 'success',
  };
}

function streakFixture(accountId: string): CheckInStreakDto {
  return {
    account_id: accountId,
    account_name: `account ${accountId}`,
    provider_id: 'provider-1',
    provider_name: 'Provider',
    current_streak: 1,
    longest_streak: 2,
    total_check_in_days: 3,
    inferred_days: 0,
    last_check_in_date: DAY,
  };
}

function dayFixture(date: string): CheckInDayDto {
  return {
    date,
    is_checked_in: true,
    income_increment: 1,
    reward_is_estimate: false,
    current_balance: 10,
    daily_consumed: 1,
    total_consumed: 2,
    total_quota: 100,
  };
}

function calendarFixture(accountId: string): CheckInCalendarDto {
  return {
    account_id: accountId,
    year: YEAR,
    month: MONTH,
    days: [dayFixture(DAY)],
    month_stats: {
      total_days: 31,
      elapsed_days: 2,
      checked_in_days: 1,
      check_in_rate: 0.03,
      total_quota_increment: 1,
    },
  };
}

function trendFixture(accountId: string): CheckInTrendDto {
  return {
    account_id: accountId,
    start_date: DAY,
    end_date: DAY,
    data_points: [
      {
        date: DAY,
        total_quota: 100,
        income_increment: 1,
        reward_is_estimate: false,
        current_balance: 10,
        is_checked_in: true,
      },
    ],
  };
}

// Renders every query that must refresh after a check-in.
function useCheckInCaches(accountId: string) {
  return {
    checkIn: useCheckIn(),
    streak: useCheckInStreak(accountId),
    calendar: useCheckInCalendar(accountId, YEAR, MONTH),
    trend: useCheckInTrend(accountId, TREND_DAYS),
    dayDetail: useCheckInDayDetail(accountId, DAY),
  };
}

describe('check-in cache invalidation', () => {
  let queryClient: QueryClient;
  let wrapper: ({ children }: { children: React.ReactNode }) => React.ReactElement;

  const seedCheckInCaches = (accountId: string) => {
    queryClient.setQueryData(checkInKeys.streak(accountId), streakFixture(accountId));
    queryClient.setQueryData(checkInKeys.calendar(accountId, YEAR, MONTH), calendarFixture(accountId));
    queryClient.setQueryData(checkInKeys.trend(accountId, TREND_DAYS), trendFixture(accountId));
    queryClient.setQueryData(checkInKeys.dayDetail(accountId, DAY), dayFixture(DAY));
    queryClient.setQueryData(checkInKeys.streaks(), [streakFixture(accountId)]);
    queryClient.setQueryData(accountKeys.balance(accountId), { current_balance: 10 });
  };

  beforeEach(() => {
    queryClient = new QueryClient({
      defaultOptions: {
        queries: { retry: false },
        mutations: { retry: false },
      },
    });

    wrapper = ({ children }: { children: React.ReactNode }) => (
      <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
    );

    vi.clearAllMocks();
  });

  it('invalidates the streak / calendar / trend / day-detail / balance prefixes of the account', async () => {
    vi.mocked(commands.executeCheckIn).mockResolvedValue({
      status: 'ok',
      data: executeResult('acc-1'),
    });
    seedCheckInCaches('acc-1');
    // A second account must stay untouched by a single check-in.
    seedCheckInCaches('acc-2');
    const invalidateSpy = vi.spyOn(queryClient, 'invalidateQueries');

    const { result } = renderHook(() => useCheckIn(), { wrapper });
    await act(async () => {
      await result.current.mutateAsync('acc-1');
    });

    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: checkInKeys.streak('acc-1') });
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: checkInKeys.calendars('acc-1') });
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: checkInKeys.trends('acc-1') });
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: checkInKeys.dayDetails('acc-1') });
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: checkInKeys.streaks() });
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: accountKeys.balance('acc-1') });
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: accountKeys.lists() });
    expect(invalidateSpy).not.toHaveBeenCalledWith({ queryKey: ['check-in-streak'] });

    // Prefix matching really reaches the cached queries (not just the spy calls).
    expect(queryClient.getQueryState(checkInKeys.streak('acc-1'))?.isInvalidated).toBe(true);
    expect(queryClient.getQueryState(checkInKeys.calendar('acc-1', YEAR, MONTH))?.isInvalidated).toBe(true);
    expect(queryClient.getQueryState(checkInKeys.trend('acc-1', TREND_DAYS))?.isInvalidated).toBe(true);
    expect(queryClient.getQueryState(checkInKeys.dayDetail('acc-1', DAY))?.isInvalidated).toBe(true);
    expect(queryClient.getQueryState(checkInKeys.streaks())?.isInvalidated).toBe(true);
    expect(queryClient.getQueryState(accountKeys.balance('acc-1'))?.isInvalidated).toBe(true);

    expect(queryClient.getQueryState(checkInKeys.streak('acc-2'))?.isInvalidated).toBe(false);
    expect(queryClient.getQueryState(checkInKeys.calendar('acc-2', YEAR, MONTH))?.isInvalidated).toBe(false);
  });

  it('refetches the queries the streak hooks registered for the account', async () => {
    vi.mocked(commands.executeCheckIn).mockResolvedValue({
      status: 'ok',
      data: executeResult('acc-1'),
    });
    vi.mocked(checkInCommands.getStreak).mockImplementation(async (accountId: string) =>
      streakFixture(accountId)
    );
    vi.mocked(checkInCommands.getCalendar).mockImplementation(
      async (accountId: string) => calendarFixture(accountId)
    );
    vi.mocked(checkInCommands.getTrend).mockImplementation(async (accountId: string) =>
      trendFixture(accountId)
    );
    vi.mocked(checkInCommands.getDayDetail).mockImplementation(async (_accountId: string, date: string) =>
      dayFixture(date)
    );

    const { result } = renderHook(() => useCheckInCaches('acc-1'), { wrapper });

    await waitFor(() => {
      expect(result.current.dayDetail.isSuccess).toBe(true);
    });
    expect(vi.mocked(checkInCommands.getStreak)).toHaveBeenCalledTimes(1);
    expect(vi.mocked(checkInCommands.getCalendar)).toHaveBeenCalledTimes(1);
    expect(vi.mocked(checkInCommands.getTrend)).toHaveBeenCalledTimes(1);
    expect(vi.mocked(checkInCommands.getDayDetail)).toHaveBeenCalledTimes(1);

    await act(async () => {
      await result.current.checkIn.mutateAsync('acc-1');
    });

    await waitFor(() => {
      expect(vi.mocked(checkInCommands.getStreak)).toHaveBeenCalledTimes(2);
      expect(vi.mocked(checkInCommands.getCalendar)).toHaveBeenCalledTimes(2);
      expect(vi.mocked(checkInCommands.getTrend)).toHaveBeenCalledTimes(2);
      expect(vi.mocked(checkInCommands.getDayDetail)).toHaveBeenCalledTimes(2);
    });
  });

  it('invalidates every account returned by the batch check-in', async () => {
    vi.mocked(commands.executeBatchCheckIn).mockResolvedValue({
      status: 'ok',
      data: {
        total: 2,
        succeeded: 2,
        failed: 0,
        results: [executeResult('acc-1'), executeResult('acc-2')],
      },
    });
    const invalidateSpy = vi.spyOn(queryClient, 'invalidateQueries');

    const { result } = renderHook(() => useBatchCheckIn(), { wrapper });
    await act(async () => {
      await result.current.mutateAsync(['acc-1', 'acc-2']);
    });

    for (const accountId of ['acc-1', 'acc-2']) {
      expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: checkInKeys.streak(accountId) });
      expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: checkInKeys.calendars(accountId) });
      expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: checkInKeys.trends(accountId) });
      expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: checkInKeys.dayDetails(accountId) });
      expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: accountKeys.balance(accountId) });
    }
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: checkInKeys.streaks() });
    expect(invalidateSpy).not.toHaveBeenCalledWith({ queryKey: ['check-in-streak'] });
  });
});
