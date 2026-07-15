import { useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';
import { ArrowRightLeft, Copy, Check } from 'lucide-react';

import { Button } from '@/components/ui/button';
import { Card } from '@/components/ui/card';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs';

type Tool = 'json' | 'base64' | 'url' | 'timestamp';
type Direction = 'encode' | 'decode';

function bytesToBase64(bytes: Uint8Array): string {
  let binary = '';
  for (const byte of bytes) {
    binary += String.fromCharCode(byte);
  }
  return btoa(binary);
}

function base64ToBytes(input: string): Uint8Array {
  const binary = atob(input.trim());
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i += 1) {
    bytes[i] = binary.charCodeAt(i);
  }
  return bytes;
}

function runTool(tool: Tool, direction: Direction, input: string): string {
  if (!input) return '';
  switch (tool) {
    case 'json':
      return JSON.stringify(JSON.parse(input), null, 2);
    case 'base64':
      return direction === 'encode'
        ? bytesToBase64(new TextEncoder().encode(input))
        : new TextDecoder().decode(base64ToBytes(input));
    case 'url':
      return direction === 'encode' ? encodeURIComponent(input) : decodeURIComponent(input);
    case 'timestamp': {
      const trimmed = input.trim();
      if (/^\d+$/.test(trimmed)) {
        const ms = trimmed.length > 10 ? Number(trimmed) : Number(trimmed) * 1000;
        return new Date(ms).toISOString();
      }
      const parsed = Date.parse(trimmed);
      if (Number.isNaN(parsed)) throw new Error('invalid date');
      return String(Math.floor(parsed / 1000));
    }
  }
}

export function EncodingTools() {
  const { t } = useTranslation();
  const [tool, setTool] = useState<Tool>('json');
  const [direction, setDirection] = useState<Direction>('encode');
  const [input, setInput] = useState('');
  const [copied, setCopied] = useState(false);

  const { output, error } = useMemo(() => {
    try {
      return { output: runTool(tool, direction, input), error: null as string | null };
    } catch (e) {
      return { output: '', error: e instanceof Error ? e.message : String(e) };
    }
  }, [tool, direction, input]);

  const hasDirection = tool === 'base64' || tool === 'url' || tool === 'timestamp';

  const handleCopy = async () => {
    if (!output) return;
    try {
      await navigator.clipboard.writeText(output);
      setCopied(true);
      toast.success(t('devToolbox.encoding.copied', '已复制'));
      setTimeout(() => setCopied(false), 1500);
    } catch {
      toast.error(t('token.copyError', '复制失败'));
    }
  };

  return (
    <Card className="p-5 space-y-4">
      <div className="space-y-0.5">
        <h3 className="text-sm font-semibold">
          {t('devToolbox.encoding.title', '编解码 / 格式化')}
        </h3>
        <p className="text-xs text-muted-foreground">
          {t('devToolbox.encoding.description', '本地离线处理，不联网、不上传。')}
        </p>
      </div>

      <div className="flex flex-wrap items-center gap-3">
        <Tabs value={tool} onValueChange={(v) => setTool(v as Tool)}>
          <TabsList>
            <TabsTrigger value="json">JSON</TabsTrigger>
            <TabsTrigger value="base64">Base64</TabsTrigger>
            <TabsTrigger value="url">URL</TabsTrigger>
            <TabsTrigger value="timestamp">{t('devToolbox.encoding.timestamp', '时间戳')}</TabsTrigger>
          </TabsList>
        </Tabs>

        {hasDirection && (
          <Button
            variant="outline"
            size="sm"
            onClick={() => setDirection((d) => (d === 'encode' ? 'decode' : 'encode'))}
          >
            <ArrowRightLeft className="h-3.5 w-3.5" />
            {direction === 'encode'
              ? t('devToolbox.encoding.encode', '编码')
              : t('devToolbox.encoding.decode', '解码')}
          </Button>
        )}
      </div>

      <div className="grid gap-4 md:grid-cols-2">
        <div className="space-y-1.5">
          <Label className="text-xs">{t('devToolbox.encoding.input', '输入')}</Label>
          <Textarea
            value={input}
            onChange={(e) => setInput(e.target.value)}
            rows={8}
            className="font-mono text-xs"
            placeholder={t('devToolbox.encoding.inputPlaceholder', '在此粘贴内容...')}
          />
        </div>
        <div className="space-y-1.5">
          <div className="flex items-center justify-between">
            <Label className="text-xs">{t('devToolbox.encoding.output', '输出')}</Label>
            <Button variant="ghost" size="sm" className="h-6 px-2" onClick={handleCopy} disabled={!output}>
              {copied ? <Check className="h-3 w-3" /> : <Copy className="h-3 w-3" />}
            </Button>
          </div>
          <Textarea
            value={error ? '' : output}
            readOnly
            rows={8}
            className="font-mono text-xs bg-muted/30"
            placeholder={error ?? ''}
          />
          {error && <p className="text-xs text-destructive">{t('devToolbox.encoding.error', '无法处理：{{error}}', { error })}</p>}
        </div>
      </div>
    </Card>
  );
}
