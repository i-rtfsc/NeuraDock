import { describe, expect, it } from 'vitest';

import type { CodexAccountDto } from '@/lib/tauri';

import { buildCodexAccountList, hasNoRemainingQuota, hasValidCodexAuth } from './accountList';

function createAccount(overrides: Partial<CodexAccountDto>): CodexAccountDto {
  return {
    id: overrides.id ?? crypto.randomUUID(),
    email: overrides.email ?? 'account@example.com',
    password: null,
    hasPassword: false,
    hasTokens: true,
    hasPaymentSession: false,
    accountId: null,
    planType: null,
    hasCredits: null,
    isUnlimited: false,
    creditBalance: null,
    primaryWindow: null,
    secondaryWindow: null,
    quotaCheckedAt: null,
    tokenExpiresAt: null,
    lastRefreshAt: null,
    source: 'register',
    status: 'active',
    createdAt: overrides.createdAt ?? '2026-01-01T00:00:00.000Z',
    updatedAt: overrides.updatedAt ?? '2026-01-01T00:00:00.000Z',
    tokenDaysRemaining: null,
    isTokenExpired: false,
    ...overrides,
  };
}

describe('accountList helpers', () => {
  it('sorts accounts by remaining quota before fallback states', () => {
    const unlimited = createAccount({
      email: 'unlimited@example.com',
      isUnlimited: true,
      createdAt: '2026-01-03T00:00:00.000Z',
    });
    const highQuota = createAccount({
      email: 'high@example.com',
      primaryWindow: {
        usedPercent: 15,
        windowMinutes: 1440,
        resetsAt: '2026-01-04T00:00:00.000Z',
      },
      createdAt: '2026-01-04T00:00:00.000Z',
    });
    const unknownQuota = createAccount({
      email: 'unknown@example.com',
      createdAt: '2026-01-05T00:00:00.000Z',
    });
    const noQuota = createAccount({
      email: 'empty@example.com',
      hasCredits: false,
      createdAt: '2026-01-06T00:00:00.000Z',
    });

    const ordered = buildCodexAccountList(
      [unknownQuota, noQuota, highQuota, unlimited],
      {
        sortOption: 'remaining-desc',
        hideNoQuota: false,
        onlyUnlimited: false,
        onlyValidAuth: false,
        statusFilter: 'all',
      }
    );

    expect(ordered.map((account) => account.email)).toEqual([
      'unlimited@example.com',
      'high@example.com',
      'unknown@example.com',
      'empty@example.com',
    ]);
  });

  it('hides only accounts with explicit no remaining quota', () => {
    const zeroPercent = createAccount({
      email: 'zero@example.com',
      primaryWindow: {
        usedPercent: 100,
        windowMinutes: 1440,
        resetsAt: '2026-01-04T00:00:00.000Z',
      },
    });
    const unknownQuota = createAccount({
      email: 'unknown@example.com',
    });
    const unlimited = createAccount({
      email: 'unlimited@example.com',
      isUnlimited: true,
    });

    const visible = buildCodexAccountList(
      [zeroPercent, unknownQuota, unlimited],
      {
        sortOption: 'remaining-desc',
        hideNoQuota: true,
        onlyUnlimited: false,
        onlyValidAuth: false,
        statusFilter: 'all',
      }
    );

    expect(hasNoRemainingQuota(zeroPercent)).toBe(true);
    expect(hasNoRemainingQuota(unknownQuota)).toBe(false);
    expect(visible.map((account) => account.email)).toEqual([
      'unlimited@example.com',
      'unknown@example.com',
    ]);
  });

  it('supports created-at sorting controls', () => {
    const older = createAccount({
      email: 'older@example.com',
      createdAt: '2026-01-01T00:00:00.000Z',
    });
    const newer = createAccount({
      email: 'newer@example.com',
      createdAt: '2026-01-03T00:00:00.000Z',
    });

    const newestFirst = buildCodexAccountList(
      [older, newer],
      {
        sortOption: 'created-desc',
        hideNoQuota: false,
        onlyUnlimited: false,
        onlyValidAuth: false,
        statusFilter: 'all',
      }
    );
    const oldestFirst = buildCodexAccountList(
      [older, newer],
      {
        sortOption: 'created-asc',
        hideNoQuota: false,
        onlyUnlimited: false,
        onlyValidAuth: false,
        statusFilter: 'all',
      }
    );

    expect(newestFirst.map((account) => account.email)).toEqual([
      'newer@example.com',
      'older@example.com',
    ]);
    expect(oldestFirst.map((account) => account.email)).toEqual([
      'older@example.com',
      'newer@example.com',
    ]);
  });

  it('supports ascending remaining-quota sorting', () => {
    const unlimited = createAccount({
      email: 'unlimited@example.com',
      isUnlimited: true,
    });
    const withQuota = createAccount({
      email: 'with-quota@example.com',
      primaryWindow: {
        usedPercent: 40,
        windowMinutes: 1440,
        resetsAt: '2026-01-04T00:00:00.000Z',
      },
    });
    const noQuota = createAccount({
      email: 'no-quota@example.com',
      hasCredits: false,
    });

    const ascending = buildCodexAccountList(
      [withQuota, unlimited, noQuota],
      {
        sortOption: 'remaining-asc',
        hideNoQuota: false,
        onlyUnlimited: false,
        onlyValidAuth: false,
        statusFilter: 'all',
      }
    );

    expect(ascending.map((account) => account.email)).toEqual([
      'no-quota@example.com',
      'with-quota@example.com',
      'unlimited@example.com',
    ]);
  });

  it('supports combining valid-auth, unlimited, and status filters', () => {
    const validUnlimited = createAccount({
      email: 'valid-unlimited@example.com',
      isUnlimited: true,
      hasTokens: true,
      isTokenExpired: false,
      status: 'active',
    });
    const expiredUnlimited = createAccount({
      email: 'expired-unlimited@example.com',
      isUnlimited: true,
      hasTokens: true,
      isTokenExpired: true,
      status: 'expired',
    });
    const validLimited = createAccount({
      email: 'valid-limited@example.com',
      hasTokens: true,
      isTokenExpired: false,
      status: 'active',
      primaryWindow: {
        usedPercent: 20,
        windowMinutes: 1440,
        resetsAt: '2026-01-04T00:00:00.000Z',
      },
    });

    const filtered = buildCodexAccountList(
      [expiredUnlimited, validLimited, validUnlimited],
      {
        sortOption: 'remaining-desc',
        hideNoQuota: false,
        onlyUnlimited: true,
        onlyValidAuth: true,
        statusFilter: 'active',
      }
    );

    expect(hasValidCodexAuth(validUnlimited)).toBe(true);
    expect(hasValidCodexAuth(expiredUnlimited)).toBe(false);
    expect(filtered.map((account) => account.email)).toEqual(['valid-unlimited@example.com']);
  });

  it('pins the active account to the first row after filtering', () => {
    const active = createAccount({
      id: 'active-account',
      email: 'active@example.com',
      hasCredits: false,
      createdAt: '2026-01-01T00:00:00.000Z',
    });
    const newer = createAccount({
      id: 'newer-account',
      email: 'newer@example.com',
      createdAt: '2026-01-04T00:00:00.000Z',
    });
    const older = createAccount({
      id: 'older-account',
      email: 'older@example.com',
      createdAt: '2026-01-02T00:00:00.000Z',
    });

    const ordered = buildCodexAccountList(
      [newer, active, older],
      {
        sortOption: 'created-desc',
        hideNoQuota: false,
        onlyUnlimited: false,
        onlyValidAuth: false,
        statusFilter: 'all',
        isPinned: (account) => account.id === active.id,
      }
    );

    expect(ordered.map((account) => account.email)).toEqual([
      'active@example.com',
      'newer@example.com',
      'older@example.com',
    ]);
  });

  it('filters by account email and plan label search query', () => {
    const plus = createAccount({
      email: 'alpha@example.com',
      planType: 'plus',
    });
    const free = createAccount({
      email: 'beta@example.com',
      planType: 'free',
    });
    const none = createAccount({
      email: 'gamma@example.com',
      planType: null,
    });

    const byAccount = buildCodexAccountList(
      [plus, free, none],
      {
        searchQuery: 'beta',
        sortOption: 'remaining-desc',
        hideNoQuota: false,
        onlyUnlimited: false,
        onlyValidAuth: false,
        statusFilter: 'all',
      }
    );

    const byPlan = buildCodexAccountList(
      [plus, free, none],
      {
        searchQuery: 'plus',
        sortOption: 'remaining-desc',
        hideNoQuota: false,
        onlyUnlimited: false,
        onlyValidAuth: false,
        statusFilter: 'all',
      }
    );

    expect(byAccount.map((account) => account.email)).toEqual(['beta@example.com']);
    expect(byPlan.map((account) => account.email)).toEqual(['alpha@example.com']);
  });

  it('supports alphabetical account sorting override', () => {
    const b = createAccount({ email: 'b@example.com', createdAt: '2026-01-01T00:00:00.000Z' });
    const a = createAccount({ email: 'a@example.com', createdAt: '2026-01-03T00:00:00.000Z' });
    const c = createAccount({ email: 'c@example.com', createdAt: '2026-01-02T00:00:00.000Z' });

    const asc = buildCodexAccountList(
      [b, a, c],
      {
        sortOption: 'created-desc',
        accountSortOrder: 'asc',
        hideNoQuota: false,
        onlyUnlimited: false,
        onlyValidAuth: false,
        statusFilter: 'all',
      }
    );

    const desc = buildCodexAccountList(
      [b, a, c],
      {
        sortOption: 'created-desc',
        accountSortOrder: 'desc',
        hideNoQuota: false,
        onlyUnlimited: false,
        onlyValidAuth: false,
        statusFilter: 'all',
      }
    );

    expect(asc.map((account) => account.email)).toEqual([
      'a@example.com',
      'b@example.com',
      'c@example.com',
    ]);
    expect(desc.map((account) => account.email)).toEqual([
      'c@example.com',
      'b@example.com',
      'a@example.com',
    ]);
  });

  it('supports alphabetical plan sorting override with missing plans at end', () => {
    const pro = createAccount({ email: 'a@example.com', planType: 'pro' });
    const free = createAccount({ email: 'b@example.com', planType: 'free' });
    const missing = createAccount({ email: 'c@example.com', planType: null });

    const asc = buildCodexAccountList(
      [pro, missing, free],
      {
        sortOption: 'created-desc',
        planSortOrder: 'asc',
        hideNoQuota: false,
        onlyUnlimited: false,
        onlyValidAuth: false,
        statusFilter: 'all',
      }
    );

    expect(asc.map((account) => account.email)).toEqual([
      'b@example.com',
      'a@example.com',
      'c@example.com',
    ]);
  });
});
