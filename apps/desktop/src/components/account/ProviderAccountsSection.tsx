import { useState } from "react";
import { RefreshCw } from "lucide-react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { AccountCard } from "./AccountCard";
import { Account } from "@/lib/tauri-commands";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "@/components/ui/collapsible";
import { AnimatePresence, LayoutGroup } from "framer-motion";

interface ProviderStats {
  provider_id: string;
  provider_name: string;
  account_count: number;
  total_balance: number;
}

interface ProviderAccountsSectionProps {
  providerName: string;
  accounts: Account[];
  stats?: ProviderStats;
  onEdit: (account: Account) => void;
  onRefreshBalances: (accounts: Account[]) => Promise<void>;
}

export function ProviderAccountsSection({
  providerName,
  accounts,
  stats,
  onEdit,
  onRefreshBalances,
}: ProviderAccountsSectionProps) {
  const { t } = useTranslation();
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [isOpen, setIsOpen] = useState(true);

  const handleRefresh = async () => {
    const enabledAccounts = accounts.filter((a) => a.enabled);
    if (enabledAccounts.length === 0) {
      toast.error(t("accounts.noEnabledAccounts"));
      return;
    }

    setIsRefreshing(true);
    try {
      await onRefreshBalances(accounts);
    } finally {
      setIsRefreshing(false);
    }
  };

  return (
    <Collapsible open={isOpen} onOpenChange={setIsOpen}>
      <Card className="border-none shadow-none bg-transparent">
        <CollapsibleTrigger asChild>
          <CardHeader className="cursor-pointer hover:bg-accent/5 transition-colors rounded-xl px-2 py-3">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-3">
                <CardTitle className="text-xl font-medium tracking-tight">{providerName}</CardTitle>
                <Badge variant="secondary" className="rounded-full px-2.5 font-normal bg-secondary/50">{accounts.length} {t("accounts.accounts")}</Badge>
                {stats && stats.total_balance > 0 && (
                  <Badge variant="outline" className="bg-primary/5 border-primary/20 text-primary rounded-full px-2.5 font-normal">
                    ¥{stats.total_balance.toFixed(2)}
                  </Badge>
                )}
              </div>
              <div className="flex items-center gap-2">
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={(e) => {
                    e.stopPropagation();
                    handleRefresh();
                  }}
                  disabled={isRefreshing}
                  className="rounded-full hover:bg-white/50 dark:hover:bg-white/10"
                >
                  <RefreshCw className={`h-3.5 w-3.5 mr-1.5 ${isRefreshing ? "animate-spin" : ""}`} />
                  <span className="text-xs">{t("accounts.refreshBalances")}</span>
                </Button>
              </div>
            </div>
          </CardHeader>
        </CollapsibleTrigger>
        <CollapsibleContent>
          <CardContent className="pt-2 px-1 pb-6">
            <LayoutGroup>
              <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
                <AnimatePresence mode="popLayout">
                  {accounts.map((account) => (
                    <AccountCard
                      key={account.id}
                      account={account}
                      onEdit={() => onEdit(account)}
                    />
                  ))}
                </AnimatePresence>
              </div>
            </LayoutGroup>
          </CardContent>
        </CollapsibleContent>
      </Card>
    </Collapsible>
  );
}
