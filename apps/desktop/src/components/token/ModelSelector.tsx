import { useTranslation } from 'react-i18next';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';

interface ModelSelectorProps {
  selectedModel: string;
  onModelChange: (model: string) => void;
  filteredModels: string[];
  isLoading: boolean;
  disabled?: boolean;
}

export function ModelSelector({
  selectedModel,
  onModelChange,
  filteredModels,
  isLoading,
  disabled = false,
}: ModelSelectorProps) {
  const { t } = useTranslation();

  return (
    <div className="space-y-2">
      <Label className="text-xs font-medium">
        {t('token.configDialog.defaultModel', 'Default Model')}
      </Label>
      <Select
        value={selectedModel}
        onValueChange={onModelChange}
        disabled={isLoading || disabled}
      >
        <SelectTrigger className="h-10 bg-background/50">
          <SelectValue
            placeholder={
              isLoading
                ? t('token.configDialog.loadingModels', 'Loading models...')
                : t('token.configDialog.autoDetect', 'Auto-detect')
            }
          />
        </SelectTrigger>
        <SelectContent>
          {filteredModels.length > 0 ? (
            filteredModels.map((model) => (
              <SelectItem key={model} value={model}>
                {model}
              </SelectItem>
            ))
          ) : (
            <div className="p-3 text-xs text-muted-foreground text-center">
              {isLoading
                ? t('token.configDialog.loadingModels', 'Loading models...')
                : t('token.configDialog.noSpecificModels', 'No specific models found')}
            </div>
          )}
        </SelectContent>
      </Select>
    </div>
  );
}
