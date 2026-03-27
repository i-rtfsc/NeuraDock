export type TransitHubTab = 'dashboard' | 'accounts' | 'providers';

export const DEFAULT_TRANSIT_HUB_TAB: TransitHubTab = 'dashboard';

export function normalizeTransitHubTab(value?: string | null): TransitHubTab {
  switch (value) {
    case 'accounts':
    case 'providers':
    case 'dashboard':
      return value;
    default:
      return DEFAULT_TRANSIT_HUB_TAB;
  }
}

export function buildTransitHubPath(tab: TransitHubTab = DEFAULT_TRANSIT_HUB_TAB) {
  return `/providers?tab=${tab}`;
}
