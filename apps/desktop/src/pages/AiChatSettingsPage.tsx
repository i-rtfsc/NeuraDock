import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Plus, MessageSquare, ExternalLink, Trash2, Edit, GripVertical, Globe } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Switch } from '@/components/ui/switch';
import { Badge } from '@/components/ui/badge';
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
import { PageContainer } from '@/components/layout/PageContainer';
import { PageContent } from '@/components/layout/PageContent';
import { LoadingState } from '@/components/ui/loading';
import { cn } from '@/lib/utils';
import {
  useAiChatServices,
  useCreateAiChatService,
  useUpdateAiChatService,
  useDeleteAiChatService,
  useToggleAiChatService,
  useOpenAiChatWebview,
  useOpenAiChatInBrowser,
  AiChatServiceDto,
} from '@/hooks/useAiChatServices';

// Service icon mapping
const SERVICE_ICONS: Record<string, string> = {
  deepseek: '🤖',
  chatgpt: '💬',
  claude: '🎭',
  gemini: '✨',
};

function getServiceIcon(icon: string | null): string {
  if (!icon) return '🔗';
  return SERVICE_ICONS[icon] || '🔗';
}

interface ServiceCardProps {
  service: AiChatServiceDto;
  onToggle: () => void;
  onEdit: () => void;
  onDelete: () => void;
  onOpen: () => void;
  onOpenBrowser: () => void;
}

function ServiceCard({ service, onToggle, onEdit, onDelete, onOpen, onOpenBrowser }: ServiceCardProps) {
  const { t } = useTranslation();

  return (
    <div
      className={cn(
        'group flex items-center gap-4 p-4 rounded-xl border transition-all duration-200',
        service.is_enabled
          ? 'bg-card/80 border-border/60 hover:border-primary/30 hover:shadow-md'
          : 'bg-muted/30 border-border/30 opacity-60'
      )}
    >
      {/* Drag handle placeholder */}
      <div className="opacity-0 group-hover:opacity-50 transition-opacity cursor-grab">
        <GripVertical className="h-5 w-5 text-muted-foreground" />
      </div>

      {/* Icon */}
      <div className="flex items-center justify-center w-12 h-12 rounded-xl bg-primary/10 text-2xl shrink-0">
        {getServiceIcon(service.icon)}
      </div>

      {/* Info */}
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2">
          <h3 className="font-semibold text-foreground truncate">{service.name}</h3>
          {service.is_builtin && (
            <Badge variant="secondary" className="text-xs shrink-0">
              {t('aiChat.builtin')}
            </Badge>
          )}
        </div>
        <p className="text-sm text-muted-foreground truncate mt-0.5">{service.url}</p>
      </div>

      {/* Actions */}
      <div className="flex items-center gap-2 shrink-0">
        {service.is_enabled && (
          <>
            <Button
              variant="default"
              size="sm"
              onClick={onOpen}
              className="h-8 px-3"
            >
              <MessageSquare className="h-4 w-4 mr-1" />
              {t('aiChat.openInApp')}
            </Button>
            <Button
              variant="ghost"
              size="icon"
              onClick={onOpenBrowser}
              className="h-8 w-8"
              title={t('aiChat.openInBrowser')}
            >
              <ExternalLink className="h-4 w-4" />
            </Button>
          </>
        )}

        <Switch
          checked={service.is_enabled}
          onCheckedChange={onToggle}
          className="ml-2"
        />

        {!service.is_builtin && (
          <>
            <Button
              variant="ghost"
              size="icon"
              onClick={onEdit}
              className="h-8 w-8"
              title={t('common.edit')}
            >
              <Edit className="h-4 w-4" />
            </Button>
            <Button
              variant="ghost"
              size="icon"
              onClick={onDelete}
              className="h-8 w-8 text-destructive hover:text-destructive"
              title={t('common.delete')}
            >
              <Trash2 className="h-4 w-4" />
            </Button>
          </>
        )}
      </div>
    </div>
  );
}

interface ServiceDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  service?: AiChatServiceDto | null;
  onSave: (name: string, url: string, icon: string | null) => void;
  isLoading: boolean;
}

function ServiceDialog({ open, onOpenChange, service, onSave, isLoading }: ServiceDialogProps) {
  const { t } = useTranslation();
  const [name, setName] = useState(service?.name || '');
  const [url, setUrl] = useState(service?.url || '');
  const [icon, setIcon] = useState(service?.icon || '');

  // Reset form when dialog opens
  const handleOpenChange = (newOpen: boolean) => {
    if (newOpen) {
      setName(service?.name || '');
      setUrl(service?.url || '');
      setIcon(service?.icon || '');
    }
    onOpenChange(newOpen);
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    onSave(name.trim(), url.trim(), icon.trim() || null);
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

          <div className="space-y-2">
            <Label htmlFor="icon">{t('aiChat.serviceIcon')}</Label>
            <Input
              id="icon"
              value={icon}
              onChange={(e) => setIcon(e.target.value)}
              placeholder={t('aiChat.serviceIconPlaceholder')}
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
  const { data: services = [], isLoading } = useAiChatServices();

  const createService = useCreateAiChatService();
  const updateService = useUpdateAiChatService();
  const deleteService = useDeleteAiChatService();
  const toggleService = useToggleAiChatService();
  const openWebview = useOpenAiChatWebview();
  const openBrowser = useOpenAiChatInBrowser();

  const [dialogOpen, setDialogOpen] = useState(false);
  const [editingService, setEditingService] = useState<AiChatServiceDto | null>(null);
  const [deleteConfirmService, setDeleteConfirmService] = useState<AiChatServiceDto | null>(null);

  const builtinServices = services.filter((s) => s.is_builtin);
  const customServices = services.filter((s) => !s.is_builtin);

  const handleCreateOrUpdate = (name: string, url: string, icon: string | null) => {
    if (editingService) {
      updateService.mutate(
        { id: editingService.id, name, url, icon },
        {
          onSuccess: () => {
            setDialogOpen(false);
            setEditingService(null);
          },
        }
      );
    } else {
      createService.mutate(
        { name, url, icon },
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
      title={t('aiChat.settings')}
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
                <div className="space-y-3">
                  {builtinServices.map((service) => (
                    <ServiceCard
                      key={service.id}
                      service={service}
                      onToggle={() => toggleService.mutate(service.id)}
                      onEdit={() => handleEdit(service)}
                      onDelete={() => handleDelete(service)}
                      onOpen={() => openWebview.mutate(service)}
                      onOpenBrowser={() => openBrowser.mutate(service.url)}
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
                <div className="space-y-3">
                  {customServices.map((service) => (
                    <ServiceCard
                      key={service.id}
                      service={service}
                      onToggle={() => toggleService.mutate(service.id)}
                      onEdit={() => handleEdit(service)}
                      onDelete={() => handleDelete(service)}
                      onOpen={() => openWebview.mutate(service)}
                      onOpenBrowser={() => openBrowser.mutate(service.url)}
                    />
                  ))}
                </div>
              ) : (
                <div className="flex flex-col items-center justify-center py-12 text-center rounded-xl border border-dashed border-border/60 bg-muted/20">
                  <Globe className="h-12 w-12 text-muted-foreground/30 mb-4" />
                  <p className="text-muted-foreground">{t('aiChat.noServicesDescription')}</p>
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
              <br />
              <span className="text-destructive">{t('aiChat.deleteWarning')}</span>
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
