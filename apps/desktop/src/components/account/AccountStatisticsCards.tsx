import { Card, CardContent } from "@/components/ui/card";
import { DollarSign } from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/utils";

interface AccountStatistics {
  total_accounts: number;
  enabled_accounts: number;
  total_balance: number;
  active_providers: number;
}

interface AccountStatisticsCardsProps {
  statistics?: AccountStatistics;
  isLoading?: boolean;
}

export function AccountStatisticsCards({
  statistics,
  isLoading,
}: AccountStatisticsCardsProps) {
  const { t } = useTranslation();

  const stats = [
    {
      label: t("accounts.totalAccounts"),
      value: statistics?.total_accounts ?? 0,
      icon: null,
    },
    {
      label: t("accounts.enabledAccounts"),
      value: statistics?.enabled_accounts ?? 0,
      icon: null,
    },
    {
      label: t("accounts.totalBalance"),
      value: statistics
        ? `¥${statistics.total_balance.toFixed(2)}`
        : "¥0.00",
      icon: <DollarSign className="h-4 w-4" />,
    },
    {
      label: t("accounts.activeProviders"),
      value: statistics?.active_providers ?? 0,
      icon: null,
    },
  ];

  return (
    <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
      {stats.map((stat, index) => (
        <Card
          key={index}
          className={cn(
            "transition-all duration-200",
            isLoading && "animate-pulse"
          )}
        >
          <CardContent className="p-6">
            <div className="flex items-center justify-between space-x-2">
              <div className="space-y-1">
                <p className="text-sm font-medium text-muted-foreground">
                  {stat.label}
                </p>
                <p className="text-2xl font-bold">{stat.value}</p>
              </div>
              {stat.icon && (
                <div className="rounded-full bg-primary/10 p-2 text-primary">
                  {stat.icon}
                </div>
              )}
            </div>
          </CardContent>
        </Card>
      ))}
    </div>
  );
}
