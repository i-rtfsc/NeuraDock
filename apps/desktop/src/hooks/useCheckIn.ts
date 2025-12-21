import { useMutation, useQueryClient } from '@tanstack/react-query';
import { toast } from 'sonner';
import { useTranslation } from 'react-i18next';
import { cacheInvalidators } from '@/lib/cacheInvalidators';

// Types for check-in
export interface CheckInResult {
  account_id: string;
  account_name: string;
  provider_id: string;
  success: boolean;
  balance?: {
    current_balance: number;
    total_consumed: number;
    total_income: number;
  };
  error?: string;
}

export interface BatchCheckInResult {
  total: number;
  succeeded: number;
  failed: number;
  results: CheckInResult[];
}

// Tauri commands
async function executeCheckIn(accountId: string): Promise<CheckInResult> {
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke('execute_check_in', { accountId });
}

async function executeBatchCheckIn(accountIds: string[]): Promise<BatchCheckInResult> {
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke('execute_batch_check_in', { accountIds });
}

// Single check-in hook
export function useCheckIn() {
  const queryClient = useQueryClient();
  const { t } = useTranslation();

  return useMutation<CheckInResult, any, string | undefined>({
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

      if (data.success) {
        const balanceInfo = data.balance
          ? t('checkIn.balanceInfo', {
              defaultValue: ' 余额: ${{amount}}',
              amount: data.balance.current_balance.toFixed(2),
            })
          : '';
        toast.success(`${t('checkIn.success', '签到成功！')}${balanceInfo}`);
      } else {
        toast.error(
          t('checkIn.failedWithReason', {
            defaultValue: '签到失败: {{reason}}',
            reason: data.error || t('common.unknownError', '未知错误'),
          })
        );
      }
    },
    onError: (error: any, accountId) => {
      console.error('Check-in error:', error);
      if (accountId) {
        queryClient.invalidateQueries({ queryKey: ['account', accountId] });
      }
      const errorMessage = error?.message || error?.toString() || t('common.unknownError', '未知错误');
      toast.error(
        t('checkIn.failedWithReason', {
          defaultValue: '签到失败: {{reason}}',
          reason: errorMessage,
        })
      );
    },
  });
}

// Batch check-in hook
export function useBatchCheckIn() {
  const queryClient = useQueryClient();
  const { t } = useTranslation();

  return useMutation({
    mutationFn: executeBatchCheckIn,
    onSuccess: (data) => {
      cacheInvalidators.invalidateAllAccounts(queryClient);
      queryClient.invalidateQueries({ queryKey: ['check-in-streak'] });

      if (data.succeeded > 0) {
        toast.success(
          t('checkIn.batchSummary', {
            defaultValue: '批量签到完成：{{succeeded}}/{{total}} 成功',
            succeeded: data.succeeded,
            total: data.total,
          })
        );
      }

      if (data.failed > 0) {
        toast.warning(
          t('checkIn.batchFailedCount', {
            defaultValue: '{{failed}} 个账号签到失败，请查看详情。',
            failed: data.failed,
          })
        );
      }
    },
    onError: (error: any) => {
      console.error('Batch check-in error:', error);
      const errorMessage = error?.message || error?.toString() || t('common.unknownError', '未知错误');
      toast.error(
        t('checkIn.batchFailed', {
          defaultValue: '批量签到失败：{{reason}}',
          reason: errorMessage,
        })
      );
    },
  });
}
