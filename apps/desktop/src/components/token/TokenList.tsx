import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Progress } from '@/components/ui/progress';
import { Settings2, Clock, Key } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type { TokenDto } from '@/types/token';
import { cn } from '@/lib/utils';

interface TokenListProps {
  tokens: TokenDto[];
  isLoading: boolean;
  onConfigureToken: (token: TokenDto) => void;
}

// Format quota as USD currency - always show 2 decimal places
function formatQuotaUSD(quota: number): string {
  // quota is in smallest unit (e.g., 1/500000 of a dollar)
  const dollars = quota / 500000;
  return `$${dollars.toFixed(2)}`;
}

export function TokenList({ tokens, isLoading, onConfigureToken }: TokenListProps) {
  const { t } = useTranslation();

  if (isLoading) {
    return (
      <Card className="border-dashed bg-muted/30">
        <CardContent className="py-16 text-center">
          <div className="flex flex-col items-center justify-center gap-4">
            <div className="h-8 w-8 animate-spin rounded-full border-4 border-primary border-t-transparent" />
            <span className="text-muted-foreground">{t('common.loading', 'Loading...')}</span>
          </div>
        </CardContent>
      </Card>
    );
  }

  if (tokens.length === 0) {
    return (
      <Card className="border-dashed bg-muted/30">
        <CardContent className="py-16 text-center text-muted-foreground">
          <div className="flex flex-col items-center gap-4">
            <div className="p-4 rounded-full bg-muted">
              <Key className="h-8 w-8 text-muted-foreground/50" />
            </div>
            <p>{t('token.noTokens', 'No tokens found for this account.')}</p>
          </div>
        </CardContent>
      </Card>
    );
  }

  return (
    <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
      {tokens.map((token) => (
        <Card
          key={token.id}
          className={cn(
            "flex flex-col border-none shadow-sm bg-background/60 backdrop-blur-xl ring-1 ring-border/50 hover:shadow-md interactive-scale",
            !token.is_active && "opacity-60 grayscale"
          )}
        >
          <CardHeader className="pb-3">
            <div className="flex items-start justify-between gap-2">
              <CardTitle className="text-base font-semibold truncate flex-1" title={token.name}>
                {token.name}
              </CardTitle>
              <Badge
                variant={token.is_active ? 'default' : 'secondary'}
                className={cn(
                  "flex-shrink-0 rounded-full px-2 py-0.5 text-[10px] h-5",
                  token.is_active ? "bg-green-500 hover:bg-green-600" : ""
                )}
              >
                {token.status_text}
              </Badge>
            </div>
            <div className="flex items-center gap-1.5 text-xs font-mono text-muted-foreground bg-muted/50 w-fit px-2 py-1 rounded-md">
              <Key className="h-3 w-3" />
              {token.masked_key}
            </div>
          </CardHeader>

          <CardContent className="flex-1 flex flex-col gap-4">
            {/* Quota Usage Section */}
            <div className="flex-1 space-y-3">
              {token.unlimited_quota ? (
                <div className="p-3 rounded-lg bg-green-50 dark:bg-green-950/20 border border-green-100 dark:border-green-900/30">
                  <div className="flex items-center justify-between mb-1">
                    <span className="text-xs font-medium text-green-700 dark:text-green-300">{t('token.quotaUnlimited', 'Unlimited')}</span>
                    <Badge variant="outline" className="text-[10px] h-4 px-1 text-green-600 border-green-600 bg-transparent">
                      ∞
                    </Badge>
                  </div>
                  <div className="text-xs text-green-600/80 dark:text-green-400/80">
                    <span>{t('token.usedQuota', 'Used')}: </span>
                    <span className="font-mono font-medium">{formatQuotaUSD(token.used_quota)}</span>
                  </div>
                </div>
              ) : (
                <div className="space-y-2">
                  <div className="flex justify-between text-xs">
                    <span className="text-muted-foreground">{t('token.quotaUsage', 'Quota Usage')}</span>
                    <span className={cn(
                      "font-medium",
                      token.usage_percentage > 90 ? "text-red-500" : "text-foreground"
                    )}>{token.usage_percentage.toFixed(1)}%</span>
                  </div>
                  <Progress 
                    value={token.usage_percentage} 
                    className="h-2"
                    indicatorClassName={cn(
                      token.usage_percentage > 90 ? "bg-red-500" : 
                      token.usage_percentage > 75 ? "bg-orange-500" : "bg-primary"
                    )}
                  />
                  <div className="flex justify-between text-[10px] text-muted-foreground">
                    <div>
                      <span>{t('token.usedQuota', 'Used')}: </span>
                      <span className="font-mono font-medium text-foreground">{formatQuotaUSD(token.used_quota)}</span>
                    </div>
                    <div>
                      <span>{t('token.remainQuota', 'Remain')}: </span>
                      <span className="font-mono font-medium text-foreground">{formatQuotaUSD(token.remain_quota)}</span>
                    </div>
                  </div>
                </div>
              )}

              <div className="grid grid-cols-2 gap-2 pt-1">
                {/* Expiration */}
                {token.expired_at && (
                  <div className="flex flex-col gap-1 p-2 rounded-lg bg-muted/30">
                    <span className="text-[10px] text-muted-foreground uppercase tracking-wider">{t('token.expiresAt', 'Expires')}</span>
                    <div className="flex items-center gap-1 text-xs font-medium">
                      <Clock className="h-3 w-3 text-muted-foreground" />
                      {new Date(token.expired_at).toLocaleDateString()}
                    </div>
                  </div>
                )}

                {/* Model Limits */}
                <div className={cn("flex flex-col gap-1 p-2 rounded-lg bg-muted/30", !token.expired_at && "col-span-2")}>
                  <span className="text-[10px] text-muted-foreground uppercase tracking-wider">{t('token.supportedModels', 'Models')}</span>
                  <div className="text-xs font-medium truncate">
                    {!token.model_limits_enabled ? (
                      <span className="text-green-600 dark:text-green-400">
                        {t('token.noLimits', 'Unrestricted')}
                      </span>
                    ) : token.model_limits_allowed.length > 0 ? (
                      <span title={token.model_limits_allowed.join(', ')}>
                        {token.model_limits_allowed.slice(0, 2).join(', ')}
                        {token.model_limits_allowed.length > 2 && ` +${token.model_limits_allowed.length - 2}`}
                      </span>
                    ) : (
                      <span className="text-muted-foreground">
                        {t('token.noModelsConfigured', 'None')}
                      </span>
                    )}
                  </div>
                </div>
              </div>
            </div>

            {/* Configure Button */}
            <Button
              variant="outline"
              size="sm"
              className="w-full mt-auto text-xs font-medium rounded-lg shadow-sm bg-gradient-to-r from-background/80 to-background/50 hover:from-primary/10 hover:to-primary/5 hover:text-primary hover:border-primary/30 transition-all duration-base ease-smooth"
              onClick={() => onConfigureToken(token)}
              disabled={!token.is_active}
            >
              <Settings2 className="mr-2 h-3.5 w-3.5" />
              {t('token.configureAI', 'Configure AI Tool')}
            </Button>
          </CardContent>
        </Card>
      ))}
    </div>
  );
}
