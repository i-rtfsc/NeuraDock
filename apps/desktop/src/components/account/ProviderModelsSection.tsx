import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { RefreshCw, Package } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';

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
    // 紧凑模式：用于主页网格卡片
    return (
      <div className="space-y-3 h-full flex flex-col">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
            <Package className="h-3.5 w-3.5" />
            <span>
              {isLoading ? (
                t('accountCard.loadingModels') || 'Loading...'
              ) : models.length > 0 ? (
                t('accountCard.supportedModels') || 'Models'
              ) : (
                t('accountCard.noModels') || 'No models'
              )}
            </span>
          </div>
          <Button
            variant="ghost"
            size="icon"
            className="h-6 w-6 rounded-full opacity-50 hover:opacity-100 transition-opacity duration-base ease-smooth"
            onClick={handleRefresh}
            disabled={refreshMutation.isPending}
            title={t('accountCard.refreshModels') || 'Refresh models'}
          >
            <RefreshCw className={`h-3 w-3 ${refreshMutation.isPending ? 'animate-spin' : ''}`} />
          </Button>
        </div>

        {/* Always show models, filling available space */}
        {models.length > 0 && (
          <div className="flex flex-wrap gap-1.5 content-start">
            {models.map((model) => (
              <Badge 
                key={model} 
                variant="secondary" 
                className="text-[10px] px-1.5 py-0 h-5 font-medium bg-muted/50 text-muted-foreground border-transparent"
              >
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
