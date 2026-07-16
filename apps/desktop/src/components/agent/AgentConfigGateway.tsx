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
  AlertTriangle,
  CheckCircle2,
} from 'lucide-react';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card } from '@/components/ui/card';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { AgentAccessDrawer, type KeyOption } from './AgentAccessDrawer';
import type { IndependentKeyDto } from '@/types/independentKey';
import type { AccountDto, TokenDto } from '@/types/token';

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

/** One agent's current access record (backend source of truth). */
export interface AgentAccess {
  agent: AgentId;
  method: AccessMethod;
  label: string;
  model: string | null;
  upstream_kind: string | null;
  impersonate: boolean | null;
}

export interface GatewayStatus {
  running: boolean;
  base_url: string | null;
  bindings: { client: AgentId }[];
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
      const all: KeyOption[] = [];
      for (const account of accounts.data ?? []) {
        try {
          const list = await invoke<TokenDto[]>('fetch_account_tokens', {
            accountId: account.id,
            forceRefresh: false,
          });
          for (const tk of list) {
            all.push({
              label: tk.name,
              masked: tk.masked_key,
              providerName: account.provider_name,
              providerId: account.provider_id,
              baseUrl: null,
              accountId: account.id,
              accountName: account.name,
              sourceGroup: 'relay',
              balance: tk.unlimited_quota ? '∞' : String(tk.remain_quota),
              source: {
                type: 'account_token',
                account_id: account.id,
                provider_id: account.provider_id,
                token_id: tk.id,
              },
            });
          }
        } catch {
          /* logged-out / expired account: skip */
        }
      }
      return all;
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
        source: { type: 'independent_key', key_id: k.id },
      }));
    return [...(tokens.data ?? []), ...indep];
  }, [independent.data, tokens.data]);

  return options;
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
  running,
  onEdit,
  onRemove,
  onStartGateway,
}: {
  agent: AgentDef;
  access?: AgentAccess;
  running: boolean;
  onEdit: () => void;
  onRemove: () => void;
  onStartGateway: () => void;
}) {
  const { t } = useTranslation();

  // ① Not configured
  if (!access) {
    return (
      <Card className="p-6 space-y-3">
        <div className="flex items-center justify-between gap-3">
          <h3 className="text-sm font-semibold">
            {t('agentConfig.notBoundTitle', '{{name}} 尚未接入任何 Key', { name: agent.name })}
          </h3>
          <Button size="sm" onClick={onEdit}>
            <Plus className="h-3.5 w-3.5" />
            {t('agentConfig.access', '接入 Key')}
          </Button>
        </div>
        <div className="h-px bg-border/50" />
        <p className="text-xs text-muted-foreground">
          {t('agentConfig.notBoundHint', '选择一个 Key，可直接写入 {{path}}，或经本地网关适配。', {
            path: agent.configPath,
          })}
        </p>
      </Card>
    );
  }

  const viaGateway = access.method === 'gateway';

  return (
    <Card className="p-6 space-y-4">
      <div className="flex items-center justify-between gap-3">
        <div className="flex items-center gap-2 min-w-0">
          <h3 className="text-sm font-semibold">
            {viaGateway
              ? t('agentConfig.boundGateway', '已接入（经网关）')
              : t('agentConfig.boundDirect', '已接入（直接写入）')}
          </h3>
        </div>
        <div className="flex gap-1 shrink-0">
          <Button variant="ghost" size="sm" onClick={onEdit}>
            <Pencil className="h-3.5 w-3.5" />
            {t('agentConfig.edit', '编辑')}
          </Button>
          <Button variant="ghost" size="sm" className="text-muted-foreground" onClick={onRemove}>
            <Unplug className="h-3.5 w-3.5" />
            {t('agentConfig.remove', '解除')}
          </Button>
        </div>
      </div>

      <div className="h-px bg-border/50" />

      <dl className="grid grid-cols-[56px_1fr] gap-y-2 gap-x-3 text-sm">
        <dt className="text-muted-foreground">{t('agentConfig.rowKey', 'Key')}</dt>
        <dd className="font-medium truncate">{access.label}</dd>
        <dt className="text-muted-foreground">{t('agentConfig.rowMethod', '方式')}</dt>
        <dd>
          {viaGateway
            ? t('agentConfig.methodGatewayShort', '经本地网关')
            : t('agentConfig.methodDirectShort', '直接写入 {{path}}', { path: agent.configPath })}
        </dd>
        {viaGateway && (
          <>
            <dt className="text-muted-foreground">{t('agentConfig.rowUpstream', '上游')}</dt>
            <dd>{kindLabel(access.upstream_kind, t)}</dd>
            <dt className="text-muted-foreground">{t('agentConfig.rowImpersonate', '伪装')}</dt>
            <dd>
              {access.impersonate
                ? t('agentConfig.impersonateOn', '已伪装为 {{name}}', { name: agent.name })
                : t('agentConfig.impersonateOff', '不伪装')}
            </dd>
          </>
        )}
      </dl>

      {viaGateway ? (
        running ? (
          <div className="flex items-center gap-2 rounded-lg bg-success-soft/40 border border-success-border px-3 py-2 text-xs text-success">
            <CheckCircle2 className="h-3.5 w-3.5 shrink-0" />
            {t('agentConfig.effective', '已生效，请求经网关转发')}
          </div>
        ) : (
          <div className="flex items-center gap-2 rounded-lg bg-warning-soft/40 border border-warning-border px-3 py-2 text-xs text-warning">
            <AlertTriangle className="h-3.5 w-3.5 shrink-0" />
            <span className="flex-1">{t('agentConfig.notRunningWarn', '网关未启动 —— 启动后接入才生效')}</span>
            <Button size="sm" variant="outline" className="h-6 px-2" onClick={onStartGateway}>
              {t('agentConfig.startNow', '立即启动')}
            </Button>
          </div>
        )
      ) : (
        <p className="text-xs text-muted-foreground">{t('agentConfig.directNote', '不经网关，无需启动网关。')}</p>
      )}
    </Card>
  );
}

/**
 * Header controls (gateway on/off + agent picker). Rendered into the page
 * header's `actions` slot, so it sits on the same bar as the page title. Shares
 * the same react-query caches as the body, so no extra fetches.
 */
export function AgentConfigHeaderControls({
  selectedAgent,
  onSelectAgent,
}: {
  selectedAgent: AgentId;
  onSelectAgent: (agent: AgentId) => void;
}) {
  const { data: status } = useGatewayStatus();
  const { data: accessList } = useAgentAccess();

  const accessByAgent = useMemo(() => {
    const map = new Map<AgentId, AgentAccess>();
    accessList?.forEach((a) => map.set(a.agent, a));
    return map;
  }, [accessList]);

  // The gateway server is only useful when some agent is bound via gateway.
  const anyGatewayAccess = (accessList ?? []).some((a) => a.method === 'gateway');

  return (
    <div className="flex items-center gap-3 min-w-0">
      <GatewayControl status={status} disabled={!anyGatewayAccess} />
      <div className="w-px h-6 bg-border shrink-0" />
      <Select value={selectedAgent} onValueChange={(v) => onSelectAgent(v as AgentId)}>
        <SelectTrigger className="h-9 w-44 shrink-0">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          {AGENTS.map((a) => (
            <SelectItem key={a.id} value={a.id}>
              <span className="flex items-center gap-2">
                {a.name}
                {accessByAgent.has(a.id) && <span className="h-1.5 w-1.5 rounded-full bg-primary inline-block" />}
              </span>
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </div>
  );
}

export function AgentConfigGateway({ selectedAgent }: { selectedAgent: AgentId }) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const { data: status } = useGatewayStatus();
  const { data: accessList } = useAgentAccess();
  const keyOptions = useKeyOptions();

  const [drawerOpen, setDrawerOpen] = useState(false);

  const agent = AGENTS.find((a) => a.id === selectedAgent)!;
  const accessByAgent = useMemo(() => {
    const map = new Map<AgentId, AgentAccess>();
    accessList?.forEach((a) => map.set(a.agent, a));
    return map;
  }, [accessList]);
  const access = accessByAgent.get(selectedAgent);

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
    <div className="space-y-5">
      <AccessStatusCard
        agent={agent}
        access={access}
        running={status?.running ?? false}
        onEdit={() => setDrawerOpen(true)}
        onRemove={remove}
        onStartGateway={startGateway}
      />

      <AgentAccessDrawer
        open={drawerOpen}
        onOpenChange={setDrawerOpen}
        agent={agent}
        keyOptions={keyOptions}
        existing={access}
        onDone={refresh}
      />
    </div>
  );
}
