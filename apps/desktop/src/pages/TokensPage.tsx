import { useState, useMemo } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import { IndependentKeyDialog } from '@/components/token/IndependentKeyDialog';
import { IndependentKeyConfigDialog } from '@/components/token/IndependentKeyConfigDialog';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Card } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Alert } from '@/components/ui/alert';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from '@/components/ui/alert-dialog';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import {
  Plus,
  Search,
  Key,
  Globe,
  Settings2,
  Edit,
  Trash2,
  Power,
  PowerOff,
  ArrowRight,
  Info,
  MoreVertical,
} from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';
import type { IndependentKeyDto } from '@/types/independentKey';
import { cn } from '@/lib/utils';
import { useNavigate } from 'react-router-dom';
import { motion, AnimatePresence } from 'framer-motion';

import { PageContainer } from '@/components/layout/PageContainer';
import { HeaderActions } from '@/components/layout/HeaderActions';
import { createFadeUpItem, createStaggerContainer } from '@/lib/motion';

type ProviderFilter = 'all' | 'openai' | 'anthropic' | 'custom';
type StatusFilter = 'all' | 'active' | 'inactive';

export function TokensPage() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [searchQuery, setSearchQuery] = useState('');
  const [providerFilter, setProviderFilter] = useState<ProviderFilter>('all');
  const [statusFilter, setStatusFilter] = useState<StatusFilter>('all');

  // Independent Key Dialog
  const [keyDialogOpen, setKeyDialogOpen] = useState(false);
  const [keyToEdit, setKeyToEdit] = useState<IndependentKeyDto | null>(null);

  // Handle dialog close - refetch data when dialog closes
  const handleKeyDialogChange = (open: boolean) => {
    setKeyDialogOpen(open);
    if (!open) {
      setTimeout(() => {
        refetch();
      }, 100);
    }
  };

  // Independent Key Config Dialog
  const [keyConfigDialogOpen, setKeyConfigDialogOpen] = useState(false);
  const [keyToConfig, setKeyToConfig] = useState<IndependentKeyDto | null>(null);
  const [configDefaultTool, setConfigDefaultTool] = useState<'claude' | 'codex'>('claude');

  // Delete confirmation
  const [deleteDialogOpen, setDeleteDialogOpen] = useState(false);
  const [keyToDelete, setKeyToDelete] = useState<IndependentKeyDto | null>(null);

  // Get independent keys
  const { data: independentKeys = [], refetch } = useQuery<IndependentKeyDto[]>({
    queryKey: ['independent-keys'],
    queryFn: async () => {
      const result = await invoke<IndependentKeyDto[]>('get_all_independent_keys');
      return result;
    },
    staleTime: 0,
    gcTime: 0,
    refetchOnMount: 'always',
    refetchOnWindowFocus: true,
    enabled: true,
  });

  // Filter keys
  const filteredKeys = useMemo(() => {
    let filtered = independentKeys;

    // Search filter
    if (searchQuery) {
      const query = searchQuery.toLowerCase();
      filtered = filtered.filter(
        (key) =>
          key.name.toLowerCase().includes(query) ||
          key.provider_type_display.toLowerCase().includes(query) ||
          key.base_url.toLowerCase().includes(query) ||
          (key.description && key.description.toLowerCase().includes(query))
      );
    }

    // Provider filter
    if (providerFilter !== 'all') {
      filtered = filtered.filter((key) => key.provider_type === providerFilter);
    }

    // Status filter
    if (statusFilter !== 'all') {
      const isActive = statusFilter === 'active';
      filtered = filtered.filter((key) => key.is_active === isActive);
    }

    return filtered;
  }, [independentKeys, searchQuery, providerFilter, statusFilter]);

  const toggleMutation = useMutation({
    mutationFn: (key: IndependentKeyDto) =>
      invoke('toggle_independent_key', {
        keyId: key.id,
        isActive: !key.is_active,
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['independent-keys'] });
      toast.success(t('independentKey.toggleSuccess', 'API Key status updated'));
    },
    onError: (error: Error) => {
      toast.error(t('independentKey.toggleError', 'Failed to toggle key: ') + error.message);
    },
  });

  const deleteMutation = useMutation({
    mutationFn: (keyId: number) => invoke('delete_independent_key', { keyId }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['independent-keys'] });
      toast.success(t('independentKey.deleted', 'API Key deleted successfully'));
      setDeleteDialogOpen(false);
      setKeyToDelete(null);
    },
    onError: (error: Error) => {
      toast.error(t('independentKey.deleteError', 'Failed to delete key: ') + error.message);
    },
  });

  const handleAddKey = () => {
    setKeyToEdit(null);
    setKeyDialogOpen(true);
  };

  const handleEditKey = (key: IndependentKeyDto) => {
    setKeyToEdit(key);
    setKeyDialogOpen(true);
  };

  const handleConfigureKey = (key: IndependentKeyDto, tool: 'claude' | 'codex' = 'claude') => {
    setKeyToConfig(key);
    setConfigDefaultTool(tool);
    setKeyConfigDialogOpen(true);
  };

  const handleDeleteKey = (key: IndependentKeyDto) => {
    setKeyToDelete(key);
    setDeleteDialogOpen(true);
  };

  const handleToggleKey = (key: IndependentKeyDto) => {
    toggleMutation.mutate(key);
  };

  const containerVariants = createStaggerContainer({ staggerChildren: 0.05 });
  const itemVariants = createFadeUpItem({ y: 20, scale: 1 });

  return (
    <PageContainer
      title={t('token.title', 'API Key')}
      actions={
        <HeaderActions className="gap-2">
          {/* Search */}
          <div className="relative w-48 lg:w-64 transition-all duration-200">
            <Search className="absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-muted-foreground pointer-events-none" />
            <Input
              placeholder={t('token.searchPlaceholder', 'Search...')}
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="pl-8 h-8 bg-background/50 hover:bg-background focus:bg-background border-border/50 text-xs"
            />
          </div>

          {/* Provider Filter */}
          <Select value={providerFilter} onValueChange={(v) => setProviderFilter(v as ProviderFilter)}>
            <SelectTrigger className="w-[110px] h-8 text-xs border-border/50 bg-background/50">
              <SelectValue placeholder={t('token.filterProvider', 'Provider')} />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">{t('common.all', 'All')}</SelectItem>
              <SelectItem value="openai">{t('providers.openai', 'OpenAI')}</SelectItem>
              <SelectItem value="anthropic">{t('providers.anthropic', 'Anthropic')}</SelectItem>
              <SelectItem value="custom">{t('token.customProvider', 'Custom')}</SelectItem>
            </SelectContent>
          </Select>

          {/* Status Filter */}
          <Select value={statusFilter} onValueChange={(v) => setStatusFilter(v as StatusFilter)}>
            <SelectTrigger className="w-[100px] h-8 text-xs border-border/50 bg-background/50">
              <SelectValue placeholder={t('token.filterStatus', 'Status')} />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">{t('common.all', 'All')}</SelectItem>
              <SelectItem value="active">{t('common.active', 'Active')}</SelectItem>
              <SelectItem value="inactive">{t('common.inactive', 'Inactive')}</SelectItem>
            </SelectContent>
          </Select>

          {/* Add Button */}
          <Button
            size="sm"
            onClick={handleAddKey}
            className="shadow-sm ml-2"
          >
            <Plus className="mr-2 h-4 w-4" />
            {t('token.addKey', 'Add API Key')}
          </Button>
        </HeaderActions>
      }
    >
      <div className="space-y-5">
        {/* Info Alert */}
        <Alert className="border-blue-200 dark:border-blue-800 bg-blue-50 dark:bg-blue-950/30">
          <div className="flex items-center gap-3">
            <Info className="h-4 w-4 text-blue-600 dark:text-blue-400 shrink-0" />
            <div className="flex items-center justify-between gap-4 flex-1 min-w-0">
              <span className="text-sm text-blue-900 dark:text-blue-100">
                {t(
                  'token.relayTokensHint',
                  'Relay API Key Configuration: Go to Account Management → Account Details → Configure'
                )}
              </span>
              <Button
                variant="ghost"
                size="sm"
                className="h-7 px-3 text-xs font-medium text-blue-600 dark:text-blue-400 hover:text-blue-700 dark:hover:text-blue-300 hover:bg-blue-100 dark:hover:bg-blue-900/50 shrink-0"
                onClick={() => navigate('/accounts')}
              >
                {t('token.goToAccounts', 'Go to Accounts')}
                <ArrowRight className="ml-1.5 h-3 w-3" />
              </Button>
            </div>
          </div>
        </Alert>

        {/* Keys List - Grid Layout */}
        <AnimatePresence mode="wait">
          {filteredKeys.length === 0 ? (
            <motion.div
              key="empty-state"
              initial={{ opacity: 0, scale: 0.95 }}
              animate={{ opacity: 1, scale: 1 }}
              exit={{ opacity: 0, scale: 0.95 }}
              transition={{ duration: 0.2 }}
              className="text-center py-12"
            >
              <Key className="h-12 w-12 mx-auto text-muted-foreground/50 mb-4" />
              <h3 className="text-lg font-medium mb-2">
                {searchQuery || providerFilter !== 'all' || statusFilter !== 'all'
                  ? t('token.noKeysFound', 'No API keys found')
                  : t('token.noKeysYet', 'No API keys yet')}
              </h3>
              <p className="text-sm text-muted-foreground mb-4">
                {searchQuery || providerFilter !== 'all' || statusFilter !== 'all'
                  ? t('token.tryDifferentFilters', 'Try adjusting your filters')
                  : t('token.addFirstKey', 'Add your first API key to get started')}
              </p>
              {!searchQuery && providerFilter === 'all' && statusFilter === 'all' && (
                <Button onClick={handleAddKey} variant="outline">
                  <Plus className="mr-2 h-4 w-4" />
                  {t('token.addKey', 'Add API Key')}
                </Button>
              )}
            </motion.div>
          ) : (
            <motion.div
              key="keys-grid"
              variants={containerVariants}
              initial="hidden"
              animate="show"
              exit={{ opacity: 0 }}
              className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4"
            >
              {filteredKeys.map((key) => (
                <motion.div key={key.id} variants={itemVariants} layout>
                  <Card
                    className={cn(
                      'group relative overflow-hidden transition-all duration-200',
                      key.is_active
                        ? 'border-border/60 bg-card hover:shadow-md hover:scale-[1.01] active:scale-[0.99] cursor-pointer'
                        : 'border-border/40 bg-muted/20 opacity-70 grayscale-[0.5] hover:opacity-100 hover:grayscale-0 cursor-default'
                    )}
                  >
                    <div className="p-4 space-y-3">
                      {/* Header */}
                      <div className="flex items-start justify-between gap-3">
                        <div className="flex-1 min-w-0">
                          <div className="flex items-center gap-2 mb-1">
                            <h3 className="text-base font-semibold text-foreground truncate leading-tight">
                              {key.name}
                            </h3>
                            {key.is_active ? (
                              <span className="relative flex h-2 w-2">
                                <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-green-400 opacity-75"></span>
                                <span className="relative inline-flex rounded-full h-2 w-2 bg-green-500"></span>
                              </span>
                            ) : (
                              <div className="h-2 w-2 rounded-full bg-muted-foreground/30" />
                            )}
                          </div>
                          <Badge variant="secondary" className="text-[10px] px-1.5 py-0 h-5 font-normal">
                            {key.provider_type_display}
                          </Badge>
                        </div>

                        <DropdownMenu>
                          <DropdownMenuTrigger asChild>
                            <Button
                              variant="ghost"
                              size="icon-sm"
                              className="h-8 w-8 -mr-2 -mt-2 opacity-60 hover:opacity-100 hover:bg-muted"
                            >
                              <MoreVertical className="h-4 w-4" />
                            </Button>
                          </DropdownMenuTrigger>
                          <DropdownMenuContent align="end" className="w-40">
                            <DropdownMenuItem onClick={() => handleEditKey(key)}>
                              <Edit className="mr-2 h-4 w-4" />
                              {t('common.edit', 'Edit')}
                            </DropdownMenuItem>
                            <DropdownMenuItem onClick={() => handleToggleKey(key)}>
                              {key.is_active ? (
                                <>
                                  <PowerOff className="mr-2 h-4 w-4" />
                                  {t('common.disable', 'Disable')}
                                </>
                              ) : (
                                <>
                                  <Power className="mr-2 h-4 w-4" />
                                  {t('common.enable', 'Enable')}
                                </>
                              )}
                            </DropdownMenuItem>
                            <DropdownMenuSeparator />
                            <DropdownMenuItem
                              onClick={() => handleDeleteKey(key)}
                              className="text-destructive focus:text-destructive"
                            >
                              <Trash2 className="mr-2 h-4 w-4" />
                              {t('common.delete', 'Delete')}
                            </DropdownMenuItem>
                          </DropdownMenuContent>
                        </DropdownMenu>
                      </div>

                      {/* Info Grid */}
                      <div className="space-y-2 pt-1">
                        <div className="flex items-center gap-2 text-xs text-muted-foreground bg-muted/30 p-1.5 rounded-md">
                          <Key className="h-3.5 w-3.5 shrink-0" />
                          <code className="font-mono truncate select-all">{key.masked_key}</code>
                        </div>

                        <div className="flex items-center gap-2 text-xs text-muted-foreground px-1.5">
                          <Globe className="h-3.5 w-3.5 shrink-0" />
                          <span className="truncate" title={key.base_url}>
                            {key.base_url}
                          </span>
                        </div>
                      </div>

                      {/* Action Button */}
                      {key.is_active && (
                        <div className="pt-2">
                          <Button
                            variant="outline"
                            size="sm"
                            className="w-full text-xs font-medium rounded-lg shadow-sm bg-gradient-to-r from-background/80 to-background/50 hover:from-primary/10 hover:to-primary/5 hover:text-primary hover:border-primary/30 transition-all duration-200"
                            onClick={() => handleConfigureKey(key, 'claude')}
                          >
                            <Settings2 className="mr-2 h-3.5 w-3.5" />
                            {t('token.configureAI', 'Configure AI Tool')}
                          </Button>
                        </div>
                      )}
                    </div>
                  </Card>
                </motion.div>
              ))}
            </motion.div>
          )}
        </AnimatePresence>
      </div>

      {/* Independent Key Dialog */}
      <IndependentKeyDialog
        open={keyDialogOpen}
        onOpenChange={handleKeyDialogChange}
        keyToEdit={keyToEdit}
      />

      {/* Independent Key Config Dialog */}
      <IndependentKeyConfigDialog
        open={keyConfigDialogOpen}
        onOpenChange={setKeyConfigDialogOpen}
        keyData={keyToConfig}
        defaultTool={configDefaultTool}
      />

      {/* Delete Confirmation Dialog */}
      <AlertDialog open={deleteDialogOpen} onOpenChange={setDeleteDialogOpen}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>
              {t('independentKey.deleteConfirmTitle', 'Delete API Key?')}
            </AlertDialogTitle>
            <AlertDialogDescription>
              {keyToDelete &&
                t(
                  'independentKey.deleteConfirmDescription',
                  'Are you sure you want to delete "{name}"? This action cannot be undone.',
                  { name: keyToDelete.name }
                )}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>{t('common.cancel', 'Cancel')}</AlertDialogCancel>
            <AlertDialogAction
              onClick={() => keyToDelete && deleteMutation.mutate(keyToDelete.id)}
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
              disabled={deleteMutation.isPending}
            >
              {deleteMutation.isPending ? t('common.deleting', 'Deleting...') : t('common.delete', 'Delete')}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </PageContainer>
  );
}
