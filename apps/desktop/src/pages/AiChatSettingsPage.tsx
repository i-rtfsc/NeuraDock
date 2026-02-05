import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { Plus, Trash2, Edit, Globe, ArrowLeft, MoreVertical, Settings } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Switch } from '@/components/ui/switch';
import { Badge } from '@/components/ui/badge';
import { Card } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
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
import { createFadeUpItem, createStaggerContainer } from '@/lib/motion';

// Cache setting select component
function CacheSettingSelect() {
  const { maxCachedWebviews, setMaxCachedWebviews } = useAiChatStore();
  
  return (
    <Select
      value={maxCachedWebviews.toString()}
      onValueChange={(value) => setMaxCachedWebviews(parseInt(value, 10))}
    >
      <SelectTrigger className="w-20 h-8 text-xs border-border/50 bg-background/50 focus:ring-primary/20 rounded-lg">
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
    <Card className={cn(
      "card-vivid group",
      !service.is_enabled && "opacity-60 grayscale-[0.5]"
    )}>
      {/* Side Accent Bar */}
      <div className={cn(
        "card-accent-bar",
        service.is_builtin ? "bg-primary/40 group-hover:bg-primary" : "bg-accent-2/40 group-hover:bg-accent-2"
      )} />

      <div className="p-5 flex flex-col flex-1">
        {/* Header */}
        <div className="flex items-start justify-between mb-5">
          <div className="flex-1 min-w-0">
            <div className="flex items-center gap-2 mb-1.5">
              <h3 className="card-title-vivid">
                {service.name}
              </h3>
              {service.is_builtin && (
                <Badge variant="soft-primary" className="shrink-0 text-[10px] h-5 font-bold uppercase tracking-wider">
                  {t('aiChat.builtin')}
                </Badge>
              )}
            </div>
            <div className="flex items-center gap-2 text-xs font-medium text-muted-foreground bg-muted/40 w-fit px-2 py-1 rounded-lg border border-border/30">
              <Globe className="h-3.5 w-3.5 text-primary/70" />
              <span className="truncate">{service.url}</span>
            </div>
          </div>

          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button
                variant="ghost"
                size="icon"
                className="h-9 w-9 -mr-2 -mt-2 opacity-40 group-hover:opacity-100 hover:bg-primary/10 hover:text-primary transition-all rounded-full"
              >
                <MoreVertical className="h-4 w-4" />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end" className="w-48 p-1 rounded-xl shadow-xl">
              <DropdownMenuItem onClick={onEdit} className="rounded-lg gap-3">
                <Edit className="h-4 w-4 text-primary" />
                {t('common.edit')}
              </DropdownMenuItem>
              {!service.is_builtin && (
                <>
                  <DropdownMenuSeparator className="my-1" />
                  <DropdownMenuItem
                    onClick={onDelete}
                    className="text-destructive focus:text-destructive focus:bg-destructive/10 rounded-lg gap-3"
                  >
                    <Trash2 className="h-4 w-4" />
                    {t('common.delete')}
                  </DropdownMenuItem>
                </>
              )}
            </DropdownMenuContent>
          </DropdownMenu>
        </div>

        {/* Footer with Toggle */}
        <div className="mt-auto flex items-center justify-between pt-4 border-t border-border/30">
          <span className="text-[10px] font-bold text-muted-foreground/60 uppercase tracking-widest px-1">
            {service.is_enabled ? t('aiChat.enabled') : t('aiChat.disabled')}
          </span>
          <Switch
            checked={service.is_enabled}
            onCheckedChange={onToggle}
            className="data-[state=checked]:bg-primary"
          />
        </div>
      </div>
    </Card>
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
      <DialogContent className="sm:max-w-[425px] rounded-2xl">
        <DialogHeader>
          <DialogTitle className="text-xl font-black tracking-tighter text-primary">
            {service ? t('aiChat.editService') : t('aiChat.addService')}
          </DialogTitle>
          <DialogDescription>{t('aiChat.description')}</DialogDescription>
        </DialogHeader>

        <form onSubmit={handleSubmit} className="space-y-4 py-4">
          <div className="space-y-2">
            <Label htmlFor="name" className="text-xs font-bold uppercase tracking-wider text-muted-foreground ml-1">{t('aiChat.serviceName')} *</Label>
            <Input
              id="name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder={t('aiChat.serviceNamePlaceholder')}
              className="rounded-xl h-10 border-border/50 focus:border-primary/50"
              autoFocus
            />
          </div>

          <div className="space-y-2">
            <Label htmlFor="url" className="text-xs font-bold uppercase tracking-wider text-muted-foreground ml-1">{t('aiChat.serviceUrl')} *</Label>
            <Input
              id="url"
              value={url}
              onChange={(e) => setUrl(e.target.value)}
              placeholder={t('aiChat.serviceUrlPlaceholder')}
              className="rounded-xl h-10 border-border/50 focus:border-primary/50"
              type="url"
            />
          </div>

          <DialogFooter className="pt-2">
            <Button
              type="button"
              variant="outline"
              onClick={() => onOpenChange(false)}
              className="rounded-xl"
            >
              {t('common.cancel')}
            </Button>
            <Button type="submit" disabled={!isValid || isLoading} className="rounded-xl btn-gradient-primary border-0 font-bold">
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

  const containerVariants = createStaggerContainer({ staggerChildren: 0.03, delayChildren: 0.05 });
  const itemVariants = createFadeUpItem({ y: 8, scale: 0.98 });

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
      className="h-full"
      title={
        <div className="flex items-center gap-3">
          <Button
            variant="ghost"
            size="icon"
            onClick={() => navigate('/ai-chat')}
            className="h-9 w-9 rounded-full hover:bg-muted"
          >
            <ArrowLeft className="h-5 w-5 text-primary" />
          </Button>
          <span className="text-2xl">{t('aiChat.settings')}</span>
        </div>
      }
      actions={
        <Button
          size="sm"
          className="shadow-md btn-gradient-primary border-0 font-bold hover:scale-105 active:scale-95 transition-all"
          onClick={() => { setEditingService(null); setDialogOpen(true); }}
        >
          <Plus className="h-4 w-4 mr-2 stroke-[3px]" />
          {t('aiChat.addService')}
        </Button>
      }
    >
      <PageContent maxWidth="lg" className="pb-32 pt-2">
        <div className="space-y-10">
          {/* Built-in Services */}
          {builtinServices.length > 0 && (
            <div className="space-y-4">
              <div className="flex items-center gap-2 px-1">
                <Badge variant="soft-primary" className="rounded-md">
                  <Settings className="h-3 w-3 mr-1.5" />
                  {t('aiChat.builtinServices')}
                </Badge>
              </div>
              <motion.div 
                variants={containerVariants}
                initial="hidden"
                animate="show"
                className="grid gap-6 sm:grid-cols-2 lg:grid-cols-3"
              >
                {builtinServices.map((service) => (
                  <motion.div key={service.id} variants={itemVariants}>
                    <ServiceCard
                      service={service}
                      onToggle={() => toggleService.mutate(service.id)}
                      onEdit={() => handleEdit(service)}
                      onDelete={() => handleDelete(service)}
                    />
                  </motion.div>
                ))}
              </motion.div>
            </div>
          )}

          {/* Custom Services */}
          <div className="space-y-4">
            <div className="flex items-center gap-2 px-1">
              <Badge variant="soft-primary" className="rounded-md bg-accent-2/10 text-accent-2 border-accent-2/20">
                <Plus className="h-3 w-3 mr-1.5" />
                {t('aiChat.customServices')}
              </Badge>
            </div>
            {customServices.length > 0 ? (
              <motion.div 
                variants={containerVariants}
                initial="hidden"
                animate="show"
                className="grid gap-6 sm:grid-cols-2 lg:grid-cols-3"
              >
                {customServices.map((service) => (
                  <motion.div key={service.id} variants={itemVariants}>
                    <ServiceCard
                      service={service}
                      onToggle={() => toggleService.mutate(service.id)}
                      onEdit={() => handleEdit(service)}
                      onDelete={() => handleDelete(service)}
                    />
                  </motion.div>
                ))}
              </motion.div>
            ) : (
              <div className="flex flex-col items-center justify-center py-12 text-center rounded-2xl border border-dashed border-border/60 bg-muted/10">
                <Globe className="h-12 w-12 text-muted-foreground/30 mb-4" />
                <p className="text-muted-foreground text-sm font-medium">{t('aiChat.noCustomServices')}</p>
                <Button
                  variant="outline"
                  size="sm"
                  className="mt-4 rounded-xl shadow-sm hover:bg-primary/5 hover:text-primary transition-all"
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
            <div className="flex items-center gap-2 px-1">
              <Badge variant="soft-primary" className="rounded-md bg-info/10 text-info border-info/20">
                <Settings className="h-3 w-3 mr-1.5" />
                {t('aiChat.cacheSettings')}
              </Badge>
            </div>
            <motion.div initial="hidden" animate="show" variants={itemVariants}>
              <Card className="card-vivid group p-6 flex flex-row items-center justify-between">
                <div className="space-y-1">
                  <Label className="text-base font-bold tracking-tight">{t('aiChat.maxCachedWebviews')}</Label>
                  <p className="text-xs text-muted-foreground font-medium">
                    {t('aiChat.maxCachedWebviewsDescription')}
                  </p>
                </div>
                <div className="pl-6">
                  <CacheSettingSelect />
                </div>
              </Card>
            </motion.div>
          </div>
        </div>
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
        <AlertDialogContent className="rounded-2xl">
          <AlertDialogHeader>
            <AlertDialogTitle className="text-xl font-black tracking-tighter text-primary">{t('aiChat.deleteConfirmTitle')}</AlertDialogTitle>
            <AlertDialogDescription>
              {t('aiChat.deleteConfirmMessage', { name: deleteConfirmService?.name })}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel className="rounded-xl">{t('common.cancel')}</AlertDialogCancel>
            <AlertDialogAction
              onClick={confirmDelete}
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90 rounded-xl"
            >
              {t('common.delete')}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </PageContainer>
  );
}