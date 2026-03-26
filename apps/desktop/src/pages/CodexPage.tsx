import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { useTranslation } from 'react-i18next';
import { PageContainer } from '@/components/layout/PageContainer';
import { PageContent } from '@/components/layout/PageContent';
import { RegisterTab } from '@/components/codex/register/RegisterTab';
import { AccountsTab } from '@/components/codex/accounts/AccountsTab';
import { UserRound, TerminalSquare } from 'lucide-react';

const TAB_CLASS =
  'text-sm font-medium px-4 h-8 rounded-md data-[state=active]:bg-background data-[state=active]:text-foreground data-[state=active]:shadow-sm transition-all duration-base ease-smooth';

export function CodexPage() {
  const { t } = useTranslation();

  return (
    <Tabs defaultValue="register" className="h-full flex flex-col w-full bg-background/95">
      <PageContainer
        className="h-full bg-muted/10 w-full"
        title={<span>Codex</span>}
        actions={
          <TabsList className="h-10 bg-muted/50 border border-border/50 p-1 rounded-[var(--radius-control)] inline-flex items-center justify-center">
            <TabsTrigger value="register" className={TAB_CLASS}>
              <TerminalSquare className="w-4 h-4 mr-2" />
              {t('codex.page.register')}
            </TabsTrigger>
            <TabsTrigger value="accounts" className={TAB_CLASS}>
              <UserRound className="w-4 h-4 mr-2" />
              {t('codex.page.accounts')}
            </TabsTrigger>
          </TabsList>
        }
      >
        <PageContent maxWidth="none" className="h-full">
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
