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
  const item: Variants = createFadeUpItem({ y: 10, scale: 1 });

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
              <Card className="card-vivid group relative overflow-hidden border-primary/20 bg-gradient-to-br from-background via-primary/5 to-accent-2/5 dark:from-background dark:via-primary/10 dark:to-accent-2/10">
                <div className="absolute top-0 right-0 p-4 opacity-5 group-hover:opacity-10 transition-opacity duration-slow">
                  <Wallet className="w-32 h-32 text-primary rotate-12" />
                </div>
                <CardHeader className="pb-2">
                  <CardTitle className="text-sm font-bold uppercase tracking-wider text-primary flex items-center gap-2">
                    <Wallet className="h-4 w-4" />
                    {t('dashboard.stats.currentBalance')}
                  </CardTitle>
                </CardHeader>
                <CardContent>
                  <div className="text-4xl font-black tracking-tight tabular-nums text-vivid bg-clip-text text-transparent bg-gradient-to-r from-primary via-accent-2 to-primary animate-gradient-x">
                    {statsLoading ? '...' : statistics ? formatCurrency(statistics.total_current_balance) : '$0.00'}
                  </div>
                  <div className="mt-4 grid grid-cols-2 gap-3">
                    <div className="rounded-xl border bg-background/60 px-3 py-2 backdrop-blur-sm">
                      <div className="flex items-center gap-2 text-xs text-muted-foreground">
                        <DollarSign className="h-3.5 w-3.5 text-info" />
                        <span>{t('dashboard.stats.totalQuota')}</span>
                      </div>
                      <div className="mt-1 text-lg font-semibold tabular-nums text-info">
                        {statistics ? formatCurrency(statistics.total_quota) : '$0.00'}
                      </div>
                    </div>
                    <div className="rounded-xl border bg-background/60 px-3 py-2 backdrop-blur-sm">
                      <div className="flex items-center gap-2 text-xs text-muted-foreground">
                        <TrendingUp className="h-3.5 w-3.5 text-warning" />
                        <span>{t('dashboard.stats.historicalConsumption')}</span>
                      </div>
                      <div className="mt-1 text-lg font-semibold tabular-nums text-warning">
                        {statistics ? formatCurrency(statistics.total_consumed) : '$0.00'}
                      </div>
                    </div>
                  </div>
                </CardContent>
              </Card>
            </motion.div>

            {/* Accounts Status */}
            <motion.div variants={item} className="md:col-span-2 lg:col-span-2">
              <Card className="card-vivid group h-full">
                <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                  <CardTitle className="text-sm font-medium text-muted-foreground">{t('dashboard.stats.totalAccounts')}</CardTitle>
                  <div className="p-2 rounded-full bg-success-soft">
                    <Users className="h-4 w-4 text-success" />
                  </div>
                </CardHeader>
                <CardContent>
                  <div className="text-3xl font-bold tabular-nums text-foreground">{isLoading ? '...' : totalAccounts}</div>
                  <div className="mt-4 flex flex-wrap gap-2">
                    <Button size="sm" className="shadow-sm border-0 bg-muted/50 hover:bg-primary/10 hover:text-primary" onClick={() => navigate('/accounts')}>
                      <Users className="mr-2 h-4 w-4" />
                      {t('nav.accounts')}
                    </Button>
                    <Button
                      size="sm"
                      variant="outline"
                      className="shadow-sm border-0 bg-muted/50 hover:bg-primary/10 hover:text-primary"
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
                      <Card className="card-vivid group h-full">
                        {/* Side Accent Bar */}
                        <div className={cn(
                          "card-accent-bar",
                          "bg-primary/40 group-hover:bg-primary"
                        )} />
                        
                        <div className="flex flex-col flex-1">
                          {/* Provider Header */}
                          <div className="p-4 border-b border-border/30 flex items-center justify-between bg-muted/20">
                            <div className="flex items-center gap-3">
                              <div className="p-2 bg-background rounded-full border shadow-sm">
                                <Zap className="h-4 w-4 text-warning" />
                              </div>
                              <div className="min-w-0">
                                <h3 className="font-bold text-foreground truncate group-hover:text-primary transition-colors">{provider.provider_name}</h3>
                                <p className="text-xs text-muted-foreground">
                                  {provider.account_count} {provider.account_count === 1 ? t('dashboard.account') : t('dashboard.accounts_plural')}
                                </p>
                              </div>
                            </div>
                            <div className="text-right shrink-0">
                               <div className="font-mono font-bold text-success text-sm">
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
                              <div className="h-full flex items-center justify-center text-sm text-muted-foreground italic min-h-[60px] bg-muted/10 rounded-lg border border-dashed border-border/50">
                                {t('dashboard.noActiveAccounts')}
                              </div>
                            )}
                          </CardContent>
                        </div>
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
