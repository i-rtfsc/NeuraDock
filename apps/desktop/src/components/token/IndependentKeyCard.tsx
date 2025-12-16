import { useState } from 'react';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import { Card } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
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
import { MoreVertical, Key, Globe, Calendar, Edit, Trash2, Power, PowerOff } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';
import type { IndependentKeyDto } from '@/types/independentKey';
import { cn } from '@/lib/utils';

interface IndependentKeyCardProps {
  keyData: IndependentKeyDto;
  onEdit: (key: IndependentKeyDto) => void;
  onConfigure: (key: IndependentKeyDto) => void;
}

export function IndependentKeyCard({ keyData, onEdit, onConfigure }: IndependentKeyCardProps) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [deleteDialogOpen, setDeleteDialogOpen] = useState(false);

  const toggleMutation = useMutation({
    mutationFn: () =>
      invoke('toggle_independent_key', {
        keyId: keyData.id,
        isActive: !keyData.is_active,
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['independent-keys'] });
      toast.success(
        keyData.is_active
          ? t('independentKey.disabled', 'API Key disabled')
          : t('independentKey.enabled', 'API Key enabled')
      );
    },
    onError: (error: Error) => {
      toast.error(t('independentKey.toggleError', 'Failed to toggle key: ') + error.message);
    },
  });

  const deleteMutation = useMutation({
    mutationFn: () => invoke('delete_independent_key', { keyId: keyData.id }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['independent-keys'] });
      toast.success(t('independentKey.deleted', 'API Key deleted successfully'));
      setDeleteDialogOpen(false);
    },
    onError: (error: Error) => {
      toast.error(t('independentKey.deleteError', 'Failed to delete key: ') + error.message);
    },
  });

  const handleToggle = () => {
    toggleMutation.mutate();
  };

  const handleDelete = () => {
    deleteMutation.mutate();
  };

  const formatDate = (dateStr: string) => {
    try {
      return new Date(dateStr).toLocaleDateString();
    } catch {
      return dateStr;
    }
  };

  return (
    <>
      <Card className={cn(
        "p-6 border-border/50 shadow-sm bg-card/50 backdrop-blur-sm transition-all duration-base ease-smooth",
        keyData.is_active ? "hover:shadow-hover-md hover:border-border" : "opacity-60"
      )}>
        <div className="space-y-4">
          {/* Header */}
          <div className="flex items-start justify-between">
            <div className="flex-1">
              <div className="flex items-center gap-3 mb-2">
                <h3 className="text-lg font-semibold">{keyData.name}</h3>
                <Badge variant={keyData.is_active ? "default" : "secondary"} className="text-xs">
                  {keyData.is_active
                    ? t('common.active', 'Active')
                    : t('common.inactive', 'Inactive')}
                </Badge>
              </div>
              <p className="text-sm text-muted-foreground">
                {keyData.provider_type_display}
                {keyData.custom_provider_name && ` - ${keyData.custom_provider_name}`}
              </p>
            </div>

            {/* Actions Menu */}
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button variant="ghost" size="icon-sm">
                  <MoreVertical className="h-4 w-4" />
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end">
                <DropdownMenuItem onClick={() => onEdit(keyData)}>
                  <Edit className="mr-2 h-4 w-4" />
                  {t('common.edit', 'Edit')}
                </DropdownMenuItem>
                <DropdownMenuItem onClick={handleToggle}>
                  {keyData.is_active ? (
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
                  onClick={() => setDeleteDialogOpen(true)}
                  className="text-destructive focus:text-destructive"
                >
                  <Trash2 className="mr-2 h-4 w-4" />
                  {t('common.delete', 'Delete')}
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          </div>

          {/* Key Details */}
          <div className="space-y-3">
            {/* API Key */}
            <div className="flex items-center gap-2 text-sm">
              <Key className="h-4 w-4 text-muted-foreground flex-shrink-0" />
              <span className="text-muted-foreground">{t('independentKey.apiKey', 'API Key')}:</span>
              <code className="flex-1 px-2 py-1 bg-muted rounded text-xs font-mono">
                {keyData.masked_key}
              </code>
            </div>

            {/* Base URL */}
            <div className="flex items-center gap-2 text-sm">
              <Globe className="h-4 w-4 text-muted-foreground flex-shrink-0" />
              <span className="text-muted-foreground">{t('independentKey.baseUrl', 'Base URL')}:</span>
              <span className="flex-1 text-xs text-foreground/80 truncate">{keyData.base_url}</span>
            </div>

            {/* Organization ID (if present) */}
            {keyData.organization_id && (
              <div className="flex items-center gap-2 text-sm">
                <span className="text-muted-foreground">
                  {t('independentKey.organizationId', 'Organization ID')}:
                </span>
                <span className="text-xs text-foreground/80">{keyData.organization_id}</span>
              </div>
            )}

            {/* Description (if present) */}
            {keyData.description && (
              <div className="pt-2 border-t border-border/50">
                <p className="text-sm text-muted-foreground">{keyData.description}</p>
              </div>
            )}

            {/* Created Date */}
            <div className="flex items-center gap-2 text-xs text-muted-foreground pt-2">
              <Calendar className="h-3.5 w-3.5" />
              <span>
                {t('independentKey.created', 'Created')}: {formatDate(keyData.created_at)}
              </span>
            </div>
          </div>

          {/* Configure Button */}
          {keyData.is_active && (
            <div className="pt-4 border-t border-border/50">
              <Button
                variant="outline"
                size="sm"
                className="w-full"
                onClick={() => onConfigure(keyData)}
              >
                {t('independentKey.configure', 'Configure to Tools')}
              </Button>
            </div>
          )}
        </div>
      </Card>

      {/* Delete Confirmation Dialog */}
      <AlertDialog open={deleteDialogOpen} onOpenChange={setDeleteDialogOpen}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>
              {t('independentKey.deleteConfirmTitle', 'Delete API Key?')}
            </AlertDialogTitle>
            <AlertDialogDescription>
              {t(
                'independentKey.deleteConfirmDescription',
                'Are you sure you want to delete "{name}"? This action cannot be undone.',
                { name: keyData.name }
              )}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>{t('common.cancel', 'Cancel')}</AlertDialogCancel>
            <AlertDialogAction
              onClick={handleDelete}
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
              disabled={deleteMutation.isPending}
            >
              {deleteMutation.isPending ? t('common.deleting', 'Deleting...') : t('common.delete', 'Delete')}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  );
}
