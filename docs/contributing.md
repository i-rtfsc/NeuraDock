# 贡献指南

感谢你有兴趣为 NeuraDock 做贡献！本指南将帮助你入门。

## 开发环境设置

### 前置要求

- **Node.js**: >= 20.0.0
- **Rust**: >= 1.70.0（通过 [rustup](https://rustup.rs/) 安装）
- **npm**: 最新版本
- **Git**: 版本控制
- **IDE**: 推荐 VS Code，配合 Rust Analyzer 和 ESLint 扩展

### 克隆和设置

```bash
# 克隆仓库
git clone https://github.com/i-rtfsc/NeuraDock.git
cd NeuraDock

# 安装依赖
make install

# 启动开发服务器
make dev
```

## 项目结构

```
NeuraDock/
├── apps/
│   └── desktop/              # Tauri 桌面应用
│       ├── src/              # React 前端
│       │   ├── components/   # UI 组件
│       │   ├── pages/        # 页面组件
│       │   ├── hooks/        # 自定义 React hooks
│       │   ├── lib/          # 工具函数和 Tauri 绑定
│       │   └── i18n/         # 国际化
│       └── src-tauri/        # Rust 后端
│           └── src/
│               ├── domain/           # 领域层 (DDD)
│               ├── application/      # 应用层 (CQRS)
│               ├── infrastructure/   # 基础设施层
│               └── presentation/     # 表示层 (Tauri IPC)
├── docs/                     # 文档
└── migrations/               # 数据库迁移
```

## 开发命令

### 快速开始

```bash
# 首次使用 - 安装依赖
make install

# 启动开发服务器（带热重载）
make dev
```

### 完整命令列表

#### 📦 安装和依赖

```bash
make install            # 安装前端依赖
make doctor             # 检查开发环境
```

#### 🚀 开发模式

```bash
make dev                # 启动开发模式（RUST_LOG=info）
make dev-debug          # 启动开发模式（RUST_LOG=debug - 详细日志）
make dev-trace          # 启动开发模式（RUST_LOG=trace - 性能追踪）
make dev-warn           # 启动开发模式（RUST_LOG=warn - 仅警告）
make kill               # 杀掉所有运行中的进程
```

#### 📦 构建命令

```bash
make build              # 构建前端 + 后端（Release，不打包）
make package            # 构建并打包（生成安装包）
make build-frontend     # 仅构建前端
make build-backend      # 仅构建后端
make bindings           # 生成 TypeScript 绑定
```

#### 🧪 测试命令

```bash
make test               # 运行所有测试
make test-backend       # 运行后端测试
make test-frontend      # 运行前端测试
```

#### 🧹 清理命令

```bash
make clean              # 清理构建产物（保留依赖）
make purge              # 深度清理（构建产物 + 依赖 + db）
```

#### ✅ 代码质量

```bash
make check              # 检查代码格式（rustfmt + clippy）
make fmt                # 自动修复 Rust 代码格式
```

#### 🔧 工具和信息

```bash
make doctor             # 检查开发环境
make migrate            # 运行数据库迁移
make help               # 显示所有命令的帮助信息
```

### 常用命令组合

```bash
# 重启开发服务器
make kill dev

# 清理后重新构建
make clean build

# 完整的发布流程
make purge
make install
make package
```

## 代码风格指南

### Rust

- 遵循 [Rust 风格指南](https://doc.rust-lang.org/nightly/style-guide/)
- 函数和变量使用 `snake_case`
- 类型、结构体和枚举使用 `PascalCase`
- 领域操作优先使用 `Result<T, DomainError>`
- 应用/基础设施操作使用 `anyhow::Result<T>`
- 提交前运行 `cargo fmt`

### TypeScript/React

- 启用严格模式
- 函数和变量使用 `camelCase`
- 组件和类型使用 `PascalCase`
- 优先使用 `const` 而非 `let`
- 使用函数组件和 hooks
- 使用 `@/` 别名导入 src 目录

## 架构指南

NeuraDock 遵循 **DDD（领域驱动设计）** 和 **CQRS** 模式：

1. **领域层** (`src-tauri/src/domain/`)
   - 包含核心业务逻辑
   - 不依赖其他层
   - 定义聚合、实体、值对象
   - 定义仓储 trait（接口）

2. **应用层** (`src-tauri/src/application/`)
   - 编排领域操作
   - 命令/查询处理器
   - DTOs 用于数据传输
   - 应用服务

3. **基础设施层** (`src-tauri/src/infrastructure/`)
   - 实现仓储 trait
   - 数据库持久化（SQLite + sqlx）
   - HTTP 客户端、浏览器自动化
   - 外部服务集成

4. **表示层** (`src-tauri/src/presentation/`)
   - Tauri 命令（IPC 端点）
   - 向前端发送事件
   - 状态管理

## 添加新功能

遵循以下检查清单：

1. **领域层优先**
   - 在 `domain/` 添加/修改聚合
   - 如需要，定义仓储 trait
   - 为验证数据创建值对象

2. **基础设施实现**
   - 在 `infrastructure/persistence/` 实现仓储 trait
   - 如需要，添加数据库迁移
   - 实现外部集成

3. **应用层服务**
   - 创建命令/查询处理器
   - 在 `application/dtos/` 定义 DTOs
   - 为复杂工作流添加服务

4. **表示层**
   - 使用 `#[tauri::command]` 和 `#[specta::specta]` 宏添加 Tauri 命令
   - 在 `main.rs` 中通过 `collect_commands![]` 注册
   - 运行开发服务器以重新生成 TypeScript 绑定

5. **前端实现**
   - 从 `@/lib/tauri` 导入
   - 创建 React 组件
   - 使用 TanStack Query 进行数据获取

## Pull Request 流程

1. **Fork** 仓库

2. **创建分支**：
   ```bash
   git checkout -b feature/your-feature-name
   ```

3. **按照上述指南进行更改**

4. **测试你的更改**：
   ```bash
   cargo test
   npm run typecheck
   ```

5. **使用清晰的提交信息**：
   ```bash
   git commit -m "feat: 添加批量账号更新功能"
   ```

   遵循 [Conventional Commits](https://www.conventionalcommits.org/):
   - `feat:` 新功能
   - `fix:` Bug 修复
   - `docs:` 文档
   - `refactor:` 代码重构
   - `test:` 添加测试
   - `chore:` 维护

6. **推送并创建 PR**：
   ```bash
   git push origin feature/your-feature-name
   ```

7. **PR 审查**：等待审查并解决反馈

## 测试

- **Rust 单元测试**：位于 `#[cfg(test)]` 模块或 `*_test.rs` 文件中
- **使用 `mockall`** 进行仓储模拟
- **领域逻辑** 应有全面的测试
- **集成测试** 用于关键路径

## 文档

- 添加功能时更新相关文档
- 为导出的 TypeScript 函数添加 JSDoc 注释
- 使用 `///` 注释记录 Rust 公共 API
- 为面向用户的更改更新 CHANGELOG.md

## 获取帮助

- **GitHub Issues**: 报告 bug 或请求功能
- **Discussions**: 提问或讨论想法
