import { useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import { toast } from 'sonner';
import { Globe, Plus, Trash2 } from 'lucide-react';

import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Separator } from '@/components/ui/separator';
import type { ProviderNode } from '@/types/token';
import type { ProviderDto } from '@/hooks/useProviders';

interface ProviderNodesDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  provider: ProviderDto | null;
}

function isHttpUrl(url: string): boolean {
  try {
    const parsed = new URL(url);
    return parsed.protocol === 'http:' || parsed.protocol === 'https:';
  } catch {
    return false;
  }
}

function parseCustomNodeId(nodeId: string): number | null {
  if (!nodeId.startsWith('custom_')) return null;
  const raw = nodeId.slice('custom_'.length);
  const numeric = Number(raw);
  return Number.isFinite(numeric) ? numeric : null;
}

export function ProviderNodesDialog({
  open,
  onOpenChange,
  provider,
}: ProviderNodesDialogProps) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();

  const providerId = provider?.id ?? '';

  const { data: nodes = [], isFetching } = useQuery<ProviderNode[]>({
    queryKey: ['provider-nodes', providerId],
    queryFn: () => invoke('get_provider_nodes', { providerId }),
    enabled: open && Boolean(providerId),
  });

  const baseNode = nodes[0] ?? null;
  const customNodes = useMemo(() => nodes.slice(1), [nodes]);

  const [name, setName] = useState('');
  const [baseUrl, setBaseUrl] = useState('');

  const addMutation = useMutation({
    mutationFn: async () => {
      const trimmedName = name.trim();
      const trimmedUrl = baseUrl.trim();
      if (!trimmedName) {
        throw new Error(t('common.required', 'Required'));
      }
      if (!isHttpUrl(trimmedUrl)) {
        throw new Error(t('providerDialog.fields.domain.invalidFormat', 'Invalid URL format'));
      }
      return await invoke<string>('add_custom_node', {
        providerId,
        name: trimmedName,
        baseUrl: trimmedUrl,
      });
    },
    onSuccess: (message) => {
      toast.success(message || t('common.success', 'Success'));
      setName('');
      setBaseUrl('');
      queryClient.invalidateQueries({ queryKey: ['provider-nodes', providerId] });
    },
    onError: (error: any) => {
      toast.error(error?.message || t('common.error', 'Error'));
    },
  });

  const deleteMutation = useMutation({
    mutationFn: async (nodeId: string) => {
      const numericId = parseCustomNodeId(nodeId);
      if (numericId == null) {
        throw new Error(t('common.error', 'Error'));
      }
      return await invoke<string>('delete_custom_node', { nodeId: numericId });
    },
    onSuccess: (message) => {
      toast.success(message || t('common.success', 'Success'));
      queryClient.invalidateQueries({ queryKey: ['provider-nodes', providerId] });
    },
    onError: (error: any) => {
      toast.error(error?.message || t('common.error', 'Error'));
    },
  });

  const busy = addMutation.isPending || deleteMutation.isPending;
  const inputsDisabled = !provider || busy;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-xl">
        <DialogHeader>
          <DialogTitle>
            {t('token.configDialog.manageNodes', 'Manage Nodes')} ·{' '}
            {provider?.name ?? ''}
          </DialogTitle>
          <DialogDescription>
            {t('token.nodeManagementDesc', 'Manage API endpoints for this provider.')}
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-6">
          <div className="space-y-3">
            <div className="text-sm font-medium">{t('common.list', 'List')}</div>

            <div className="rounded-xl border bg-muted/20">
              <div className="p-3 space-y-2">
                {isFetching && (
                  <div className="px-3 py-6 text-center text-sm text-muted-foreground">
                    {t('common.loading', 'Loading...')}
                  </div>
                )}
                {baseNode && (
                  <div className="flex items-center justify-between gap-3 rounded-lg bg-background px-3 py-2">
                    <div className="min-w-0">
                      <div className="text-sm font-medium truncate">
                        {baseNode.name}{' '}
                        <span className="text-xs text-muted-foreground">
                          ({t('common.default', 'Default')})
                        </span>
                      </div>
                      <div className="text-xs text-muted-foreground truncate">
                        {baseNode.base_url}
                      </div>
                    </div>
                    <Globe className="h-4 w-4 text-muted-foreground shrink-0" />
                  </div>
                )}

                {customNodes.length === 0 && (
                  <div className="px-3 py-6 text-center text-sm text-muted-foreground">
                    {t('providers.noCustomNodes', 'No custom endpoints yet.')}
                  </div>
                )}

                {customNodes.map((node) => (
                  <div
                    key={node.id}
                    className="flex items-center justify-between gap-3 rounded-lg bg-background px-3 py-2"
                  >
                    <div className="min-w-0">
                      <div className="text-sm font-medium truncate">{node.name}</div>
                      <div className="text-xs text-muted-foreground truncate">
                        {node.base_url}
                      </div>
                    </div>

                    <Button
                      variant="ghost"
                      size="icon"
                      className="h-8 w-8 text-destructive hover:text-destructive"
                      disabled={inputsDisabled}
                      onClick={() => deleteMutation.mutate(node.id)}
                      title={t('common.delete')}
                    >
                      <Trash2 className="h-4 w-4" />
                    </Button>
                  </div>
                ))}
              </div>
            </div>
          </div>

          <Separator />

          <div className="space-y-4">
            <div className="text-sm font-medium">
              {t('providers.addNode', 'Add Endpoint')}
            </div>

            <div className="grid gap-4">
              <div className="space-y-2">
                <Label className="text-xs">{t('common.name')}</Label>
                <Input
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                  placeholder={t('providers.nodeNamePlaceholder', 'e.g. Shanghai')}
                  disabled={inputsDisabled}
                />
              </div>

              <div className="space-y-2">
                <Label className="text-xs">{t('providers.endpoint', 'Endpoint')}</Label>
                <Input
                  value={baseUrl}
                  onChange={(e) => setBaseUrl(e.target.value)}
                  placeholder="https://..."
                  disabled={inputsDisabled}
                />
              </div>
            </div>

            <div className="flex justify-end gap-2">
              <Button
                variant="outline"
                onClick={() => onOpenChange(false)}
                disabled={busy}
              >
                {t('common.cancel')}
              </Button>
              <Button
                onClick={() => addMutation.mutate()}
                disabled={inputsDisabled || !name.trim() || !baseUrl.trim()}
              >
                <Plus className="mr-2 h-4 w-4" />
                {t('common.add', 'Add')}
              </Button>
            </div>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}
