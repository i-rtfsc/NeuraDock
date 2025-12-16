import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { RefreshCw, Package, ChevronDown, ChevronUp } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';
import { useState } from 'react';

interface ProviderModelsSectionProps {
  providerId: string;
  providerName: string;
  accountId: string;
  compact?: boolean; // 紧凑模式用于卡片，非紧凑模式用于主页
}

export function ProviderModelsSection({
  providerId,
  accountId,
  compact = false
}: ProviderModelsSectionProps) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [isExpanded, setIsExpanded] = useState(false);

  // Fetch cached models (no API call, just read from database)
  const { data: models = [], isLoading } = useQuery<string[]>({
    queryKey: ['provider-models', providerId],
    queryFn: () => invoke<string[]>('get_cached_provider_models', {
      providerId,
    }),
    staleTime: Infinity, // Cache never stale, only refresh on mutation
    gcTime: 24 * 60 * 60 * 1000, // Keep in cache for 24 hours
    retry: 1,
  });

  // Refresh with WAF bypass mutation
  const refreshMutation = useMutation({
    mutationFn: () => invoke<string[]>('refresh_provider_models_with_waf', {
      providerId,
      accountId,
    }),
    onSuccess: (newModels) => {
      // Update cache for this provider
      queryClient.setQueryData(['provider-models', providerId], newModels);
      toast.success(t('accountCard.modelsRefreshed') || 'Models refreshed successfully');
    },
    onError: (err: Error) => {
      console.error('Failed to refresh models:', err);
      toast.error(t('accountCard.modelsRefreshError') || 'Failed to refresh models: ' + err.message);
    },
  });

  const handleRefresh = () => {
    refreshMutation.mutate();
  };

  if (compact) {
    // 紧凑模式：用于账号卡片
    return (
      <div className="space-y-2">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
            <Package className="h-3.5 w-3.5" />
            <span>
              {isLoading ? (
                t('accountCard.loadingModels') || 'Loading...'
              ) : models.length > 0 ? (
                <>
                  {t('accountCard.supportedModels') || 'Supported Models'}: {models.length}
                </>
              ) : (
                t('accountCard.noModels') || 'No models cached'
              )}
            </span>
          </div>
          <div className="flex items-center gap-1">
            {models.length > 0 && (
              <Button
                variant="ghost"
                size="icon"
                className="h-6 w-6 rounded-full"
                onClick={() => setIsExpanded(!isExpanded)}
              >
                {isExpanded ? (
                  <ChevronUp className="h-3 w-3" />
                ) : (
                  <ChevronDown className="h-3 w-3" />
                )}
              </Button>
            )}
            <Button
              variant="ghost"
              size="icon"
              className="h-6 w-6 rounded-full"
              onClick={handleRefresh}
              disabled={refreshMutation.isPending}
              title={t('accountCard.refreshModels') || 'Refresh models (with WAF bypass)'}
            >
              <RefreshCw className={`h-3 w-3 ${refreshMutation.isPending ? 'animate-spin' : ''}`} />
            </Button>
          </div>
        </div>

        {isExpanded && models.length > 0 && (
          <div className="flex flex-wrap gap-1 max-h-24 overflow-y-auto bg-muted/20 rounded-lg p-2">
            {models.map((model) => (
              <Badge key={model} variant="secondary" className="text-xs py-0 px-1.5">
                {model}
              </Badge>
            ))}
          </div>
        )}
      </div>
    );
  }

  // 非紧凑模式：用于主页
  return (
    <div className="space-y-3">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <Package className="h-4 w-4 text-muted-foreground" />
          <span className="text-sm font-medium">
            {t('dashboard.supportedModels') || 'Supported Models'}
          </span>
          {models.length > 0 && (
            <Badge variant="outline" className="text-xs">
              {models.length}
            </Badge>
          )}
        </div>
        <Button
          variant="outline"
          size="sm"
          onClick={handleRefresh}
          disabled={refreshMutation.isPending || isLoading}
          className="gap-1.5"
        >
          <RefreshCw className={`h-3.5 w-3.5 ${refreshMutation.isPending ? 'animate-spin' : ''}`} />
          {t('common.refresh') || 'Refresh'}
        </Button>
      </div>

      {isLoading && (
        <div className="flex items-center gap-2 text-sm text-muted-foreground py-2">
          <div className="h-4 w-4 animate-spin rounded-full border-2 border-primary border-t-transparent" />
          <span>{t('accountCard.loadingModels') || 'Loading models...'}</span>
        </div>
      )}

      {!isLoading && models.length > 0 && (
        <div className="flex flex-wrap gap-1.5 max-h-32 overflow-y-auto">
          {models.map((model) => (
            <Badge key={model} variant="secondary" className="text-xs">
              {model}
            </Badge>
          ))}
        </div>
      )}

      {!isLoading && models.length === 0 && (
        <div className="text-sm text-muted-foreground py-2">
          {t('accountCard.noModels') || 'No models found. Click refresh to fetch from provider.'}
        </div>
      )}
    </div>
  );
}
