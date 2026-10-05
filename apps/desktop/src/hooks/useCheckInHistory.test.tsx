import React from 'react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { act, renderHook, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { toast } from 'sonner';
import { commands } from '@/lib/tauri';
import type {
  CheckInHistoryPageDto,
  CheckInStateDto,
  CheckInStatsDto,
  RunningJobDto,
} from '@/lib/tauri';
import { checkInKeys } from '@/lib/query-keys';
import {
  useCheckInHistory,
  useCheckInState,
  useCheckInStats,
  useRunningJobs,
  useStopCheckIn,
} from './useCheckInHistory';

vi.mock('@/lib/tauri', () => ({
  commands: {
    getCheckInState: vi.fn(),
    getCheckInHistory: vi.fn(),
    getCheckInStats: vi.fn(),
    getRunningJobs: vi.fn(),
    stopCheckIn: vi.fn(),
  },
}));

vi.mock('sonner', () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
    warning: vi.fn(),
    info: vi.fn(),
  },
}));

const ACCOUNT = 'acc-1';

function stateFixture(state: string): CheckInStateDto {
  return {
    account_id: ACCOUNT,
    quota_day: '2026-01-02',
    state,
    confidence: state === 'done' ? 'confirmed' : null,
    until: null,
    retryable_failures: 0,
  };
}

function historyFixture(page: number, total = 25): CheckInHistoryPageDto {
  return {
    total,
    page,
    page_size: 10,
    attempts: [
      {
        id: page,
        account_id: ACCOUNT,
        account_name: 'Alpha',
        provider_name: 'Provider',
        quota_day: '2026-01-02',
        trigger: 'manual',
        outcome: 'success',
        error_code: null,
        http_status: null,
        started_at: '2026-01-02T03:04:05Z',
        finished_at: '2026-01-02T03:04:06Z',
        duration_ms: 1200,
        reward: 25,
        source: 'provider_response',
      },
    ],
  };
}

function statsFixture(periodDays: number): CheckInStatsDto {
  return {
    account_id: ACCOUNT,
    period_days: periodDays,
    start_date: '2025-12-04',
    end_date: '2026-01-02',
    attempts: 10,
    succeeded: 8,
    failed: 1,
    skipped: 1,
    success_rate: 88.9,
    average_duration_ms: 1500,
    confirmed_days: 7,
    inferred_days: 1,
    total_reward: 175,
    failures_by_code: [{ error_code: 'timeout', count: 1 }],
  };
}

function jobFixture(overrides: Partial<RunningJobDto> = {}): RunningJobDto {
  return {
    id: 'job-1',
    account_ids: [ACCOUNT],
    started_at: '2026-01-02T03:04:05Z',
    processed: 1,
    total: 2,
    cancelled: false,
    ...overrides,
  };
}

describe('check-in read-model hooks', () => {
  let queryClient: QueryClient;
  let wrapper: ({ children }: { children: React.ReactNode }) => React.ReactElement;

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

  /** Defaulted query options, which the cache keeps but does not type. */
  const queryOptions = (key: readonly unknown[]) =>
    queryClient.getQueryCache().find({ queryKey: key })?.options as
      | { staleTime?: number; refetchInterval?: number | false }
      | undefined;

  it("loads today's state with a short stale time", async () => {
    vi.mocked(commands.getCheckInState).mockResolvedValue({
      status: 'ok',
      data: stateFixture('due'),
    });

    const { result } = renderHook(() => useCheckInState(ACCOUNT), { wrapper });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(commands.getCheckInState).toHaveBeenCalledWith(ACCOUNT);
    expect(result.current.data?.state).toBe('due');
    expect(queryOptions(checkInKeys.state(ACCOUNT))?.staleTime).toBe(15000);
  });

  it('requests statistics for the selected period', async () => {
    vi.mocked(commands.getCheckInStats).mockResolvedValue({
      status: 'ok',
      data: statsFixture(30),
    });

    const { result } = renderHook(() => useCheckInStats(ACCOUNT, 30), { wrapper });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(commands.getCheckInStats).toHaveBeenCalledWith(ACCOUNT, 30);
    expect(result.current.data?.success_rate).toBeCloseTo(88.9);
    expect(result.current.data?.failures_by_code).toEqual([{ error_code: 'timeout', count: 1 }]);
  });

  it('does not query when disabled', () => {
    renderHook(() => useCheckInState(ACCOUNT, false), { wrapper });
    renderHook(() => useCheckInHistory(ACCOUNT, 1, 10, false), { wrapper });
    renderHook(() => useCheckInStats(ACCOUNT, 30, false), { wrapper });

    expect(commands.getCheckInState).not.toHaveBeenCalled();
    expect(commands.getCheckInHistory).not.toHaveBeenCalled();
    expect(commands.getCheckInStats).not.toHaveBeenCalled();
  });

  it('keeps the previous history page on screen while the next one loads', async () => {
    vi.mocked(commands.getCheckInHistory).mockResolvedValue({
      status: 'ok',
      data: historyFixture(1),
    });

    const { result, rerender } = renderHook(
      ({ page }: { page: number }) => useCheckInHistory(ACCOUNT, page, 10),
      { wrapper, initialProps: { page: 1 } }
    );

    await waitFor(() => expect(result.current.data?.page).toBe(1));

    let release!: (value: { status: 'ok'; data: CheckInHistoryPageDto }) => void;
    vi.mocked(commands.getCheckInHistory).mockReturnValueOnce(
      new Promise<{ status: 'ok'; data: CheckInHistoryPageDto }>((resolve) => {
        release = resolve;
      })
    );

    rerender({ page: 2 });

    await waitFor(() => expect(result.current.isPlaceholderData).toBe(true));
    expect(result.current.data?.page).toBe(1);
    expect(commands.getCheckInHistory).toHaveBeenLastCalledWith(ACCOUNT, 2, 10);

    release({ status: 'ok', data: historyFixture(2) });
    await waitFor(() => expect(result.current.data?.page).toBe(2));
    expect(result.current.isPlaceholderData).toBe(false);
  });

  it('passes null through to the all-accounts history query', async () => {
    vi.mocked(commands.getCheckInHistory).mockResolvedValue({
      status: 'ok',
      data: historyFixture(1),
    });

    const { result } = renderHook(() => useCheckInHistory(null, 1, 10), { wrapper });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(commands.getCheckInHistory).toHaveBeenCalledWith(null, 1, 10);
    expect(
      queryClient.getQueryCache().find({ queryKey: checkInKeys.history(null, 1, 10) })
    ).toBeDefined();
  });

  it('polls running jobs only while enabled', async () => {
    vi.mocked(commands.getRunningJobs).mockResolvedValue({ status: 'ok', data: [jobFixture()] });

    const enabled = renderHook(() => useRunningJobs(true), { wrapper });
    await waitFor(() => expect(enabled.result.current.isSuccess).toBe(true));
    expect(queryOptions(checkInKeys.runningJobs())?.refetchInterval).toBe(1500);
    enabled.unmount();
    queryClient.clear();

    const disabled = renderHook(() => useRunningJobs(false), { wrapper });
    expect(commands.getRunningJobs).toHaveBeenCalledTimes(1);
    expect(queryOptions(checkInKeys.runningJobs())?.refetchInterval).toBe(false);
    disabled.unmount();
  });

  it('invalidates the running-job list after a successful stop', async () => {
    vi.mocked(commands.stopCheckIn).mockResolvedValue({ status: 'ok', data: true });
    const invalidateSpy = vi.spyOn(queryClient, 'invalidateQueries');

    const { result } = renderHook(() => useStopCheckIn(), { wrapper });
    await act(async () => {
      await result.current.mutateAsync('job-1');
    });

    expect(commands.stopCheckIn).toHaveBeenCalledWith('job-1');
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: checkInKeys.runningJobs() });
    expect(toast.success).toHaveBeenCalledWith('checkIn.stopRequested');
  });

  it('tells the user when the job finished before the click landed', async () => {
    vi.mocked(commands.stopCheckIn).mockResolvedValue({ status: 'ok', data: false });

    const { result } = renderHook(() => useStopCheckIn(), { wrapper });
    await act(async () => {
      await result.current.mutateAsync('job-1');
    });

    expect(toast.info).toHaveBeenCalledWith('checkIn.stopTooLate');
  });
});
