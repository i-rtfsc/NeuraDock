import { useEffect, useRef } from 'react';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { toast } from 'sonner';
import { useTranslation } from 'react-i18next';
import { cacheInvalidators } from '@/lib/cacheInvalidators';
import { accountKeys } from '@/lib/query-keys';
import { commands } from '@/lib/tauri';
import {
  checkInFailureLabel,
  checkInOutcomeTone,
  type CheckInTone,
} from '@/lib/checkInOutcome';

const unwrap = <T,>(r: { status: 'ok'; data: T } | { status: 'error'; error: { message: string } }): T => { if (r.status === 'error') throw new Error(r.error.message); return r.data; };

/** Pick the sonner entry point that matches an outcome's tone. */
function toastForTone(tone: CheckInTone) {
  switch (tone) {
    case 'success':
      return toast.success;
    case 'info':
      return toast.info;
    case 'warning':
      return toast.warning;
    default:
      return toast.error;
  }
}

// Types for check-in
export interface CheckInResult {
  account_id: string;
  account_name: string;
  provider_id: string;
  success: boolean;
  balance?: {
    current_balance: number;
    total_consumed: number;
    total_quota: number;
  };
  error?: string | null;
  /** Stable token for what happened (`success`, `waf_blocked`, ...). */
  outcome: string;
  /** Stable failure reason code (`network`, `http_status_server`, ...). */
  error_code: string | null;
}

export interface BatchCheckInResult {
  total: number;
  succeeded: number;
  failed: number;
  results: CheckInResult[];
}

// Tauri commands
async function executeCheckIn(accountId: string): Promise<CheckInResult> {
  const result = unwrap(await commands.executeCheckIn(accountId));
  return { ...result, balance: result.balance ?? undefined };
}

async function executeBatchCheckIn(accountIds: string[]): Promise<BatchCheckInResult> {
  const result = unwrap(await commands.executeBatchCheckIn(accountIds));
  return { ...result, results: result.results.map((item) => ({ ...item, balance: item.balance ?? undefined })) };
}

// Single check-in hook
export function useCheckIn() {
  const queryClient = useQueryClient();
  const { t } = useTranslation();

  return useMutation<CheckInResult, unknown, string | undefined>({
    mutationFn: (accountId) => {
      if (!accountId) {
        return Promise.reject(new Error('accountId is required'));
      }
      return executeCheckIn(accountId);
    },
    onSuccess: (data, accountId) => {
      if (accountId) {
        cacheInvalidators.invalidateAfterCheckIn(queryClient, accountId);
      }

      const tone = checkInOutcomeTone(data.outcome);
      const show = toastForTone(tone);

      if (tone === 'success') {
        if (data.outcome === 'already_confirmed') {
          show(t('checkIn.alreadyConfirmed'));
          return;
        }
        const balanceInfo = data.balance
          ? t('checkIn.balanceInfo', {
              defaultValue: ' 余额: ${{amount}}',
              amount: data.balance.current_balance.toFixed(2),
            })
          : '';
        show(`${t('checkIn.success', '签到成功！')}${balanceInfo}`);
        return;
      }

      // Prefer the translated reason code; the backend message is only a
      // fallback for codes this build does not know yet.
      const reason =
        checkInFailureLabel(data.error_code, t) ??
        data.error ??
        t('common.unknownError', '未知错误');
      show(
        data.outcome === 'skipped_guard'
          ? t('checkIn.skippedWithReason', { reason })
          : t('checkIn.failedWithReason', { reason })
      );
    },
    onError: (error: unknown, accountId) => {
      console.error('Check-in error:', error);
      if (accountId) {
        queryClient.invalidateQueries({ queryKey: accountKeys.detail(accountId) });
      }
      const errorMessage = error instanceof Error ? error.message : String(error);
      toast.error(
        t('checkIn.failedWithReason', {
          defaultValue: '签到失败: {{reason}}',
          reason: errorMessage,
        })
      );
    },
  });
}

export interface BatchCheckInOptions {
  /**
   * Called when the user clicks "view details" on the batch result toast.
   * Accounts page uses it to open the per-account results dialog.
   */
  onViewDetails?: (results: CheckInResult[]) => void;
}

// Batch check-in hook
export function useBatchCheckIn(options: BatchCheckInOptions = {}) {
  const queryClient = useQueryClient();
  const { t } = useTranslation();
  const onViewDetails = useRef(options.onViewDetails);

  useEffect(() => {
    onViewDetails.current = options.onViewDetails;
  }, [options.onViewDetails]);

  return useMutation({
    mutationFn: executeBatchCheckIn,
    onSuccess: (data) => {
      cacheInvalidators.invalidateAllAccounts(queryClient);
      // Invalidate the check-in caches (state/history/stats/streak/calendar/
      // trend/day-detail, plus the aggregate streak list) for every account
      // involved in the batch.
      for (const accountId of new Set(data.results.map((item) => item.account_id))) {
        cacheInvalidators.invalidateCheckIn(queryClient, accountId);
      }

      const tone: CheckInTone =
        data.failed === 0 ? 'success' : data.succeeded > 0 ? 'warning' : 'error';
      const message =
        data.succeeded > 0
          ? t('checkIn.batchSummary', {
              defaultValue: '批量签到完成：{{succeeded}}/{{total}} 成功',
              succeeded: data.succeeded,
              total: data.total,
            })
          : t('checkIn.batchAllFailed', {
              defaultValue: '批量签到失败：{{failed}}/{{total}} 个账号未成功',
              failed: data.failed,
              total: data.total,
            });

      toastForTone(tone)(message, {
        duration: 8000,
        action: {
          label: t('checkIn.viewDetails', '查看详情'),
          onClick: () => onViewDetails.current?.(data.results),
        },
      });
    },
    onError: (error: unknown) => {
      console.error('Batch check-in error:', error);
      const errorMessage = error instanceof Error ? error.message : String(error);
      toast.error(
        t('checkIn.batchFailed', {
          defaultValue: '批量签到失败：{{reason}}',
          reason: errorMessage,
        })
      );
    },
  });
}
