import { useState, useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { Plus, Search, Server } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { PageContainer } from '@/components/layout/PageContainer';
import { ProviderCard } from '@/components/provider/ProviderCard';
import { ProviderDialog, ProviderFormValues } from '@/components/provider/ProviderDialog';
import { useProviders } from '@/hooks/useProviders';
import { useProviderActions } from '@/hooks/useProviderActions';
import { Skeleton } from '@/components/ui/skeleton';

export function ProvidersPage() {
  const { t } = useTranslation();
  const { data: providers = [], isLoading } = useProviders();
  const {
    dialogOpen,
    editingProvider,
    handleCreate,
    handleEdit,
    handleDialogClose,
    handleDelete,
    createMutation,
    updateMutation,
    deleteMutation,
  } = useProviderActions();

  const [searchQuery, setSearchQuery] = useState('');

  // Filter providers
  const filteredProviders = useMemo(() => {
    if (!providers) return [];
    if (!searchQuery) return providers;

    const query = searchQuery.toLowerCase();
    return providers.filter(
      (provider) =>
        provider.name.toLowerCase().includes(query) ||
        provider.domain.toLowerCase().includes(query)
    );
  }, [providers, searchQuery]);

  // Separate builtin and custom providers
  const builtinProviders = filteredProviders.filter((p) => p.is_builtin);
  const customProviders = filteredProviders.filter((p) => !p.is_builtin);

  const handleSubmit = async (values: ProviderFormValues) => {
    if (editingProvider) {
      // Update
      await updateMutation.mutateAsync({
        provider_id: editingProvider.id,
        name: values.name,
        domain: values.domain,
        needs_waf_bypass: values.needs_waf_bypass,
        supports_check_in: values.supports_check_in,
        check_in_bugged: values.check_in_bugged,
        login_path: values.login_path || undefined,
        sign_in_path: values.sign_in_path || undefined,
        user_info_path: values.user_info_path || undefined,
        token_api_path: values.token_api_path || undefined,
        models_path: values.models_path || undefined,
        api_user_key: values.api_user_key || undefined,
      });
    } else {
      // Create
      await createMutation.mutateAsync({
        name: values.name,
        domain: values.domain,
        needs_waf_bypass: values.needs_waf_bypass,
        supports_check_in: values.supports_check_in,
        check_in_bugged: values.check_in_bugged,
        login_path: values.login_path || undefined,
        sign_in_path: values.sign_in_path || undefined,
        user_info_path: values.user_info_path || undefined,
        token_api_path: values.token_api_path || undefined,
        models_path: values.models_path || undefined,
        api_user_key: values.api_user_key || undefined,
      });
    }
  };

  return (
    <PageContainer
      title={t('providers.title', '中转站管理')}
      actions={
        <div className="flex items-center gap-3">
          {/* Search */}
          <div className="relative w-64">
            <Search className="absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-muted-foreground" />
            <Input
              placeholder={t('providers.searchPlaceholder', '搜索中转站名称或域名...')}
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="pl-8 h-9 bg-background shadow-sm border-border/50 text-sm"
            />
          </div>

          <div className="w-px h-6 bg-border" />

          {/* Add Provider Button */}
          <Button onClick={handleCreate} size="sm" className="shadow-sm">
            <Plus className="mr-2 h-4 w-4" />
            {t('providers.addButton', '添加中转站')}
          </Button>
        </div>
      }
    >
      {/* Loading State */}
      {isLoading && (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          {[1, 2, 3].map((i) => (
            <Skeleton key={i} className="h-48" />
          ))}
        </div>
      )}

      {/* Providers List */}
      {!isLoading && (
        <div className="space-y-8">
          {/* Builtin Providers */}
          {builtinProviders.length > 0 && (
            <div>
              <div className="flex items-center gap-2 mb-4">
                <Server className="h-5 w-5 text-muted-foreground" />
                <h2 className="text-lg font-semibold">{t('providers.builtinSection', '内置中转站')}</h2>
                <span className="text-sm text-muted-foreground">
                  ({builtinProviders.length})
                </span>
              </div>
              <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                {builtinProviders.map((provider) => (
                  <ProviderCard
                    key={provider.id}
                    provider={provider}
                    onEdit={handleEdit}
                    onDelete={handleDelete}
                    isDeleting={deleteMutation.isPending}
                  />
                ))}
              </div>
            </div>
          )}

          {/* Custom Providers */}
          {customProviders.length > 0 && (
            <div>
              <div className="flex items-center gap-2 mb-4">
                <Server className="h-5 w-5 text-muted-foreground" />
                <h2 className="text-lg font-semibold">{t('providers.customSection', '自定义中转站')}</h2>
                <span className="text-sm text-muted-foreground">
                  ({customProviders.length})
                </span>
              </div>
              <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                {customProviders.map((provider) => (
                  <ProviderCard
                    key={provider.id}
                    provider={provider}
                    onEdit={handleEdit}
                    onDelete={handleDelete}
                    isDeleting={deleteMutation.isPending}
                  />
                ))}
              </div>
            </div>
          )}

          {/* Empty State */}
          {filteredProviders.length === 0 && !isLoading && (
            <div className="text-center py-12">
              <Server className="h-12 w-12 mx-auto text-muted-foreground/50 mb-4" />
              <h3 className="text-lg font-medium mb-2">
                {searchQuery 
                  ? t('providers.noMatchFound', '未找到匹配的中转站')
                  : t('providers.noProviders', '暂无自定义中转站')}
              </h3>
              <p className="text-sm text-muted-foreground mb-4">
                {searchQuery
                  ? t('providers.tryDifferentSearch', '尝试使用其他关键词搜索')
                  : t('providers.addFirstProvider', '点击"添加中转站"按钮创建您的第一个自定义中转站')}
              </p>
              {!searchQuery && (
                <Button onClick={handleCreate} variant="outline">
                  <Plus className="h-4 w-4 mr-2" />
                  {t('providers.addFirstButton', '添加中转站')}
                </Button>
              )}
            </div>
          )}
        </div>
      )}

      {/* Provider Dialog */}
      <ProviderDialog
        open={dialogOpen}
        onOpenChange={handleDialogClose}
        mode={editingProvider ? 'edit' : 'create'}
        providerId={editingProvider?.id}
        defaultValues={
          editingProvider
            ? {
                name: editingProvider.name,
                domain: editingProvider.domain,
                needs_waf_bypass: editingProvider.needs_waf_bypass,
                supports_check_in: editingProvider.supports_check_in,
                check_in_bugged: editingProvider.check_in_bugged,
                login_path: editingProvider.login_path,
                sign_in_path: editingProvider.sign_in_path || undefined,
                user_info_path: editingProvider.user_info_path,
                token_api_path: editingProvider.token_api_path || undefined,
                models_path: editingProvider.models_path || undefined,
                api_user_key: editingProvider.api_user_key,
              }
            : undefined
        }
        onSubmit={handleSubmit}
        isSubmitting={createMutation.isPending || updateMutation.isPending}
      />
    </PageContainer>
  );
}
