export function relayShortName(name: string): string {
  const normalized = name.trim().toLowerCase();
  if (normalized.includes('anyrouter')) return 'any';
  if (normalized.includes('agentrouter')) return 'agent';
  return normalized.replace(/[^a-z0-9]+/g, '-').replace(/^-+|-+$/g, '') || 'relay';
}

export function emailDomainPrefix(account: string | null | undefined): string {
  const domain = account?.match(/@([a-z0-9-]+)(?:\.[a-z0-9-]+)+/i)?.[1];
  return domain?.toLowerCase() ?? '';
}

export function availableProfileSlug(base: string, used: readonly string[]): string {
  const taken = new Set(used.map((slug) => slug.toLowerCase()));
  if (!taken.has(base.toLowerCase())) return base;
  let suffix = 2;
  while (taken.has(`${base}-${suffix}`.toLowerCase())) suffix++;
  return `${base}-${suffix}`;
}
