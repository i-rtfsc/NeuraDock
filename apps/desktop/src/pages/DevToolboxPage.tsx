import { Hammer } from 'lucide-react';
import { useTranslation } from 'react-i18next';

import { PageContainer } from '@/components/layout/PageContainer';
import { PageContent } from '@/components/layout/PageContent';
import { EmptyState } from '@/components/ui/empty-state';

export function DevToolboxPage() {
  const { t } = useTranslation();

  return (
    <PageContainer title={t('nav.devToolbox', '开发工具箱')}>
      <PageContent maxWidth="md" centered>
        <EmptyState
          icon={<Hammer className="h-12 w-12" />}
          title={t('devToolbox.comingSoonTitle', '建设中')}
          description={t(
            'devToolbox.comingSoonDescription',
            '开发工具箱功能还在规划中，后续会逐步补充。'
          )}
        />
      </PageContent>
    </PageContainer>
  );
}
