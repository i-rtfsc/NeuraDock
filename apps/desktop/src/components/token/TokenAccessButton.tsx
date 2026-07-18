import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Settings2, Code2, Terminal } from 'lucide-react';

import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { TokenAgentAccessDialog } from './TokenAgentAccessDialog';
import { AGENTS, type AgentId } from '@/components/agent/AgentConfigGateway';
import type { TokenDto, AccountDto } from '@/types/token';

const AGENT_ICONS: Record<AgentId, typeof Code2> = {
  codex: Code2,
  claude_code: Terminal,
};

/** "接入 Agent" button on an account-detail token card. Picks a target agent,
 *  then opens the shared access drawer preset to this token. */
export function TokenAccessButton({ token, account }: { token: TokenDto; account: AccountDto }) {
  const { t } = useTranslation();
  const [agentId, setAgentId] = useState<AgentId | null>(null);
  const [pickerOpen, setPickerOpen] = useState(false);
  const disabled = !token.is_active || !account.enabled;

  return (
    <>
      <Button
        variant="outline"
        size="sm"
        className="mt-auto h-10 w-full rounded-xl border-primary/20 text-xs font-semibold shadow-sm transition-all hover:border-primary hover:bg-primary/5 hover:text-primary"
        disabled={disabled}
        onClick={() => setPickerOpen(true)}
        title={
          disabled
            ? t('token.accessAgentDisabled', '账号或 Key 已禁用，无法接入 Agent')
            : undefined
        }
      >
        <Settings2 className="mr-2 h-4 w-4" />
        {t('token.accessAgent', '接入 Agent')}
      </Button>

      <Dialog open={pickerOpen} onOpenChange={setPickerOpen}>
        <DialogContent size="sm" className="gap-0 overflow-hidden p-0">
          <DialogHeader className="app-dialog-header">
            <DialogTitle>{t('token.pickAgentTitle', '选择要接入的 Agent')}</DialogTitle>
            <DialogDescription>
              {t('token.pickAgentDesc', '当前账号和 Key 将自动带入接入配置。')}
            </DialogDescription>
          </DialogHeader>
          <div className="grid gap-3 p-5 sm:grid-cols-2">
          {AGENTS.map((a) => {
            const Icon = AGENT_ICONS[a.id];
            return (
              <button
                key={a.id}
                type="button"
                onClick={() => {
                  setPickerOpen(false);
                  setAgentId(a.id);
                }}
                className="flex min-w-0 items-start gap-3 rounded-xl bg-muted/25 p-4 text-left transition-colors hover:bg-primary/5"
              >
                <span className="flex h-9 w-9 shrink-0 items-center justify-center rounded-lg bg-background text-primary shadow-sm">
                  <Icon className="h-4 w-4" />
                </span>
                <span className="min-w-0">
                  <span className="block text-sm font-semibold normal-case">{a.name}</span>
                  <span className="mt-1 block truncate font-mono text-[11px] text-muted-foreground">
                    {a.configPath}
                  </span>
                </span>
              </button>
            );
          })}
          </div>
        </DialogContent>
      </Dialog>

      {agentId && (
        <TokenAgentAccessDialog
          open={!!agentId}
          onOpenChange={(o) => !o && setAgentId(null)}
          token={token}
          account={account}
          agentId={agentId}
        />
      )}
    </>
  );
}
