export function isClaudeModel(name: string): boolean {
  const normalized = name.trim().toLowerCase();
  const parts = normalized.split(/[/:]/);
  const alias = parts[parts.length - 1] ?? normalized;
  return normalized.includes('claude') || /^(sonnet|opus|haiku)(?:[-_.]|$)/.test(alias);
}

export function compatibleModels(models: readonly string[], agent: 'codex' | 'claude_code', chatGateway = false): string[] {
  return models.filter((name) => {
    if (agent === 'codex') return !isClaudeModel(name);
    if (chatGateway) return true;
    const parts = name.trim().toLowerCase().split(/[/:]/);
    const normalized = parts[parts.length - 1] ?? '';
    return !(/^(gpt|chatgpt|o[134])(?:[-_.]|$)/.test(normalized) || normalized.includes('codex'));
  });
}
