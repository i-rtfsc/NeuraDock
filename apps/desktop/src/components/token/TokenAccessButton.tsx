import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Settings2, Code2, Terminal } from 'lucide-react';

import { Button } from '@/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
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

  return (
    <>
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <Button
            variant="outline"
            size="sm"
            className="w-full mt-auto text-xs font-black rounded-xl h-10 border-primary/20 hover:border-primary hover:bg-primary/5 hover:text-primary hover:scale-[1.02] shadow-sm active:scale-95 transition-all uppercase tracking-tight"
            disabled={!token.is_active}
          >
            <Settings2 className="mr-2 h-4 w-4" />
            {t('token.accessAgent', '接入 Agent')}
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="center" className="w-44">
          {AGENTS.map((a) => {
            const Icon = AGENT_ICONS[a.id];
            return (
              <DropdownMenuItem key={a.id} onClick={() => setAgentId(a.id)}>
                <Icon className="mr-2 h-4 w-4" />
                {a.name}
              </DropdownMenuItem>
            );
          })}
        </DropdownMenuContent>
      </DropdownMenu>

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
