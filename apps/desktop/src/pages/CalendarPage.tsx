import { PageContainer } from '@/components/layout/PageContainer';
import { PageContent } from '@/components/layout/PageContent';
import { CalendarGrid } from '@/components/calendar/CalendarGrid';
import { useTranslation } from 'react-i18next';

export function CalendarPage() {
  const { t } = useTranslation();

  return (
    <PageContainer title={t('nav.calendar')}>
      <PageContent maxWidth="md" centered className="page-enter-stagger">
        <CalendarGrid />
      </PageContent>
    </PageContainer>
  );
}
