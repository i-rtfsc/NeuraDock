import { describe, expect, it } from 'vitest';

import { buildTransitHubPath, normalizeTransitHubTab } from './transitHub';

describe('transitHub helpers', () => {
  it('normalizes unknown tabs to the dashboard tab', () => {
    expect(normalizeTransitHubTab('dashboard')).toBe('dashboard');
    expect(normalizeTransitHubTab('accounts')).toBe('accounts');
    expect(normalizeTransitHubTab('providers')).toBe('providers');
    expect(normalizeTransitHubTab('unknown')).toBe('dashboard');
    expect(normalizeTransitHubTab(null)).toBe('dashboard');
  });

  it('builds stable hub paths for each tab', () => {
    expect(buildTransitHubPath()).toBe('/providers?tab=dashboard');
    expect(buildTransitHubPath('accounts')).toBe('/providers?tab=accounts');
    expect(buildTransitHubPath('providers')).toBe('/providers?tab=providers');
  });
});
