# 技术实现细节

本文档详细说明 NeuraDock 的技术实现细节，包括 DDD 各层实现、数据库设计、IPC 通信、浏览器自动化等核心技术。

## 目录

- [项目结构](#项目结构)
- [DDD 架构实现](#ddd-架构实现)
- [数据库设计](#数据库设计)
- [IPC 通信机制](#ipc-通信机制)
- [浏览器自动化](#浏览器自动化)
- [调度系统](#调度系统)
- [内置中转站配置](#内置中转站配置)
- [安全机制](#安全机制)
- [性能优化](#性能优化)

---

## 项目结构

### Rust Workspace 组织

```
apps/desktop/src-tauri/
├── Cargo.toml                 # Workspace 配置
├── crates/
│   ├── neuradock-domain/      # 领域层 (~3,500 行)
│   ├── neuradock-app/         # 应用层 + 表示层 (~5,200 行)
│   └── neuradock-infrastructure/  # 基础设施层 (~4,100 行)
└── migrations/                # 数据库迁移
```

### 代码规模统计

| 层次 | 代码行数 | 职责 |
|-----|---------|------|
| Domain | ~3,500 | 核心业务逻辑 |
| Application | ~5,200 | 命令/查询处理 |
| Infrastructure | ~4,100 | 外部集成 |
| **总计** | **~12,800** | 后端代码 |

---

## DDD 架构实现

### 1. 领域层 (neuradock-domain)

#### 目录结构

```
neuradock-domain/src/
├── account/              # 账号聚合
│   ├── aggregate.rs      # Account 聚合根
│   ├── repository.rs     # AccountRepository trait
│   └── value_objects.rs  # Credentials 值对象
├── balance/              # 余额聚合
│   ├── aggregate.rs      # Balance 聚合根
│   └── repository.rs     # BalanceRepository trait
├── check_in/             # 签到聚合
│   ├── aggregate.rs      # CheckInJob 聚合根
│   ├── provider.rs       # Provider 实体
│   └── repository.rs     # CheckInRepository trait
├── session/              # 会话聚合
│   ├── aggregate.rs      # Session 聚合根
│   └── repository.rs     # SessionRepository trait
├── notification/         # 通知聚合
│   ├── aggregate.rs      # NotificationChannel 聚合根
│   └── repository.rs     # NotificationRepository trait
├── token/                # Token 聚合
│   ├── aggregate.rs      # ApiToken 聚合根
│   └── repository.rs     # TokenRepository trait
├── custom_node/          # 自定义节点
├── shared/               # 共享类型
│   ├── ids.rs            # 类型安全 ID
│   ├── errors.rs         # DomainError
│   └── value_objects.rs  # 通用值对象
└── events/               # 领域事件
    └── mod.rs            # DomainEvent trait
```

> **内置中转站配置**：默认的 AnyRouter、AgentRouter 等中转站定义在 `apps/desktop/src-tauri/config/providers/builtin_providers.json` 中，应用首次运行且 `providers` 表为空时会自动写入数据库，后续如需新增内置中转站仅需更新该 JSON。

#### 核心聚合设计

**Account 聚合根**
```rust
pub struct Account {
    id: AccountId,
    name: String,
    provider_id: ProviderId,
    credentials: Credentials,
    api_user: String,
    enabled: bool,
    auto_checkin_config: AutoCheckinConfig,
    last_check_in: Option<DateTime<Utc>>,
}

impl Account {
    // 业务方法
    pub fn enable(&mut self) -> Result<(), DomainError>
    pub fn disable(&mut self) -> Result<(), DomainError>
    pub fn update_credentials(&mut self, credentials: Credentials)
    pub fn record_check_in(&mut self, result: CheckInResult)
    pub fn configure_auto_checkin(&mut self, hour: u8, minute: u8)
}
```

**值对象设计**
```rust
// 类型安全 ID
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AccountId(String);

// 凭证值对象
pub struct Credentials {
    cookies: HashMap<String, String>,
}

impl Credentials {
    pub fn new(cookies: HashMap<String, String>) -> Result<Self, DomainError> {
        // 验证逻辑
        if !cookies.contains_key("session") {
            return Err(DomainError::InvalidCredentials);
        }
        Ok(Self { cookies })
    }
}
```

#### 仓储模式

所有仓储接口定义在领域层，由基础设施层实现：

```rust
#[async_trait]
pub trait AccountRepository: Send + Sync {
    async fn find_by_id(&self, id: &AccountId) 
        -> Result<Option<Account>, DomainError>;
    
    async fn find_all(&self) 
        -> Result<Vec<Account>, DomainError>;
    
    async fn save(&self, account: &Account) 
        -> Result<(), DomainError>;
    
    async fn delete(&self, id: &AccountId) 
        -> Result<(), DomainError>;
    
    async fn find_by_provider(&self, provider_id: &ProviderId) 
        -> Result<Vec<Account>, DomainError>;
}
```

### 2. 应用层 (neuradock-app)

#### 目录结构

```
neuradock-app/src/
├── application/
│   ├── commands/         # 命令处理器 (写操作)
│   │   ├── account_commands.rs
│   │   ├── check_in_commands.rs
│   │   ├── notification_commands.rs
│   │   └── command_handler.rs
│   ├── queries/          # 查询处理器 (读操作)
│   │   ├── account_queries.rs
│   │   ├── balance_queries.rs
│   │   └── query_handler.rs
│   ├── services/         # 应用服务
│   │   ├── check_in_executor.rs
│   │   ├── scheduler.rs
│   │   └── balance_updater.rs
│   ├── dtos/             # 数据传输对象
│   │   ├── account_dto.rs
│   │   ├── balance_dto.rs
│   │   └── check_in_dto.rs
│   └── event_handlers/   # 领域事件处理器
│       └── mod.rs
└── presentation/         # 表示层 (Tauri IPC)
    ├── commands.rs       # Tauri 命令
    ├── events.rs         # 前端事件
    └── state.rs          # 应用状态
```

#### CQRS 实现

**命令 (Command) - 写操作**
```rust
pub struct CreateAccountCommand {
    pub name: String,
    pub provider_id: String,
    pub cookies: HashMap<String, String>,
    pub api_user: String,
    pub auto_checkin_enabled: bool,
    pub auto_checkin_hour: u8,
    pub auto_checkin_minute: u8,
}

impl CommandHandler<CreateAccountCommand> {
    pub async fn handle(&self, cmd: CreateAccountCommand) 
        -> Result<AccountId, ApplicationError> {
        // 1. 验证输入
        // 2. 创建领域对象
        // 3. 调用仓储保存
        // 4. 发布领域事件
    }
}
```

**查询 (Query) - 读操作**
```rust
pub struct GetAccountQuery {
    pub account_id: String,
}

impl QueryHandler<GetAccountQuery> {
    pub async fn handle(&self, query: GetAccountQuery) 
        -> Result<AccountDto, ApplicationError> {
        // 1. 验证查询参数
        // 2. 从仓储读取
        // 3. 转换为 DTO
        // 4. 返回结果
    }
}
```

#### 应用服务

**CheckInExecutor - 签到执行器**
```rust
pub struct CheckInExecutor {
    http_client: Arc<HttpClient>,
    session_repo: Arc<dyn SessionRepository>,
    waf_bypass: Arc<WafBypassService>,
}

impl CheckInExecutor {
    pub async fn execute_check_in(
        &self,
        account: &Account,
        provider: &Provider,
    ) -> Result<CheckInResult, ApplicationError> {
        // 1. 检查会话是否有效
        // 2. 如需要，执行 WAF 绕过
        // 3. 调用签到 API
        // 4. 更新余额
        // 5. 记录历史
    }
}
```

**AutoCheckInScheduler - 自动签到调度器**
```rust
pub struct AutoCheckInScheduler {
    account_repo: Arc<dyn AccountRepository>,
}

impl AutoCheckInScheduler {
    pub async fn reload_schedules(&self) {
        // 1. 加载所有启用自动签到的账号
        // 2. 为每个账号创建定时任务
        // 3. 计算下次执行时间
        // 4. 使用 tokio::spawn 异步调度
    }
    
    fn spawn_check_in_task(&self, account: Account) {
        tokio::spawn(async move {
            loop {
                // 计算等待时间
                let wait_duration = calculate_next_run(hour, minute);
                tokio::time::sleep(wait_duration).await;
                
                // 执行签到
                execute_check_in(&account).await;
            }
        });
    }
}
```

### 3. 基础设施层 (neuradock-infrastructure)

#### 目录结构

```
neuradock-infrastructure/src/
├── persistence/          # 数据持久化
│   ├── repositories/     # 仓储实现
│   │   ├── account_repo.rs
│   │   ├── balance_repo.rs
│   │   ├── session_repo.rs
│   │   └── token_repo.rs
│   └── db.rs             # 数据库连接
├── http/                 # HTTP 客户端
│   ├── client.rs         # HTTP 客户端
│   └── waf_bypass.rs     # WAF 绕过
├── browser/              # 浏览器自动化
│   └── mod.rs            # Chromium 控制
├── notification/         # 通知服务
│   ├── feishu.rs         # 飞书通知
│   └── email.rs          # 邮件通知
├── security/             # 安全服务
│   └── encryption.rs     # AES-GCM 加密
├── monitoring/           # 性能监控
│   └── metrics.rs        # 指标收集
└── config/               # 配置管理（包括 `providers/` 内置中转站 JSON）
    └── mod.rs            # 配置加载
```

#### 仓储实现

```rust
pub struct SqliteAccountRepository {
    pool: Arc<SqlitePool>,
}

#[async_trait]
impl AccountRepository for SqliteAccountRepository {
    async fn find_by_id(&self, id: &AccountId) 
        -> Result<Option<Account>, DomainError> {
        sqlx::query_as!(
            AccountRow,
            r#"
            SELECT id, name, provider_id, cookies, api_user,
                   enabled, auto_checkin_enabled, 
                   auto_checkin_hour, auto_checkin_minute,
                   last_check_in, created_at, updated_at
            FROM accounts
            WHERE id = ?
            "#,
            id.as_str()
        )
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| DomainError::RepositoryError(e.to_string()))?
        .map(|row| row.into_domain())
        .transpose()
    }
}
```

---

## 数据库设计

### Schema 概览

#### 核心表

**accounts - 账号表**
```sql
CREATE TABLE accounts (
    id TEXT PRIMARY KEY,                    -- UUID
    name TEXT NOT NULL,                     -- 账号名称
    provider_id TEXT NOT NULL,              -- 服务商 ID
    cookies TEXT NOT NULL,                  -- JSON: {"session": "xxx"}
    api_user TEXT NOT NULL,                 -- API 用户标识
    enabled BOOLEAN NOT NULL DEFAULT 1,     -- 是否启用
    
    -- 自动签到配置
    auto_checkin_enabled BOOLEAN NOT NULL DEFAULT 0,
    auto_checkin_hour INTEGER NOT NULL DEFAULT 9,
    auto_checkin_minute INTEGER NOT NULL DEFAULT 0,
    
    -- 会话缓存
    last_login_at TIMESTAMP,
    session_token TEXT,
    session_expires_at TIMESTAMP,
    
    -- 余额缓存
    last_balance_check_at TIMESTAMP,
    current_balance REAL,
    total_consumed REAL,
    total_income REAL,
    
    -- 签到记录
    last_check_in TIMESTAMP,
    
    -- 时间戳
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    
    FOREIGN KEY (provider_id) REFERENCES providers(id)
);
```

**providers - 服务提供商表**
```sql
CREATE TABLE providers (
    id TEXT PRIMARY KEY,                    -- 如 'anyrouter'
    name TEXT NOT NULL UNIQUE,              -- 'AnyRouter'
    domain TEXT NOT NULL,                   -- 'https://anyrouter.top'
    login_path TEXT NOT NULL,               -- '/login'
    sign_in_path TEXT,                      -- '/api/user/sign_in' (可选)
    user_info_path TEXT NOT NULL,           -- '/api/user/self'
    api_user_key TEXT NOT NULL,             -- 'new-api-user'
    bypass_method TEXT,                     -- 'waf_cookies' (可选)
    is_builtin BOOLEAN NOT NULL DEFAULT 0,  -- 是否内置
    created_at TIMESTAMP NOT NULL
);

-- 内置提供商
INSERT INTO providers VALUES
('anyrouter', 'AnyRouter', 'https://anyrouter.top', 
 '/login', '/api/user/sign_in', '/api/user/self', 
 'new-api-user', 'waf_cookies', 1, CURRENT_TIMESTAMP),
 
('agentrouter', 'AgentRouter', 'https://agentrouter.org',
 '/login', NULL, '/api/user/self',
 'new-api-user', NULL, 1, CURRENT_TIMESTAMP);
```

**api_tokens - API Token 表**
```sql
CREATE TABLE api_tokens (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    account_id TEXT NOT NULL,               -- 关联账号
    
    -- Token 信息
    token_id INTEGER NOT NULL,              -- API 返回的 token_id
    token_name TEXT NOT NULL,               -- Token 名称
    token_key TEXT NOT NULL,                -- API Key
    
    -- 状态和配额
    status INTEGER NOT NULL DEFAULT 1,      -- 1=启用 2=禁用 3=过期
    used_quota INTEGER NOT NULL DEFAULT 0,  -- 已用配额
    remain_quota INTEGER NOT NULL DEFAULT 0,-- 剩余配额
    unlimited_quota INTEGER NOT NULL DEFAULT 0, -- 无限配额标志
    
    -- 时间信息
    expired_time INTEGER,                   -- 过期时间 (-1=永不过期)
    
    -- 模型限制 (JSON)
    model_limits_allowed TEXT,              -- ["gpt-4", "claude-3"]
    model_limits_denied TEXT,               -- ["gpt-3.5"]
    model_limits_enabled BOOLEAN DEFAULT 0, -- 是否启用限制
    
    -- 缓存时间
    fetched_at TEXT NOT NULL,
    
    FOREIGN KEY (account_id) REFERENCES accounts(id) ON DELETE CASCADE,
    UNIQUE (account_id, token_id)
);
```

**provider_models - 提供商模型表**
```sql
CREATE TABLE provider_models (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    provider_id TEXT NOT NULL,
    models TEXT NOT NULL,                   -- JSON 数组
    fetched_at TEXT NOT NULL,
    UNIQUE(provider_id)
);
```

**check_in_jobs - 签到任务表**
```sql
CREATE TABLE check_in_jobs (
    id TEXT PRIMARY KEY,
    account_id TEXT NOT NULL,
    provider_id TEXT NOT NULL,
    status TEXT NOT NULL,                   -- pending/running/success/failed
    scheduled_at TIMESTAMP NOT NULL,
    started_at TIMESTAMP,
    completed_at TIMESTAMP,
    result_json TEXT,                       -- JSON 结果
    error TEXT,
    
    FOREIGN KEY (account_id) REFERENCES accounts(id) ON DELETE CASCADE,
    FOREIGN KEY (provider_id) REFERENCES providers(id)
);
```

**balance_history - 余额历史表**
```sql
CREATE TABLE balance_history (
    id TEXT PRIMARY KEY,
    account_id TEXT NOT NULL,
    current_balance REAL NOT NULL,
    total_consumed REAL NOT NULL,
    total_income REAL NOT NULL,
    recorded_at TIMESTAMP NOT NULL,
    
    FOREIGN KEY (account_id) REFERENCES accounts(id) ON DELETE CASCADE
);
```

**custom_nodes - 自定义节点表**
```sql
CREATE TABLE custom_nodes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    provider_id TEXT NOT NULL,
    name TEXT NOT NULL,
    base_url TEXT NOT NULL,
    created_at TEXT NOT NULL,
    
    UNIQUE(provider_id, name)
);
```

**waf_cookies - WAF Cookies 缓存表**
```sql
CREATE TABLE waf_cookies (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    domain TEXT NOT NULL UNIQUE,
    cookies TEXT NOT NULL,                  -- JSON cookies
    fetched_at TEXT NOT NULL,
    expires_at TEXT
);
```

**notification_channels - 通知渠道表**
```sql
CREATE TABLE notification_channels (
    id TEXT PRIMARY KEY,
    channel_type TEXT NOT NULL,             -- feishu/email/telegram
    config TEXT NOT NULL,                   -- JSON 配置
    enabled BOOLEAN NOT NULL DEFAULT 1,
    created_at TIMESTAMP NOT NULL
);
```

### 性能索引

```sql
-- 账号相关索引
CREATE INDEX idx_accounts_enabled 
    ON accounts(enabled) WHERE enabled = 1;

CREATE INDEX idx_accounts_provider 
    ON accounts(provider_id);

CREATE INDEX idx_accounts_auto_checkin 
    ON accounts(auto_checkin_enabled, auto_checkin_hour, auto_checkin_minute) 
    WHERE auto_checkin_enabled = 1;

-- 会话过期索引
CREATE INDEX idx_accounts_session_expiry 
    ON accounts(session_expires_at) 
    WHERE session_expires_at IS NOT NULL;

-- 余额检查索引
CREATE INDEX idx_accounts_balance_check 
    ON accounts(last_balance_check_at) 
    WHERE last_balance_check_at IS NOT NULL;

-- 签到任务索引
CREATE INDEX idx_jobs_account ON check_in_jobs(account_id);
CREATE INDEX idx_jobs_status ON check_in_jobs(status);
CREATE INDEX idx_jobs_scheduled ON check_in_jobs(scheduled_at);

-- 余额历史索引
CREATE INDEX idx_balance_account_time 
    ON balance_history(account_id, recorded_at DESC);

-- Token 索引
CREATE INDEX idx_api_tokens_account_id ON api_tokens(account_id);
CREATE INDEX idx_api_tokens_status ON api_tokens(status);
```

### 数据库迁移

使用 sqlx 迁移系统，迁移文件按时间顺序组织：

```
migrations/
├── 20250121000001_initial_schema.sql
├── 20250129000001_separate_session_balance.sql
├── 20250130000001_add_performance_indexes.sql
├── 20250201000001_remove_legacy_account_fields.sql
├── 20251202000001_add_api_tokens.sql
├── 20251203000001_add_custom_nodes.sql
├── 20251204000001_add_provider_models.sql
├── 20251204000002_add_waf_cookies.sql
└── 20251205000002_fix_model_limits_enabled.sql
```

---

## IPC 通信机制

### tauri-specta 类型安全 IPC

#### 命令定义

```rust
// Rust 端定义
#[tauri::command]
#[specta::specta]  // 自动生成 TypeScript 类型
pub async fn create_account(
    input: CreateAccountInput,
    state: State<'_, AppState>,
) -> Result<String, String> {
    // 命令实现
}

#[derive(Serialize, Deserialize, specta::Type)]
pub struct CreateAccountInput {
    pub name: String,
    pub provider_id: String,
    pub cookies: HashMap<String, String>,
    pub api_user: String,
    pub auto_checkin_enabled: bool,
    pub auto_checkin_hour: u8,
    pub auto_checkin_minute: u8,
}
```

#### TypeScript 绑定生成

构建时自动生成：
```typescript
// src/lib/tauri-commands.ts (自动生成)
export namespace commands {
  export function createAccount(
    input: CreateAccountInput
  ): Promise<string>;
  
  export function getAllAccounts(
    enabledOnly: boolean
  ): Promise<AccountDto[]>;
  
  // ... 其他命令
}

export interface CreateAccountInput {
  name: string;
  provider_id: string;
  cookies: Record<string, string>;
  api_user: string;
  auto_checkin_enabled: boolean;
  auto_checkin_hour: number;
  auto_checkin_minute: number;
}
```

#### 前端调用

```typescript
import { commands } from '@/lib/tauri-commands';

// 类型安全的调用
const account = await commands.createAccount({
  name: 'user@example.com',
  provider_id: 'anyrouter',
  cookies: { session: 'token' },
  api_user: 'user_id',
  auto_checkin_enabled: true,
  auto_checkin_hour: 8,
  auto_checkin_minute: 0,
});
```

### 事件系统

#### 后端发送事件

```rust
use tauri::Manager;

// 发送事件到前端
app_handle.emit_all("check_in_completed", CheckInEvent {
    account_id: account.id().to_string(),
    success: true,
    message: "签到成功".to_string(),
})?;
```

#### 前端监听事件

```typescript
import { listen } from '@tauri-apps/api/event';

// 监听签到完成事件
const unlisten = await listen<CheckInEvent>(
  'check_in_completed',
  (event) => {
    console.log('签到完成:', event.payload);
    // 更新 UI
  }
);
```

### 应用状态管理

```rust
pub struct AppState {
    pub db: Arc<SqlitePool>,
    pub command_handlers: Arc<CommandHandlers>,
    pub query_handlers: Arc<QueryHandlers>,
    pub scheduler: Arc<AutoCheckInScheduler>,
    pub http_client: Arc<HttpClient>,
}

impl AppState {
    pub async fn new(db_path: &str) -> Result<Self, AppError> {
        // 1. 初始化数据库连接池
        let db = create_pool(db_path).await?;
        
        // 2. 运行迁移
        sqlx::migrate!("../migrations").run(&db).await?;
        
        // 3. 初始化仓储
        let account_repo = Arc::new(SqliteAccountRepository::new(db.clone()));
        
        // 4. 初始化服务
        let scheduler = Arc::new(AutoCheckInScheduler::new(account_repo).await?);
        
        // 5. 启动调度器
        scheduler.start().await?;
        
        Ok(Self { db, scheduler, ... })
    }
}
```

---

## 浏览器自动化

### WAF 绕过实现

#### 架构设计

NeuraDock 使用 `chromiumoxide` 库进行浏览器自动化，绕过 Cloudflare 等 WAF 保护。

#### 浏览器检测

```rust
fn find_browser() -> Option<PathBuf> {
    let browser_paths = vec![
        // macOS
        "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
        "/Applications/Brave Browser.app/Contents/MacOS/Brave Browser",
        "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
        
        // Linux
        "/usr/bin/google-chrome",
        "/usr/bin/chromium",
        "/usr/bin/brave-browser",
        
        // Windows (通过注册表和环境变量检测)
        "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe",
        "%LOCALAPPDATA%\\Google\\Chrome\\Application\\chrome.exe",
    ];
    
    for path in browser_paths {
        if PathBuf::from(path).exists() {
            return Some(PathBuf::from(path));
        }
    }
    
    None
}
```

#### WAF Cookie 获取

```rust
pub async fn get_waf_cookies(
    domain: &str,
    headless: bool,
) -> Result<HashMap<String, String>> {
    // 1. 检查缓存
    if let Some(cached) = check_waf_cache(domain).await? {
        return Ok(cached);
    }
    
    // 2. 启动浏览器
    let browser = Browser::new(
        BrowserConfig::builder()
            .chrome_executable(find_browser()?)
            .with_head()  // 非无头模式通过率更高
            .viewport(Some(Viewport {
                width: 1920,
                height: 1080,
            }))
            .build()?
    ).await?;
    
    // 3. 创建页面
    let page = browser.new_page("about:blank").await?;
    
    // 4. 设置 User-Agent
    page.set_user_agent(USER_AGENT).await?;
    
    // 5. 访问目标域名
    page.goto(domain).await?;
    
    // 6. 等待 WAF 验证完成
    tokio::time::sleep(Duration::from_secs(5)).await;
    
    // 7. 获取 Cookies
    let cookies = page.get_cookies().await?;
    
    // 8. 提取 WAF Cookies
    let waf_cookies: HashMap<String, String> = cookies
        .iter()
        .filter(|c| REQUIRED_WAF_COOKIES.contains(&c.name.as_str()))
        .map(|c| (c.name.clone(), c.value.clone()))
        .collect();
    
    // 9. 缓存 Cookies
    cache_waf_cookies(domain, &waf_cookies).await?;
    
    // 10. 清理浏览器
    browser.close().await?;
    
    Ok(waf_cookies)
}

const REQUIRED_WAF_COOKIES: &[&str] = &[
    "acw_tc",      // 阿里云 WAF
    "cdn_sec_tc",  // CDN 安全
    "acw_sc__v2",  // 阿里云 WAF v2
];
```

#### Cookie 缓存策略

```rust
async fn check_waf_cache(domain: &str) -> Result<Option<HashMap<String, String>>> {
    let row = sqlx::query!(
        "SELECT cookies, expires_at FROM waf_cookies WHERE domain = ?",
        domain
    )
    .fetch_optional(&pool)
    .await?;
    
    if let Some(row) = row {
        // 检查是否过期
        if let Some(expires_at) = row.expires_at {
            let expires = DateTime::parse_from_rfc3339(&expires_at)?;
            if Utc::now() < expires.into() {
                return Ok(Some(serde_json::from_str(&row.cookies)?));
            }
        }
    }
    
    Ok(None)
}

async fn cache_waf_cookies(
    domain: &str, 
    cookies: &HashMap<String, String>
) -> Result<()> {
    let expires_at = Utc::now() + Duration::hours(6);  // 6小时过期
    
    sqlx::query!(
        "INSERT OR REPLACE INTO waf_cookies (domain, cookies, fetched_at, expires_at)
         VALUES (?, ?, ?, ?)",
        domain,
        serde_json::to_string(cookies)?,
        Utc::now().to_rfc3339(),
        expires_at.to_rfc3339()
    )
    .execute(&pool)
    .await?;
    
    Ok(())
}
```

### HTTP 客户端集成

```rust
pub struct HttpClient {
    client: reqwest::Client,
    waf_bypass: Arc<WafBypassService>,
}

impl HttpClient {
    pub async fn request_with_waf_bypass(
        &self,
        account: &Account,
        url: &str,
        bypass_method: Option<&str>,
    ) -> Result<Response> {
        let mut cookies = account.credentials().cookies().clone();
        
        // 如果需要 WAF 绕过
        if bypass_method == Some("waf_cookies") {
            let domain = extract_domain(url)?;
            let waf_cookies = self.waf_bypass
                .get_waf_cookies(domain, true)
                .await?;
            
            // 合并 WAF Cookies
            cookies.extend(waf_cookies);
        }
        
        // 构建请求
        self.client
            .get(url)
            .header("User-Agent", USER_AGENT)
            .header("Cookie", format_cookies(&cookies))
            .send()
            .await
    }
}
```

---

## 调度系统

### 自动签到调度实现

#### 基于 Tokio 的调度系统

```rust
pub struct AutoCheckInScheduler {
    account_repo: Arc<dyn AccountRepository>,
}

impl AutoCheckInScheduler {
    pub async fn reload_schedules(
        &self,
        providers: HashMap<String, Provider>,
        account_repo: Arc<dyn AccountRepository>,
        app_handle: tauri::AppHandle,
    ) -> Result<()> {
        // 1. 获取所有启用自动签到的账号
        let accounts = account_repo.find_all().await?;
        
        let mut scheduled_count = 0;
        for account in accounts {
            if account.auto_checkin_enabled() && account.is_enabled() {
                let provider_id = account.provider_id().as_str();
                
                if let Some(provider) = providers.get(provider_id) {
                    // 2. 为每个账号创建独立的定时任务
                    self.spawn_check_in_task(
                        account.id().clone(),
                        account.name().to_string(),
                        account.auto_checkin_hour(),
                        account.auto_checkin_minute(),
                        provider.clone(),
                        account_repo.clone(),
                        app_handle.clone(),
                    );
                    scheduled_count += 1;
                }
            }
        }
        
        info!("✅ Scheduled {} auto check-in jobs", scheduled_count);
        Ok(())
    }
}
```

#### 定时任务实现

```rust
fn spawn_check_in_task(
    &self,
    account_id: AccountId,
    account_name: String,
    hour: u8,
    minute: u8,
    provider: Provider,
    account_repo: Arc<dyn AccountRepository>,
    app_handle: tauri::AppHandle,
) {
    tokio::spawn(async move {
        loop {
            // 1. 计算下次执行时间
            let now = Local::now();
            let target_hour = hour as u32;
            let target_minute = minute as u32;
            
            let mut next_run = now
                .date_naive()
                .and_hms_opt(target_hour, target_minute, 0)
                .unwrap()
                .and_local_timezone(now.timezone())
                .unwrap();
            
            // 如果今天的时间已过，调度到明天
            if next_run <= now {
                next_run = next_run + chrono::Duration::days(1);
            }
            
            // 2. 计算等待时长
            let wait_duration = (next_run - now)
                .to_std()
                .unwrap_or(std::time::Duration::from_secs(0));
            
            info!(
                "⏰ Next check-in for '{}' at {} (in {} seconds)",
                account_name,
                next_run.format("%Y-%m-%d %H:%M:%S"),
                wait_duration.as_secs()
            );
            
            // 3. 等待到执行时间
            tokio::time::sleep(wait_duration).await;
            
            // 4. 执行签到
            info!("🚀 Executing auto check-in for '{}'", account_name);
            
            match execute_auto_check_in(
                &account_id,
                &provider,
                &account_repo,
                &app_handle,
            ).await {
                Ok(_) => info!("✅ Auto check-in success for '{}'", account_name),
                Err(e) => error!("❌ Auto check-in failed for '{}': {}", account_name, e),
            }
            
            // 5. 发送事件到前端
            let _ = app_handle.emit_all("auto_check_in_completed", json!({
                "account_id": account_id.as_str(),
                "account_name": account_name,
                "timestamp": Utc::now().to_rfc3339(),
            }));
        }
    });
}
```

#### 时区处理

```rust
use chrono::{Local, Utc, TimeZone};

// 使用本地时区
let local_time = Local::now();
info!("Local time: {}", local_time.format("%Y-%m-%d %H:%M:%S %Z"));

// 转换为 UTC 存储
let utc_time = local_time.with_timezone(&Utc);

// 用户配置的时间是本地时区
let user_hour = 8;  // 用户配置：上午 8 点
let local_scheduled = Local::today()
    .and_hms_opt(user_hour, 0, 0)
    .unwrap();
```

---

## 内置中转站配置

### JSON 驱动的默认服务商

- **配置文件**：`apps/desktop/src-tauri/config/providers/builtin_providers.json`
- **内容**：一个数组，每项定义了中转站的 `id`、`name`、`domain`、`login_path`、`sign_in_path`、`user_info_path`、`token_api_path`、`models_path`、`api_user_key`、`bypass_method` 等字段。
- **维护方式**：新增或调整默认中转站只需修改该 JSON，不再需要单独的 Rust 插件代码。

```jsonc
[
  {
    "id": "anyrouter",
    "name": "AnyRouter",
    "domain": "https://anyrouter.top",
    "login_path": "/login",
    "sign_in_path": "/api/user/sign_in",
    "user_info_path": "/api/user/self",
    "token_api_path": "/api/token/",
    "models_path": "/api/user/models",
    "api_user_key": "new-api-user",
    "bypass_method": "waf_cookies"
  }
]
```

> JSON 仅示例一项，其余内置中转站同样在该文件维护。

### 首次运行自动写入数据库

1. `AppState::new` 初始化 `SqliteProviderRepository` 后调用 `seed_builtin_providers`。
2. Seeder 检查 `providers` 表是否为空；若为空则解析 JSON 并使用 `Provider::builtin` 构造实体。
3. 通过 `ProviderRepository::save` 持久化，并设置 `is_builtin = true`。
4. 这样前端 Provider 管理与账号页面在首次启动时即可看到默认中转站，同时仍支持用户自定义增删。

### 运行期行为

- 所有 HTTP/WAF/token/model 逻辑直接读取数据库中的 Provider 配置，彻底去除对具体服务商的硬编码。
- 新增内置中转站流程：编辑 JSON → 打包发布；无需再编写/注册插件。

---

## 安全机制

### 凭证加密 (计划中)

#### AES-GCM 加密

```rust
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};

pub struct EncryptionService {
    cipher: Aes256Gcm,
}

impl EncryptionService {
    pub fn new(key: &[u8; 32]) -> Self {
        let cipher = Aes256Gcm::new(key.into());
        Self { cipher }
    }
    
    pub fn encrypt(&self, plaintext: &str) -> Result<Vec<u8>> {
        let nonce = Nonce::from_slice(b"unique nonce");
        let ciphertext = self.cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|e| AppError::EncryptionError(e.to_string()))?;
        Ok(ciphertext)
    }
    
    pub fn decrypt(&self, ciphertext: &[u8]) -> Result<String> {
        let nonce = Nonce::from_slice(b"unique nonce");
        let plaintext = self.cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| AppError::DecryptionError(e.to_string()))?;
        Ok(String::from_utf8(plaintext)?)
    }
}
```

### 密钥管理

```rust
// 从系统 keyring 获取加密密钥
fn get_encryption_key() -> Result<[u8; 32]> {
    // 使用 OS keyring (macOS Keychain, Windows Credential Manager)
    let key = keyring::get("neuradock", "encryption_key")?;
    
    // 或从用户派生
    use argon2::Argon2;
    let mut key = [0u8; 32];
    Argon2::default()
        .hash_password_into(user_password.as_bytes(), &salt, &mut key)?;
    
    Ok(key)
}
```

---

## 性能优化

### 1. 数据库连接池

```rust
pub async fn create_pool(db_path: &str) -> Result<SqlitePool> {
    SqlitePoolOptions::new()
        .max_connections(5)          // 最大连接数
        .min_connections(1)          // 最小连接数
        .acquire_timeout(Duration::from_secs(30))
        .connect(&format!("sqlite://{}?mode=rwc", db_path))
        .await
}
```

### 2. 余额缓存策略

```rust
const BALANCE_CACHE_TTL: Duration = Duration::hours(1);

pub async fn get_balance_with_cache(
    account: &Account,
    force_refresh: bool,
) -> Result<BalanceInfo> {
    // 检查缓存是否有效
    if !force_refresh {
        if let Some(last_check) = account.last_balance_check_at() {
            let age = Utc::now() - last_check;
            if age < BALANCE_CACHE_TTL {
                // 返回缓存数据
                return Ok(BalanceInfo {
                    current_balance: account.current_balance(),
                    total_consumed: account.total_consumed(),
                    total_income: account.total_income(),
                    cached: true,
                });
            }
        }
    }
    
    // 从 API 获取最新数据
    let balance = fetch_balance_from_api(account).await?;
    
    // 更新缓存
    update_balance_cache(account, &balance).await?;
    
    Ok(balance)
}
```

### 3. 会话缓存

```rust
pub async fn get_valid_session(
    account: &Account,
) -> Result<Option<String>> {
    if let Some(token) = account.session_token() {
        if let Some(expires_at) = account.session_expires_at() {
            if Utc::now() < expires_at {
                // 会话未过期
                return Ok(Some(token.to_string()));
            }
        }
    }
    
    // 会话过期或不存在
    Ok(None)
}
```

### 4. 异步并发

```rust
// 批量签到 - 并发执行
pub async fn batch_check_in(
    account_ids: Vec<AccountId>,
) -> Result<Vec<CheckInResult>> {
    let tasks: Vec<_> = account_ids
        .into_iter()
        .map(|id| {
            tokio::spawn(async move {
                execute_check_in(id).await
            })
        })
        .collect();
    
    // 并发等待所有任务完成
    let results = futures::future::join_all(tasks).await;
    
    results
        .into_iter()
        .map(|r| r??)
        .collect()
}
```

### 5. 索引优化

```sql
-- 部分索引 - 只索引启用的账号
CREATE INDEX idx_accounts_enabled 
    ON accounts(enabled) 
    WHERE enabled = 1;

-- 复合索引 - 自动签到查询
CREATE INDEX idx_accounts_auto_checkin 
    ON accounts(auto_checkin_enabled, auto_checkin_hour, auto_checkin_minute) 
    WHERE auto_checkin_enabled = 1;

-- 覆盖索引 - 包含常用字段
CREATE INDEX idx_balance_account_time 
    ON balance_history(account_id, recorded_at DESC);
```

---

## 技术栈总结

### 后端 (Rust)

| 类别 | 技术 | 版本 | 用途 |
|-----|------|------|------|
| **框架** | Tauri | 2.1 | 桌面应用框架 |
| **运行时** | Tokio | 1.41 | 异步运行时 |
| **数据库** | SQLx + SQLite | 0.8 | ORM + 数据库 |
| **HTTP** | reqwest | 0.12 | HTTP 客户端 |
| **浏览器** | chromiumoxide | 0.7 | 浏览器自动化 |
| **调度** | tokio-cron-scheduler | 0.13 | 定时任务 |
| **IPC** | tauri-specta | 2.0-rc.20 | 类型安全 IPC |
| **序列化** | serde + serde_json | 1.0 | 序列化/反序列化 |
| **时间** | chrono | 0.4 | 日期时间处理 |
| **错误处理** | thiserror + anyhow | 2.0 + 1.0 | 错误定义和处理 |
| **日志** | tracing | 0.1 | 结构化日志 |
| **加密** | aes-gcm + argon2 | 0.10 + 0.5 | AES加密 + 密钥派生 |
| **邮件** | lettre | 0.11 | SMTP 邮件发送 |
| **UUID** | uuid | 1.11 | UUID 生成 |

### 前端 (React)

| 类别 | 技术 | 版本 | 用途 |
|-----|------|------|------|
| **框架** | React | 18 | UI 框架 |
| **语言** | TypeScript | 5 | 类型安全 |
| **构建** | Vite | 6 | 构建工具 |
| **路由** | React Router | 7 | 路由管理 |
| **状态** | Zustand | 5 | 轻量状态管理 |
| **服务器状态** | TanStack Query | 5 | 数据获取和缓存 |
| **UI组件** | Radix UI | - | 无障碍组件库 |
| **样式** | Tailwind CSS | 3 | 实用优先CSS |
| **图标** | Lucide React | - | 图标库 |
| **图表** | Recharts | 2 | 数据可视化 |
| **表单** | React Hook Form + Zod | 7 + 4 | 表单管理 + 验证 |
| **i18n** | react-i18next | 16 | 国际化 |
| **通知** | Sonner | 1 | Toast 通知 |
| **日期** | date-fns | 4 | 日期处理 |

### 开发工具

| 工具 | 版本 | 用途 |
|-----|------|------|
| **pnpm** | - | 包管理器 |
| **rustfmt** | - | Rust 代码格式化 |
| **clippy** | - | Rust 代码检查 |
| **sqlx-cli** | - | 数据库迁移 |
| **cargo-tarpaulin** | - | 测试覆盖率 |

---

## 补充技术细节

### 1. 错误处理策略

```rust
// 领域错误
#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    #[error("Account not found: {0}")]
    AccountNotFound(String),
    
    #[error("Invalid credentials")]
    InvalidCredentials,
    
    #[error("Repository error: {0}")]
    RepositoryError(String),
}

// 应用错误
#[derive(Debug, thiserror::Error)]
pub enum ApplicationError {
    #[error("Domain error: {0}")]
    Domain(#[from] DomainError),
    
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

// 转换为前端友好的错误消息
impl From<ApplicationError> for String {
    fn from(err: ApplicationError) -> Self {
        match err {
            ApplicationError::Domain(DomainError::AccountNotFound(_)) => {
                "账号不存在".to_string()
            }
            ApplicationError::Http(_) => {
                "网络请求失败".to_string()
            }
            _ => "操作失败".to_string(),
        }
    }
}
```

### 2. 日志系统

```rust
use tracing::{info, warn, error, debug, instrument};

#[instrument(skip(self, account))]
pub async fn execute_check_in(&self, account: &Account) -> Result<()> {
    info!("Starting check-in for account: {}", account.name());
    
    debug!("Fetching WAF cookies");
    let waf_cookies = self.get_waf_cookies().await?;
    
    info!("Calling check-in API");
    let response = self.call_api(&waf_cookies).await?;
    
    if response.success {
        info!("Check-in successful");
    } else {
        warn!("Check-in failed: {}", response.message);
    }
    
    Ok(())
}
```

### 3. 事件驱动架构

```rust
// 领域事件
pub enum DomainEvent {
    AccountCreated { account_id: AccountId },
    CheckInCompleted { account_id: AccountId, result: CheckInResult },
    BalanceUpdated { account_id: AccountId, balance: BalanceInfo },
}

// 事件发布
pub trait EventPublisher: Send + Sync {
    async fn publish(&self, event: DomainEvent) -> Result<()>;
}

// 事件处理器
#[async_trait]
pub trait EventHandler: Send + Sync {
    async fn handle(&self, event: &DomainEvent) -> Result<()>;
}
```

### 4. 测试策略

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    // 单元测试 - 领域逻辑
    #[test]
    fn test_account_enable() {
        let mut account = Account::new(...);
        assert!(!account.is_enabled());
        
        account.enable().unwrap();
        assert!(account.is_enabled());
    }
    
    // 集成测试 - 仓储
    #[tokio::test]
    async fn test_account_repository() {
        let pool = create_test_pool().await;
        let repo = SqliteAccountRepository::new(pool);
        
        let account = Account::new(...);
        repo.save(&account).await.unwrap();
        
        let found = repo.find_by_id(account.id()).await.unwrap();
        assert_eq!(found.unwrap().name(), account.name());
    }
}
```

---

## 相关文档

- [架构概览](./architecture_overview.md) - 高层架构设计
- [ADR-003: DDD 架构](./adr/003-ddd-architecture.md) - DDD 决策记录
- [ADR-004: CQRS 模式](./adr/004-cqrs-pattern.md) - CQRS 决策记录
- [ADR-005: WAF 绕过策略](./adr/005-waf-bypass-strategy.md) - WAF 绕过设计
- [API 参考](../api/api_reference.md) - IPC 命令文档
