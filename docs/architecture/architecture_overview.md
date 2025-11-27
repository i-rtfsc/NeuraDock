# 架构概览

## 高层架构

NeuraDock 采用**领域驱动设计（DDD）**方法和**CQRS（命令查询职责分离）**模式构建，后端使用 Rust，前端使用 React。

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
│  │              表示层                                    │   │
│  │  commands.rs  │  events.rs  │  state.rs              │   │
│  └──────────────────────────┬───────────────────────────┘   │
│                             │                                │
│  ┌──────────────────────────▼───────────────────────────┐   │
│  │              应用层 (CQRS)                             │   │
│  │  Commands  │  Queries  │  DTOs  │  Services          │   │
│  │  (写操作)   │  (读操作)  │        │  CheckInExecutor   │   │
│  └──────────────────────────┬───────────────────────────┘   │
│                             │                                │
│  ┌──────────────────────────▼───────────────────────────┐   │
│  │              领域层 (核心)                             │   │
│  │  Account 聚合     │  CheckInJob 聚合                 │   │
│  │  值对象           │  仓储 Traits                     │   │
│  │  领域事件         │  领域错误                        │   │
│  └──────────────────────────┬───────────────────────────┘   │
│                             │                                │
│  ┌──────────────────────────▼───────────────────────────┐   │
│  │              基础设施层                               │   │
│  │  SQLite 仓储    │  HTTP 客户端  │  WAF 绕过          │   │
│  │  浏览器自动化   │  通知服务                          │   │
│  └──────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

## 分层职责

### 表示层
- **位置**: `src-tauri/src/presentation/`
- **目的**: 处理 Tauri IPC 通信
- **组件**:
  - `commands.rs`: Tauri 命令处理器（IPC 端点）
  - `events.rs`: 前端通知的事件定义
  - `state.rs`: 应用状态管理（数据库、调度器）

### 应用层
- **位置**: `src-tauri/src/application/`
- **目的**: 编排业务操作
- **组件**:
  - `commands/`: 命令处理器（写操作）
  - `queries/`: 查询处理器（读操作）
  - `dtos/`: 跨层通信的数据传输对象
  - `services/`: 应用服务（CheckInExecutor、Scheduler）

### 领域层
- **位置**: `src-tauri/src/domain/`
- **目的**: 核心业务逻辑（框架无关）
- **组件**:
  - `account/`: Account 聚合（根实体、值对象、仓储 trait）
  - `check_in/`: CheckInJob 聚合、Provider 配置
  - `shared/`: 共享值对象、ID 类型、错误
  - `events/`: 领域事件

### 基础设施层
- **位置**: `src-tauri/src/infrastructure/`
- **目的**: 外部关注点和实现
- **组件**:
  - `persistence/`: SQLite 仓储实现
  - `http/`: HTTP 客户端、WAF 绕过逻辑
  - `browser/`: 浏览器自动化（chromiumoxide）
  - `notification/`: 通知服务
  - `security/`: 加密（占位符）

## 关键设计模式

### 仓储模式
领域层定义仓储 trait（接口）：
```rust
// domain/account/repository.rs
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
// infrastructure/persistence/repositories/account_repo.rs
impl AccountRepository for SqliteAccountRepository {
    // 使用 sqlx 实现
}
```

### 聚合模式
每个聚合是一个一致性边界：
- **Account**: 管理账号状态、凭证、自动签到配置
- **CheckInJob**: 表示一次签到执行及其状态转换

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
5. 基础设施层: 如需要进行 WAF 绕过（浏览器自动化）
        │
        ▼
6. 基础设施层: 向服务商 API 发送 HTTP 请求
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
    -- 余额缓存
    quota REAL, used_quota REAL, remaining REAL,
    last_balance_check_at TEXT,
    -- 会话缓存
    session_token TEXT, session_expiry TEXT,
    -- 时间戳
    last_check_in_at TEXT,
    created_at TEXT, updated_at TEXT
)

check_in_jobs (
    id TEXT PRIMARY KEY,
    account_id TEXT REFERENCES accounts(id),
    provider_id TEXT,
    status TEXT,                  -- pending/running/completed/failed
    message TEXT,
    started_at TEXT, completed_at TEXT, created_at TEXT
)

balance_history (
    id TEXT PRIMARY KEY,
    account_id TEXT REFERENCES accounts(id),
    quota REAL, used_quota REAL, remaining REAL,
    recorded_at TEXT
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

## 相关文档

- [ADR-001: 使用 React 作为前端](./adr/001-use-react.md)
- [ADR-002: 数据库选型](./adr/002-database-selection.md)
- [ADR-003: DDD 架构](./adr/003-ddd-architecture.md)
- [ADR-004: CQRS 模式](./adr/004-cqrs-pattern.md)
- [ADR-005: WAF 绕过策略](./adr/005-waf-bypass-strategy.md)
