# ADR 006: Gateway Binding Service Boundary

## 状态

进行中

## 背景

Gateway 同时处理 Tauri IPC、配置文件接管、账号 Token/Independent Key 解析、运行时状态、持久化和回滚。全部逻辑集中在 command 会导致测试困难、跨层依赖和回滚风险。

## 决策

采用渐进式模块化单体：

- Tauri command 负责输入 DTO、权限边界、状态编排和错误映射。
- Application service 负责绑定规则、模型/context1m 规范化和 Token 可用性策略。
- Infrastructure Gateway 负责 HTTP、协议转换、SSE、Header/Auth 和可靠性。
- 持久化、Profile 发现、fingerprint、配置脱敏和状态投影分别使用独立模块。
- 任何新的 resolver/service 不得依赖 `presentation::commands` 或 `CommandError`。
- 引入新的 application service 时，必须同时接入至少一个生产调用点，禁止只添加未使用 trait/DTO。

## 后续 resolver 约束

上游解析应通过 application port 接入：

```text
GatewayBindingSourceInput
        -> GatewayUpstreamResolver
        -> ResolvedGatewaySource
        -> Infrastructure GatewayUpstream
```

Resolver adapter 可以依赖 repository traits 和 TokenService，但不能把 `Services`/`Repositories` presentation 聚合向下传递。

## 结果

优点：command 变薄、规则可单测、Gateway 协议代码和桌面状态代码边界清晰。代价：短期会存在 adapter 和 DTO 映射层，需通过完整可编译 commit 渐进迁移。
