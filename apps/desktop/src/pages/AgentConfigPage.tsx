import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Bot } from 'lucide-react';

import { PageContainer } from '@/components/layout/PageContainer';
import { PageContent } from '@/components/layout/PageContent';
import {
  AgentConfigGateway,
  AgentConfigHeaderControls,
  type AgentId,
} from '@/components/agent/AgentConfigGateway';

export function AgentConfigPage() {
  const { t } = useTranslation();
  const [selectedAgent, setSelectedAgent] = useState<AgentId>('codex');

  return (
    <PageContainer
      className="h-full bg-muted/10 w-full"
      title={
        <span className="flex items-center gap-2">
          <Bot className="h-5 w-5" />
          {t('nav.agentConfig', 'Agent 配置')}
        </span>
      }
      actions={
        <AgentConfigHeaderControls selectedAgent={selectedAgent} onSelectAgent={setSelectedAgent} />
      }
    >
      <PageContent maxWidth="lg" className="page-enter-stagger pb-8">
        <p className="text-sm text-muted-foreground mb-5">
          {t(
            'agentConfig.pageDescription',
            '给 Codex / Claude Code 等 Agent 配置中转站 Key。原生协议直接透传，只支持 chat 的中转站经本地网关适配；可伪装为标准客户端以通过中转站校验。'
          )}
        </p>
        <AgentConfigGateway selectedAgent={selectedAgent} />
      </PageContent>
    </PageContainer>
  );
}
