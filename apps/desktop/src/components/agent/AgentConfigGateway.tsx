import { useMemo, useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';
import {
  Copy,
  Check,
  Power,
  Plus,
  Pencil,
  Unplug,
  Code2,
  Terminal,
} from 'lucide-react';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { AgentAccessDrawer, type KeyOption } from './AgentAccessDrawer';
import type { IndependentKeyDto } from '@/types/independentKey';
import type { AccountDto, TokenDto } from '@/types/token';
import { formatQuotaUSD } from '@/lib/formatters';
import { cn } from '@/lib/utils';

/** Agents the gateway can serve. Adding one is a single entry here. */
export type AgentId = 'codex' | 'claude_code';

export interface AgentDef {
  id: AgentId;
  name: string;
  nativeKind: 'openai_responses' | 'anthropic';
  configPath: string;
}

export const AGENTS: AgentDef[] = [
  { id: 'codex', name: 'Codex', nativeKind: 'openai_responses', configPath: '~/.codex/config.toml' },
  {
    id: 'claude_code',
    name: 'Claude Code',
    nativeKind: 'anthropic',
    configPath: '~/.claude/settings.json',
  },
];

type AccessMethod = 'direct' | 'gateway';

export type AgentAccessSource =
  | {
      type: 'account_token';
      account_id: string;
      provider_id: string;
      token_id: number;
      base_url?: string | null;
    }
  | { type: 'independent_key'; key_id: number };

/** One agent's current access record (backend source of truth). */
export interface AgentAccess {
  agent: AgentId;
  method: AccessMethod;
  label: string;
  model: string | null;
  upstream_kind: string | null;
  impersonate: boolean | null;
  source?: AgentAccessSource | null;
  context1m?: boolean | null;
}

export interface GatewayStatus {
  running: boolean;
  base_url: string | null;
  bindings: { client: AgentId; model?: string | null; context1m?: boolean }[];
}

export const GATEWAY_STATUS_QUERY_KEY = ['gateway-status'];
export const AGENT_ACCESS_QUERY_KEY = ['agent-access'];

export function useGatewayStatus() {
  return useQuery<GatewayStatus>({
    queryKey: GATEWAY_STATUS_QUERY_KEY,
    queryFn: () => invoke<GatewayStatus>('get_gateway_status'),
    refetchInterval: 5000,
  });
}

function useAgentAccess() {
  return useQuery<AgentAccess[]>({
    queryKey: AGENT_ACCESS_QUERY_KEY,
    queryFn: () => invoke<AgentAccess[]>('get_agent_access'),
  });
}

const AGENT_ICONS: Record<AgentId, typeof Code2> = {
  codex: Code2,
  claude_code: Terminal,
};

export function AgentSummarySelector({
  selectedAgent,
  onSelectAgent,
  accessList,
  keyOptions,
}: {
  selectedAgent: AgentId;
  onSelectAgent: (agent: AgentId) => void;
  accessList?: AgentAccess[];
  keyOptions?: KeyOption[];
}) {
  const { t } = useTranslation();
  const accessByAgent = useMemo(() => {
    const map = new Map<AgentId, AgentAccess>();
    accessList?.forEach((access) => map.set(access.agent, access));
    return map;
  }, [accessList]);

  return (
    <div className="flex gap-2 overflow-x-auto pb-1 lg:flex-col lg:overflow-visible lg:pb-0">
      {AGENTS.map((agent) => {
        const Icon = AGENT_ICONS[agent.id];
        const access = accessByAgent.get(agent.id);
        const sourceOption = access?.source
          ? keyOptions?.find((option) => {
              if (access.source?.type !== option.source.type) return false;
              if (access.source.type === 'account_token' && option.source.type === 'account_token') {
                return access.source.account_id === option.source.account_id && access.source.token_id === option.source.token_id;
              }
              return access.source.type === 'independent_key' && option.source.type === 'independent_key' && access.source.key_id === option.source.key_id;
            })
          : undefined;
        const active = agent.id === selectedAgent;
        return (
          <button
            key={agent.id}
            type="button"
            onClick={() => onSelectAgent(agent.id)}
            className={cn(
              'min-w-[184px] rounded-xl border px-3 py-3 text-left transition-colors lg:min-w-0',
              active
                ? 'border-primary/55 bg-primary/5 ring-1 ring-primary/15'
                : 'border-border/60 bg-background hover:border-primary/25 hover:bg-muted/25'
            )}
          >
            <span className="flex items-center gap-2">
              <span className={cn('rounded-lg p-1.5', active ? 'bg-primary/10 text-primary' : 'bg-muted text-muted-foreground')}>
                <Icon className="h-4 w-4" />
              </span>
              <span className="min-w-0 flex-1 truncate text-sm font-semibold">{agent.name}</span>
              {access ? (
                <Badge variant="outline" className="h-5 shrink-0 px-1.5 text-[9px] font-medium">
                  {access.method === 'gateway'
                    ? t('agentConfig.gatewayBadge', '网关')
                    : t('agentConfig.directBadge', '直连')}
                </Badge>
              ) : (
                <span className="h-2 w-2 shrink-0 rounded-full bg-muted-foreground/25" />
              )}
            </span>
            <span className="mt-2 block truncate text-xs text-muted-foreground">
              {access
                ? sourceOption?.sourceGroup === 'independent'
                  ? t('agentConfig.groupIndependent', '独立 API Key')
                  : sourceOption?.providerName ?? access.label.split(' · ')[0]
                : t('agentConfig.notConfiguredShort', '尚未配置')}
            </span>
            <span className="mt-1 flex min-w-0 items-center gap-1.5 text-[11px] text-muted-foreground">
              <span className="truncate font-mono">{access?.model ?? agent.configPath}</span>
              {access?.context1m && (
                <Badge variant="outline" className="h-4 shrink-0 px-1 text-[9px]">
                  1M
                </Badge>
              )}
            </span>
          </button>
        );
      })}
    </div>
  );
}

/** Every selectable key (relay account tokens + independent keys). */
function useKeyOptions() {
  const independent = useQuery<IndependentKeyDto[]>({
    queryKey: ['independent-keys'],
    queryFn: () => invoke<IndependentKeyDto[]>('get_all_independent_keys'),
  });
  const accounts = useQuery<AccountDto[]>({
    queryKey: ['accounts', 'gateway-picker'],
    queryFn: () => invoke<AccountDto[]>('get_all_accounts', { enabledOnly: false }),
  });
  const tokens = useQuery<KeyOption[]>({
    queryKey: ['gateway-account-tokens', accounts.data?.map((a) => a.id)],
    enabled: !!accounts.data && accounts.data.length > 0,
    queryFn: async () => {
      const enabledAccounts = (accounts.data ?? []).filter((account) => account.enabled);
      const groups = await Promise.all(
        enabledAccounts.map(async (account): Promise<KeyOption[]> => {
          try {
            const list = await invoke<TokenDto[]>('fetch_account_tokens', {
              accountId: account.id,
              forceRefresh: false,
            });
            const options: KeyOption[] = [];
            for (const tk of list) {
              // Disabled tokens can't be configured; leave them out of the picker.
              if (!tk.is_active) continue;
              options.push({
                label: tk.name,
                masked: tk.masked_key,
                providerName: account.provider_name,
                providerId: account.provider_id,
                baseUrl: null,
                accountId: account.id,
                accountName: account.name,
                sourceGroup: 'relay',
                balance: tk.unlimited_quota ? '∞' : formatQuotaUSD(tk.remain_quota),
                modelLimitsEnabled: tk.model_limits_enabled,
                allowedModels: tk.model_limits_allowed,
                deniedModels: tk.model_limits_denied,
                source: {
                  type: 'account_token',
                  account_id: account.id,
                  provider_id: account.provider_id,
                  token_id: tk.id,
                },
              });
            }
            return options;
          } catch {
            return [];
          }
        })
      );
      return groups.flat();
    },
  });

  const options: KeyOption[] = useMemo(() => {
    const indep: KeyOption[] = (independent.data ?? [])
      .filter((k) => k.is_active)
      .map((k) => ({
        label: k.name,
        masked: k.masked_key,
        providerName: k.provider_type_display,
        providerId: null,
        baseUrl: k.base_url,
        accountId: null,
        accountName: null,
        sourceGroup: 'independent',
        balance: null,
        modelLimitsEnabled: false,
        allowedModels: [],
        deniedModels: [],
        source: { type: 'independent_key', key_id: k.id },
      }));
    return [...(tokens.data ?? []), ...indep];
  }, [independent.data, tokens.data]);

  return {
    options,
    isLoading: independent.isLoading || accounts.isLoading || tokens.isLoading,
  };
}

function GatewayControl({ status, disabled }: { status?: GatewayStatus; disabled: boolean }) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [busy, setBusy] = useState(false);
  const [copied, setCopied] = useState(false);
  const running = status?.running ?? false;

  const toggle = async () => {
    setBusy(true);
    try {
      if (running) {
        await invoke('stop_gateway_server');
        toast.success(t('agentConfig.gatewayStopped', '网关已停止，工具配置已还原'));
      } else {
        await invoke('start_gateway_server');
        toast.success(t('agentConfig.gatewayStarted', '网关已启动'));
      }
      queryClient.invalidateQueries({ queryKey: GATEWAY_STATUS_QUERY_KEY });
    } catch (e) {
      toast.error(String(e));
    } finally {
      setBusy(false);
    }
  };

  const copy = async () => {
    if (!status?.base_url) return;
    await navigator.clipboard.writeText(status.base_url);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  };

  return (
    <div className="flex items-center gap-2.5">
      <span
        className={
          'inline-block h-2.5 w-2.5 rounded-full shrink-0 ' +
          (running ? 'bg-success shadow-[0_0_6px_var(--success)]' : 'bg-muted-foreground/40')
        }
      />
      <span className="text-sm font-medium whitespace-nowrap">{t('agentConfig.gatewayLabel', '网关')}</span>
      <Badge variant="outline" className={running ? 'text-success border-success/40' : 'text-muted-foreground'}>
        {running ? t('agentConfig.running', '运行中') : t('agentConfig.stopped', '已停止')}
      </Badge>
      {running && status?.base_url && (
        <button onClick={copy} className="flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground">
          <code className="font-mono">{status.base_url}</code>
          {copied ? <Check className="h-3 w-3" /> : <Copy className="h-3 w-3" />}
        </button>
      )}
      <Button
        onClick={toggle}
        disabled={busy || (!running && disabled)}
        variant={running ? 'outline' : 'default'}
        size="sm"
      >
        <Power className="h-3.5 w-3.5" />
        {running ? t('agentConfig.stop', '停止') : t('agentConfig.start', '启动')}
      </Button>
    </div>
  );
}

function kindLabel(kind: string | null, t: ReturnType<typeof useTranslation>['t']): string {
  switch (kind) {
    case 'openai_responses':
      return t('agentConfig.kindResponses', 'OpenAI Responses（原生透传）');
    case 'anthropic':
      return t('agentConfig.kindAnthropic', 'Anthropic（原生透传）');
    case 'openai_chat':
      return t('agentConfig.kindChat', 'OpenAI Chat（网关转换）');
    default:
      return '—';
  }
}

function AccessStatusCard({
  agent,
  access,
  sourceOption,
  running,
  onEdit,
  onRemove,
  onStartGateway,
}: {
  agent: AgentDef;
  access?: AgentAccess;
  sourceOption?: KeyOption;
  running: boolean;
  onEdit: () => void;
  onRemove: () => void;
  onStartGateway: () => void;
}) {
  const { t } = useTranslation();

  // ① Not configured
  if (!access) {
    return (
      <div className="flex min-h-80 flex-col items-center justify-center px-6 py-12 text-center">
        <div className="flex h-12 w-12 items-center justify-center rounded-2xl bg-muted text-muted-foreground">
          <Plus className="h-5 w-5" />
        </div>
        <h3 className="mt-4 text-base font-semibold">
          {t('agentConfig.notBoundTitle', '{{name}} 尚未接入任何 Key', { name: agent.name })}
        </h3>
        <p className="mt-2 max-w-md text-sm leading-6 text-muted-foreground">
          {t('agentConfig.notBoundHint', '选择一个 Key，可直接写入 {{path}}，或经本地网关适配。', {
            path: agent.configPath,
          })}
        </p>
        <Button className="mt-5" onClick={onEdit}>
          <Plus className="h-4 w-4" />
          {t('agentConfig.access', '接入 Key')}
        </Button>
      </div>
    );
  }

  const viaGateway = access.method === 'gateway';
  const sourceTitle = sourceOption
    ? sourceOption.sourceGroup === 'relay'
      ? sourceOption.providerName
      : t('agentConfig.groupIndependent', '独立 API Key')
    : access.label;
  const sourceDetail = sourceOption
    ? sourceOption.sourceGroup === 'relay'
      ? [sourceOption.accountName, sourceOption.label].filter(Boolean).join(' / ')
      : sourceOption.label
    : access.label;

  return (
    <div className="space-y-4">
      <section className="card-vivid cursor-default space-y-3 rounded-xl p-4 sm:p-5">
        <div className="flex items-center justify-between gap-3">
          <h3 className="text-sm font-semibold">{t('agentConfig.sourceSectionTitle', '接入来源')}</h3>
          <Button variant="ghost" size="sm" onClick={onEdit}>
            <Pencil className="h-3.5 w-3.5" />
            {t('agentConfig.edit', '编辑')}
          </Button>
        </div>
        <div className="rounded-lg bg-muted/25 px-3 py-3">
          <p className="truncate text-sm font-semibold">{sourceTitle}</p>
          <p className="mt-1 truncate text-sm text-muted-foreground">{sourceDetail}</p>
        </div>
      </section>

      <section className="card-vivid cursor-default space-y-3 rounded-xl p-4 sm:p-5">
        <h3 className="text-sm font-semibold">{t('agentConfig.runtimeSectionTitle', '运行配置')}</h3>
        <dl className="grid gap-2 text-sm">
          <DetailRow label={t('agentConfig.rowModel', '模型')}>
            <span className="flex min-w-0 items-center gap-2">
              <span className="truncate font-mono text-xs">{access.model ?? '—'}</span>
              {access.context1m && (
                <Badge variant="outline" className="h-5 shrink-0 px-1.5 text-[10px]">1M</Badge>
              )}
            </span>
          </DetailRow>
          <DetailRow label={t('agentConfig.rowMethod', '方式')}>
            {viaGateway
              ? t('agentConfig.methodGatewayShort', '经本地网关')
              : t('agentConfig.methodDirectShort', '直接写入 {{path}}', { path: agent.configPath })}
          </DetailRow>
          {viaGateway && (
            <>
              <DetailRow label={t('agentConfig.rowUpstream', '上游')}>
                {kindLabel(access.upstream_kind, t)}
              </DetailRow>
              <DetailRow label={t('agentConfig.rowCompatibility', '客户端兼容')}>
                {access.impersonate
                  ? t('agentConfig.impersonateOn', '已伪装为 {{name}}', { name: agent.name })
                  : t('agentConfig.impersonateOff', '不伪装')}
              </DetailRow>
            </>
          )}
          <DetailRow label={t('agentConfig.gatewayLabel', '网关')}>
            <span className="flex items-center gap-2">
              <span className={cn('h-2 w-2 rounded-full', viaGateway && running ? 'bg-success' : 'bg-muted-foreground/30')} />
              {viaGateway
                ? running
                  ? t('agentConfig.running', '运行中')
                  : t('agentConfig.stopped', '已停止')
                : t('agentConfig.notApplicable', '不适用')}
              {viaGateway && !running && (
                <Button size="sm" variant="ghost" className="ml-1 h-7 px-2" onClick={onStartGateway}>
                  {t('agentConfig.startNow', '立即启动')}
                </Button>
              )}
            </span>
          </DetailRow>
        </dl>
      </section>

      <div className="flex justify-end px-1 pt-1">
        <Button variant="destructive" size="sm" onClick={onRemove}>
          <Unplug className="h-3.5 w-3.5" />
          {t('agentConfig.remove', '解除接入')}
        </Button>
      </div>
    </div>
  );
}

function DetailRow({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="grid min-h-11 grid-cols-[88px_minmax(0,1fr)] items-center gap-4 rounded-lg bg-muted/20 px-3 py-2.5">
      <dt className="text-muted-foreground">{label}</dt>
      <dd className="min-w-0">{children}</dd>
    </div>
  );
}

/**
 * Header controls (gateway on/off + agent picker). Rendered into the page
 * header's `actions` slot, so it sits on the same bar as the page title. Shares
 * the same react-query caches as the body, so no extra fetches.
 */
export function AgentConfigHeaderControls() {
  const { data: status } = useGatewayStatus();
  const { data: accessList } = useAgentAccess();

  // The gateway server is only useful when some agent is bound via gateway.
  const anyGatewayAccess = (accessList ?? []).some((a) => a.method === 'gateway');

  return (
    <div className="flex min-w-0 items-center gap-3">
      <GatewayControl status={status} disabled={!anyGatewayAccess} />
    </div>
  );
}

export function AgentConfigGateway({
  selectedAgent,
  onSelectAgent,
}: {
  selectedAgent: AgentId;
  onSelectAgent: (agent: AgentId) => void;
}) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const { data: status } = useGatewayStatus();
  const { data: accessList } = useAgentAccess();
  const { options: keyOptions, isLoading: keyOptionsLoading } = useKeyOptions();

  const [drawerOpen, setDrawerOpen] = useState(false);

  const agent = AGENTS.find((a) => a.id === selectedAgent)!;
  const accessByAgent = useMemo(() => {
    const map = new Map<AgentId, AgentAccess>();
    accessList?.forEach((a) => map.set(a.agent, a));
    return map;
  }, [accessList]);
  const access = accessByAgent.get(selectedAgent);
  const sourceOption = useMemo(() => {
    if (!access?.source) return undefined;
    return keyOptions.find((option) => {
      if (access.source?.type !== option.source.type) return false;
      if (access.source.type === 'account_token' && option.source.type === 'account_token') {
        return (
          access.source.account_id === option.source.account_id &&
          access.source.token_id === option.source.token_id
        );
      }
      return (
        access.source.type === 'independent_key' &&
        option.source.type === 'independent_key' &&
        access.source.key_id === option.source.key_id
      );
    });
  }, [access, keyOptions]);

  const refresh = () => {
    queryClient.invalidateQueries({ queryKey: GATEWAY_STATUS_QUERY_KEY });
    queryClient.invalidateQueries({ queryKey: AGENT_ACCESS_QUERY_KEY });
  };

  const remove = async () => {
    try {
      if (access?.method === 'gateway') {
        await invoke('clear_gateway_binding', { client: selectedAgent });
      } else {
        const command = selectedAgent === 'claude_code' ? 'clear_claude_global' : 'clear_codex_global';
        await invoke<string>(command);
        await invoke('clear_agent_access', { agent: selectedAgent });
      }
      toast.success(t('agentConfig.unbound', '已解除接入并还原配置'));
      refresh();
    } catch (e) {
      toast.error(String(e));
    }
  };

  const startGateway = async () => {
    try {
      await invoke('start_gateway_server');
      toast.success(t('agentConfig.gatewayStarted', '网关已启动'));
      refresh();
    } catch (e) {
      toast.error(String(e));
    }
  };

  return (
    <div className="space-y-3">
      <div className="flex items-center gap-2 px-1">
        <Badge variant="soft-primary" className="h-6 rounded-md px-2 text-[10px] font-bold tracking-wider">
          {t('agentConfig.agentListTitle', 'Agents')}
        </Badge>
        <span className="text-xs text-muted-foreground">
          {t('agentConfig.agentListHint', '选择 Agent 查看当前接入状态')}
        </span>
      </div>

      <div className="grid gap-4 lg:grid-cols-[220px_minmax(0,1fr)]">
        <aside className="card-vivid cursor-default rounded-xl p-2">
          <div className="mb-3 px-1">
            <p className="text-xs font-medium text-muted-foreground">
              {t('agentConfig.agentListTitle', 'Agents')}
            </p>
          </div>
          <AgentSummarySelector
            selectedAgent={selectedAgent}
            onSelectAgent={onSelectAgent}
            accessList={accessList}
            keyOptions={keyOptions}
          />
        </aside>

        <section className="min-w-0 space-y-4">
          <header className="flex flex-wrap items-center justify-between gap-3 px-1 py-1">
            <div className="min-w-0">
              <h2 className="truncate text-lg font-semibold">{agent.name}</h2>
              <p className="mt-0.5 truncate font-mono text-xs text-muted-foreground">
                {agent.configPath}
              </p>
            </div>
            <Badge variant={access ? 'secondary' : 'outline'}>
              {access
                ? access.method === 'gateway'
                  ? t('agentConfig.boundGateway', '已接入（经网关）')
                  : t('agentConfig.boundDirect', '已接入（直接写入）')
                : t('agentConfig.notConfiguredShort', '尚未配置')}
            </Badge>
          </header>

          <AccessStatusCard
            agent={agent}
            access={access}
            sourceOption={sourceOption}
            running={status?.running ?? false}
            onEdit={() => setDrawerOpen(true)}
            onRemove={remove}
            onStartGateway={startGateway}
          />
        </section>
      </div>

      <AgentAccessDrawer
        open={drawerOpen}
        onOpenChange={setDrawerOpen}
        agent={agent}
        keyOptions={keyOptions}
        loading={keyOptionsLoading}
        existing={access}
        onDone={refresh}
      />
    </div>
  );
}
