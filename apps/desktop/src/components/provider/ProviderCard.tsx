import { useState } from 'react';
import { useTranslation } from 'react-i18next';
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
import {
  MoreVertical,
  Edit,
  Settings,
  Trash2,
  Globe,
  Shield,
  ShieldOff,
  Users,
} from 'lucide-react';
import { motion } from 'framer-motion';
import { cn } from '@/lib/utils';
import { ProviderDto } from '@/hooks/useProviders';

interface ProviderCardProps {
  provider: ProviderDto;
  onEdit: (provider: ProviderDto) => void;
  onDelete: (providerId: string) => void;
  onManageNodes?: (provider: ProviderDto) => void;
  isDeleting?: boolean;
}

export function ProviderCard({
  provider,
  onEdit,
  onDelete,
  onManageNodes,
  isDeleting = false,
}: ProviderCardProps) {
  const { t } = useTranslation();
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);

  const handleDelete = async () => {
    await onDelete(provider.id);
    setShowDeleteConfirm(false);
  };

  const needsWafBypass = provider.needs_waf_bypass;

  return (
    <>
      <motion.div
        layout
        initial={{ opacity: 0, y: 15 }}
        animate={{ opacity: 1, y: 0 }}
        exit={{ opacity: 0, scale: 0.95 }}
        transition={{ duration: 0.2, ease: 'easeOut' }}
      >
        <Card className="card-vivid group">
          {/* Side Accent Bar */}
          <div className={cn(
            "card-accent-bar",
            provider.is_builtin ? "bg-primary/40 group-hover:bg-primary" : "bg-accent-2/40 group-hover:bg-accent-2"
          )} />

          <div className="p-5 flex flex-col flex-1">
            {/* Header */}
            <div className="flex items-start justify-between mb-5">
              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2 mb-1.5">
                  <h3 className="card-title-vivid">
                    {provider.name}
                  </h3>
                  {provider.is_builtin && (
                    <Badge variant="soft-primary" className="shrink-0 text-[10px] h-5 font-bold uppercase tracking-wider">
                      {t('providerCard.builtin')}
                    </Badge>
                  )}
                </div>
                <div className="flex items-center gap-2 text-xs font-medium text-foreground/80 bg-muted/40 w-fit px-2 py-1 rounded-lg border border-border/30">
                  <Globe className="h-3.5 w-3.5 text-primary/80" />
                  <span className="truncate">{provider.domain}</span>
                </div>
              </div>

              {/* Actions Menu */}
              <DropdownMenu>
                <DropdownMenuTrigger asChild>
                  <Button
                    variant="ghost"
                    size="icon-sm"
                    className="h-9 w-9 -mr-2 -mt-2 opacity-40 group-hover:opacity-100 hover:bg-primary/10 hover:text-primary transition-all rounded-full"
                  >
                    <MoreVertical className="h-4 w-4" />
                  </Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent align="end" className="w-48 p-1 rounded-xl shadow-xl">
                  <DropdownMenuItem onClick={() => onEdit(provider)} className="rounded-lg gap-3">
                    <Edit className="h-4 w-4 text-primary" />
                    {t('common.edit')}
                  </DropdownMenuItem>
                  {onManageNodes && (
                    <DropdownMenuItem onClick={() => onManageNodes(provider)} className="rounded-lg gap-3">
                      <Settings className="h-4 w-4 text-primary" />
                      {t('token.configDialog.manageNodes', 'Manage Nodes')}
                    </DropdownMenuItem>
                  )}
                  <DropdownMenuSeparator className="my-1" />
                  <DropdownMenuItem
                    onClick={() => setShowDeleteConfirm(true)}
                    disabled={isDeleting}
                    className="text-destructive focus:text-destructive focus:bg-destructive/10 rounded-lg gap-3"
                  >
                    <Trash2 className="h-4 w-4" />
                    {t('common.delete')}
                  </DropdownMenuItem>
                </DropdownMenuContent>
              </DropdownMenu>
            </div>

            {/* Stats - Grid style for energy */}
            <div className="mt-auto grid grid-cols-2 gap-3 pt-4 border-t border-border/30">
              <div className="flex flex-col gap-1">
                <span className="text-[10px] font-black text-foreground/50 uppercase tracking-widest px-1">
                  {t('dashboard.accounts_plural', 'Accounts')}
                </span>
                <div className="flex items-center gap-2 px-2 py-1.5 rounded-xl bg-primary/5 border border-primary/10 group-hover:bg-primary/10 transition-colors">
                  <Users className="h-3.5 w-3.5 text-primary" />
                  <span className="text-sm font-bold text-primary">
                    {provider.account_count || 0}
                  </span>
                </div>
              </div>

              <div className="flex flex-col gap-1">
                <span className="text-[10px] font-black text-foreground/50 uppercase tracking-widest px-1">
                  Security
                </span>
                <div className={cn(
                  "flex items-center gap-2 px-2 py-1.5 rounded-xl border transition-colors",
                  needsWafBypass 
                    ? "bg-warning/5 border-warning/10 text-warning group-hover:bg-warning/10" 
                    : "bg-success/5 border-success/10 text-success group-hover:bg-success/10"
                )}>
                  {needsWafBypass ? (
                    <Shield className="h-3.5 w-3.5" />
                  ) : (
                    <ShieldOff className="h-3.5 w-3.5 opacity-80" />
                  )}
                  <span className="text-sm font-bold truncate">
                    {needsWafBypass ? "WAF" : "Open"}
                  </span>
                </div>
              </div>
            </div>
          </div>
        </Card>
      </motion.div>

      {/* Delete Confirmation Dialog */}
      <AlertDialog open={showDeleteConfirm} onOpenChange={setShowDeleteConfirm}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>{t('providerCard.deleteConfirmTitle')}</AlertDialogTitle>
            <AlertDialogDescription>
              {t('providerCard.deleteConfirmMessage', { name: provider.name })}
              {provider.account_count && provider.account_count > 0 && (
                <span className="block mt-2 text-destructive font-medium">
                  {t('providerCard.deleteWarning', { count: provider.account_count })}
                </span>
              )}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>{t('common.cancel')}</AlertDialogCancel>
            <AlertDialogAction
              onClick={handleDelete}
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
            >
              {t('providerCard.confirmDelete')}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  );
}
