import { useEffect, useState } from 'react';
import { Home, Server, UserCircle } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useSearchParams } from 'react-router-dom';

import { PageContainer } from '@/components/layout/PageContainer';
import { usePersistedState } from '@/hooks/usePersistedState';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { HomePage } from './HomePage';
import { AccountsPage } from './AccountsPage';
import { ProvidersPage } from './ProvidersPage';
import {
  DEFAULT_TRANSIT_HUB_TAB,
  normalizeTransitHubTab,
  type TransitHubTab,
} from '@/lib/transitHub';

const TAB_CLASS =
  'text-sm font-medium px-4 h-8 rounded-md data-[state=active]:bg-background data-[state=active]:text-foreground data-[state=active]:shadow-sm transition-all duration-base ease-smooth';

export function TransitHubPage() {
  const { t } = useTranslation();
  const [searchParams, setSearchParams] = useSearchParams();
  const [persistedTab, setPersistedTab] = usePersistedState<TransitHubTab>(
    'page-tab:transit-hub',
    DEFAULT_TRANSIT_HUB_TAB
  );
  const [embeddedHeaderActionsContainer, setEmbeddedHeaderActionsContainer] =
    useState<HTMLDivElement | null>(null);
  const tabParam = searchParams.get('tab');
  const activeTab = normalizeTransitHubTab(tabParam ?? persistedTab);

  useEffect(() => {
    if (persistedTab !== activeTab) {
      setPersistedTab(activeTab);
    }
  }, [activeTab, persistedTab, setPersistedTab]);

  useEffect(() => {
    if (tabParam === activeTab) {
      return;
    }

    const next = new URLSearchParams(searchParams);
    next.set('tab', activeTab);
    setSearchParams(next, { replace: true });
  }, [activeTab, searchParams, setSearchParams, tabParam]);

  const handleTabChange = (value: string) => {
    const nextTab = normalizeTransitHubTab(value);
    const next = new URLSearchParams(searchParams);
    next.set('tab', nextTab);
    setPersistedTab(nextTab);
    setSearchParams(next, { replace: true });
  };

  return (
    <Tabs value={activeTab} onValueChange={handleTabChange} className="h-full flex flex-col w-full bg-background/95">
      <PageContainer
        className="h-full bg-muted/10 w-full"
        headerClassName="h-auto min-h-[var(--layout-page-header-height)]"
        title={<span>{t('nav.providers')}</span>}
        actions={
          <div className="grid min-w-0 flex-1 grid-cols-[minmax(0,1fr)_auto] items-center gap-3">
            <div
              ref={setEmbeddedHeaderActionsContainer}
              className="flex min-w-0 items-center justify-end overflow-x-auto py-1 scrollbar-hide"
            />

            <TabsList className="h-10 shrink-0 bg-muted/50 border border-border/50 p-1 rounded-[var(--radius-control)] inline-flex items-center justify-center">
              <TabsTrigger value="dashboard" className={TAB_CLASS}>
                <Home className="w-4 h-4 mr-2" />
                {t('nav.dashboard')}
              </TabsTrigger>
              <TabsTrigger value="accounts" className={TAB_CLASS}>
                <UserCircle className="w-4 h-4 mr-2" />
                {t('nav.accounts')}
              </TabsTrigger>
              <TabsTrigger value="providers" className={TAB_CLASS}>
                <Server className="w-4 h-4 mr-2" />
                {t('transitHub.manageTab', 'Manage')}
              </TabsTrigger>
            </TabsList>
          </div>
        }
      >
        <div className="pb-8 w-full h-full">
          <TabsContent value="dashboard" className="mt-0 outline-none w-full h-full">
            <HomePage embedded />
          </TabsContent>
          <TabsContent value="accounts" className="mt-0 outline-none w-full h-full">
            <AccountsPage
              embedded
              embeddedHeaderActionsContainer={embeddedHeaderActionsContainer}
            />
          </TabsContent>
          <TabsContent value="providers" className="mt-0 outline-none w-full h-full">
            <ProvidersPage
              embedded
              embeddedHeaderActionsContainer={embeddedHeaderActionsContainer}
            />
          </TabsContent>
        </div>
      </PageContainer>
    </Tabs>
  );
}
