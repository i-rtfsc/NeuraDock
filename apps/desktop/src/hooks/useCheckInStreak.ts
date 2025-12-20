import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { checkInCommands } from '@/lib/tauri-commands';

// Hook: Get streak for a single account
export function useCheckInStreak(accountId: string, enabled = true) {
  return useQuery({
    queryKey: ['checkInStreak', accountId],
    queryFn: () => checkInCommands.getStreak(accountId),
    enabled: enabled && !!accountId,
    staleTime: 60000, // 1 minute
  });
}

// Hook: Get all streaks
export function useAllCheckInStreaks() {
  return useQuery({
    queryKey: ['checkInStreaks'],
    queryFn: () => checkInCommands.getAllStreaks(),
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
    queryKey: ['checkInCalendar', accountId, year, month],
    queryFn: () => checkInCommands.getCalendar(accountId, year, month),
    enabled: enabled && !!accountId,
    staleTime: 60000, // 1 minute
  });
}

// Hook: Get trend data (last N days)
export function useCheckInTrend(accountId: string, days: number = 30, enabled = true) {
  return useQuery({
    queryKey: ['checkInTrend', accountId, days],
    queryFn: () => checkInCommands.getTrend(accountId, days),
    enabled: enabled && !!accountId,
    staleTime: 60000, // 1 minute
  });
}

// Hook: Get day detail
export function useCheckInDayDetail(accountId: string, date: string, enabled = true) {
  return useQuery({
    queryKey: ['checkInDayDetail', accountId, date],
    queryFn: () => checkInCommands.getDayDetail(accountId, date),
    enabled: enabled && !!accountId && !!date,
  });
}

// Hook: Recalculate all streaks (mutation)
export function useRecalculateStreaks() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: () => checkInCommands.recalculateStreaks(),
    onSuccess: () => {
      // Invalidate all streak-related queries
      queryClient.invalidateQueries({ queryKey: ['checkInStreaks'] });
      queryClient.invalidateQueries({ queryKey: ['checkInStreak'] });
      queryClient.invalidateQueries({ queryKey: ['checkInCalendar'] });
      queryClient.invalidateQueries({ queryKey: ['checkInTrend'] });
    },
  });
}
