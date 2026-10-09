import { useMemo } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';

import { AgentAccessDrawer, type KeyOption } from '@/components/agent/AgentAccessDrawer';
import { AGENTS, type AgentAccess, type AgentId } from '@/components/agent/AgentConfigGateway';
import type { TokenDto, AccountDto } from '@/types/token';
import { formatQuotaUSD } from '@/lib/formatters';

interface Props {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  token: TokenDto;
  account: AccountDto;
  /** Which agent to configure (chosen by the caller in the account detail). */
  agentId: AgentId;
}

/** Account-detail entry point: opens the shared Agent access drawer preset to a
 *  specific relay account token, so the same UI/logic drives both the account
 *  detail and the Agent config page. */
export function TokenAgentAccessDialog({ open, onOpenChange, token, account, agentId }: Props) {
  const agent = AGENTS.find((a) => a.id === agentId)!;
  const queryClient = useQueryClient();
  const usable = token.is_active && account.enabled;

  const option: KeyOption = useMemo(
    () => ({
      label: token.name,
      masked: token.masked_key,
      providerName: account.provider_name,
      providerId: account.provider_id,
      baseUrl: null,
      accountId: account.id,
      accountName: account.name,
      sourceGroup: 'relay',
      balance: token.unlimited_quota ? '∞' : formatQuotaUSD(token.remain_quota),
      modelLimitsEnabled: token.model_limits_enabled,
      allowedModels: token.model_limits_allowed,
      deniedModels: token.model_limits_denied,
      source: {
        type: 'account_token',
        account_id: account.id,
        provider_id: account.provider_id,
        token_id: token.id,
      },
    }),
    [token, account]
  );

  const presetOptionId = `at:${account.id}:${token.id}`;

  // Current access for this agent (so the drawer can pre-fill method/kind).
  const { data: accessList } = useQuery<AgentAccess[]>({
    queryKey: ['agent-access'],
    queryFn: () => invoke<AgentAccess[]>('get_agent_access'),
  });
  const existing = accessList?.find((a) => a.agent === agentId && a.source?.type === 'account_token' && a.source.account_id === account.id && a.source.token_id === token.id);

  return (
    <AgentAccessDrawer
      open={open}
      onOpenChange={onOpenChange}
      agent={agent}
      keyOptions={usable ? [option] : []}
      existing={existing}
      presetOptionId={presetOptionId}
      onDone={() => {
        queryClient.invalidateQueries({ queryKey: ['agent-access'] });
        queryClient.invalidateQueries({ queryKey: ['gateway-status'] });
        queryClient.invalidateQueries({ queryKey: ['agent-configuration-states'] });
      }}
    />
  );
}
