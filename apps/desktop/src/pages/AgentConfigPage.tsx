import { useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';

import { PageContainer } from '@/components/layout/PageContainer';
import { PageContent } from '@/components/layout/PageContent';
import {
  AgentConfigGateway,
  AgentConfigHeaderControls,
  AgentGatewayHeaderControl,
  AGENTS,
  useAgentConfigurationStates,
  type AgentId,
} from '@/components/agent/AgentConfigGateway';

export function AgentConfigPage() {
  const { t } = useTranslation();
  const [selectedAgent, setSelectedAgent] = useState<AgentId>('codex');
  const initialSelectionResolved = useRef(false);
  const { data: configurationStates } = useAgentConfigurationStates();

  useEffect(() => {
    if (initialSelectionResolved.current || !configurationStates) return;
    initialSelectionResolved.current = true;
    const configuredAgents = AGENTS.filter((agent) =>
      configurationStates.some(
        (configuration) =>
          configuration.agent === agent.id && configuration.state !== 'unconfigured'
      )
    );
    if (configuredAgents.length === 1) {
      setSelectedAgent(configuredAgents[0].id);
    }
  }, [configurationStates]);

  return (
    <PageContainer
      className="h-full bg-muted/10 w-full"
      title={
        <div className="flex min-w-0 items-center gap-2">
          <h1 className="shrink-0 truncate">{t('nav.agentConfig', 'Agent 配置')}</h1>
          <AgentGatewayHeaderControl />
        </div>
      }
      actions={
        <AgentConfigHeaderControls
          selectedAgent={selectedAgent}
          onSelectAgent={setSelectedAgent}
        />
      }
    >
      <PageContent maxWidth="lg" className="page-enter-stagger pb-8">
        <AgentConfigGateway selectedAgent={selectedAgent} />
      </PageContent>
    </PageContainer>
  );
}
