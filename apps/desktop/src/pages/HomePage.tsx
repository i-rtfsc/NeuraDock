import { useEffect, useMemo } from 'react';
import { Users, DollarSign, TrendingUp, Wallet, Activity, Zap, Server } from 'lucide-react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { useAccounts } from '@/hooks/useAccounts';
import { useBalanceStatistics } from '@/hooks/useBalance';
import { ProviderModelsSection } from '@/components/account/ProviderModelsSection';
import { useTranslation } from 'react-i18next';
import { motion, type Variants } from 'framer-motion';
import { formatCurrency } from '@/lib/formatters';
import { useNavigate } from 'react-router-dom';

import { PageContainer } from '@/components/layout/PageContainer';
import { PageContent, Section } from '@/components/layout/PageContent';
import { BentoGrid } from '@/components/layout/CardGrid';
import { DashboardSkeleton } from '@/components/skeletons/DashboardSkeleton';
import { cn } from '@/lib/utils';
import { Button } from '@/components/ui/button';
import { createFadeUpItem, createStaggerContainer } from '@/lib/motion';

export function HomePage() {
  const { data: accounts, isLoading } = useAccounts();
  const { data: statistics, isLoading: statsLoading } = useBalanceStatistics();
  const { t } = useTranslation();
  const navigate = useNavigate();

  const shouldAnimate = useMemo(() => {
    try {
      return sessionStorage.getItem('neuradock-booted') === '1';
    } catch {
      return true;
    }
  }, []);

  useEffect(() => {
    try {
      sessionStorage.setItem('neuradock-booted', '1');
    } catch {
      // ignore
    }
  }, []);

  const sortedProviders = useMemo(() => {
    if (!statistics?.providers) return [];
    return [...statistics.providers].sort((a, b) => {
      const balanceDiff = (b.current_balance ?? 0) - (a.current_balance ?? 0);
      if (balanceDiff !== 0) return balanceDiff;
      return (b.total_quota ?? 0) - (a.total_quota ?? 0);
    });
  }, [statistics?.providers]);

  if (isLoading || statsLoading) {
    return (
      <PageContainer>
        <DashboardSkeleton />
      </PageContainer>
    );
  }

  const totalAccounts = accounts?.length || 0;

  const container: Variants = createStaggerContainer({ staggerChildren: 0.05, delayChildren: 0.1 });
  const item: Variants = createFadeUpItem({ y: 10, scale: 0.98 });

  // Common card interactive styles
  const interactiveCardClass = "bg-card border shadow-sm hover:shadow-md cursor-pointer interactive-scale";

  return (
    <PageContainer title={t('dashboard.title')}>
      <PageContent maxWidth="lg">
        {/* Bento Grid Overview */}
        <motion.div
          variants={container}
          initial={shouldAnimate ? 'hidden' : false}
          animate="show"
        >
          <BentoGrid>
            {/* Main Balance Card - Spans 2 cols on large screens */}
            <motion.div variants={item} className="md:col-span-2">
              <Card className={cn(
                "h-full relative overflow-hidden border-primary/20 shadow-md",
                "bg-gradient-to-br from-background via-background to-primary/5 dark:from-background dark:via-background dark:to-primary/10",
                "hover:shadow-lg cursor-pointer interactive-scale"
              )}>
                <div className="absolute top-0 right-0 p-4 opacity-10">
                  <Wallet className="w-24 h-24 text-primary" />
                </div>
                <CardHeader className="pb-2">
                  <CardTitle className="text-sm font-medium text-muted-foreground flex items-center gap-2">
                    <Wallet className="h-4 w-4 text-primary" />
                    {t('dashboard.stats.currentBalance')}
                  </CardTitle>
                </CardHeader>
                <CardContent>
                  <div className="text-4xl font-bold tracking-tight tabular-nums text-foreground">
                    {statsLoading ? '...' : statistics ? formatCurrency(statistics.total_current_balance) : '$0.00'}
                  </div>
                  <div className="mt-4 grid grid-cols-2 gap-3">
                    <div className="rounded-xl border bg-background/60 px-3 py-2 backdrop-blur-sm">
                      <div className="flex items-center gap-2 text-xs text-muted-foreground">
                        <DollarSign className="h-3.5 w-3.5 text-blue-600 dark:text-blue-400" />
                        <span>{t('dashboard.stats.totalQuota')}</span>
                      </div>
                      <div className="mt-1 text-lg font-semibold tabular-nums text-foreground">
                        {statistics ? formatCurrency(statistics.total_quota) : '$0.00'}
                      </div>
                    </div>
                    <div className="rounded-xl border bg-background/60 px-3 py-2 backdrop-blur-sm">
                      <div className="flex items-center gap-2 text-xs text-muted-foreground">
                        <TrendingUp className="h-3.5 w-3.5 text-orange-600 dark:text-orange-400" />
                        <span>{t('dashboard.stats.historicalConsumption')}</span>
                      </div>
                      <div className="mt-1 text-lg font-semibold tabular-nums text-foreground">
                        {statistics ? formatCurrency(statistics.total_consumed) : '$0.00'}
                      </div>
                    </div>
                  </div>
                </CardContent>
              </Card>
            </motion.div>

            {/* Accounts Status */}
            <motion.div variants={item} className="md:col-span-2 lg:col-span-2">
              <Card className={cn(interactiveCardClass, "h-full")}>
                <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                  <CardTitle className="text-sm font-medium text-muted-foreground">{t('dashboard.stats.totalAccounts')}</CardTitle>
                  <div className="p-2 rounded-full bg-emerald-50 dark:bg-emerald-900/20">
                    <Users className="h-4 w-4 text-emerald-600 dark:text-emerald-400" />
                  </div>
                </CardHeader>
                <CardContent>
                  <div className="text-3xl font-bold tabular-nums text-foreground">{isLoading ? '...' : totalAccounts}</div>
                  <div className="mt-4 flex flex-wrap gap-2">
                    <Button size="sm" className="shadow-sm" onClick={() => navigate('/accounts')}>
                      <Users className="mr-2 h-4 w-4" />
                      {t('nav.accounts')}
                    </Button>
                    <Button
                      size="sm"
                      variant="outline"
                      className="shadow-sm"
                      onClick={() => navigate('/providers')}
                    >
                      <Server className="mr-2 h-4 w-4" />
                      {t('nav.providers')}
                    </Button>
                  </div>
                </CardContent>
              </Card>
            </motion.div>
          </BentoGrid>
        </motion.div>

        {/* Provider Breakdown Section */}
        {statistics && sortedProviders.length > 0 && (
          <motion.div
            variants={container}
            initial={shouldAnimate ? 'hidden' : false}
            animate="show"
            className="mt-8"
          >
            <Section
              title={
                <div className="flex items-center gap-2">
                  <Activity className="h-5 w-5 text-primary" />
                  <span>{t('dashboard.providerBreakdown')}</span>
                </div>
              }
            >
              {/* Grid layout for scalability */}
              <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-3">
                {sortedProviders.map((provider) => {
                  const providerAccount = accounts?.find(
                    (acc) => acc.provider_id === provider.provider_id && acc.enabled
                  );

                  return (
                    <motion.div key={provider.provider_id} variants={item} className="h-full">
                      <Card className={cn(interactiveCardClass, "h-full flex flex-col overflow-hidden")}>
                        {/* Provider Header */}
                        <div className="p-4 bg-muted/30 border-b flex items-center justify-between">
                          <div className="flex items-center gap-3">
                            <div className="p-2 bg-background rounded-full border shadow-sm">
                              <Zap className="h-4 w-4 text-yellow-500" />
                            </div>
                            <div>
                              <h3 className="font-semibold text-foreground">{provider.provider_name}</h3>
                              <p className="text-xs text-muted-foreground">
                                {provider.account_count} {provider.account_count === 1 ? t('dashboard.account') : t('dashboard.accounts_plural')}
                              </p>
                            </div>
                          </div>
                          <div className="text-right">
                             <div className="font-mono font-bold text-green-600 dark:text-green-400">
                               {formatCurrency(provider.current_balance)}
                             </div>
                          </div>
                        </div>

                        {/* Models List - Flex grow to push content */}
                        <CardContent className="p-4 flex-grow">
                          {providerAccount ? (
                            <ProviderModelsSection
                              providerId={provider.provider_id}
                              providerName={provider.provider_name}
                              accountId={providerAccount.id}
                              compact={true} // Use compact mode for grid layout
                            />
                          ) : (
                            <div className="h-full flex items-center justify-center text-sm text-muted-foreground italic min-h-[60px]">
                              {t('dashboard.noActiveAccounts')}
                            </div>
                          )}
                        </CardContent>
                      </Card>
                    </motion.div>
                  );
                })}
              </div>
            </Section>
          </motion.div>
        )}
      </PageContent>
    </PageContainer>
  );
}
