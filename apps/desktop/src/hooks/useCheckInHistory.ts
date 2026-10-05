import { keepPreviousData, useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';
import { checkInKeys } from '@/lib/query-keys';
import { commands } from '@/lib/tauri';
import type {
  CheckInHistoryPageDto,
  CheckInStateDto,
  CheckInStatsDto,
  RunningJobDto,
} from '@/lib/tauri';

const unwrap = <T,>(
  r: { status: 'ok'; data: T } | { status: 'error'; error: { message: string } }
): T => {
  if (r.status === 'error') throw new Error(r.error.message);
  return r.data;
};

/**
 * Today's check-in read model for one account.
 *
 * Short `staleTime`: the state changes from `due` to `in_flight` to `done`
 * while the user is looking at the page, and a stale "ready to check in"
 * affordance is worse than an extra cheap query.
 */
export function useCheckInState(accountId: string, enabled = true) {
  return useQuery({
    queryKey: checkInKeys.state(accountId),
    queryFn: async (): Promise<CheckInStateDto> => unwrap(await commands.getCheckInState(accountId)),
    enabled: enabled && !!accountId,
    staleTime: 15_000,
  });
}

/**
 * Paginated attempt history. `accountId` may be `null` to span all accounts.
 * Keeps the previous page on screen while the next one loads.
 */
export function useCheckInHistory(
  accountId: string | null,
  page: number,
  pageSize: number,
  enabled = true
) {
  return useQuery({
    queryKey: checkInKeys.history(accountId, page, pageSize),
    queryFn: async (): Promise<CheckInHistoryPageDto> =>
      unwrap(await commands.getCheckInHistory(accountId, page, pageSize)),
    enabled,
    placeholderData: keepPreviousData,
    staleTime: 30_000,
  });
}

/** Aggregate check-in statistics for one account over a quota-day window. */
export function useCheckInStats(accountId: string, periodDays: number, enabled = true) {
  return useQuery({
    queryKey: checkInKeys.stats(accountId, periodDays),
    queryFn: async (): Promise<CheckInStatsDto> =>
      unwrap(await commands.getCheckInStats(accountId, periodDays)),
    enabled: enabled && !!accountId,
    staleTime: 30_000,
  });
}

/**
 * In-progress check-in runs. Polled only while someone is watching, which is
 * why `refetchInterval` is tied to `enabled` instead of a fixed number.
 */
export function useRunningJobs(enabled = true) {
  return useQuery({
    queryKey: checkInKeys.runningJobs(),
    queryFn: async (): Promise<RunningJobDto[]> => unwrap(await commands.getRunningJobs()),
    enabled,
    refetchInterval: enabled ? 1500 : false,
    staleTime: 0,
  });
}

/**
 * Ask a running batch to stop. Cancellation is cooperative on the backend: an
 * account already in flight finishes, the rest are skipped.
 */
export function useStopCheckIn() {
  const queryClient = useQueryClient();
  const { t } = useTranslation();

  return useMutation({
    mutationFn: async (jobId: string): Promise<boolean> =>
      unwrap(await commands.stopCheckIn(jobId)),
    onSuccess: (stopped) => {
      queryClient.invalidateQueries({ queryKey: checkInKeys.runningJobs() });
      if (stopped) {
        toast.success(t('checkIn.stopRequested'));
      } else {
        // The job finished between the render and the click.
        toast.info(t('checkIn.stopTooLate'));
      }
    },
    onError: (error: unknown) => {
      const reason = error instanceof Error ? error.message : String(error);
      toast.error(t('checkIn.stopFailed', { reason }));
      console.error('Failed to stop check-in job:', reason);
    },
  });
}
