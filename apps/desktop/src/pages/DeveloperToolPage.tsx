import { useEffect, useMemo, useState, type ReactNode } from 'react';
import { open } from '@tauri-apps/plugin-shell';
import { useTranslation } from 'react-i18next';
import { ExternalLink, Plus, RefreshCw, Save, Settings2, Trash2 } from 'lucide-react';
import { toast } from 'sonner';

import { PageContainer } from '@/components/layout/PageContainer';
import { PageContent } from '@/components/layout/PageContent';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import { Textarea } from '@/components/ui/textarea';
import { extractErrorMessage } from '@/lib/errorHandling';
import { safeExternalUrl } from '@/lib/externalUrl';
import { commands, type CommandError, type DeveloperToolConnectionDto, type GerritChangeDto, type JiraIssueDto, type JiraMetadataDto, type SavedDeveloperToolQueryDto } from '@/lib/tauri';

type Service = 'jira' | 'gerrit';

type Connection = DeveloperToolConnectionDto;
/*
 * The generated client returns a typed Result so command failures cannot be
 * mistaken for successful payloads at the page boundary.
 */
function unwrap<T>(result: { status: 'ok'; data: T } | { status: 'error'; error: CommandError }): T {
  if (result.status === 'error') throw new Error(result.error.message);
  return result.data;
}

/* interface Connection {
  id: string;
  name: string;
  service: Service;
  base_url: string;
  deployment: string;
  auth_method: string;
  username: string | null;
  has_secret: boolean;
  profile_json: string | null;
} */

/* interface JiraIssue {
  key: string;
  summary: string;
  project: string;
  issue_type: string;
  priority: string;
  status: string;
  status_category: string;
  assignee: string;
  updated: string;
  url: string;
  semantic_fields_json: string;
} */

/* interface GerritChange {
  id: string;
  project: string;
  branch: string;
  subject: string;
  owner: string;
  status: string;
  updated: string;
  url: string;
} */

type JiraIssue = JiraIssueDto;
type GerritChange = GerritChangeDto;
type SavedQuery = SavedDeveloperToolQueryDto;
type JiraMetadata = JiraMetadataDto;

const EMPTY_PROFILE = JSON.stringify({ schema_version: 1, name: 'Local Jira profile', field_mappings: {}, status_mappings: {} }, null, 2);

export function JiraToolPage() { return <DeveloperToolPage service="jira" />; }
export function GerritToolPage() { return <DeveloperToolPage service="gerrit" />; }

function DeveloperToolPage({ service }: { service: Service }) {
  const { t } = useTranslation();
  const [connections, setConnections] = useState<Connection[]>([]);
  const [connectionId, setConnectionId] = useState('');
  const [query, setQuery] = useState(service === 'jira' ? 'assignee = currentUser() ORDER BY updated DESC' : 'reviewer:self status:open');
  const [results, setResults] = useState<(JiraIssue | GerritChange)[]>([]);
  const [savedQueries, setSavedQueries] = useState<SavedQuery[]>([]);
  const [loading, setLoading] = useState(false);
  const [dialogOpen, setDialogOpen] = useState(false);
  const [editing, setEditing] = useState<Connection | null>(null);
  const [metadata, setMetadata] = useState<JiraMetadata | null>(null);
  const [saveQueryOpen, setSaveQueryOpen] = useState(false);
  const [queryName, setQueryName] = useState('');

  const active = connections.find((item) => item.id === connectionId);

  const loadConnections = async () => {
    const all = unwrap(await commands.listDeveloperToolConnections());
    const filtered = all.filter((item) => item.service === service);
    setConnections(filtered);
    setConnectionId((current) => filtered.some((item) => item.id === current) ? current : filtered[0]?.id ?? '');
  };

  useEffect(() => { loadConnections().catch((error) => toast.error(extractErrorMessage(error))); }, [service]);
  useEffect(() => {
    if (!connectionId) { setSavedQueries([]); return; }
    commands.listDeveloperToolSavedQueries(connectionId)
      .then(unwrap).then(setSavedQueries).catch((error) => toast.error(extractErrorMessage(error)));
  }, [connectionId]);

  const runQuery = async () => {
    if (!connectionId || !query.trim()) return;
    setLoading(true);
    try {
      const data = service === 'jira'
        ? unwrap(await commands.queryJiraIssues(connectionId, query))
        : unwrap(await commands.queryGerritChanges(connectionId, query));
      setResults(data as (JiraIssue | GerritChange)[]);
    } catch (error) { toast.error(extractErrorMessage(error)); }
    finally { setLoading(false); }
  };

  const saveCurrentQuery = async () => {
    if (!connectionId || !query.trim()) return;
    const name = queryName.trim();
    if (!name) return;
    const saved = unwrap(await commands.saveDeveloperToolQuery(connectionId, null, name, query));
    setSavedQueries((items) => [saved, ...items]);
    setQueryName('');
    setSaveQueryOpen(false);
  };

  const deleteSavedQuery = async (queryId: string) => {
    unwrap(await commands.deleteDeveloperToolQuery(queryId));
    setSavedQueries((items) => items.filter((item) => item.id !== queryId));
  };

  const deleteConnection = async () => {
    if (!active) return;
    unwrap(await commands.deleteDeveloperToolConnection(active.id));
    setResults([]); setMetadata(null); await loadConnections();
  };

  const loadMetadata = async () => {
    if (!connectionId) return;
    setLoading(true);
    try { setMetadata(unwrap(await commands.getJiraMetadata(connectionId))); }
    catch (error) { toast.error(extractErrorMessage(error)); }
    finally { setLoading(false); }
  };

  return (
    <PageContainer title={service === 'jira' ? 'Jira' : 'Gerrit'}>
      <PageContent maxWidth="none" className="space-y-4">
        <section className="flex flex-wrap items-center gap-2 border-b border-border/60 pb-3">
          <Select value={connectionId} onValueChange={setConnectionId}>
            <SelectTrigger className="w-56"><SelectValue placeholder={t('devTools.selectConnection', '选择连接')} /></SelectTrigger>
            <SelectContent>{connections.map((item) => <SelectItem key={item.id} value={item.id}>{item.name}</SelectItem>)}</SelectContent>
          </Select>
          <Button variant="outline" onClick={() => { setEditing(null); setDialogOpen(true); }}><Plus className="h-4 w-4" />{t('devTools.addConnection', '添加连接')}</Button>
          {active && <Button variant="ghost" size="icon" title={t('common.edit', '编辑')} onClick={() => { setEditing(active); setDialogOpen(true); }}><Settings2 className="h-4 w-4" /></Button>}
          {active && <Button variant="ghost" size="icon" title={t('common.delete', '删除')} onClick={() => deleteConnection().catch((e) => toast.error(extractErrorMessage(e)))}><Trash2 className="h-4 w-4" /></Button>}
          {service === 'jira' && active && <Button variant="outline" onClick={loadMetadata}><RefreshCw className="h-4 w-4" />{t('devTools.metadata', '字段与状态')}</Button>}
        </section>

        {active ? <>
          <section className="space-y-2">
            <div className="flex gap-2">
              <Input className="font-mono" value={query} onChange={(event) => setQuery(event.target.value)} onKeyDown={(event) => { if (event.key === 'Enter') runQuery(); }} />
              <Button onClick={runQuery} disabled={loading}>{loading && <RefreshCw className="h-4 w-4 animate-spin" />}{t('devTools.run', '查询')}</Button>
              <Button variant="outline" size="icon" title={t('devTools.saveQuery', '保存查询')} onClick={() => setSaveQueryOpen(true)}><Save className="h-4 w-4" /></Button>
            </div>
            {savedQueries.length > 0 && <div className="flex flex-wrap gap-1.5">{savedQueries.map((item) => <div key={item.id} className="flex items-center rounded-md border border-border/60"><Button size="sm" variant="ghost" onClick={() => setQuery(item.query)}>{item.name}</Button><Button size="icon-sm" variant="ghost" title={t('common.delete', '删除')} onClick={() => deleteSavedQuery(item.id).catch((e) => toast.error(extractErrorMessage(e)))}><Trash2 className="h-3.5 w-3.5" /></Button></div>)}</div>}
          </section>
          <ResultTable service={service} results={results} />
          {service === 'jira' && metadata && <MetadataPanel metadata={metadata} />}
        </> : <div className="py-24 text-center text-sm text-muted-foreground">{t('devTools.configureFirst', '添加连接后即可开始查询。')}</div>}
      </PageContent>
      <ConnectionDialog service={service} open={dialogOpen} connection={editing} onOpenChange={setDialogOpen} onSaved={loadConnections} />
      <Dialog open={saveQueryOpen} onOpenChange={setSaveQueryOpen}><DialogContent size="sm"><DialogHeader><DialogTitle>{t('devTools.saveQuery', '保存查询')}</DialogTitle></DialogHeader><Field label={t('devTools.queryName', '查询名称')}><Input autoFocus value={queryName} onChange={(event) => setQueryName(event.target.value)} onKeyDown={(event) => { if (event.key === 'Enter') saveCurrentQuery().catch((e) => toast.error(extractErrorMessage(e))); }} /></Field><DialogFooter><Button variant="outline" onClick={() => setSaveQueryOpen(false)}>{t('common.cancel', '取消')}</Button><Button onClick={() => saveCurrentQuery().catch((e) => toast.error(extractErrorMessage(e)))}>{t('common.save', '保存')}</Button></DialogFooter></DialogContent></Dialog>
    </PageContainer>
  );
}

function ResultTable({ service, results }: { service: Service; results: (JiraIssue | GerritChange)[] }) {
  return <div className="overflow-hidden rounded-md border"><Table><TableHeader><TableRow>
    <TableHead>{service === 'jira' ? 'Key' : 'ID'}</TableHead><TableHead>{service === 'jira' ? 'Summary' : 'Subject'}</TableHead><TableHead>Project</TableHead><TableHead>Status</TableHead><TableHead>{service === 'jira' ? 'Assignee' : 'Owner'}</TableHead><TableHead className="w-10" />
  </TableRow></TableHeader><TableBody>{results.map((item) => {
    const jira = service === 'jira' ? item as JiraIssue : null; const gerrit = service === 'gerrit' ? item as GerritChange : null;
    const url = jira?.url ?? gerrit?.url ?? '';
    const safeUrl = safeExternalUrl(url);
    return <TableRow key={jira?.key ?? gerrit?.id}><TableCell className="font-mono text-xs">{jira?.key ?? gerrit?.id}</TableCell><TableCell className="max-w-lg truncate">{jira?.summary ?? gerrit?.subject}</TableCell><TableCell>{jira?.project ?? gerrit?.project}</TableCell><TableCell><Badge variant="outline">{jira?.status ?? gerrit?.status}</Badge></TableCell><TableCell>{jira?.assignee ?? gerrit?.owner}</TableCell><TableCell>{safeUrl ? <Button variant="ghost" size="icon-sm" title="Open" onClick={() => open(safeUrl)}><ExternalLink className="h-3.5 w-3.5" /></Button> : null}</TableCell></TableRow>;
  })}</TableBody></Table></div>;
}

function MetadataPanel({ metadata }: { metadata: JiraMetadata }) {
  const custom = metadata.fields.filter((field) => field.custom);
  return <section className="grid gap-3 md:grid-cols-2"><div className="rounded-md border p-3"><h3 className="text-sm font-semibold">Statuses ({metadata.statuses.length})</h3><div className="mt-2 flex flex-wrap gap-1">{metadata.statuses.map((status) => <Badge key={status.id} variant="outline">{status.name} · {status.category}</Badge>)}</div></div><div className="rounded-md border p-3"><h3 className="text-sm font-semibold">Custom fields ({custom.length})</h3><div className="mt-2 max-h-52 space-y-1 overflow-auto">{custom.map((field) => <div key={field.id} className="flex justify-between gap-3 text-xs"><span>{field.name}</span><code className="text-muted-foreground">{field.id}</code></div>)}</div></div></section>;
}

function ConnectionDialog({ service, open, connection, onOpenChange, onSaved }: { service: Service; open: boolean; connection: Connection | null; onOpenChange: (open: boolean) => void; onSaved: () => Promise<void> }) {
  const { t } = useTranslation();
  const initial = useMemo(() => ({ name: connection?.name ?? '', baseUrl: connection?.base_url ?? '', deployment: connection?.deployment ?? 'auto', authMethod: connection?.auth_method ?? (service === 'jira' ? 'basic' : 'http_password'), username: connection?.username ?? '', secret: '', profile: connection?.profile_json ?? EMPTY_PROFILE }), [connection, service, open]);
  const [form, setForm] = useState(initial);
  useEffect(() => setForm(initial), [initial]);
  const save = async () => {
    unwrap(await commands.saveDeveloperToolConnection({ id: connection?.id ?? null, name: form.name, service, base_url: form.baseUrl, deployment: form.deployment, auth_method: form.authMethod, username: form.username || null, secret: form.secret || null, profile_json: service === 'jira' ? form.profile : null }));
    await onSaved(); onOpenChange(false); toast.success(t('common.saved', '已保存'));
  };
  const test = async () => {
    if (!connection) { toast.info(t('devTools.saveBeforeTest', '请先保存连接')); return; }
    const result = unwrap(await commands.testDeveloperToolConnection(connection.id));
    toast.success(`${result.deployment}${result.version ? ` · ${result.version}` : ''}`);
  };
  return <Dialog open={open} onOpenChange={onOpenChange}><DialogContent size="lg"><DialogHeader><DialogTitle>{service === 'jira' ? 'Jira' : 'Gerrit'} {t('devTools.connection', '连接')}</DialogTitle></DialogHeader><div className="grid gap-3 py-2 md:grid-cols-2">
    <Field label={t('common.name', '名称')}><Input value={form.name} onChange={(e) => setForm({ ...form, name: e.target.value })} /></Field>
    <Field label="Base URL"><Input value={form.baseUrl} onChange={(e) => setForm({ ...form, baseUrl: e.target.value })} placeholder={service === 'jira' ? 'https://jira.example.com' : 'https://gerrit.example.com'} /></Field>
    <Field label={t('devTools.deployment', '部署类型')}><Select value={form.deployment} onValueChange={(deployment) => setForm({ ...form, deployment })}><SelectTrigger><SelectValue /></SelectTrigger><SelectContent><SelectItem value="auto">Auto</SelectItem>{service === 'jira' && <><SelectItem value="jira_cloud">Jira Cloud</SelectItem><SelectItem value="jira_data_center">Jira Data Center</SelectItem></>} {service === 'gerrit' && <SelectItem value="gerrit">Gerrit</SelectItem>}</SelectContent></Select></Field>
    <Field label={t('devTools.authMethod', '认证方式')}><Select value={form.authMethod} onValueChange={(authMethod) => setForm({ ...form, authMethod })}><SelectTrigger><SelectValue /></SelectTrigger><SelectContent>{service === 'jira' ? <><SelectItem value="basic">Basic</SelectItem><SelectItem value="api_token">Cloud API Token</SelectItem><SelectItem value="pat">PAT Bearer</SelectItem></> : <><SelectItem value="http_password">HTTP Password</SelectItem><SelectItem value="bearer">Bearer</SelectItem><SelectItem value="anonymous">Anonymous</SelectItem></>}</SelectContent></Select></Field>
    <Field label={t('devTools.username', '用户名')}><Input value={form.username} onChange={(e) => setForm({ ...form, username: e.target.value })} /></Field>
    <Field label={connection?.has_secret ? t('devTools.secretKeep', '凭证（留空保持不变）') : t('devTools.secret', '凭证')}><Input type="password" value={form.secret} onChange={(e) => setForm({ ...form, secret: e.target.value })} /></Field>
    {service === 'jira' && <div className="md:col-span-2"><Label>Semantic Profile JSON</Label><Textarea className="mt-1 min-h-64 font-mono text-xs" value={form.profile} onChange={(e) => setForm({ ...form, profile: e.target.value })} /></div>}
  </div><DialogFooter><Button variant="outline" onClick={() => test().catch((e) => toast.error(extractErrorMessage(e)))}>{t('devTools.testConnection', '测试连接')}</Button><Button onClick={() => save().catch((e) => toast.error(extractErrorMessage(e)))}>{t('common.save', '保存')}</Button></DialogFooter></DialogContent></Dialog>;
}

function Field({ label, children }: { label: string; children: ReactNode }) { return <div><Label>{label}</Label><div className="mt-1">{children}</div></div>; }
