import { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import { AccountSelector } from '@/components/token/AccountSelector';
import { TokenList } from '@/components/token/TokenList';
import { ConfigDialog } from '@/components/token/ConfigDialog';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
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
import { RefreshCw, Plus, Trash2, Server, XCircle } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';
import type { TokenDto, AccountDto, ProviderNode } from '@/types/token';

export function TokenManagerPage() {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [selectedAccount, setSelectedAccount] = useState<string | null>(null);
  const [configDialogOpen, setConfigDialogOpen] = useState(false);
  const [selectedToken, setSelectedToken] = useState<TokenDto | null>(null);

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
  const tokenProviders = accounts.filter(
    (acc) => acc.provider_id === 'anyrouter' || acc.provider_id === 'agentrouter'
  );

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

  const handleRefresh = async () => {
    if (selectedAccount) {
      try {
        await invoke('fetch_account_tokens', {
          accountId: selectedAccount,
          forceRefresh: true,
        });
        await refetch();
        toast.success(t('token.refreshSuccess', 'Tokens refreshed successfully'));
      } catch (error) {
        toast.error(t('token.refreshError', 'Failed to refresh tokens'));
        console.error(error);
      }
    }
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

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold">
            {t('token.title', 'Token Manager')}
          </h1>
          <p className="text-muted-foreground">
            {t('token.description', 'Manage API tokens for Claude Code and other AI tools')}
          </p>
        </div>
        <div className="flex items-center gap-2">
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
            disabled={!selectedAccount || isFetching}
            variant="outline"
            size="sm"
          >
            <RefreshCw className={`mr-2 h-4 w-4 ${isFetching ? 'animate-spin' : ''}`} />
            {t('common.refresh', 'Refresh')}
          </Button>
        </div>
      </div>

      {/* Account Selector */}
      <AccountSelector
        accounts={tokenProviders}
        selectedAccount={selectedAccount}
        onSelectAccount={setSelectedAccount}
      />

      {/* Token List */}
      {selectedAccount && (
        <TokenList
          tokens={tokens}
          isLoading={isLoading}
          onConfigureToken={handleConfigureToken}
        />
      )}

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
    </div>
  );
}
