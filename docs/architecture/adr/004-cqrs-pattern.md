# ADR-004: CQRS 模式

## 状态
已采纳（部分实现）

## 背景
NeuraDock 对以下操作有不同的需求：
- **命令**（写）：创建账号、执行签到、更新设置
- **查询**（读）：列出账号、获取余额、查看历史

这些操作有不同的：
- 性能特性
- 验证需求
- 副作用

## 决策
我们在应用层采用了 **CQRS（命令查询职责分离）**：

```
应用层
├── commands/          # 写操作
│   ├── create_account.rs
│   ├── execute_check_in.rs
│   └── ...
├── queries/           # 读操作
│   ├── get_accounts.rs
│   ├── get_balance.rs
│   └── ...
└── dtos/              # 数据传输对象
    ├── account_dto.rs
    └── ...
```

### 原则
1. 命令修改状态，返回最少数据（成功/失败）
2. 查询读取状态，返回丰富的 DTO
3. 命令可能发出领域事件
4. 查询无副作用

## 后果

### 优点
- 读写清晰分离
- 更容易独立优化查询
- 命令可以异步验证和处理
- 天然适合事件溯源（未来增强）

### 缺点
- 更多代码组织开销
- 需要维护单独的命令/查询处理器
- 如果不小心可能导致代码重复

### 当前实现状态

**已实现**：
- 数据传输 DTO
- presentation/commands.rs 中的类命令操作
- 应用服务（CheckInExecutor、Scheduler）

**尚未实现**：
- 独立的命令/查询处理器模块
- 领域事件发布/订阅
- 用于审计跟踪的事件溯源

### 目标结构示例

```rust
// 命令
pub struct CreateAccountCommand {
    pub name: String,
    pub provider_id: String,
    pub cookies: HashMap<String, String>,
}

// 命令处理器
impl CommandHandler<CreateAccountCommand> for CreateAccountHandler {
    async fn handle(&self, cmd: CreateAccountCommand) -> Result<AccountId, Error> {
        // 验证、创建聚合、保存、发出事件
    }
}

// 查询
pub struct GetAccountsQuery {
    pub enabled_only: bool,
}

// 查询处理器
impl QueryHandler<GetAccountsQuery> for GetAccountsHandler {
    async fn handle(&self, query: GetAccountsQuery) -> Result<Vec<AccountDto>, Error> {
        // 从仓储读取，映射到 DTO
    }
}
```

### 当前折中方案
业务逻辑目前在 `presentation/commands.rs` 中而不是单独的处理器中。这可行但应该重构以提高可维护性。

## 考虑过的替代方案
- **简单 CRUD**: 更容易但混合了读写关注点
- **事件溯源**: 对当前需求过于复杂
- **带独立读模型的完整 CQRS**: 对桌面应用来说过度

## 相关
- [架构概览](../architecture_overview.md)
- [ADR-003: DDD 架构](./003-ddd-architecture.md)
