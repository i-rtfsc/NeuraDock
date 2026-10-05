# NeuraDock 1.2.0 架构与代码质量评审报告

评审日期：2026-08-24
评审范围：仓库当前工作区（包含未提交改动）
评审方式：静态代码审查、目录/依赖盘点、构建与测试命令验证、关键路径抽样

## 1. 执行摘要

NeuraDock 是一个 Tauri 2 + React/TypeScript + Rust + SQLite 的桌面应用，核心能力包括账号/Token 管理、自动签到、余额与历史记录、AI 聊天、Codex 管理、本地网关、开发者工具和配置文件接管。

总体判断：架构方向正确，领域层、应用层、基础设施层和表示层已经形成清晰意图；前端 TypeScript 严格模式有效，IPC 已引入 tauri-specta，数据库迁移和仓储抽象较完整，已有单元/集成测试。当前主要问题不在“缺少架构”，而在架构边界被大型文件、动态 IPC、重复装配、密钥管理和工程入口不一致逐步侵蚀。

综合评级：**B-（可持续演进，但不适合继续无治理地堆叠功能）**。

### 1.1 整改进展（更新于 2026-09）

本报告最初记录的是整改前基线。当前状态如下：

- **已完成**：版本统一为 `1.2.0`，并由脚本校验 package、Tauri、Cargo、README 和 release metadata。
- **已完成**：`make check` 已覆盖 rustfmt、workspace check、workspace 全 target Clippy、TypeScript 严格检查、文档命令和版本一致性。
- **已完成**：CI 已覆盖前后端测试、依赖审计、cargo-audit、Gitleaks；Release 生成 SHA-256 校验文件。
- **已完成**：新安装使用随机 256-bit master key；旧固定密码仅保留为存量数据迁移兼容路径，并具备 pending key 中断恢复测试。
- **已完成**：Codex 敏感凭据数据库加密，DTO 不再返回密码明文。
- **部分完成**：生产前端 IPC 的业务组件已改用生成的 typed client，但**并未全量收敛**：手写的 `lib/tauri-commands.ts` 仍有 22 处裸字符串 `invoke`，`hooks/useBalance.ts` 4 处，另有 token/preferences/proxy/reminder 等若干处。`verify-bindings` 只校验生成物 `tauri.ts`，覆盖不到这些手写包装，因此该门禁无法阻止反弹。
- **已完成**：签到模块按 ADR-007 重构为「证据账本 + 尝试履历 + provider quota day」：今日是否已签到由账本判定（不再用 24h 间隔近似），额度上涨在运行时即被推断为签到证据，`accounts.last_check_in` 的读写错位（其实是"上次余额快照"）已删除，并补齐签到历史、统计、运行中任务与取消。
- **已完成**：Tauri bindings 已纳入版本控制；前端检查先生成绑定，CI 再校验生成结果无漂移。
- **已完成**：Tauri capability 已移除 `fs:default`、`dialog:default`、`store:default`，外链仅允许无 credentials 的 HTTP(S) URL。
- **已完成**：删除子 crate 下未被构建读取的重复 capability 文件，权限单一来源为根 `tauri.conf.json`。
- **已完成**：Gateway 上游解析下沉到 application service，并补充账户 Token、独立 Key、多 Profile、持久化失败和 takeover 回滚测试。
- **已完成**：`make dev` 的 Tauri context 路径已修复，并实际验证应用、Vite、数据库迁移和 AppState 可正常启动。
- **已完成**：`make check` 覆盖 Tauri CLI 在开发模式使用的精确 Cargo feature 组合，防止 context 路径问题回归。
- **部分完成**：Gateway 和 command 已拆出多个模块，但核心文件仍偏大，需要继续按生命周期和协议职责拆分。
- **部分完成**：根 Tauri 兼容 package 已可稳定构建，但影子 package 尚未彻底移除。
- **待实施**：平台 Keychain/Credential Manager/Secret Service 接入，以及存量迁移完成后删除固定兼容密码。
- **待实施**：三平台 UI smoke test、签名/公证、数据库降级策略和更完整的 release checklist。

最高优先级事项：

1. 移除源码内固定加密密码，改用操作系统密钥链/Keychain 或首次启动生成并安全保存的随机数据。
2. 解决 Cargo/Tauri 的影子 package 与配置路径问题，使标准构建入口和 CI 入口一致。
3. 将动态 `invoke('command', ...)` 收敛到生成的类型安全 API，禁止业务组件直接拼命令字符串。
4. 拆分超大模块，尤其是 `gateway/server.rs`、`commands/gateway.rs`、Codex/Agent 页面和开发者工具服务。
5. 将格式、Clippy、测试、安全扫描纳入强制 CI 门禁，并补足端到端测试。

## 2. 现状架构

### 2.1 技术栈与模块

- 前端：React 18、TypeScript 5、Vite、React Router、TanStack Query、Zustand、Tailwind/Radix、i18next、Vitest。
- 桌面壳：Tauri 2，窗口、文件、Shell、通知、Store 等插件。
- Rust：Cargo workspace，包含 `neuradock-domain`、`neuradock-application`（当前名为 app）、`neuradock-infrastructure`，另有根级 Tauri 兼容 package。
- 持久化：SQLite + SQLx migrations + repository traits。
- 通讯：Tauri IPC；部分能力还有本地 Axum gateway、Reqwest/Rquest、Chromiumoxide。
- 安全：AES-GCM、Argon2、随机盐、随机 master key、文件权限控制；固定密码仅用于旧安装迁移兼容。

### 2.2 当前分层

```text
React pages/components/hooks
          │
          ▼
Tauri commands / DTOs / state / IPC events
          │
          ▼
Application services / queries / schedulers
          │
          ▼
Domain aggregates / repository traits / domain services
          │
          ▼
Infrastructure repositories / HTTP / gateway / browser / encryption / migrations
```

该分层符合 Clean Architecture/Hexagonal Architecture 的基本方向。问题是应用层和表示层承担了过多编排职责，部分基础设施细节穿透到 command 和页面，导致修改成本随功能数量增长。

## 3. 评审证据与验证结果

### 3.1 通过项

- `pnpm -C apps/desktop exec tsc --noEmit`：通过。
- `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml -p neuradock-domain -p neuradock-infrastructure -p neuradock-app`：通过。
- `pnpm -C apps/desktop run test:run`：通过，6 个测试文件、21 个测试通过。
- 前端 `tsconfig.json` 已启用 `strict`、`noUnusedLocals`、`noUnusedParameters`、`noFallthroughCasesInSwitch`。
- IPC 绑定生成流程已存在，说明项目具备收敛到强类型接口的基础。
- 数据库迁移、仓储 trait、领域聚合和基础设施集成测试已形成可复用测试基础。

### 3.2 初始门禁问题（已整改）

- `make check` 初始因 rustfmt 差异失败；现已修复并升级为全 target Clippy 门禁。
- 标准 workspace 和 `make dev` 曾因 Tauri context 路径失败；现已通过 package 专属 build cfg 修复。影子 package 本身仍是待清理的结构问题。
- 测试命令会先生成 `src/lib/tauri.ts`，生成文件属于构建产物，却被 Makefile 的 clean 目标删除；应明确其是源码生成物还是缓存，并统一策略。

## 4. 主要发现

### P0：本地敏感数据加密密钥可被源码公开恢复

证据：`apps/desktop/src-tauri/crates/neuradock-app/src/presentation/bootstrap.rs` 中使用固定字符串 `neuradock_default_password_2024` 派生加密服务密钥。

影响：数据库或备份文件一旦泄露，攻击者可以从公开二进制/源码推导密码并解密 API Key、账号 Token、OAuth 凭据等敏感数据。盐只能防止相同密码的预计算，不能弥补固定密码问题。

解决：优先使用 macOS Keychain、Windows Credential Manager、Linux Secret Service；跨平台失败时首次启动生成随机 32 字节主密钥并以平台安全存储保存。增加密钥版本、轮换、迁移和“无法解密时安全失败”机制。禁止日志打印密钥、明文凭据和完整响应体。

### P1：Tauri/Cargo 工程入口存在影子 package 和路径耦合

证据：根 `Cargo.toml` 同时声明 workspace crates 和一个用于 Tauri CLI 的 `neuradock` package；代码中存在相对路径 `generate_context!("../../tauri.conf.json")`，Makefile 注释明确说明标准 workspace 会失败。

影响：IDE、贡献者和外部 CI 运行常规 `cargo check --workspace` 时得到误导性失败；同一源码可能被重复编译；配置路径重构风险高。

解决：选择单一方案：将 Tauri package 放在 `apps/desktop/src-tauri` 根并让真实 app crate 成为唯一 package，或使用 build script 传递绝对/规范化配置路径并移除影子 package。最终验收要求 `cargo check --workspace`、`cargo test --workspace`、`cargo clippy --workspace --all-targets -- -D warnings` 均可直接运行。

### P1：IPC 类型安全未形成强制边界

证据：前端大量直接使用 `invoke('...')`，个别代码根据条件拼接命令名，例如开发者工具页面；同时存在 `invoke<any>`/宽泛返回值和 `input: any`。

影响：命令重命名、参数改动和返回 DTO 改动无法在编译期完整发现；错误处理和缓存失效策略容易分散在页面中。

解决：以生成的 `src/lib/tauri.ts` 为唯一 IPC 客户端，按领域封装 `accountApi`、`tokenApi`、`gatewayApi`、`developerToolsApi`；页面只依赖 hooks/use-case，不直接调用 Tauri。命令名、输入、输出、错误码和事件全部由 Rust schema 生成并在 CI 检查生成文件无漂移。

### P1：超大文件和多职责模块阻碍维护

抽样规模：`gateway/server.rs` 3275 行、`commands/gateway.rs` 2726 行、`AccountsTab.tsx` 1437 行、`AgentAccessDrawer.tsx` 1310 行、`PreferencesPage.tsx` 1057 行、开发者工具服务 927 行。

影响：职责边界不清、审查困难、局部修改容易引入回归，测试难以针对单一用例编写。

解决：按用例和端口拆分：Gateway listener、routing、auth、stream、transform、reliability、usage、lifecycle；前端按 page shell、query hooks、form、table、dialogs、view-model 拆分。单文件目标建议 <400 行，复杂协议转换允许例外但必须有模块级测试和设计说明。

### P1：动态外部集成缺少统一的边界模型

项目同时使用 Reqwest、Rquest、Chromiumoxide、Axum、临时邮箱、Jira/Gerrit 等外部服务。当前部分 command/service 直接处理 URL、认证、HTTP 响应、重试和 UI DTO。

解决：建立 `integration` 端口和 adapter：统一 timeout、proxy、重试、退避、状态码映射、脱敏日志、请求 ID、速率限制和取消语义。外部 JSON 先映射为 provider-specific DTO，再进入 domain/application，禁止把任意 JSON 直接贯穿 UI。

### P1：错误模型和日志策略不统一

证据：代码中同时使用 `anyhow`、`String`、`Box<dyn Error>`、`CommandError`、`unwrap/expect`；前端仍有直接 `console.error`，后端测试和协议转换中存在大量 `unwrap`。

解决：定义分层错误：DomainError、ApplicationError、InfrastructureError、CommandError，并保留稳定错误码和用户可见消息。生产路径禁止 `unwrap/expect`，仅允许在“已由代码保证”的内部不变量处使用并加注释；日志统一 tracing，自动脱敏 token、Authorization、Cookie、密码和响应体。

### P2：Tauri 能力权限偏宽，配置格式也有可维护性问题

证据：`tauri.conf.json` 对主窗口使用 `fs:default`、`shell:allow-open`、`process:allow-restart` 等能力，且存在缩进不一致；capability 文件与内嵌配置存在重复来源。

解决：按窗口和功能拆分 capability，采用最小权限；Shell 仅允许明确协议/目标，FS 限制到 app data 目录，开发工具和内嵌 webview 使用独立窗口权限。删除重复 capability 来源，加入 JSON schema 校验。

### P2：测试覆盖偏重纯函数，关键用户流程覆盖不足

当前前端只有 19 个测试，主要覆盖排序、路径归一化、数字输入和 mutation factory。后端有较多领域和基础设施测试，但尚未形成覆盖“启动→迁移→密钥→命令→UI”全链路的稳定门禁。

解决：补充 repository migration tests、IPC contract tests、gateway protocol golden tests、敏感数据加解密/迁移测试、外部服务 mock contract、Tauri smoke tests 和 Playwright 核心流程。目标不是盲目追求行覆盖率，而是覆盖高风险状态转换和失败路径。

### P2：开发文档和自动化入口存在漂移

文档曾提到不存在的构建 target，且发布流程未说明 tag、版本同步、签名、公证、回滚策略；本轮已将构建命令统一为 Makefile 现有 target，并加入文档命令校验。

解决：将 `make help` 作为命令单一来源；`make check` 校验文档中的命令；增加 release checklist、版本自动同步、签名/公证说明、数据库迁移兼容策略和回滚方案。

### P3：当前工作区存在大量未提交功能改动

本次审查发现开发者工具、Gateway、Token、UI 和迁移等多处未提交修改。报告以工作区实际状态为准，但建议在后续整改前先按 feature/bugfix 拆分提交，并为每批改动保留可回滚边界。

## 5. 目标架构方案

建议演进为“模块化单体 + 明确端口”的架构，不建议立即拆分微服务。桌面应用的本地数据库、密钥、网关和 UI 强耦合于本机，模块化单体能以最低运维成本获得清晰边界。

```text
UI Shell
  └─ domain hooks / view-models
      └─ typed IPC client
          └─ presentation commands (thin adapters)
              └─ application use cases
                  ├─ domain aggregates + policies
                  └─ ports
                      ├─ repositories
                      ├─ external providers
                      ├─ secret store
                      ├─ gateway runtime
                      └─ clock / scheduler / filesystem
                          └─ infrastructure adapters
```

推荐目录：

```text
rust/
  domain/{account,token,gateway,provider,check_in}/
  application/{use_cases,ports,dtos,errors}/
  infrastructure/{sqlite,http,secrets,filesystem,gateway,telemetry}/
  presentation/{commands,events,state}
frontend/
  app/{router,providers}
  features/{accounts,tokens,gateway,developer-tools,...}
  shared/{api,ui,query,errors,i18n}
```

每个 feature 公开少量 use case 和 DTO；Repository 只在 application port 中出现；Infrastructure 不向 UI 暴露；页面不直接依赖 `invoke`、SQL、HTTP 或文件路径。

## 6. 完整解决计划

### 阶段 0：基线冻结（1～2 天）

- 将当前未提交改动按功能拆分并打标签。
- 记录当前版本、数据库迁移版本、生成绑定版本。
- 建立统一质量门禁：格式、类型检查、定向 Cargo check、前后端测试。
- 修复所有 `cargo fmt` 差异，确保门禁可重复执行。

验收：干净环境执行一次命令即可得到确定结果；失败输出包含明确阶段。

### 阶段 1：安全止血（2～5 天）

- 接入平台安全存储，移除固定密码。
- 增加 master-key version、轮换和旧数据迁移。
- 全局日志脱敏；对 HTTP 错误禁止输出完整 body。
- 复核 Tauri capability，删除默认宽权限。
- 为密钥、Token、Cookie、OAuth 文件补充权限和泄露回归测试。

验收：源码/二进制中不再出现固定加密密码；导出的数据库无法脱离本机密钥解密；安全测试通过。

### 阶段 2：工程入口和契约治理（3～5 天）

- 消除影子 package 或将其改为唯一合法 Tauri package。
- 统一 Cargo、Makefile、pnpm、CI 入口。
- IPC 全量迁移到 generated typed client，禁止业务代码裸调用字符串命令。
- 将错误码、事件和 DTO 纳入 schema diff 检查。

验收：标准 `cargo check/test/clippy --workspace`、`pnpm build/test` 和 CI 均通过；绑定漂移会失败。

### 阶段 3：模块拆分（1～2 周）

- 先拆 Gateway 和 Codex，再拆 Agent/Preferences/Accounts 页面。
- 每个拆分模块先补 characterization tests，再进行纯重构。
- 把 HTTP provider、文件配置接管、scheduler、secret store 变成可替换 adapter。
- 统一 query/mutation hooks、cache keys 和失效策略。

验收：核心文件控制在目标规模内；模块依赖图无反向依赖；高风险 use case 可单独测试。

### 阶段 4：质量与可观测性（1 周）

- 引入 rustfmt、Clippy、ESLint、Prettier、依赖审计和 secret scanning。
- 统一 tracing 字段：request_id、command、feature、provider、duration、result。
- 为 gateway、签到、外部 API 添加 metrics/health 状态和可诊断错误。
- 记录迁移耗时、失败原因、外部请求重试和取消。

验收：CI 对格式、静态检查、依赖漏洞和凭据泄露实施阻断；生产日志可定位一次失败请求的完整链路且不泄露秘密。

### 阶段 5：测试与发布治理（1～2 周）

- 增加 IPC contract、SQLite migration、gateway golden、密钥迁移、核心 UI smoke tests。
- 确定测试数据库和临时目录策略，禁止测试污染用户目录。
- 版本号由单一脚本同步到 package、Tauri、Cargo、README/release metadata。
- 发布流程加入签名、公证、artifact checksum、迁移兼容检查、回滚说明。

验收：主流程、升级流程、失败恢复流程在三平台 CI 至少有 smoke coverage；发布产物可校验、可追溯、可回滚。

## 7. 代码规范建议

- Rust：rustfmt + Clippy `-D warnings`；公共 API 必须有文档；生产代码避免 `unwrap/expect`；错误使用稳定枚举和上下文。
- TypeScript：禁止 `any`；命令调用只允许在 API 层；组件控制在单一职责；表单使用统一 schema 校验；异步状态统一 React Query。
- 命名：领域概念使用统一词汇，DTO、command、repository、adapter 后缀明确；避免 `utils` 继续膨胀。
- 日志：统一 tracing；敏感字段结构化脱敏；禁止把 token、Cookie、密码、Authorization 和完整第三方响应写入日志。
- 提交：Conventional Commits；每个提交可编译、可测试；重构与功能变更分离。
- 文档：每个跨层模块维护 README/ADR；架构决策记录替代散落注释。

## 8. 优先级清单

| 优先级 | 事项 | 风险 | 建议时限 |
|---|---|---|---|
| P0 | 固定密码改为平台密钥存储 | 敏感数据可被离线解密 | 立即 |
| P1 | 修复 Cargo/Tauri 单一构建入口 | CI/IDE/发布不一致 | 1 周内 |
| P1 | IPC 全量类型化 | 接口漂移、运行时错误 | 1～2 周 |
| P1 | Gateway/Commands 大模块拆分 | 回归和维护成本高 | 2 周 |
| P1 | CI 强制 fmt/clippy/test/security | 质量无法持续 | 1 周 |
| P2 | 收敛外部集成 adapter | 超时/重试/错误不一致 | 2～3 周 |
| P2 | 补充端到端和迁移测试 | 升级/失败路径风险 | 2～4 周 |
| P2 | 收敛 capability 权限 | 桌面攻击面偏大 | 1～2 周 |
| P3 | 文档和 release 流程同步 | 新成员和发布易出错 | 1 周 |

## 9. 结论

项目不需要推倒重写。最合适的路线是保留现有领域模型、仓储和 Tauri 运行时，先完成安全止血和工程入口治理，再以用例为单位拆分超大模块，并把现有 tauri-specta 能力真正落实为前端唯一 API 边界。完成上述五个阶段后，NeuraDock 将从“功能丰富的模块化桌面应用”提升为“可验证、可升级、可持续维护的模块化单体”。
