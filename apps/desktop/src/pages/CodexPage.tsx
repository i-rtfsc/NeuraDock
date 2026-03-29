import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { useTranslation } from 'react-i18next';
import { PageContainer } from '@/components/layout/PageContainer';
import { PageContent } from '@/components/layout/PageContent';
import { RegisterTab } from '@/components/codex/register/RegisterTab';
import { AccountsTab } from '@/components/codex/accounts/AccountsTab';
import { usePersistedState } from '@/hooks/usePersistedState';
import { UserRound, TerminalSquare } from 'lucide-react';
import { cn } from '@/lib/utils';

type CodexPageTab = 'register' | 'accounts';

const TAB_CLASS =
  'text-sm font-medium px-4 h-8 rounded-md data-[state=active]:bg-background data-[state=active]:text-foreground data-[state=active]:shadow-sm transition-all duration-base ease-smooth';

const DEFAULT_CODEX_TAB: CodexPageTab = 'register';

function normalizeCodexTab(value?: string | null): CodexPageTab {
  switch (value) {
    case 'accounts':
    case 'register':
      return value;
    default:
      return DEFAULT_CODEX_TAB;
  }
}

export function CodexPage() {
  const { t } = useTranslation();
  const [activeTab, setActiveTab] = usePersistedState<CodexPageTab>('page-tab:codex', DEFAULT_CODEX_TAB);

  return (
    <Tabs
      value={normalizeCodexTab(activeTab)}
      onValueChange={(value) => setActiveTab(normalizeCodexTab(value))}
      className="h-full flex flex-col w-full bg-background/95"
    >
      <PageContainer
        className="h-full bg-muted/10 w-full"
        headerClassName="h-auto min-h-[var(--layout-page-header-height)]"
        title={<span>Codex</span>}
        actions={
          <div className="grid min-w-0 flex-1 grid-cols-[minmax(0,1fr)_auto] items-center gap-3">
            <div className="min-w-0" />
            <TabsList className="h-10 shrink-0 bg-muted/50 border border-border/50 p-1 rounded-[var(--radius-control)] inline-flex items-center justify-center">
              <TabsTrigger value="register" className={cn(TAB_CLASS, 'min-w-[112px]')}>
                <TerminalSquare className="w-4 h-4 mr-2" />
                {t('codex.page.register')}
              </TabsTrigger>
              <TabsTrigger value="accounts" className={cn(TAB_CLASS, 'min-w-[112px]')}>
                <UserRound className="w-4 h-4 mr-2" />
                {t('codex.page.accounts')}
              </TabsTrigger>
            </TabsList>
          </div>
        }
      >
        <PageContent maxWidth="none" className="h-full page-enter-stagger">
          <div className="pb-8 w-full h-full">
            <TabsContent
              value="register"
              forceMount
              className="mt-0 outline-none w-full h-full data-[state=inactive]:hidden"
            >
              <RegisterTab />
            </TabsContent>
            <TabsContent
              value="accounts"
              forceMount
              className="mt-0 outline-none w-full data-[state=inactive]:hidden"
            >
              <AccountsTab />
            </TabsContent>
          </div>
        </PageContent>
      </PageContainer>
    </Tabs>
  );
}
