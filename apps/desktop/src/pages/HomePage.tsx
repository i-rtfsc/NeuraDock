import { Users, DollarSign, TrendingUp, Wallet, Activity } from 'lucide-react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { useAccounts } from '@/hooks/useAccounts';
import { useBalanceStatistics } from '@/hooks/useBalance';
import { ProviderModelsSection } from '@/components/account/ProviderModelsSection';
import { useTranslation } from 'react-i18next';
import { motion } from 'framer-motion';
import { formatCurrency } from '@/lib/formatters';

import { PageContainer } from '@/components/layout/PageContainer';
import { PageContent, Section } from '@/components/layout/PageContent';
import { BentoGrid } from '@/components/layout/CardGrid';
import { DashboardSkeleton } from '@/components/skeletons/DashboardSkeleton';

export function HomePage() {
  const { data: accounts, isLoading } = useAccounts();
  const { data: statistics, isLoading: statsLoading } = useBalanceStatistics();
  const { t } = useTranslation();

  if (isLoading || statsLoading) {
    return (
      <PageContainer>
        <DashboardSkeleton />
      </PageContainer>
    );
  }

  const enabledAccounts = accounts?.filter(a => a.enabled).length || 0;
  const totalAccounts = accounts?.length || 0;

  // Animation variants
  const container = {
    hidden: { opacity: 0 },
    show: {
      opacity: 1,
      transition: {
        staggerChildren: 0.1
      }
    }
  };

  const item = {
    hidden: { opacity: 0, y: 20 },
    show: { opacity: 1, y: 0 }
  };

  return (
    <PageContainer title={t('dashboard.title')}>
      <PageContent maxWidth="lg">
        {/* Bento Grid Overview */}
        <motion.div
          variants={container}
          initial="hidden"
          animate="show"
        >
          <BentoGrid>
            {/* Main Balance Card - Spans 2 cols on large screens */}
            <motion.div variants={item} className="md:col-span-2">
              <Card className="h-full bg-gradient-to-br from-primary/10 via-primary/5 to-background border-primary/20">
                <CardHeader className="pb-2">
                  <CardTitle className="text-sm font-medium text-primary flex items-center gap-2">
                    <Wallet className="h-4 w-4" />
                    {t('dashboard.stats.currentBalance')}
                  </CardTitle>
                </CardHeader>
                <CardContent>
                  <div className="text-4xl font-bold tracking-tight tabular-nums text-primary">
                    {statsLoading ? '...' : statistics ? formatCurrency(statistics.total_current_balance) : '$0.00'}
                  </div>
                  <p className="text-sm text-muted-foreground mt-1">
                    {t('dashboard.stats.availableBalance')}
                  </p>
                </CardContent>
              </Card>
            </motion.div>

            {/* Income & Consumption Stack */}
            <motion.div variants={item} className="space-y-card-gap">
              <Card className="bg-muted/30 border-none shadow-none">
                <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                  <CardTitle className="text-sm font-medium">{t('dashboard.stats.totalIncome')}</CardTitle>
                  <DollarSign className="h-4 w-4 text-blue-500" />
                </CardHeader>
                <CardContent>
                  <div className="text-2xl font-bold tabular-nums text-blue-600">
                    {statsLoading ? '...' : statistics ? formatCurrency(statistics.total_income) : '$0.00'}
                  </div>
                </CardContent>
              </Card>
              <Card className="bg-muted/30 border-none shadow-none">
                <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                  <CardTitle className="text-sm font-medium">{t('dashboard.stats.historicalConsumption')}</CardTitle>
                  <TrendingUp className="h-4 w-4 text-orange-500" />
                </CardHeader>
                <CardContent>
                  <div className="text-2xl font-bold tabular-nums text-orange-600">
                    {statsLoading ? '...' : statistics ? formatCurrency(statistics.total_consumed) : '$0.00'}
                  </div>
                </CardContent>
              </Card>
            </motion.div>

            {/* Accounts Status */}
            <motion.div variants={item}>
              <Card className="h-full flex flex-col justify-center bg-muted/30 border-none shadow-none">
                <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                  <CardTitle className="text-sm font-medium">{t('dashboard.stats.totalAccounts')}</CardTitle>
                  <Users className="h-4 w-4 text-muted-foreground" />
                </CardHeader>
                <CardContent>
                  <div className="text-3xl font-bold tabular-nums">{isLoading ? '...' : totalAccounts}</div>
                  <div className="flex items-center gap-2 mt-2">
                    <span className="relative flex h-2 w-2">
                      <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-green-400 opacity-75"></span>
                      <span className="relative inline-flex rounded-full h-2 w-2 bg-green-500"></span>
                    </span>
                    <p className="text-xs text-muted-foreground">
                      {enabledAccounts} {t('dashboard.stats.enabled')}
                    </p>
                  </div>
                </CardContent>
              </Card>
            </motion.div>
          </BentoGrid>
        </motion.div>

        {/* Provider Breakdown Section */}
        {statistics && statistics.providers.length > 0 && (
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: 0.4 }}
          >
            <Section
              title={
                <div className="flex items-center gap-2">
                  <Activity className="h-5 w-5 text-primary" />
                  <span>{t('dashboard.providerBreakdown')}</span>
                </div>
              }
            >
              <div className="grid gap-section-gap-sm">
                {statistics.providers.map((provider) => {
                  const providerAccount = accounts?.find(
                    (acc) => acc.provider_id === provider.provider_id && acc.enabled
                  );

                  return (
                    <Card key={provider.provider_id} className="overflow-hidden border-none shadow-sm bg-card/50">
                      <CardContent className="p-0">
                        {/* Provider Header */}
                        <div className="p-card bg-muted/20 flex items-center justify-between">
                          <div>
                            <h3 className="text-lg font-semibold">{provider.provider_name}</h3>
                            <p className="text-sm text-muted-foreground">
                              {provider.account_count} {provider.account_count === 1 ? t('dashboard.account') : t('dashboard.accounts_plural')}
                            </p>
                          </div>
                          <div className="flex gap-8 text-sm">
                            <div className="text-right">
                              <p className="text-xs text-muted-foreground mb-1">{t('dashboard.stats.currentBalance')}</p>
                              <p className="font-bold text-lg tabular-nums text-green-600">{formatCurrency(provider.current_balance)}</p>
                            </div>
                          </div>
                        </div>

                        {/* Models List */}
                        {providerAccount && (
                          <div className="p-card">
                            <ProviderModelsSection
                              providerId={provider.provider_id}
                              providerName={provider.provider_name}
                              accountId={providerAccount.id}
                              compact={false}
                            />
                          </div>
                        )}
                      </CardContent>
                    </Card>
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
