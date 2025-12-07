import { useState, useMemo } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import { TokenList } from '@/components/token/TokenList';
import { ConfigDialog } from '@/components/token/ConfigDialog';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Card } from '@/components/ui/card';
import { ScrollArea } from '@/components/ui/scroll-area';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { RefreshCw, Plus, Trash2, Server, XCircle, Search, Layers, Box } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';
import type { TokenDto, AccountDto, ProviderNode } from '@/types/token';
import { cn } from '@/lib/utils';

import { PageContainer } from '@/components/layout/PageContainer';
import { SidebarPageLayout } from '@/components/layout/SidebarPageLayout';

export function TokenManagerPage() {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [selectedAccount, setSelectedAccount] = useState<string | null>(null);
  const [configDialogOpen, setConfigDialogOpen] = useState(false);
  const [selectedToken, setSelectedToken] = useState<TokenDto | null>(null);
  const [searchQuery, setSearchQuery] = useState('');

  // Node management state
  const [nodeDialogOpen, setNodeDialogOpen] = useState(false);
  const [selectedProvider, setSelectedProvider] = useState<string>('anyrouter');
  const [newNodeName, setNewNodeName] = useState('');
  const [newNodeUrl, setNewNodeUrl] = useState('');

  // Clear config state
  const [clearConfigDialogOpen, setClearConfigDialogOpen] = useState(false);
  const [selectedToolToClear, setSelectedToolToClear] = useState<string>('claude');

  // Get accounts
  const { data: accounts = [] } = useQuery<AccountDto[]>({
    queryKey: ['accounts'],
    queryFn: () => invoke('get_all_accounts', { enabledOnly: false }),
  });

  // Filter to AnyRouter/AgentRouter accounts
  const tokenProviders = useMemo(() => {
    const filtered = accounts.filter(
      (acc) => acc.provider_id === 'anyrouter' || acc.provider_id === 'agentrouter'
    );
    if (searchQuery) {
      const query = searchQuery.toLowerCase();
      return filtered.filter(
        (acc) =>
          acc.name.toLowerCase().includes(query) ||
          acc.provider_name.toLowerCase().includes(query)
      );
    }
    return filtered;
  }, [accounts, searchQuery]);

  // Group accounts by provider
  const providerGroups = useMemo(() => {
    const groupsMap = new Map<string, { providerId: string; providerName: string; accounts: AccountDto[] }>();

    tokenProviders.forEach((account) => {
      if (!groupsMap.has(account.provider_id)) {
        groupsMap.set(account.provider_id, {
          providerId: account.provider_id,
          providerName: account.provider_name,
          accounts: [],
        });
      }
      groupsMap.get(account.provider_id)!.accounts.push(account);
    });

    return Array.from(groupsMap.values());
  }, [tokenProviders]);

  // Get tokens for selected account
  const {
    data: tokens = [],
    isLoading,
    refetch,
    isFetching,
  } = useQuery<TokenDto[]>({
    queryKey: ['tokens', selectedAccount],
    queryFn: () =>
      invoke<TokenDto[]>('fetch_account_tokens', {
        accountId: selectedAccount!,
        forceRefresh: false,
      }),
    enabled: !!selectedAccount,
    staleTime: 0,
    gcTime: 0,
  });

  // Get nodes for selected provider
  const { data: nodes = [], refetch: refetchNodes } = useQuery<ProviderNode[]>({
    queryKey: ['provider-nodes', selectedProvider],
    queryFn: () => invoke('get_provider_nodes', { providerId: selectedProvider }),
    enabled: nodeDialogOpen,
  });

  // Add node mutation
  const addNodeMutation = useMutation({
    mutationFn: () =>
      invoke<string>('add_custom_node', {
        providerId: selectedProvider,
        name: newNodeName,
        baseUrl: newNodeUrl,
      }),
    onSuccess: (message) => {
      toast.success(message);
      setNewNodeName('');
      setNewNodeUrl('');
      refetchNodes();
      // Also invalidate provider-nodes queries for ConfigDialog
      queryClient.invalidateQueries({ queryKey: ['provider-nodes'] });
    },
    onError: (error: Error) => {
      toast.error(t('token.addNodeError', 'Failed to add node: ') + error.message);
    },
  });

  // Delete node mutation
  const deleteNodeMutation = useMutation({
    mutationFn: (nodeId: number) => invoke<string>('delete_custom_node', { nodeId }),
    onSuccess: () => {
      toast.success(t('token.nodeDeleted', 'Node deleted successfully'));
      refetchNodes();
      queryClient.invalidateQueries({ queryKey: ['provider-nodes'] });
    },
    onError: (error: Error) => {
      toast.error(t('token.deleteNodeError', 'Failed to delete node: ') + error.message);
    },
  });

  // Clear config mutation
  const clearConfigMutation = useMutation({
    mutationFn: (tool: string) => {
      if (tool === 'claude') {
        return invoke<string>('clear_claude_global');
      } else {
        return invoke<string>('clear_codex_global');
      }
    },
    onSuccess: (message) => {
      toast.success(message);
      setClearConfigDialogOpen(false);
    },
    onError: (error: Error) => {
      toast.error(t('token.clearConfigError', 'Failed to clear config: ') + error.message);
    },
  });

  // Refresh mutation
  const refreshMutation = useMutation({
    mutationFn: async () => {
      if (!selectedAccount) throw new Error('No account selected');
      await invoke('fetch_account_tokens', {
        accountId: selectedAccount,
        forceRefresh: true,
      });
    },
    onSuccess: () => {
      refetch();
      toast.success(t('token.refreshSuccess', 'Tokens refreshed successfully'));
    },
    onError: (error: Error) => {
      console.error(error);
      toast.error(error.message || t('token.refreshError', 'Failed to refresh tokens'));
    },
  });

  const handleRefresh = () => {
    refreshMutation.mutate();
  };

  const handleConfigureToken = (token: TokenDto) => {
    setSelectedToken(token);
    setConfigDialogOpen(true);
  };

  const handleDeleteNode = (nodeId: string) => {
    if (nodeId.startsWith('custom_')) {
      const id = parseInt(nodeId.substring(7));
      deleteNodeMutation.mutate(id);
    }
  };

  const sidebarContent = (
    <>
      <div className="relative">
        <Search className="absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-muted-foreground" />
        <Input
          placeholder={t('accounts.searchPlaceholder')}
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          className="pl-8 h-9 bg-background shadow-sm border-border/50 text-sm"
        />
      </div>
      
      <Card className="flex-1 border-border/50 shadow-sm bg-background/50 backdrop-blur-sm overflow-hidden">
        <ScrollArea className="h-full">
          <div className="p-2 space-y-1">
            {providerGroups.length === 0 ? (
              <div className="p-4 text-center text-sm text-muted-foreground">
                {t('token.noAccounts', 'No accounts found')}
              </div>
            ) : (
              providerGroups.map((group) => (
                <div key={group.providerId} className="mt-4 first:mt-2">
                  <div className="px-3 mb-1 text-xs font-semibold text-muted-foreground/50 tracking-wider flex items-center gap-2">
                    <Box className="h-3 w-3" />
                    {group.providerName}
                  </div>
                  <div className="space-y-1">
                    {group.accounts.map((account) => {
                      const isActive = selectedAccount === account.id;
                      return (
                        <button
                          key={account.id}
                          onClick={() => setSelectedAccount(account.id)}
                          title={account.name}
                          className={cn(
                            "w-full flex items-center justify-between px-3 py-2 rounded-lg text-sm font-medium transition-colors",
                            isActive 
                              ? "bg-primary text-primary-foreground shadow-sm" 
                              : "text-muted-foreground hover:bg-muted hover:text-foreground"
                          )}
                        >
                          <span className="truncate">{account.name}</span>
                        </button>
                      );
                    })}
                  </div>
                </div>
              ))
            )}
          </div>
        </ScrollArea>
      </Card>
    </>
  );

  return (
    <PageContainer 
      className="h-full overflow-hidden"
      title={t('token.title', 'Token Manager')}
      actions={
        <>
          <Button
            variant="outline"
            size="sm"
            onClick={() => setClearConfigDialogOpen(true)}
          >
            <XCircle className="mr-2 h-4 w-4" />
            {t('token.clearConfig', 'Clear Config')}
          </Button>
          <Button
            variant="outline"
            size="sm"
            onClick={() => setNodeDialogOpen(true)}
          >
            <Server className="mr-2 h-4 w-4" />
            {t('token.manageNodes', 'Manage Nodes')}
          </Button>
          <Button
            onClick={handleRefresh}
            disabled={!selectedAccount || refreshMutation.isPending}
            variant="outline"
            size="sm"
          >
            <RefreshCw className={`mr-2 h-4 w-4 ${refreshMutation.isPending ? 'animate-spin' : ''}`} />
            {t('common.refresh', 'Refresh')}
          </Button>
        </>
      }
    >
      <SidebarPageLayout sidebar={sidebarContent}>
        <div className="animate-in fade-in slide-in-from-bottom-4 duration-500">
          {selectedAccount ? (
            <TokenList
              tokens={tokens}
              isLoading={isLoading}
              onConfigureToken={handleConfigureToken}
            />
          ) : (
            <Card className="border-dashed bg-muted/30">
              <div className="flex flex-col items-center justify-center h-64 gap-4">
                <p className="text-muted-foreground">{t('token.selectAccountPlaceholder', 'Select an account to view tokens')}</p>
              </div>
            </Card>
          )}
        </div>
      </SidebarPageLayout>

      {/* Config Dialog */}
      <ConfigDialog
        open={configDialogOpen}
        onOpenChange={setConfigDialogOpen}
        token={selectedToken}
        account={tokenProviders.find((acc) => acc.id === selectedAccount) ?? null}
      />

      {/* Node Management Dialog */}
      <Dialog open={nodeDialogOpen} onOpenChange={setNodeDialogOpen}>
        <DialogContent className="max-w-lg">
          <DialogHeader>
            <DialogTitle>{t('token.nodeManagement', 'Node Management')}</DialogTitle>
            <DialogDescription>
              {t('token.nodeManagementDesc', 'Manage custom nodes for each provider.')}
            </DialogDescription>
          </DialogHeader>

          <div className="space-y-4">
            {/* Provider Selector */}
            <div className="space-y-2">
              <Label>{t('token.selectProvider', 'Provider')}</Label>
              <Select value={selectedProvider} onValueChange={setSelectedProvider}>
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="anyrouter">AnyRouter</SelectItem>
                  <SelectItem value="agentrouter">AgentRouter</SelectItem>
                </SelectContent>
              </Select>
            </div>

            {/* Add Node Form */}
            <div className="space-y-3 p-3 border rounded-lg bg-muted/30">
              <Label className="text-sm font-medium">{t('token.addNewNode', 'Add New Node')}</Label>
              <div className="grid grid-cols-2 gap-2">
                <Input
                  placeholder={t('token.nodeName', 'Node Name')}
                  value={newNodeName}
                  onChange={(e) => setNewNodeName(e.target.value)}
                />
                <Input
                  placeholder="https://example.com"
                  value={newNodeUrl}
                  onChange={(e) => setNewNodeUrl(e.target.value)}
                />
              </div>
              <Button
                size="sm"
                onClick={() => addNodeMutation.mutate()}
                disabled={!newNodeName || !newNodeUrl || addNodeMutation.isPending}
                className="w-full"
              >
                <Plus className="h-4 w-4 mr-1" />
                {addNodeMutation.isPending ? t('common.adding', 'Adding...') : t('common.add', 'Add')}
              </Button>
            </div>

            {/* Node List */}
            <div className="space-y-2 max-h-[300px] overflow-y-auto">
              <Label className="text-sm font-medium">{t('token.existingNodes', 'Existing Nodes')}</Label>
              {nodes.map((node) => (
                <div
                  key={node.id}
                  className="flex items-center justify-between p-2 border rounded bg-background"
                >
                  <div className="flex-1 min-w-0 mr-2">
                    <div className="flex items-center gap-2">
                      <span className="font-medium text-sm">{node.name}</span>
                      {node.id.startsWith('custom_') && (
                        <span className="text-xs px-1.5 py-0.5 rounded bg-blue-100 text-blue-700 dark:bg-blue-900 dark:text-blue-300">
                          {t('token.customNode', 'Custom')}
                        </span>
                      )}
                    </div>
                    <span className="text-xs text-muted-foreground truncate block">
                      {node.base_url}
                    </span>
                  </div>
                  {node.id.startsWith('custom_') && (
                    <Button
                      variant="ghost"
                      size="sm"
                      className="h-8 w-8 p-0 text-destructive hover:text-destructive"
                      onClick={() => handleDeleteNode(node.id)}
                      disabled={deleteNodeMutation.isPending}
                    >
                      <Trash2 className="h-4 w-4" />
                    </Button>
                  )}
                </div>
              ))}
              {nodes.length === 0 && (
                <p className="text-sm text-muted-foreground text-center py-4">
                  {t('token.noNodes', 'No nodes available')}
                </p>
              )}
            </div>
          </div>
        </DialogContent>
      </Dialog>

      {/* Clear Config Dialog */}
      <Dialog open={clearConfigDialogOpen} onOpenChange={setClearConfigDialogOpen}>
        <DialogContent className="max-w-sm">
          <DialogHeader>
            <DialogTitle>{t('token.clearConfigTitle', 'Clear Global Configuration')}</DialogTitle>
            <DialogDescription>
              {t('token.clearConfigDesc', 'This will only remove configurations managed by NeuraDock, preserving your other settings.')}
            </DialogDescription>
          </DialogHeader>

          <div className="space-y-4">
            <div className="space-y-2">
              <Label>{t('token.selectTool', 'Select AI Tool')}</Label>
              <Select value={selectedToolToClear} onValueChange={setSelectedToolToClear}>
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="claude">Claude Code</SelectItem>
                  <SelectItem value="codex">Codex</SelectItem>
                </SelectContent>
              </Select>
            </div>

            <div className="text-sm text-muted-foreground">
              {selectedToolToClear === 'claude' ? (
                <p>{t('token.clearClaudeDesc', 'Clears ANTHROPIC_API_KEY, ANTHROPIC_BASE_URL and other managed settings from ~/.claude/settings.json')}</p>
              ) : (
                <p>{t('token.clearCodexDesc', 'Removes ~/.codex/auth.json file')}</p>
              )}
            </div>

            <div className="flex justify-end gap-2">
              <Button
                variant="outline"
                onClick={() => setClearConfigDialogOpen(false)}
              >
                {t('common.cancel', 'Cancel')}
              </Button>
              <Button
                variant="destructive"
                onClick={() => clearConfigMutation.mutate(selectedToolToClear)}
                disabled={clearConfigMutation.isPending}
              >
                {clearConfigMutation.isPending
                  ? t('common.clearing', 'Clearing...')
                  : t('token.clearConfigBtn', 'Clear Configuration')}
              </Button>
            </div>
          </div>
        </DialogContent>
      </Dialog>
    </PageContainer>
  );
}
