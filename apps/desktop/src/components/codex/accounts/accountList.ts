import { getRemainingPercent, pickPrimaryDisplayWindow } from '@/components/codex/shared/quota';
import type { CodexAccountDto } from '@/lib/tauri';

export type CodexAccountSortOption = 'remaining-desc' | 'remaining-asc' | 'created-desc' | 'created-asc';
export type CodexAccountStatusFilter = 'all' | 'active' | 'expired' | 'banned';

type CodexAccountListOptions = {
  searchQuery?: string;
  hideNoQuota: boolean;
  onlyUnlimited: boolean;
  onlyValidAuth: boolean;
  sortOption: CodexAccountSortOption;
  statusFilter: CodexAccountStatusFilter;
  planSortOrder?: 'none' | 'asc' | 'desc';
  accountSortOrder?: 'none' | 'asc' | 'desc';
  isPinned?: (account: CodexAccountDto) => boolean;
};

export function buildCodexAccountList(
  accounts: CodexAccountDto[],
  options: CodexAccountListOptions
) {
  const filteredAccounts = accounts.filter((account) => matchesCodexAccountFilters(account, options));

  return [...filteredAccounts].sort((left, right) => {
    const leftPinned = options.isPinned?.(left) ?? false;
    const rightPinned = options.isPinned?.(right) ?? false;

    if (leftPinned !== rightPinned) {
      return leftPinned ? -1 : 1;
    }

    const accountSortDiff = compareByAccountLabel(left, right, options.accountSortOrder ?? 'none');
    if (accountSortDiff !== 0) {
      return accountSortDiff;
    }

    const planSortDiff = compareByPlanLabel(left, right, options.planSortOrder ?? 'none');
    if (planSortDiff !== 0) {
      return planSortDiff;
    }

    return compareCodexAccounts(left, right, options.sortOption);
  });
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
  if (!matchesSearchQuery(account, options.searchQuery)) {
    return false;
  }

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

function matchesSearchQuery(account: CodexAccountDto, searchQuery?: string) {
  const normalizedQuery = searchQuery?.trim().toLowerCase();
  if (!normalizedQuery) {
    return true;
  }

  const accountLabel = (account.email ?? '').toLowerCase();
  const planLabel = (account.planType ?? '').toLowerCase();
  return accountLabel.includes(normalizedQuery) || planLabel.includes(normalizedQuery);
}

function compareByAccountLabel(
  left: CodexAccountDto,
  right: CodexAccountDto,
  order: 'none' | 'asc' | 'desc'
) {
  if (order === 'none') {
    return 0;
  }

  const leftLabel = left.email.toLowerCase();
  const rightLabel = right.email.toLowerCase();
  const diff = leftLabel.localeCompare(rightLabel);
  return order === 'asc' ? diff : -diff;
}

function compareByPlanLabel(
  left: CodexAccountDto,
  right: CodexAccountDto,
  order: 'none' | 'asc' | 'desc'
) {
  if (order === 'none') {
    return 0;
  }

  const leftMissing = !left.planType;
  const rightMissing = !right.planType;
  if (leftMissing !== rightMissing) {
    return leftMissing ? 1 : -1;
  }

  const leftLabel = getPlanSortLabel(left);
  const rightLabel = getPlanSortLabel(right);
  const diff = leftLabel.localeCompare(rightLabel);
  return order === 'asc' ? diff : -diff;
}

function getPlanSortLabel(account: CodexAccountDto) {
  return account.planType?.toLowerCase() ?? '';
}

function compareCodexAccounts(
  left: CodexAccountDto,
  right: CodexAccountDto,
  sortOption: CodexAccountSortOption
) {
  switch (sortOption) {
    case 'remaining-asc': {
      const scoreDiff = getRemainingQuotaScore(left) - getRemainingQuotaScore(right);
      if (scoreDiff !== 0) {
        return scoreDiff;
      }

      const createdDiff = compareByCreatedAt(left, right);
      if (createdDiff !== 0) {
        return createdDiff;
      }

      return left.email.localeCompare(right.email);
    }
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
