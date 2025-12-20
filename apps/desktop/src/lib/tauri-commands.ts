import { invoke } from '@tauri-apps/api/core';

import type {
  AccountDetailDto,
  AccountDto,
  BatchCheckInResult,
  BatchImportResult,
  CheckInHistoryDto,
  CheckInCalendarDto,
  CheckInDayDto,
  CheckInStreakDto,
  CheckInTrendDto,
  CreateAccountInput,
  ExecuteCheckInResult,
  ExportAccountsInput,
  MonthStatsDto,
  TrendDataPoint,
  UpdateAccountInput,
} from './tauri';

export type Account = AccountDto;
export type AccountDetail = AccountDetailDto;
export type { CreateAccountInput, ExportAccountsInput, UpdateAccountInput };
export type {
  CheckInCalendarDto,
  CheckInDayDto,
  CheckInStreakDto,
  CheckInTrendDto,
  MonthStatsDto,
  TrendDataPoint,
};

// Account Commands
export const accountCommands = {
  getAll: (enabledOnly: boolean = false) =>
    invoke<AccountDto[]>('get_all_accounts', { enabledOnly }),

  getDetail: (accountId: string) =>
    invoke<AccountDetailDto>('get_account_detail', { accountId }),

  create: (input: CreateAccountInput) =>
    invoke<string>('create_account', { input }),

  update: (input: UpdateAccountInput) =>
    invoke<boolean>('update_account', { input }),

  delete: (accountId: string) =>
    invoke<boolean>('delete_account', { accountId }),

  toggle: (accountId: string, enabled: boolean) =>
    invoke<boolean>('toggle_account', { accountId, enabled }),

  importFromJson: (jsonData: string) =>
    invoke<string>('import_account_from_json', { jsonData }),

  importBatch: (jsonData: string) =>
    invoke<BatchImportResult>('import_accounts_batch', { jsonData }),

  exportToJson: (accountIds: string[], includeCredentials: boolean) =>
    invoke<string>('export_accounts_to_json', {
      input: {
        account_ids: accountIds,
        include_credentials: includeCredentials,
      } satisfies ExportAccountsInput,
    }),
};

// Check-in Commands (placeholder for future implementation)
export const checkInCommands = {
  execute: (accountId: string) =>
    invoke<ExecuteCheckInResult>('execute_check_in', { accountId }),

  executeBatch: (accountIds: string[]) =>
    invoke<BatchCheckInResult>('execute_batch_check_in', { accountIds }),

  getHistory: (accountId: string, page: number, pageSize: number) =>
    invoke<CheckInHistoryDto[]>('get_check_in_history', { accountId, page, pageSize }),

  getStreak: (accountId: string) =>
    invoke<CheckInStreakDto>('get_check_in_streak', { accountId }),

  getAllStreaks: () => invoke<CheckInStreakDto[]>('get_all_check_in_streaks'),

  getCalendar: (accountId: string, year: number, month: number) =>
    invoke<CheckInCalendarDto>('get_check_in_calendar', { accountId, year, month }),

  getTrend: (accountId: string, days: number = 30) =>
    invoke<CheckInTrendDto>('get_check_in_trend', { accountId, days }),

  getDayDetail: (accountId: string, date: string) =>
    invoke<CheckInDayDto>('get_check_in_day_detail', { accountId, date }),

  recalculateStreaks: () => invoke<void>('recalculate_check_in_streaks'),
};
