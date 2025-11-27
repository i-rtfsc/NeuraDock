# ADR-003: DDD 架构

## 状态
已采纳

## 背景
NeuraDock 具有复杂的业务逻辑：
- 带凭证的账号管理
- 带重试逻辑的签到执行
- 余额追踪和缓存
- 基于时间触发的自动调度

我们需要一个架构来：
- 将业务逻辑与基础设施关注点分离
- 使代码库随着增长保持可维护性
- 允许独立测试业务逻辑
- 提供清晰的关注点边界

## 决策
我们采用了**领域驱动设计（DDD）**的 4 层架构：

```
┌─────────────────────────────────────┐
│  表示层 (Tauri IPC)                  │
└─────────────────────────────────────┘
                  ↓
┌─────────────────────────────────────┐
│  应用层 (编排)                       │
└─────────────────────────────────────┘
                  ↓
┌─────────────────────────────────────┐
│  领域层 (业务逻辑)                   │
└─────────────────────────────────────┘
                  ↓
┌─────────────────────────────────────┐
│  基础设施层 (外部)                   │
└─────────────────────────────────────┘
```

### 分层规则
1. **领域层**不依赖其他层
2. 依赖向内流动（表示层 → 应用层 → 领域层 ← 基础设施层）
3. 领域层定义仓储 trait，基础设施层实现
4. 使用 DTO 进行跨层通信

## 后果

### 优点
- 业务逻辑隔离在领域层
- 无需 mock 基础设施即可轻松测试领域逻辑
- 清晰的关注点分离
- 框架无关的领域（可移植到不同 UI）
- 仓储模式支持更换存储实现

### 缺点
- 更多样板代码（DTO、仓储 trait）
- 对 DDD 新手有学习曲线
- 对简单功能可能过度工程化
- 需要纪律来维护层边界

### 关键聚合
- **Account**: 管理账号状态、凭证、余额缓存的根实体
- **CheckInJob**: 表示带状态机的签到执行

### 实现示例

```rust
// 领域层 - 纯业务逻辑
impl Account {
    pub fn record_check_in(&mut self) {
        self.last_check_in_at = Some(Utc::now());
    }

    pub fn is_balance_stale(&self, max_age_hours: i64) -> bool {
        // 纯业务逻辑，无基础设施依赖
    }
}

// 领域层 - 仓储 trait
#[async_trait]
pub trait AccountRepository: Send + Sync {
    async fn find_by_id(&self, id: &AccountId) -> Result<Option<Account>, DomainError>;
    async fn save(&self, account: &Account) -> Result<(), DomainError>;
}

// 基础设施层 - 实现
impl AccountRepository for SqliteAccountRepository {
    async fn find_by_id(&self, id: &AccountId) -> Result<Option<Account>, DomainError> {
        // SQLite 特定实现
    }
}
```

### 考虑过的替代方案
- **Clean Architecture**: 类似但层更多（用例、实体等）
- **六边形架构**: 端口和适配器 - DDD 类似但更关注领域
- **简单 MVC**: 可行但随着复杂度增长更难维护

## 相关
- [架构概览](../architecture_overview.md)
- [ADR-004: CQRS 模式](./004-cqrs-pattern.md)
