import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Progress } from '@/components/ui/progress';
import { Settings2, Clock, Infinity } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type { TokenDto } from '@/types/token';

interface TokenListProps {
  tokens: TokenDto[];
  isLoading: boolean;
  onConfigureToken: (token: TokenDto) => void;
}

// Format quota as USD currency
function formatQuotaUSD(quota: number): string {
  // quota is in smallest unit (e.g., 1/500000 of a dollar)
  const dollars = quota / 500000;
  if (dollars >= 1000) {
    return `$${(dollars / 1000).toFixed(1)}K`;
  }
  if (dollars >= 1) {
    return `$${dollars.toFixed(2)}`;
  }
  if (dollars >= 0.01) {
    return `$${dollars.toFixed(2)}`;
  }
  return `$${dollars.toFixed(4)}`;
}

export function TokenList({ tokens, isLoading, onConfigureToken }: TokenListProps) {
  const { t } = useTranslation();

  if (isLoading) {
    return (
      <Card>
        <CardContent className="py-8 text-center">
          <div className="flex items-center justify-center gap-2">
            <div className="h-4 w-4 animate-spin rounded-full border-2 border-primary border-t-transparent" />
            <span>{t('common.loading', 'Loading...')}</span>
          </div>
        </CardContent>
      </Card>
    );
  }

  if (tokens.length === 0) {
    return (
      <Card>
        <CardContent className="py-8 text-center text-muted-foreground">
          {t('token.noTokens', 'No tokens found for this account.')}
        </CardContent>
      </Card>
    );
  }

  return (
    <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
      {tokens.map((token) => (
        <Card
          key={token.id}
          className={`flex flex-col ${!token.is_active ? 'opacity-60' : ''}`}
        >
          <CardHeader className="pb-3">
            <div className="flex items-start justify-between gap-2">
              <CardTitle className="text-lg truncate flex-1" title={token.name}>
                {token.name}
              </CardTitle>
              <Badge
                variant={token.is_active ? 'default' : 'secondary'}
                className="flex-shrink-0"
              >
                {token.status_text}
              </Badge>
            </div>
            <p className="text-sm font-mono text-muted-foreground">
              {token.masked_key}
            </p>
          </CardHeader>

          <CardContent className="flex-1 flex flex-col">
            {/* Quota Usage Section */}
            <div className="flex-1 space-y-3">
              {token.unlimited_quota ? (
                <div className="space-y-2">
                  <div className="flex items-center gap-2 text-sm">
                    <Infinity className="h-4 w-4 text-green-500" />
                    <span className="font-medium">{t('token.quotaUnlimited', 'Quota Unlimited')}</span>
                  </div>
                  <div className="text-sm text-muted-foreground">
                    <span>{t('token.usedQuota', 'Used')}: </span>
                    <span className="font-medium text-foreground">{formatQuotaUSD(token.used_quota)}</span>
                  </div>
                </div>
              ) : (
                <div className="space-y-2">
                  <div className="flex justify-between text-sm">
                    <span className="text-muted-foreground">{t('token.quotaUsage', 'Quota Usage')}</span>
                    <span className="font-medium">{token.usage_percentage.toFixed(1)}%</span>
                  </div>
                  <Progress value={token.usage_percentage} />
                  <div className="grid grid-cols-2 gap-2 text-xs">
                    <div>
                      <span className="text-muted-foreground">{t('token.usedQuota', 'Used')}: </span>
                      <span className="font-medium">{formatQuotaUSD(token.used_quota)}</span>
                    </div>
                    <div className="text-right">
                      <span className="text-muted-foreground">{t('token.remainQuota', 'Remain')}: </span>
                      <span className="font-medium">{formatQuotaUSD(token.remain_quota)}</span>
                    </div>
                  </div>
                </div>
              )}

              {/* Expiration */}
              {token.expired_at && (
                <div className="flex items-center gap-1 text-xs text-muted-foreground">
                  <Clock className="h-3 w-3" />
                  <span>
                    {t('token.expiresAt', 'Expires')}: {new Date(token.expired_at).toLocaleDateString()}
                  </span>
                </div>
              )}

              {/* Model Limits */}
              {token.model_limits_allowed.length > 0 && (
                <div className="text-xs">
                  <span className="text-muted-foreground">
                    {t('token.supportedModels', 'Models')}:{' '}
                  </span>
                  <span className="font-medium">
                    {token.model_limits_allowed.slice(0, 3).join(', ')}
                    {token.model_limits_allowed.length > 3 && ` +${token.model_limits_allowed.length - 3}`}
                  </span>
                </div>
              )}
            </div>

            {/* Configure Button - Always at bottom */}
            <div className="mt-4 pt-3 border-t">
              <Button
                className="w-full"
                size="sm"
                variant="outline"
                onClick={() => onConfigureToken(token)}
                disabled={!token.is_active}
              >
                <Settings2 className="mr-2 h-4 w-4" />
                {t('token.configureAI', 'Configure AI Tool')}
              </Button>
            </div>
          </CardContent>
        </Card>
      ))}
    </div>
  );
}
