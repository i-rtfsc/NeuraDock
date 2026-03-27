import { describe, expect, it } from 'vitest';

import { normalizeAiToolboxSection } from './aiToolbox';

describe('aiToolbox helpers', () => {
  it('normalizes unknown sections to transit', () => {
    expect(normalizeAiToolboxSection('transit')).toBe('transit');
    expect(normalizeAiToolboxSection('tokens')).toBe('tokens');
    expect(normalizeAiToolboxSection('chat')).toBe('chat');
    expect(normalizeAiToolboxSection('codex')).toBe('codex');
    expect(normalizeAiToolboxSection('unknown')).toBe('transit');
    expect(normalizeAiToolboxSection(null)).toBe('transit');
  });
});
