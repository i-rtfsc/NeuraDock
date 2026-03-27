export type AiToolboxSection = 'transit' | 'tokens' | 'chat' | 'codex';

export const DEFAULT_AI_TOOLBOX_SECTION: AiToolboxSection = 'transit';

export function normalizeAiToolboxSection(value?: string | null): AiToolboxSection {
  switch (value) {
    case 'tokens':
    case 'chat':
    case 'codex':
    case 'transit':
      return value;
    default:
      return DEFAULT_AI_TOOLBOX_SECTION;
  }
}
