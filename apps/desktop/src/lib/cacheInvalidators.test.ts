import { describe, expect, it, vi, beforeEach } from 'vitest';
import { QueryClient } from '@tanstack/react-query';
import { cacheInvalidators } from './cacheInvalidators';
import { accountKeys, checkInKeys } from './query-keys';

describe('checkInKeys', () => {
  it('namespaces every new read model under the check-in family', () => {
    expect(checkInKeys.state('acc-1')).toEqual(['check-in', 'state', 'acc-1']);
    expect(checkInKeys.history('acc-1', 2, 25)).toEqual([
      'check-in',
      'history',
      'acc-1',
      { page: 2, pageSize: 25 },
    ]);
    expect(checkInKeys.history(null, 1, 10)).toEqual([
      'check-in',
      'history',
      'all',
      { page: 1, pageSize: 10 },
    ]);
    expect(checkInKeys.stats('acc-1', 30)).toEqual([
      'check-in',
      'stats',
      'acc-1',
      { periodDays: 30 },
    ]);
    expect(checkInKeys.runningJobs()).toEqual(['check-in', 'running-jobs']);
  });

  it('keeps every history page of an account under one prefix', () => {
    const prefix = checkInKeys.histories('acc-1');
    const page = checkInKeys.history('acc-1', 3, 10);
    expect(page.slice(0, prefix.length)).toEqual([...prefix]);
    // Another account, and the all-accounts listing, stay outside the prefix.
    expect(checkInKeys.history('acc-2', 1, 10).slice(0, prefix.length)).not.toEqual([...prefix]);
    expect(checkInKeys.history(null, 1, 10).slice(0, prefix.length)).not.toEqual([...prefix]);
  });

  it('keeps every stats period of an account under the bare-account prefix', () => {
    const prefix = checkInKeys.stats('acc-1');
    expect(prefix).toEqual(['check-in', 'stats', 'acc-1']);
    expect(checkInKeys.stats('acc-1', 90).slice(0, prefix.length)).toEqual([...prefix]);
    expect(checkInKeys.stats('acc-2', 90).slice(0, prefix.length)).not.toEqual([...prefix]);
  });
});

describe('cacheInvalidators.invalidateCheckIn', () => {
  let queryClient: QueryClient;

  beforeEach(() => {
    queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  });

  it('invalidates the new read models', () => {
    const invalidateSpy = vi.spyOn(queryClient, 'invalidateQueries');

    cacheInvalidators.invalidateCheckIn(queryClient, 'acc-1');

    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: checkInKeys.state('acc-1') });
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: checkInKeys.histories('acc-1') });
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: checkInKeys.stats('acc-1') });
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: checkInKeys.runningJobs() });
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: accountKeys.detail('acc-1') });
  });

  it('really reaches the cached new queries, and only for the account involved', () => {
    queryClient.setQueryData(checkInKeys.state('acc-1'), { state: 'due' });
    queryClient.setQueryData(checkInKeys.history('acc-1', 1, 10), { total: 0, attempts: [] });
    queryClient.setQueryData(checkInKeys.stats('acc-1', 30), { attempts: 0 });
    queryClient.setQueryData(checkInKeys.runningJobs(), []);
    queryClient.setQueryData(checkInKeys.state('acc-2'), { state: 'due' });

    cacheInvalidators.invalidateCheckIn(queryClient, 'acc-1');

    expect(queryClient.getQueryState(checkInKeys.state('acc-1'))?.isInvalidated).toBe(true);
    expect(queryClient.getQueryState(checkInKeys.history('acc-1', 1, 10))?.isInvalidated).toBe(true);
    expect(queryClient.getQueryState(checkInKeys.stats('acc-1', 30))?.isInvalidated).toBe(true);
    expect(queryClient.getQueryState(checkInKeys.runningJobs())?.isInvalidated).toBe(true);
    expect(queryClient.getQueryState(checkInKeys.state('acc-2'))?.isInvalidated).toBe(false);
  });
});
