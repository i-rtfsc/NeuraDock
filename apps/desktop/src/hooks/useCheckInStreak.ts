import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { checkInCommands } from '@/lib/tauri-commands';
import { checkInKeys } from '@/lib/query-keys';

export function useConfirmCheckInReward(accountId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (date: string) => checkInCommands.confirmReward(accountId, date),
    onSuccess: async () => {
      // Confirming a reward changes this account's streak, calendar, trend and
      // day detail; `checkInKeys.all` covers all four families by prefix.
      await queryClient.invalidateQueries({ queryKey: checkInKeys.all });
    },
  });
}

// Hook: Get streak for a single account
export function useCheckInStreak(accountId: string, enabled = true) {
  return useQuery({
    queryKey: checkInKeys.streak(accountId),
    queryFn: () => checkInCommands.getStreak(accountId),
    enabled: enabled && !!accountId,
    staleTime: 60000, // 1 minute
  });
}

// Hook: Get calendar for a specific month
export function useCheckInCalendar(
  accountId: string,
  year: number,
  month: number,
  enabled = true
) {
  return useQuery({
    queryKey: checkInKeys.calendar(accountId, year, month),
    queryFn: () => checkInCommands.getCalendar(accountId, year, month),
    enabled: enabled && !!accountId,
    staleTime: 60000, // 1 minute
  });
}

// Hook: Get trend data (last N days)
export function useCheckInTrend(accountId: string, days: number = 30, enabled = true) {
  return useQuery({
    queryKey: checkInKeys.trend(accountId, days),
    queryFn: () => checkInCommands.getTrend(accountId, days),
    enabled: enabled && !!accountId,
    staleTime: 60000, // 1 minute
  });
}

// Hook: Get day detail
export function useCheckInDayDetail(accountId: string, date: string, enabled = true) {
  return useQuery({
    queryKey: checkInKeys.dayDetail(accountId, date),
    queryFn: () => checkInCommands.getDayDetail(accountId, date),
    enabled: enabled && !!accountId && !!date,
  });
}
