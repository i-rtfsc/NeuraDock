import { render, screen, fireEvent, waitFor, within } from '@testing-library/react';
import { beforeEach, expect, it, vi } from 'vitest';
import { AgentProfilesSection, type AgentConfigurationState } from './AgentConfigGateway';

const remove = vi.hoisted(() => vi.fn());
vi.mock('@/lib/tauri', () => ({ commands: { deleteAgentProfile: remove } }));
vi.mock('sonner', () => ({ toast: { success: vi.fn(), error: vi.fn() } }));

beforeEach(() => { remove.mockReset().mockResolvedValue({ status: 'ok', data: null }); });

it('deletes the selected named profile through the more menu', async () => {
  const changed = vi.fn();
  const profile = (slug: string) => ({ kind: 'isolated' as const, slug, path: `~/.codex/profiles.d/${slug}.config.toml`,
    exists: true, parse_ok: true, configured: true, endpoint: 'https://relay.example/v1', model: 'deepseek-v4-flash',
    context1m: false, credential_present: true, managed: true, account_label: `${slug}@example.com`,
    provider_label: 'AnyRouter', gateway_enabled: false, method: 'direct' as const, models: ['deepseek-v4-flash'] });
  const configuration = { profiles: [profile('any-qq'), profile('any-gmail')] } as AgentConfigurationState;
  render(<AgentProfilesSection agent={{ id: 'codex', name: 'Codex', nativeKind: 'openai_responses', configPath: '~/.codex/config.toml' }}
    configuration={configuration} onAdd={vi.fn()} onEdit={vi.fn()} onChanged={changed} />);
  const row = screen.getByText('any-qq').closest('tr')!;
  fireEvent.keyDown(within(row).getByRole('button', { name: 'agentConfig.moreActions' }), { key: 'Enter' });
  fireEvent.click(await screen.findByRole('menuitem', { name: 'agentConfig.deleteProfile' }));
  expect(remove).not.toHaveBeenCalled();
  const dialog = await screen.findByRole('alertdialog');
  expect(within(dialog).getByText('~/.codex/profiles.d/any-qq.config.toml')).toBeInTheDocument();
  fireEvent.click(within(dialog).getByRole('button', { name: 'common.delete' }));
  await waitFor(() => expect(remove).toHaveBeenCalledWith('codex', 'any-qq'));
  expect(remove).toHaveBeenCalledTimes(1);
  await waitFor(() => expect(changed).toHaveBeenCalled());
});
