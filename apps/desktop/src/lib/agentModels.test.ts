import { expect, it } from 'vitest';
import { compatibleModels } from './agentModels';

it('keeps Responses-compatible candidates and excludes Anthropic names for Codex', () => {
  const models = ['gpt-test', 'deepseek-v4-flash', 'claude-sonnet-test', 'anthropic/claude-opus-test', 'opus', 'custom-model'];
  expect(compatibleModels(models, 'codex')).toEqual(['gpt-test', 'deepseek-v4-flash', 'custom-model']);
  expect(compatibleModels(models, 'codex', true)).toEqual(['gpt-test', 'deepseek-v4-flash', 'custom-model']);
});

it('allows OpenAI models for Claude only through the chat gateway', () => {
  expect(compatibleModels(['gpt-test', 'claude-sonnet-test'], 'claude_code')).toEqual(['claude-sonnet-test']);
  expect(compatibleModels(['gpt-test', 'claude-sonnet-test'], 'claude_code', true)).toEqual(['gpt-test', 'claude-sonnet-test']);
});
