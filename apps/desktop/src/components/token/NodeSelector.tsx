import { useTranslation } from 'react-i18next';
import { Plus, Settings } from 'lucide-react';
import { toast } from 'sonner';
import { Button } from '@/components/ui/button';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Separator } from '@/components/ui/separator';
import type { ProviderNode } from '@/types/token';

interface NodeSelectorProps {
  selectedNode: string;
  onNodeChange: (node: string) => void;
  nodes: ProviderNode[];
  disabled?: boolean;
}

export function NodeSelector({
  selectedNode,
  onNodeChange,
  nodes,
  disabled = false,
}: NodeSelectorProps) {
  const { t } = useTranslation();

  const handleManageNodes = () => {
    toast.info(
      t('token.manageNodesHint', 'Go to Settings > Nodes to manage your API endpoints.')
    );
  };

  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between">
        <Label className="text-xs font-medium">
          {t('token.configDialog.apiEndpoint', 'API Endpoint')}
        </Label>
        <Button
          variant="ghost"
          size="sm"
          className="h-5 px-2 text-[10px] text-primary hover:text-primary/80 hover:bg-primary/10"
          onClick={handleManageNodes}
        >
          <Plus className="h-3 w-3 mr-1" />
          {t('token.configDialog.manageNodes', 'Manage Nodes')}
        </Button>
      </div>
      <Select
        value={selectedNode}
        onValueChange={onNodeChange}
        disabled={disabled}
      >
        <SelectTrigger className="h-10 bg-background/50">
          <SelectValue
            placeholder={t('token.chooseNode', 'Select an endpoint...')}
          />
        </SelectTrigger>
        <SelectContent>
          {nodes.map((node) => (
            <SelectItem key={node.id} value={node.base_url}>
              <div className="flex flex-col py-0.5">
                <span className="font-medium">{node.name}</span>
                <span className="text-[10px] text-muted-foreground">
                  {node.base_url}
                </span>
              </div>
            </SelectItem>
          ))}
          <Separator className="my-1" />
          <Button
            variant="ghost"
            className="w-full justify-start h-8 px-2 text-xs font-normal text-muted-foreground hover:text-primary"
            onClick={(e) => {
              e.preventDefault();
              handleManageNodes();
            }}
          >
            <Settings className="h-3 w-3 mr-2" />
            {t('token.configDialog.manageNodes', 'Manage Nodes...')}
          </Button>
        </SelectContent>
      </Select>
    </div>
  );
}
