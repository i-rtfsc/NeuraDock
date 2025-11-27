# API 参考

## 概览

NeuraDock 使用 **Tauri IPC** 进行 React 前端和 Rust 后端之间的通信。所有命令都是类型安全的，TypeScript 绑定通过 **tauri-specta** 自动生成。

## 导入命令

```typescript
import { commands } from '@/lib/tauri';

// 调用命令
const accounts = await commands.getAllAccounts(false);
```

---

## 账号命令

### `createAccount`

创建新账号。

```typescript
interface CreateAccountInput {
  name: string;
  provider_id: string;
  cookies: Record<string, string>;
  api_user?: string;
  auto_checkin_enabled?: boolean;
  auto_checkin_hour?: number;
  auto_checkin_minute?: number;
}

async function createAccount(input: CreateAccountInput): Promise<AccountDto>
```

**示例：**
```typescript
const account = await commands.createAccount({
  name: 'user@example.com',
  provider_id: 'anyrouter',
  cookies: { session: 'token_value' },
  api_user: 'user_123',
  auto_checkin_enabled: true,
  auto_checkin_hour: 8,
  auto_checkin_minute: 0,
});
```

### `updateAccount`

更新现有账号。

```typescript
interface UpdateAccountInput {
  account_id: string;
  name?: string;
  cookies?: Record<string, string>;
  api_user?: string;
  auto_checkin_enabled?: boolean;
  auto_checkin_hour?: number;
  auto_checkin_minute?: number;
}

async function updateAccount(input: UpdateAccountInput): Promise<AccountDto>
```

### `deleteAccount`

通过 ID 删除账号。

```typescript
async function deleteAccount(accountId: string): Promise<void>
```

### `toggleAccount`

启用或禁用账号。

```typescript
async function toggleAccount(accountId: string, enabled: boolean): Promise<AccountDto>
```

### `getAllAccounts`

获取所有账号，可选按启用状态过滤。

```typescript
async function getAllAccounts(enabledOnly: boolean): Promise<AccountListDto[]>
```

**响应：**
```typescript
interface AccountListDto {
  id: string;
  name: string;
  provider_id: string;
  provider_name: string;
  enabled: boolean;
  auto_checkin_enabled: boolean;
  auto_checkin_hour: number;
  auto_checkin_minute: number;
  last_check_in_at?: string;
  created_at: string;
}
```

### `getAccountDetail`

获取账号详细信息，包括凭证。

```typescript
async function getAccountDetail(accountId: string): Promise<AccountDetailDto>
```

**响应：**
```typescript
interface AccountDetailDto {
  id: string;
  name: string;
  provider_id: string;
  provider_name: string;
  cookies: Record<string, string>;
  cookie_count: number;
  api_user?: string;
  enabled: boolean;
  auto_checkin_enabled: boolean;
  auto_checkin_hour: number;
  auto_checkin_minute: number;
  quota?: number;
  used_quota?: number;
  remaining?: number;
  last_check_in_at?: string;
  last_balance_check_at?: string;
  created_at: string;
  updated_at: string;
}
```

---

## 签到命令

### `executeCheckIn`

为单个账号执行签到。

```typescript
async function executeCheckIn(accountId: string): Promise<CheckInResultDto>
```

**响应：**
```typescript
interface CheckInResultDto {
  success: boolean;
  message: string;
  user_info?: UserInfoDto;
}

interface UserInfoDto {
  email?: string;
  quota?: number;
  used?: number;
  remaining?: number;
}
```

### `executeBatchCheckIn`

为多个账号执行签到。

```typescript
async function executeBatchCheckIn(accountIds: string[]): Promise<BatchCheckInResultDto>
```

**响应：**
```typescript
interface BatchCheckInResultDto {
  total: number;
  success: number;
  failed: number;
  results: CheckInResultItemDto[];
}

interface CheckInResultItemDto {
  account_id: string;
  account_name: string;
  success: boolean;
  message: string;
}
```

---

## 余额命令

### `fetchAccountBalance`

获取账号余额（如果新鲜则使用缓存）。

```typescript
async function fetchAccountBalance(accountId: string): Promise<BalanceDto>
```

**响应：**
```typescript
interface BalanceDto {
  quota?: number;
  used_quota?: number;
  remaining?: number;
  last_check_at?: string;
  is_fresh: boolean;
}
```

### `fetchAccountsBalances`

获取多个账号的余额。

```typescript
async function fetchAccountsBalances(accountIds: string[]): Promise<BatchBalanceResultDto>
```

---

## 导入/导出命令

### `importAccountFromJson`

从 JSON 导入单个账号。

```typescript
async function importAccountFromJson(jsonData: string): Promise<AccountDto>
```

### `importAccountsBatch`

从 JSON 数组批量导入账号。

```typescript
async function importAccountsBatch(jsonData: string): Promise<BatchImportResultDto>
```

**响应：**
```typescript
interface BatchImportResultDto {
  total: number;
  imported: number;
  failed: number;
  errors: string[];
}
```

### `exportAccountsToJson`

导出账号到 JSON。

```typescript
interface ExportOptions {
  account_ids?: string[];
  include_credentials: boolean;
}

async function exportAccountsToJson(options: ExportOptions): Promise<string>
```

### `updateAccountsBatch`

批量更新账号凭证。

```typescript
interface BatchUpdateInput {
  accounts: AccountUpdateItem[];
  create_if_not_exists: boolean;
}

interface AccountUpdateItem {
  name: string;
  provider: string;
  cookies: Record<string, string>;
  api_user?: string;
}

async function updateAccountsBatch(input: BatchUpdateInput): Promise<BatchUpdateResult>
```

**响应：**
```typescript
interface BatchUpdateResult {
  updated: number;
  created: number;
  failed: number;
  results: UpdateItemResult[];
}

interface UpdateItemResult {
  name: string;
  action: 'updated' | 'created' | 'failed';
  message?: string;
}
```

---

## 服务商命令

### `getBuiltinProviders`

获取内置服务商列表。

```typescript
async function getBuiltinProviders(): Promise<ProviderDto[]>
```

**响应：**
```typescript
interface ProviderDto {
  id: string;
  name: string;
  base_url: string;
  requires_waf_bypass: boolean;
}
```

---

## 事件

### 监听事件

```typescript
import { events } from '@/lib/tauri';

// 监听签到进度
const unlisten = await events.checkInProgress.listen((event) => {
  console.log(event.payload);
});

// 清理
unlisten();
```

### `checkInProgress`

签到执行期间发出。

```typescript
interface CheckInProgressEvent {
  account_id: string;
  status: 'started' | 'waf_bypass' | 'checking_in' | 'completed' | 'failed';
  message: string;
  progress: number; // 0-100
}
```

### `balanceUpdated`

账号余额更新时发出。

```typescript
interface BalanceUpdatedEvent {
  account_id: string;
  quota?: number;
  used_quota?: number;
  remaining?: number;
}
```

---

## 错误处理

所有命令返回 `Result<T, String>`。错误以字符串消息形式返回。

```typescript
try {
  const account = await commands.createAccount(input);
} catch (error) {
  // error 是后端返回的字符串消息
  console.error('创建账号失败:', error);
}
```

### 常见错误消息

| 错误 | 原因 |
|-----|------|
| "Account not found" | 无效的账号 ID |
| "Provider not found" | 无效的服务商 ID |
| "Invalid credentials" | 格式错误的 cookies 或 API 用户 |
| "WAF bypass failed" | 浏览器自动化失败 |
| "Network error" | 连接问题 |
| "Session expired" | 缓存的会话不再有效 |
