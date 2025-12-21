# 架构概览

## 高层架构

NeuraDock 采用**领域驱动设计（DDD）**方法和**CQRS（命令查询职责分离）**模式构建，后端使用 Rust，前端使用 React。

## v0.5.0 架构改进

### 类型安全 IPC
引入 **tauri-specta** 实现 TypeScript 绑定自动生成，提供完整的类型安全保障。所有 Tauri 命令在编译时就能确保类型一致性，显著减少运行时错误。

### DDD 层解耦
应用层与基础设施层完全解耦，移除 sqlx 对上层的依赖。领域层现在是真正的纯业务逻辑，不依赖任何基础设施实现，提升代码可维护性和可测试性。

### 模块化重构
后端代码按职责拆分为多个专注的子模块，每个模块职责单一、边界清晰。例如：
- balance.rs → balance/query.rs, balance/update.rs, balance/history.rs
- check_in_executor.rs → check_in/executor.rs, check_in/validator.rs
- waf_bypass.rs → waf/browser_launcher.rs, waf/cookie_extractor.rs

```
┌─────────────────────────────────────────────────────────────┐
│                    前端 (React)                              │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────────────────┐ │
│  │    Pages    │ │ Components  │ │   TanStack Query        │ │
│  │ Dashboard   │ │ AccountCard │ │   (服务器状态)           │ │
│  │ Accounts    │ │ CheckInBtn  │ │                         │ │
│  │ Settings    │ │ Dialogs     │ │   Zustand (UI 状态)     │ │
│  └─────────────┘ └─────────────┘ └─────────────────────────┘ │
└───────────────────────────┬─────────────────────────────────┘
                            │ 类型安全 IPC (tauri-specta)
┌───────────────────────────▼─────────────────────────────────┐
│                    后端 (Rust/Tauri)                         │
│  ┌──────────────────────────────────────────────────────┐   │
│  │              表示层 (neuradock-app/presentation)      │   │
│  │  commands.rs  │  events.rs  │  state.rs              │   │
│  └──────────────────────────┬───────────────────────────┘   │
│                             │                                │
│  ┌──────────────────────────▼───────────────────────────┐   │
│  │         应用层 (neuradock-app/application)            │   │
│  │  Commands  │  Queries  │  DTOs  │  Services          │   │
│  │  (写操作)   │  (读操作)  │        │  CheckInExecutor   │   │
│  └──────────────────────────┬───────────────────────────┘   │
│                             │                                │
│  ┌──────────────────────────▼───────────────────────────┐   │
│  │           领域层 (neuradock-domain)                   │   │
│  │  Account │ Balance │ Session │ CheckIn │ Notification│   │
│  │  值对象  │ 仓储 Traits │ 领域事件 │ 领域错误          │   │
│  └──────────────────────────┬───────────────────────────┘   │
│                             │                                │
│  ┌──────────────────────────▼───────────────────────────┐   │
│  │       基础设施层 (neuradock-infrastructure)           │   │
│  │  SQLite仓储 │ HTTP客户端 │ WAF绕过 │ 通知服务         │   │
│  │  浏览器自动化 │ 加密服务 │ 事件总线 │ 监控           │   │
│  └──────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

## 分层职责

### 多 Crate 组织

NeuraDock 采用 Rust workspace 多 crate 架构，实现更好的模块化和职责分离：

- **neuradock-app**: 应用层和表示层
- **neuradock-domain**: 领域层（核心业务逻辑）
- **neuradock-infrastructure**: 基础设施层（外部集成）

### 表示层
- **位置**: `src-tauri/crates/neuradock-app/src/presentation/`
- **目的**: 处理 Tauri IPC 通信
- **组件**:
  - `commands.rs`: Tauri 命令处理器（IPC 端点）
  - `events.rs`: 前端通知的事件定义
  - `state.rs`: 应用状态管理（数据库、调度器）

### 应用层
- **位置**: `src-tauri/crates/neuradock-app/src/application/`
- **目的**: 编排业务操作
- **组件**:
  - `commands/`: 命令处理器（写操作）
  - `queries/`: 查询处理器（读操作）
  - `dtos/`: 跨层通信的数据传输对象
  - `services/`: 应用服务（CheckInExecutor、Scheduler）
  - `event_handlers/`: 领域事件处理器

### 领域层
- **位置**: `src-tauri/crates/neuradock-domain/src/`
- **目的**: 核心业务逻辑（框架无关）
- **组件**:
  - `account/`: Account 聚合（根实体、值对象、仓储 trait）
  - `balance/`: Balance 聚合（余额追踪）
  - `check_in/`: CheckIn 聚合、Provider 配置
  - `session/`: Session 聚合（会话管理）
  - `notification/`: Notification 聚合（通知管理）
  - `proxy_config/`: ProxyConfig 聚合（代理配置）
  - `provider_models/`: ProviderModels 聚合（Provider节点管理）
  - `waf_cookies/`: WafCookies 聚合（WAF绕过Cookie管理）
  - `shared/`: 共享值对象、ID 类型、错误
  - `events/`: 领域事件
  - `config/providers/`: 内置中转站（relay）配置 JSON（首次运行时自动写入数据库）

### 基础设施层
- **位置**: `src-tauri/crates/neuradock-infrastructure/src/`
- **目的**: 外部关注点和实现
- **组件**:
  - `persistence/`: SQLite 仓储实现
  - `http/`: HTTP 客户端、WAF 绕过逻辑
  - `browser/`: 浏览器自动化（chromiumoxide）
  - `notification/`: 通知服务（飞书、邮件等）
  - `security/`: 加密服务
  - `events/`: 事件总线实现
  - `monitoring/`: 性能监控
  - `migrations/`: 数据库迁移

## 关键设计模式

### 仓储模式
领域层定义仓储 trait（接口）：
```rust
// neuradock-domain/src/account/repository.rs
#[async_trait]
pub trait AccountRepository: Send + Sync {
    async fn find_by_id(&self, id: &AccountId) -> Result<Option<Account>, DomainError>;
    async fn save(&self, account: &Account) -> Result<(), DomainError>;
    async fn delete(&self, id: &AccountId) -> Result<(), DomainError>;
    // ...
}
```

基础设施层实现这些 trait：
```rust
// neuradock-infrastructure/src/persistence/repositories/account_repo.rs
impl AccountRepository for SqliteAccountRepository {
    // 使用 sqlx 实现
}
```

### 聚合模式
每个聚合是一个一致性边界：
- **Account**: 管理账号状态、凭证、自动签到配置、签到间隔设置
- **Balance**: 追踪账号余额和历史记录
- **Session**: 管理会话令牌和登录状态
- **CheckIn**: 表示一次签到执行及其状态
- **Notification**: 管理通知渠道和发送记录
- **ProxyConfig**: 管理应用代理配置（HTTP/HTTPS/SOCKS5）
- **ProviderModels**: 管理 Provider 的自定义节点配置
- **WafCookies**: 管理 WAF 绕过过程中获取的 Cookie 缓存

### 值对象
不可变的、经过验证的数据：
- `AccountId`、`ProviderId`: 类型安全的标识符
- `Credentials`: 带验证的 Cookie 存储

### 类型安全 IPC
使用 tauri-specta 自动生成 TypeScript 绑定：
```rust
#[tauri::command]
#[specta::specta]
pub async fn create_account(input: CreateAccountInput, state: State<'_, AppState>)
    -> Result<AccountDto, String> {
    // ...
}
```

在开发模式下，TypeScript 绑定会自动生成到 `src/lib/tauri.ts`，前端可以直接导入并获得完整的类型提示：

```typescript
import { create_account } from '@/lib/tauri';

// 完整的类型推导和检查
const account = await create_account({
  name: "test@example.com",
  provider_id: "anyrouter",
  cookies: { session: "token" },
  api_user: "user123"
});
```

## 数据流

### 签到流程
```
1. 前端: 用户点击"签到"按钮
        │
        ▼
2. IPC: 通过 tauri-specta 调用 execute_check_in(account_id)
        │
        ▼
3. 表示层: commands.rs 接收请求，加载 Account 和 Provider
        │
        ▼
4. 应用层: 使用 HTTP 客户端创建 CheckInExecutor
        │
        ▼
4.5 基础设施层: 如果启用了代理，配置 HTTP 客户端使用代理
        │
        ▼
5. 基础设施层: 如需要进行 WAF 绕过（浏览器自动化）
        │
        ▼
6. 基础设施层: 向服务商 API 发送 HTTP 请求（通过代理，如已配置）
        │
        ▼
7. 领域层: Account.record_check_in() 更新状态
        │
        ▼
8. 基础设施层: 保存到数据库
        │
        ▼
9. 表示层: 返回结果给前端
        │
        ▼
10. 前端: 通过 TanStack Query 失效刷新 UI
```

### 自动签到调度
```
1. 启动: 在 AppState::new() 中初始化调度器
        │
        ▼
2. 加载: scheduler.reload_schedules() 加载启用的账号
        │
        ▼
3. 调度: tokio-cron-scheduler 为每个账号创建任务
        │
        ▼
4. 触发: 在预定时间执行签到
        │
        ▼
5. 更新: 账号状态更新，结果记录日志
```

## 数据库 Schema

```sql
-- 账号表
accounts (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    provider_id TEXT NOT NULL,
    cookies TEXT NOT NULL,        -- JSON HashMap
    api_user TEXT,
    enabled INTEGER DEFAULT 1,
    auto_checkin_enabled INTEGER DEFAULT 0,
    auto_checkin_hour INTEGER DEFAULT 8,
    auto_checkin_minute INTEGER DEFAULT 0,
    check_in_interval_hours INTEGER DEFAULT 0,  -- v0.5.0: 可配置签到间隔（0表示无限制）
    last_check_in_at TEXT,
    created_at TEXT, updated_at TEXT
)

-- 会话表（独立管理）
sessions (
    id TEXT PRIMARY KEY,
    account_id TEXT REFERENCES accounts(id),
    session_token TEXT,
    last_login_at TEXT,
    expires_at TEXT,
    created_at TEXT
)

-- 余额表（独立管理）
balances (
    id TEXT PRIMARY KEY,
    account_id TEXT REFERENCES accounts(id),
    current_balance REAL,
    total_consumed REAL,
    total_quota REAL,           -- v0.5.0: 重命名自 total_income
    last_check_at TEXT,
    created_at TEXT, updated_at TEXT
)

-- 余额历史表
balance_history (
    id TEXT PRIMARY KEY,
    account_id TEXT REFERENCES accounts(id),
    current_balance REAL NOT NULL,
    total_consumed REAL NOT NULL,
    total_quota REAL NOT NULL,   -- v0.5.0: 重命名自 total_income
    recorded_at TEXT NOT NULL
)

-- 通知渠道表
notification_channels (
    id TEXT PRIMARY KEY,
    channel_type TEXT NOT NULL,  -- feishu/email/telegram
    config TEXT NOT NULL,         -- JSON 配置
    enabled INTEGER DEFAULT 1,
    created_at TEXT
)

-- 服务提供商表
providers (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    domain TEXT NOT NULL,
    login_path TEXT NOT NULL,
    sign_in_path TEXT,
    user_info_path TEXT NOT NULL,
    api_user_key TEXT NOT NULL,
    bypass_method TEXT,
    is_builtin INTEGER DEFAULT 0,
    created_at TEXT
)

-- 代理配置表（v0.5.0新增）
proxy_settings (
    id TEXT PRIMARY KEY,
    proxy_type TEXT NOT NULL,    -- http/https/socks5
    proxy_host TEXT NOT NULL,
    proxy_port INTEGER NOT NULL,
    enabled INTEGER DEFAULT 0,
    created_at TEXT, updated_at TEXT
)

-- Provider节点表（v0.5.0新增）
provider_models (
    id TEXT PRIMARY KEY,
    provider_id TEXT REFERENCES providers(id),
    node_url TEXT NOT NULL,
    description TEXT,
    created_at TEXT
)

-- WAF Cookie缓存表（v0.5.0新增）
waf_cookies (
    id TEXT PRIMARY KEY,
    provider_id TEXT REFERENCES providers(id),
    cookies TEXT NOT NULL,       -- JSON格式的Cookie
    created_at TEXT,
    expires_at TEXT
)
```

## 安全架构

**当前状态**（问题）：
- 凭证以明文存储在 SQLite 中
- API 响应包含敏感 cookies
- Tauri 未配置 CSP

**目标状态**（计划）：
- 静态凭证使用 AES-GCM 加密
- API 响应中排除凭证
- 正确配置 Tauri 安全设置

## 性能考虑

1. **余额缓存**: 1小时缓存减少 API 调用
2. **会话缓存**: 减少浏览器自动化开销
3. **查询优化**: 常用查询模式建立索引
4. **延迟加载**: 仅在过期时获取余额
5. **事件驱动**: 异步处理领域事件，提高响应速度
6. **连接池**: SQLite 连接池管理，优化数据库访问

## 相关文档

- [ADR-001: 使用 React 作为前端](./adr/001-use-react.md)
- [ADR-002: 数据库选型](./adr/002-database-selection.md)
- [ADR-003: DDD 架构](./adr/003-ddd-architecture.md)
- [ADR-004: CQRS 模式](./adr/004-cqrs-pattern.md)
- [ADR-005: WAF 绕过策略](./adr/005-waf-bypass-strategy.md)
