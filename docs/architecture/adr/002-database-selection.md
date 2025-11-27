# ADR-002: 数据库选型

## 状态
已采纳

## 背景
NeuraDock 需要数据库来存储：
- 账号信息和凭证
- 签到任务历史
- 时间序列余额历史
- 应用设置

需求：
- 嵌入式（无需外部服务器）
- 跨平台（macOS、Windows、Linux）
- 良好的 Rust 支持
- 合理的查询能力
- 基于文件便于备份

## 决策
我们选择了 **SQLite** 配合 **sqlx** crate：
- 数据库引擎：SQLite 3
- Rust 驱动：sqlx 带编译时查询检查
- 迁移：sqlx::migrate!() 宏（嵌入二进制文件）

## 后果

### 优点
- 零配置，基于文件存储
- 优秀的跨平台支持
- sqlx 提供编译时查询验证
- 启动时自动迁移
- 成熟的技术

### 缺点
- 并发写入性能有限（对桌面应用可接受）
- 无内置加密（必须单独实现）
- 迁移在生产环境不易回滚
- 外键约束默认不强制执行（需要 PRAGMA）

### 实现说明
```rust
// 启用外键
sqlx::query("PRAGMA foreign_keys = ON").execute(&pool).await?;

// 编译时嵌入迁移
sqlx::migrate!("./migrations").run(&pool).await?;
```

### 安全考虑
SQLite 存储数据未加密。凭证应在应用层加密后再存储。参见[安全架构](../architecture_overview.md#安全架构)。

### 考虑过的替代方案
- **PostgreSQL/MySQL**: 对桌面应用来说过于复杂，需要外部服务器
- **RocksDB**: 仅键值存储，无 SQL 查询
- **Sled**: 纯 Rust 但不够成熟
- **DuckDB**: 适合分析但不太适合 OLTP

## 相关
- [架构概览](../architecture_overview.md)
- [数据库 Schema](../architecture_overview.md#数据库-schema)
