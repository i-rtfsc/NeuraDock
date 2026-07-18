import { useState } from 'react';
import { useTranslation } from 'react-i18next';

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
      title={t('nav.agentConfig', 'Agent 配置')}
      actions={
        <AgentConfigHeaderControls />
      }
    >
      <PageContent maxWidth="lg" className="page-enter-stagger pb-8">
        <AgentConfigGateway selectedAgent={selectedAgent} onSelectAgent={setSelectedAgent} />
      </PageContent>
    </PageContainer>
  );
}
