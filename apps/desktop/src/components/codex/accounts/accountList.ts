import { getRemainingPercent, pickPrimaryDisplayWindow } from '@/components/codex/shared/quota';
import type { CodexAccountDto } from '@/lib/tauri';

export type CodexAccountSortOption = 'remaining-desc' | 'created-desc' | 'created-asc';
export type CodexAccountStatusFilter = 'all' | 'active' | 'expired' | 'banned';

type CodexAccountListOptions = {
  hideNoQuota: boolean;
  onlyUnlimited: boolean;
  onlyValidAuth: boolean;
  sortOption: CodexAccountSortOption;
  statusFilter: CodexAccountStatusFilter;
};

export function buildCodexAccountList(
  accounts: CodexAccountDto[],
  options: CodexAccountListOptions
) {
  const filteredAccounts = accounts.filter((account) => matchesCodexAccountFilters(account, options));

  return [...filteredAccounts].sort((left, right) => compareCodexAccounts(left, right, options.sortOption));
}

export function hasValidCodexAuth(account: CodexAccountDto) {
  return account.hasTokens && !account.isTokenExpired;
}

export function hasNoRemainingQuota(account: CodexAccountDto) {
  if (account.isUnlimited) {
    return false;
  }

  const displayWindow = pickPrimaryDisplayWindow(account.secondaryWindow, account.primaryWindow);
  const remainingPercent = getRemainingPercent(displayWindow);

  if (remainingPercent != null) {
    return remainingPercent <= 0;
  }

  return account.hasCredits === false;
}

function matchesCodexAccountFilters(account: CodexAccountDto, options: CodexAccountListOptions) {
  if (options.hideNoQuota && hasNoRemainingQuota(account)) {
    return false;
  }

  if (options.onlyUnlimited && !account.isUnlimited) {
    return false;
  }

  if (options.onlyValidAuth && !hasValidCodexAuth(account)) {
    return false;
  }

  if (options.statusFilter !== 'all' && account.status !== options.statusFilter) {
    return false;
  }

  return true;
}

function compareCodexAccounts(
  left: CodexAccountDto,
  right: CodexAccountDto,
  sortOption: CodexAccountSortOption
) {
  switch (sortOption) {
    case 'created-asc':
      return compareByCreatedAt(left, right);
    case 'created-desc':
      return compareByCreatedAt(right, left);
    case 'remaining-desc':
    default: {
      const scoreDiff = getRemainingQuotaScore(right) - getRemainingQuotaScore(left);
      if (scoreDiff !== 0) {
        return scoreDiff;
      }

      const createdDiff = compareByCreatedAt(right, left);
      if (createdDiff !== 0) {
        return createdDiff;
      }

      return left.email.localeCompare(right.email);
    }
  }
}

function getRemainingQuotaScore(account: CodexAccountDto) {
  if (account.isUnlimited) {
    return 101;
  }

  const displayWindow = pickPrimaryDisplayWindow(account.secondaryWindow, account.primaryWindow);
  const remainingPercent = getRemainingPercent(displayWindow);

  if (remainingPercent != null) {
    return remainingPercent > 0 ? remainingPercent : -1;
  }

  if (account.hasCredits === true) {
    return 0.5;
  }

  if (account.hasCredits === false) {
    return -1;
  }

  return 0;
}

function compareByCreatedAt(left: CodexAccountDto, right: CodexAccountDto) {
  return getCreatedAtTimestamp(left) - getCreatedAtTimestamp(right);
}

function getCreatedAtTimestamp(account: CodexAccountDto) {
  const timestamp = new Date(account.createdAt).getTime();
  return Number.isNaN(timestamp) ? 0 : timestamp;
}
