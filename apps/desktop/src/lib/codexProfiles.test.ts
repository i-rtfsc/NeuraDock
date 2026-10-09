import { describe, expect, it } from 'vitest';
import { availableProfileSlug, emailDomainPrefix, relayShortName } from './codexProfiles';

describe('Codex profile names', () => {
  it('uses relay abbreviations and email domains without exposing email usernames', () => {
    expect(`${relayShortName('AnyRouter')}-${emailDomainPrefix('private-user@qq.com')}`).toBe('any-qq');
    expect(`${relayShortName('AgentRouter')}-${emailDomainPrefix('Work <someone@gmail.com>')}`).toBe('agent-gmail');
    expect(emailDomainPrefix('Main account')).toBe('');
  });

  it('finds an unused suffix for accounts sharing the same domain', () => {
    expect(availableProfileSlug('any-qq', ['any-qq', 'any-qq-2'])).toBe('any-qq-3');
    expect(availableProfileSlug('agent-gmail', ['any-qq'])).toBe('agent-gmail');
  });
});
