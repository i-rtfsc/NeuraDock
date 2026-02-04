import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { Plus, Trash2, Edit, Globe, ArrowLeft, MoreVertical } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Switch } from '@/components/ui/switch';
import { Badge } from '@/components/ui/badge';
import { Card } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { ScrollArea } from '@/components/ui/scroll-area';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
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
import { PageContainer } from '@/components/layout/PageContainer';
import { PageContent } from '@/components/layout/PageContent';
import { LoadingState } from '@/components/ui/loading';
import { cn } from '@/lib/utils';
import { motion } from 'framer-motion';
import {
  useAiChatServices,
  useCreateAiChatService,
  useUpdateAiChatService,
  useDeleteAiChatService,
  useToggleAiChatService,
  AiChatServiceDto,
} from '@/hooks/useAiChatServices';
import { useAiChatStore } from '@/hooks/useAiChatStore';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';

// Cache setting select component
function CacheSettingSelect() {
  const { maxCachedWebviews, setMaxCachedWebviews } = useAiChatStore();
  
  return (
    <Select
      value={maxCachedWebviews.toString()}
      onValueChange={(value) => setMaxCachedWebviews(parseInt(value, 10))}
    >
      <SelectTrigger className="w-20">
        <SelectValue />
      </SelectTrigger>
      <SelectContent>
        {[1, 2, 3, 4, 5, 6].map((n) => (
          <SelectItem key={n} value={n.toString()}>
            {n}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );
}

interface ServiceCardProps {
  service: AiChatServiceDto;
  onToggle: () => void;
  onEdit: () => void;
  onDelete: () => void;
}

function ServiceCard({ service, onToggle, onEdit, onDelete }: ServiceCardProps) {
  const { t } = useTranslation();

  return (
    <motion.div
      layout
      initial={{ opacity: 0, y: 15 }}
      animate={{ opacity: 1, y: 0 }}
      exit={{ opacity: 0, scale: 0.95 }}
      transition={{ duration: 0.2, ease: 'easeOut' }}
    >
      <Card
        className={cn(
          'group relative overflow-hidden transition-all duration-base ease-smooth',
          'bg-card border shadow-sm',
          service.is_enabled
            ? 'hover:shadow-md hover:border-primary/50'
            : 'opacity-60'
        )}
      >
        <div className="p-6">
          {/* Header */}
          <div className="flex items-start justify-between mb-4">
            <div className="flex-1 min-w-0">
              <div className="flex items-center gap-2 mb-1">
                <h3 className="text-lg font-semibold truncate">
                  {service.name}
                </h3>
                {service.is_builtin && (
                  <Badge variant="secondary" className="shrink-0">
                    {t('aiChat.builtin')}
                  </Badge>
                )}
              </div>
              <div className="flex items-center gap-2 text-sm text-muted-foreground">
                <Globe className="h-3.5 w-3.5" />
                <span className="truncate">{service.url}</span>
              </div>
            </div>

            {/* Actions Menu */}
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button
                  variant="ghost"
                  size="icon"
                  className="h-8 w-8 opacity-0 group-hover:opacity-100 transition-opacity duration-base ease-smooth"
                >
                  <MoreVertical className="h-4 w-4" />
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end" className="w-48">
                <DropdownMenuItem onClick={onEdit}>
                  <Edit className="h-4 w-4 mr-2" />
                  {t('common.edit')}
                </DropdownMenuItem>
                {!service.is_builtin && (
                  <>
                    <DropdownMenuSeparator />
                    <DropdownMenuItem
                      onClick={onDelete}
                      className="text-destructive focus:text-destructive"
                    >
                      <Trash2 className="h-4 w-4 mr-2" />
                      {t('common.delete')}
                    </DropdownMenuItem>
                  </>
                )}
              </DropdownMenuContent>
            </DropdownMenu>
          </div>

          {/* Footer with Toggle */}
          <div className="flex items-center justify-between pt-4 border-t border-border/50">
            <span className="text-sm text-muted-foreground">
              {service.is_enabled ? t('aiChat.enabled') : t('aiChat.disabled')}
            </span>
            <Switch
              checked={service.is_enabled}
              onCheckedChange={onToggle}
            />
          </div>
        </div>
      </Card>
    </motion.div>
  );
}

interface ServiceDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  service?: AiChatServiceDto | null;
  onSave: (name: string, url: string) => void;
  isLoading: boolean;
}

function ServiceDialog({ open, onOpenChange, service, onSave, isLoading }: ServiceDialogProps) {
  const { t } = useTranslation();
  const [name, setName] = useState(service?.name || '');
  const [url, setUrl] = useState(service?.url || '');

  // Reset form when dialog opens
  const handleOpenChange = (newOpen: boolean) => {
    if (newOpen) {
      setName(service?.name || '');
      setUrl(service?.url || '');
    }
    onOpenChange(newOpen);
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    onSave(name.trim(), url.trim());
  };

  const isValid = name.trim().length > 0 && url.trim().length > 0 && (url.startsWith('http://') || url.startsWith('https://'));

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogContent className="sm:max-w-[425px]">
        <DialogHeader>
          <DialogTitle>
            {service ? t('aiChat.editService') : t('aiChat.addService')}
          </DialogTitle>
          <DialogDescription>{t('aiChat.description')}</DialogDescription>
        </DialogHeader>

        <form onSubmit={handleSubmit} className="space-y-4 py-4">
          <div className="space-y-2">
            <Label htmlFor="name">{t('aiChat.serviceName')} *</Label>
            <Input
              id="name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder={t('aiChat.serviceNamePlaceholder')}
              autoFocus
            />
          </div>

          <div className="space-y-2">
            <Label htmlFor="url">{t('aiChat.serviceUrl')} *</Label>
            <Input
              id="url"
              value={url}
              onChange={(e) => setUrl(e.target.value)}
              placeholder={t('aiChat.serviceUrlPlaceholder')}
              type="url"
            />
          </div>

          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              onClick={() => onOpenChange(false)}
            >
              {t('common.cancel')}
            </Button>
            <Button type="submit" disabled={!isValid || isLoading}>
              {isLoading ? t('common.loading') : service ? t('common.save') : t('common.create')}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

export function AiChatSettingsPage() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { data: services = [], isLoading } = useAiChatServices();

  const createService = useCreateAiChatService();
  const updateService = useUpdateAiChatService();
  const deleteService = useDeleteAiChatService();
  const toggleService = useToggleAiChatService();

  const [dialogOpen, setDialogOpen] = useState(false);
  const [editingService, setEditingService] = useState<AiChatServiceDto | null>(null);
  const [deleteConfirmService, setDeleteConfirmService] = useState<AiChatServiceDto | null>(null);

  const builtinServices = services.filter((s) => s.is_builtin);
  const customServices = services.filter((s) => !s.is_builtin);

  const handleCreateOrUpdate = (name: string, url: string) => {
    if (editingService) {
      updateService.mutate(
        { id: editingService.id, name, url, icon: null },
        {
          onSuccess: () => {
            setDialogOpen(false);
            setEditingService(null);
          },
        }
      );
    } else {
      createService.mutate(
        { name, url, icon: null },
        {
          onSuccess: () => {
            setDialogOpen(false);
          },
        }
      );
    }
  };

  const handleEdit = (service: AiChatServiceDto) => {
    setEditingService(service);
    setDialogOpen(true);
  };

  const handleDelete = (service: AiChatServiceDto) => {
    setDeleteConfirmService(service);
  };

  const confirmDelete = () => {
    if (deleteConfirmService) {
      deleteService.mutate(deleteConfirmService.id, {
        onSuccess: () => setDeleteConfirmService(null),
      });
    }
  };

  if (isLoading) {
    return <LoadingState className="h-full" />;
  }

  return (
    <PageContainer
      className="h-full bg-muted/10"
      title={
        <div className="flex items-center gap-2">
          <Button variant="ghost" size="icon" onClick={() => navigate('/ai-chat')} className="h-8 w-8">
            <ArrowLeft className="h-4 w-4" />
          </Button>
          <span className="text-2xl font-bold tracking-tight">{t('aiChat.settings')}</span>
        </div>
      }
      actions={
        <Button onClick={() => { setEditingService(null); setDialogOpen(true); }}>
          <Plus className="h-4 w-4 mr-2" />
          {t('aiChat.addService')}
        </Button>
      }
    >
      <PageContent maxWidth="lg" className="h-full">
        <ScrollArea className="h-full">
          <div className="space-y-8 pb-32">
            {/* Built-in Services */}
            {builtinServices.length > 0 && (
              <div className="space-y-4">
                <h2 className="text-sm font-semibold text-muted-foreground uppercase tracking-wide px-1">
                  {t('aiChat.builtinServices')}
                </h2>
                <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
                  {builtinServices.map((service) => (
                    <ServiceCard
                      key={service.id}
                      service={service}
                      onToggle={() => toggleService.mutate(service.id)}
                      onEdit={() => handleEdit(service)}
                      onDelete={() => handleDelete(service)}
                    />
                  ))}
                </div>
              </div>
            )}

            {/* Custom Services */}
            <div className="space-y-4">
              <h2 className="text-sm font-semibold text-muted-foreground uppercase tracking-wide px-1">
                {t('aiChat.customServices')}
              </h2>
              {customServices.length > 0 ? (
                <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
                  {customServices.map((service) => (
                    <ServiceCard
                      key={service.id}
                      service={service}
                      onToggle={() => toggleService.mutate(service.id)}
                      onEdit={() => handleEdit(service)}
                      onDelete={() => handleDelete(service)}
                    />
                  ))}
                </div>
              ) : (
                <div className="flex flex-col items-center justify-center py-12 text-center rounded-xl border border-dashed border-border/60 bg-muted/20">
                  <Globe className="h-12 w-12 text-muted-foreground/30 mb-4" />
                  <p className="text-muted-foreground">{t('aiChat.noCustomServices')}</p>
                  <Button
                    variant="outline"
                    className="mt-4"
                    onClick={() => { setEditingService(null); setDialogOpen(true); }}
                  >
                    <Plus className="h-4 w-4 mr-2" />
                    {t('aiChat.addService')}
                  </Button>
                </div>
              )}
            </div>

            {/* Settings */}
            <div className="space-y-4">
              <h2 className="text-sm font-semibold text-muted-foreground uppercase tracking-wide px-1">
                {t('aiChat.cacheSettings')}
              </h2>
              <Card className="p-4">
                <div className="flex items-center justify-between">
                  <div className="space-y-1">
                    <Label className="text-sm font-medium">{t('aiChat.maxCachedWebviews')}</Label>
                    <p className="text-xs text-muted-foreground">
                      {t('aiChat.maxCachedWebviewsDescription')}
                    </p>
                  </div>
                  <CacheSettingSelect />
                </div>
              </Card>
            </div>
          </div>
        </ScrollArea>
      </PageContent>

      {/* Create/Edit Dialog */}
      <ServiceDialog
        open={dialogOpen}
        onOpenChange={(open) => {
          setDialogOpen(open);
          if (!open) setEditingService(null);
        }}
        service={editingService}
        onSave={handleCreateOrUpdate}
        isLoading={createService.isPending || updateService.isPending}
      />

      {/* Delete Confirmation */}
      <AlertDialog
        open={!!deleteConfirmService}
        onOpenChange={(open) => !open && setDeleteConfirmService(null)}
      >
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>{t('aiChat.deleteConfirmTitle')}</AlertDialogTitle>
            <AlertDialogDescription>
              {t('aiChat.deleteConfirmMessage', { name: deleteConfirmService?.name })}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>{t('common.cancel')}</AlertDialogCancel>
            <AlertDialogAction
              onClick={confirmDelete}
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
            >
              {t('common.delete')}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </PageContainer>
  );
}
