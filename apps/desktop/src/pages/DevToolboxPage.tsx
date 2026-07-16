import { useTranslation } from 'react-i18next';
import { Hammer } from 'lucide-react';

import { PageContainer } from '@/components/layout/PageContainer';
import { PageContent } from '@/components/layout/PageContent';
import { EmptyState } from '@/components/ui/empty-state';

export function DevToolboxPage() {
  const { t } = useTranslation();

  return (
    <PageContainer title={t('nav.devToolbox', '开发工具箱')}>
      <PageContent maxWidth="md" centered className="page-enter-stagger">
        <EmptyState
          icon={<Hammer className="h-12 w-12" />}
          title={t('devToolbox.comingSoonTitle', '建设中')}
          description={t(
            'devToolbox.comingSoonDescription',
            '从左侧「开发工具箱」分组进入各工具（API Key、Codex 等）。'
          )}
        />
      </PageContent>
    </PageContainer>
  );
}
