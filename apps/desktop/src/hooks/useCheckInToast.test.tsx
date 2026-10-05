import React from 'react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { act, renderHook } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { toast } from 'sonner';
import { commands } from '@/lib/tauri';
import type { ExecuteCheckInResult } from '@/lib/tauri';
import { useBatchCheckIn, useCheckIn, type CheckInResult } from './useCheckIn';

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
    info: vi.fn(),
  },
}));

// The shared setup mock returns the key untouched, which would hide whether the
// reason code or the backend sentence ended up in the toast. These templates
// keep the interpolation so the assertions can tell them apart.
const { tMock } = vi.hoisted(() => {
  const templates: Record<string, string> = {
    'checkIn.success': 'ok',
    'checkIn.alreadyConfirmed': 'already',
    'checkIn.balanceInfo': ' balance {{amount}}',
    'checkIn.failedWithReason': 'failed: {{reason}}',
    'checkIn.skippedWithReason': 'skipped: {{reason}}',
    'checkIn.batchSummary': 'batch {{succeeded}}/{{total}}',
    'checkIn.batchAllFailed': 'batchfail {{failed}}/{{total}}',
    'checkIn.viewDetails': 'VIEW',
    'checkIn.errorCode.network': 'NETWORK-LABEL',
    'checkIn.errorCode.timeout': 'TIMEOUT-LABEL',
    'common.unknownError': 'unknown',
  };
  return {
    tMock: vi.fn((key: string, options?: Record<string, unknown>) => {
      const template = templates[key] ?? key;
      if (!options) return template;
      return Object.entries(options).reduce(
        (text, [name, value]) => text.replace(`{{${name}}}`, String(value)),
        template
      );
    }),
  };
});

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: tMock,
    i18n: { language: 'zh-CN', changeLanguage: vi.fn() },
  }),
}));

function executeResult(overrides: Partial<ExecuteCheckInResult>): ExecuteCheckInResult {
  return {
    account_id: 'acc-1',
    account_name: 'Alpha',
    provider_id: 'provider-1',
    success: false,
    balance: null,
    error: null,
    outcome: 'failed_retryable',
    error_code: null,
    ...overrides,
  };
}

describe('check-in result toasts', () => {
  let wrapper: ({ children }: { children: React.ReactNode }) => React.ReactElement;

  beforeEach(() => {
    const queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
    });
    wrapper = ({ children }: { children: React.ReactNode }) => (
      <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
    );
    vi.clearAllMocks();
  });

  it('prefers the translated reason code over the backend message', async () => {
    vi.mocked(commands.executeCheckIn).mockResolvedValue({
      status: 'ok',
      data: executeResult({
        outcome: 'failed_retryable',
        error_code: 'network',
        error: 'connection reset by peer',
      }),
    });

    const { result } = renderHook(() => useCheckIn(), { wrapper });
    await act(async () => {
      await result.current.mutateAsync('acc-1');
    });

    // failed_retryable -> warning tone.
    expect(toast.warning).toHaveBeenCalledWith('failed: NETWORK-LABEL');
    expect(toast.error).not.toHaveBeenCalled();
    expect(tMock).toHaveBeenCalledWith('checkIn.errorCode.network');
  });

  it('falls back to the backend message for an unknown reason code', async () => {
    vi.mocked(commands.executeCheckIn).mockResolvedValue({
      status: 'ok',
      data: executeResult({
        outcome: 'failed_permanent',
        error_code: 'guard_blocked',
        error: 'provider refused the request',
      }),
    });

    const { result } = renderHook(() => useCheckIn(), { wrapper });
    await act(async () => {
      await result.current.mutateAsync('acc-1');
    });

    expect(toast.error).toHaveBeenCalledWith('failed: provider refused the request');
  });

  it('reports an already-done day as a success, not a failure', async () => {
    vi.mocked(commands.executeCheckIn).mockResolvedValue({
      status: 'ok',
      data: executeResult({ success: true, outcome: 'already_confirmed', error_code: null }),
    });

    const { result } = renderHook(() => useCheckIn(), { wrapper });
    await act(async () => {
      await result.current.mutateAsync('acc-1');
    });

    expect(toast.success).toHaveBeenCalledWith('already');
    expect(toast.error).not.toHaveBeenCalled();
  });

  it('reports a guard skip as info', async () => {
    vi.mocked(commands.executeCheckIn).mockResolvedValue({
      status: 'ok',
      data: executeResult({
        outcome: 'skipped_guard',
        error: 'another attempt is already in flight',
      }),
    });

    const { result } = renderHook(() => useCheckIn(), { wrapper });
    await act(async () => {
      await result.current.mutateAsync('acc-1');
    });

    expect(toast.info).toHaveBeenCalledWith(
      'skipped: another attempt is already in flight'
    );
  });

  it('attaches a view-details action to the batch toast and hands over the results', async () => {
    const results = [
      executeResult({ account_id: 'acc-1', success: true, outcome: 'success' }),
      executeResult({ account_id: 'acc-2', outcome: 'failed_retryable', error_code: 'timeout' }),
    ];
    vi.mocked(commands.executeBatchCheckIn).mockResolvedValue({
      status: 'ok',
      data: { total: 2, succeeded: 1, failed: 1, results },
    });
    const onViewDetails = vi.fn();

    const { result } = renderHook(() => useBatchCheckIn({ onViewDetails }), { wrapper });
    await act(async () => {
      await result.current.mutateAsync(['acc-1', 'acc-2']);
    });

    // One partial success -> a warning toast carrying the details action.
    expect(toast.warning).toHaveBeenCalledTimes(1);
    const [message, options] = vi.mocked(toast.warning).mock.calls[0] as unknown as [
      string,
      { action?: { label: string; onClick: () => void } },
    ];
    expect(message).toBe('batch 1/2');
    expect(options.action?.label).toBe('VIEW');

    act(() => {
      options.action?.onClick();
    });
    // The hook normalises `balance: null` to `undefined`, so compare the fields
    // that matter for the results dialog.
    expect(onViewDetails).toHaveBeenCalledTimes(1);
    const handedOver = onViewDetails.mock.calls[0][0] as CheckInResult[];
    expect(handedOver.map((item) => [item.account_id, item.outcome, item.error_code])).toEqual([
      ['acc-1', 'success', null],
      ['acc-2', 'failed_retryable', 'timeout'],
    ]);
  });

  it('uses an error tone when every account failed', async () => {
    vi.mocked(commands.executeBatchCheckIn).mockResolvedValue({
      status: 'ok',
      data: {
        total: 2,
        succeeded: 0,
        failed: 2,
        results: [
          executeResult({ account_id: 'acc-1', outcome: 'failed_permanent' }),
          executeResult({ account_id: 'acc-2', outcome: 'failed_permanent' }),
        ],
      },
    });

    const { result } = renderHook(() => useBatchCheckIn(), { wrapper });
    await act(async () => {
      await result.current.mutateAsync(['acc-1', 'acc-2']);
    });

    expect(toast.error).toHaveBeenCalledTimes(1);
    expect(vi.mocked(toast.error).mock.calls[0][0]).toBe('batchfail 2/2');
  });
});
