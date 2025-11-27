import { useMutation, useQueryClient } from '@tanstack/react-query';
import { toast } from 'sonner';

// Types for check-in
export interface CheckInResult {
  job_id: string;
  success: boolean;
  balance?: {
    quota: number;
    used: number;
    remaining: number;
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

  return useMutation({
    mutationFn: executeCheckIn,
    onSuccess: (data, accountId) => {
      // Invalidate accounts query to refresh balance
      queryClient.invalidateQueries({ queryKey: ['accounts'] });
      // Also invalidate the specific account's balance query
      queryClient.invalidateQueries({ queryKey: ['balance', accountId] });
      // Invalidate balance statistics
      queryClient.invalidateQueries({ queryKey: ['balance-statistics'] });
      
      if (data.success) {
        const balanceInfo = data.balance
          ? ` Balance: $${data.balance.quota.toFixed(2)}`
          : '';
        toast.success(`Check-in successful!${balanceInfo}`);
      } else {
        toast.error(`Check-in failed: ${data.error || 'Unknown error'}`);
      }
    },
    onError: (error: any) => {
      console.error('Check-in error:', error);
      const errorMessage = error?.message || error?.toString() || 'Unknown error';
      toast.error(`Check-in failed: ${errorMessage}`);
    },
  });
}

// Batch check-in hook
export function useBatchCheckIn() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: executeBatchCheckIn,
    onSuccess: (data) => {
      // Invalidate accounts query to refresh all balances
      queryClient.invalidateQueries({ queryKey: ['accounts'] });
      // Invalidate all balance queries
      queryClient.invalidateQueries({ queryKey: ['balance'] });
      // Invalidate balance statistics
      queryClient.invalidateQueries({ queryKey: ['balance-statistics'] });
      
      if (data.succeeded > 0) {
        toast.success(
          `Batch check-in completed: ${data.succeeded}/${data.total} successful`
        );
      }
      
      if (data.failed > 0) {
        toast.warning(
          `${data.failed} account(s) failed to check in. Check details for more info.`
        );
      }
    },
    onError: (error: any) => {
      console.error('Batch check-in error:', error);
      const errorMessage = error?.message || error?.toString() || 'Unknown error';
      toast.error(`Batch check-in failed: ${errorMessage}`);
    },
  });
}
