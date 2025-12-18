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
  isDeleting?: boolean;
}

export function ProviderCard({
  provider,
  onEdit,
  onDelete,
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
        <Card
          className={cn(
            'group relative overflow-hidden transition-all duration-200',
            'bg-card border shadow-sm',
            'hover:shadow-md hover:scale-[1.01] active:scale-[0.99] cursor-pointer',
            'hover:border-primary/50'
          )}
        >
          <div className="p-6">
            {/* Header */}
            <div className="flex items-start justify-between mb-4">
              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2 mb-1">
                  <h3 className="text-lg font-semibold truncate">
                    {provider.name}
                  </h3>
                  {provider.is_builtin && (
                    <Badge variant="secondary" className="shrink-0">
                      {t('providerCard.builtin')}
                    </Badge>
                  )}
                </div>
                <div className="flex items-center gap-2 text-sm text-muted-foreground">
                  <Globe className="h-3.5 w-3.5" />
                  <span className="truncate">{provider.domain}</span>
                </div>
              </div>

              {/* Actions Menu */}
              <DropdownMenu>
                <DropdownMenuTrigger asChild>
                  <Button
                    variant="ghost"
                    size="icon"
                    className="h-8 w-8 opacity-0 group-hover:opacity-100 transition-opacity"
                  >
                    <MoreVertical className="h-4 w-4" />
                  </Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent align="end" className="w-48">
                  <DropdownMenuItem onClick={() => onEdit(provider)}>
                    <Edit className="h-4 w-4 mr-2" />
                    {t('common.edit')}
                  </DropdownMenuItem>
                  <DropdownMenuSeparator />
                  <DropdownMenuItem
                    onClick={() => setShowDeleteConfirm(true)}
                    disabled={isDeleting}
                    className="text-destructive focus:text-destructive"
                  >
                    <Trash2 className="h-4 w-4 mr-2" />
                    {t('common.delete')}
                  </DropdownMenuItem>
                </DropdownMenuContent>
              </DropdownMenu>
            </div>

            {/* Stats */}
            <div className="flex items-center gap-4 pt-4 border-t border-border/50">
              <div className="flex items-center gap-2 text-sm">
                <Users className="h-4 w-4 text-muted-foreground" />
                <span className="text-muted-foreground">
                  {t('providerCard.accountCount', { count: provider.account_count || 0 })}
                </span>
              </div>

              <div className="flex items-center gap-2 text-sm">
                {needsWafBypass ? (
                  <>
                    <Shield className="h-4 w-4 text-yellow-500" />
                    <span className="text-muted-foreground">{t('providerCard.wafProtected')}</span>
                  </>
                ) : (
                  <>
                    <ShieldOff className="h-4 w-4 text-muted-foreground" />
                    <span className="text-muted-foreground">{t('providerCard.noWaf')}</span>
                  </>
                )}
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
