import { useTranslation } from 'react-i18next';
import { Settings2, Wand2 } from 'lucide-react';

import { PageContainer } from '@/components/layout/PageContainer';
import { PageContent } from '@/components/layout/PageContent';
import { Tabs, TabsList, TabsTrigger, TabsContent } from '@/components/ui/tabs';
import { usePersistedState } from '@/hooks/usePersistedState';
import { AiToolConfigurator } from '@/components/devtoolbox/AiToolConfigurator';
import { EncodingTools } from '@/components/devtoolbox/EncodingTools';

type DevToolboxTab = 'ai-config' | 'encoding';

export function DevToolboxPage() {
  const { t } = useTranslation();
  const [activeTab, setActiveTab] = usePersistedState<DevToolboxTab>(
    'page-tab:dev-toolbox',
    'ai-config'
  );

  return (
    <PageContainer title={t('nav.devToolbox', '开发工具箱')}>
      <PageContent maxWidth="lg" className="page-enter-stagger">
        <Tabs value={activeTab} onValueChange={(v) => setActiveTab(v as DevToolboxTab)}>
          <TabsList className="mb-6">
            <TabsTrigger value="ai-config" className="gap-2">
              <Settings2 className="h-4 w-4" />
              {t('devToolbox.tabs.aiConfig', 'AI 工具配置')}
            </TabsTrigger>
            <TabsTrigger value="encoding" className="gap-2">
              <Wand2 className="h-4 w-4" />
              {t('devToolbox.tabs.encoding', '编解码工具')}
            </TabsTrigger>
          </TabsList>

          <TabsContent value="ai-config">
            <AiToolConfigurator />
          </TabsContent>
          <TabsContent value="encoding">
            <EncodingTools />
          </TabsContent>
        </Tabs>
      </PageContent>
    </PageContainer>
  );
}
