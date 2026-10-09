import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { AgentAccessDrawer, type KeyOption } from './AgentAccessDrawer';
import type { AgentAccess } from './AgentConfigGateway';

const mocks = vi.hoisted(() => ({
  save: vi.fn(), claudeSave: vi.fn(), refresh: vi.fn(), states: vi.fn(), nodes: vi.fn(), models: vi.fn(),
}));
vi.mock('@/lib/tauri', () => ({ commands: {
  configureCodexProfile: mocks.save,
  configureClaudeProfile: mocks.claudeSave,
  fetchProviderModels: mocks.refresh,
  getAgentConfigurationStates: mocks.states,
  getProviderNodes: mocks.nodes,
  getCachedProviderModels: mocks.models,
} }));
vi.mock('sonner', () => ({ toast: { success: vi.fn(), error: vi.fn() } }));

const option: KeyOption = {
  label: 'Key A', masked: 'sk-***', providerName: 'AnyRouter', providerId: 'provider-1',
  baseUrl: null, accountId: 'account-1', accountName: 'private@qq.com', sourceGroup: 'relay',
  balance: null, modelLimitsEnabled: false, allowedModels: [], deniedModels: [],
  source: { type: 'account_token', account_id: 'account-1', provider_id: 'provider-1', token_id: 1 },
};

function openDrawer(existing?: AgentAccess, agentId: 'codex' | 'claude_code' = 'codex') {
  const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  render(<QueryClientProvider client={queryClient}>
    <AgentAccessDrawer open onOpenChange={vi.fn()}
      agent={{ id: agentId, name: agentId === 'codex' ? 'Codex' : 'Claude Code', nativeKind: agentId === 'codex' ? 'openai_responses' : 'anthropic', configPath: agentId === 'codex' ? '~/.codex/config.toml' : '~/.claude/settings.json' }}
      keyOptions={[option]} presetOptionId="at:account-1:1" existing={existing} onDone={vi.fn()} />
  </QueryClientProvider>);
}

beforeEach(() => {
  vi.clearAllMocks();
  mocks.save.mockResolvedValue({ status: 'ok', data: 'saved' });
  mocks.claudeSave.mockResolvedValue({ status: 'ok', data: 'saved' });
  mocks.refresh.mockResolvedValue({ status: 'ok', data: ['deepseek-v4-pro'] });
  mocks.nodes.mockResolvedValue({ status: 'ok', data: [{ id: 'node-1', name: 'Node', base_url: 'https://relay.example/v1' }] });
  mocks.models.mockResolvedValue({ status: 'ok', data: ['deepseek-v4-flash', 'deepseek-v4-pro'] });
  mocks.states.mockResolvedValue({ status: 'ok', data: [{ agent: 'codex', profiles: [{ slug: 'any-qq' }] }] });
});

describe('named Codex profiles', () => {
  it('excludes Claude models and aliases from the Codex catalog', async () => {
    mocks.models.mockResolvedValue({ status: 'ok', data: ['gpt-test', 'deepseek-v4-flash', 'claude-sonnet-test', 'anthropic/claude-opus-test', 'sonnet'] });
    openDrawer();
    await screen.findByRole('checkbox', { name: 'gpt-test' });
    expect(screen.getByRole('checkbox', { name: 'deepseek-v4-flash' })).toBeInTheDocument();
    expect(screen.queryByRole('checkbox', { name: 'claude-sonnet-test' })).not.toBeInTheDocument();
    expect(screen.queryByRole('checkbox', { name: 'anthropic/claude-opus-test' })).not.toBeInTheDocument();
    expect(screen.queryByRole('checkbox', { name: 'sonnet' })).not.toBeInTheDocument();
  });

  it('saves multiple models under an unused relay/account name', async () => {
    openDrawer();
    fireEvent.click(await screen.findByRole('checkbox', { name: 'deepseek-v4-flash' }));
    fireEvent.click(screen.getByRole('checkbox', { name: 'deepseek-v4-pro' }));
    expect(screen.queryByPlaceholderText('agentConfig.modelNamePlaceholder')).not.toBeInTheDocument();
    const save = screen.getByRole('button', { name: 'agentConfig.confirmAccess' });
    await waitFor(() => expect(save).toBeEnabled());
    fireEvent.click(save);
    await waitFor(() => expect(mocks.save).toHaveBeenCalledWith(expect.objectContaining({
      profileSlug: 'any-qq-2', models: ['deepseek-v4-flash', 'deepseek-v4-pro'],
      model: 'deepseek-v4-flash', context1m: false, overwrite: false,
    })));
  });

  it('uses the refreshed server list and removes models that disappeared', async () => {
    openDrawer();
    fireEvent.click(await screen.findByRole('checkbox', { name: 'deepseek-v4-flash' }));
    fireEvent.click(screen.getByRole('button', { name: 'agentConfig.refreshModels' }));
    await waitFor(() => expect(screen.queryByRole('checkbox', { name: 'deepseek-v4-flash' })).not.toBeInTheDocument());
    expect(screen.getByRole('button', { name: 'agentConfig.confirmAccess' })).toBeDisabled();
    fireEvent.click(screen.getByRole('checkbox', { name: 'deepseek-v4-pro' }));
    fireEvent.click(screen.getByRole('button', { name: 'agentConfig.confirmAccess' }));
    await waitFor(() => expect(mocks.save).toHaveBeenCalledWith(expect.objectContaining({ models: ['deepseek-v4-pro'], model: 'deepseek-v4-pro' })));
  });

  it('creates a Claude account profile without the settings prefix', async () => {
    mocks.models.mockResolvedValue({ status: 'ok', data: ['claude-sonnet-test'] });
    mocks.states.mockResolvedValue({ status: 'ok', data: [{ agent: 'claude_code', profiles: [{ slug: 'any-qq' }] }] });
    openDrawer(undefined, 'claude_code');
    await screen.findByText('claude --settings ~/.claude/profiles.d/any-qq-2.json');
    const save = screen.getByRole('button', { name: 'agentConfig.confirmAccess' });
    await waitFor(() => expect(save).toBeEnabled());
    fireEvent.click(save);
    await waitFor(() => expect(mocks.claudeSave).toHaveBeenCalledWith(expect.objectContaining({
      client: 'claude_code', profileSlug: 'any-qq-2', model: 'claude-sonnet-test', overwrite: false,
    })));
  });

  it('requires a new node when an existing profile points at retired rainapp', async () => {
    openDrawer({ agent: 'codex', method: 'direct', label: 'AnyRouter · Key A',
      account_label: option.accountName, model: 'deepseek-v4-flash', upstream_kind: null, impersonate: null,
      source: { type: 'account_token', account_id: 'account-1', provider_id: 'provider-1', token_id: 1,
        base_url: 'https://pmpjfbhq.cn-nb1.rainapp.top' },
      isolated_slug: 'any-qq', models: ['deepseek-v4-flash'],
    });
    await screen.findByText('agentConfig.nodeUnavailable');
    expect(screen.getByRole('button', { name: 'agentConfig.confirmAccess' })).toBeDisabled();
    expect(mocks.save).not.toHaveBeenCalled();
  });

  it('restores selected models and preserves the name while editing', async () => {
    openDrawer({ agent: 'codex', method: 'direct', label: 'AnyRouter · Key A',
      account_label: option.accountName, model: 'deepseek-v4-flash', upstream_kind: null, impersonate: null,
      source: { ...option.source, base_url: 'https://relay.example/v1' } as AgentAccess['source'],
      isolated_slug: 'any-qq', models: ['deepseek-v4-flash', 'deepseek-v4-pro'],
    });
    const flash = await screen.findByRole('checkbox', { name: 'deepseek-v4-flash' });
    expect(flash).toBeChecked();
    fireEvent.click(flash);
    const save = screen.getByRole('button', { name: 'agentConfig.confirmAccess' });
    await waitFor(() => expect(save).toBeEnabled());
    fireEvent.click(save);
    await waitFor(() => expect(mocks.save).toHaveBeenCalledWith(expect.objectContaining({
      profileSlug: 'any-qq', models: ['deepseek-v4-pro'], model: 'deepseek-v4-pro', overwrite: true,
    })));
  });
});
