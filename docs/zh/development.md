# 开发者文档

本文档面向 NeuraDock 的开发者，包含构建、发布和贡献指南。

## 📋 目录

- [快速开始](#快速开始)
- [本地构建](#本地构建)
- [CI/CD 配置](#cicd-配置)
- [发布流程](#发布流程)
- [贡献指南](#贡献指南)

---

## 🚀 快速开始

### 环境要求

- Node.js 20+
- Rust (通过 rustup)
- macOS / Windows / Linux

### 安装依赖

```bash
# 首次安装
make setup

# 或者
cd apps/desktop && npm install --legacy-peer-deps
```

### 开发模式

```bash
# 启动开发服务器（带热重载）
make dev

# 不同日志级别
make dev-debug    # DEBUG 级别
make dev-trace    # TRACE 级别（性能追踪）
make dev-warn     # 仅警告
```

---

## 📦 本地构建

### macOS 构建

#### 1. 当前架构（最快）

```bash
make build-release
```

生成：
- Apple Silicon Mac: `NeuraDock_0.1.0_aarch64.dmg`
- Intel Mac: `NeuraDock_0.1.0_x64.dmg`

#### 2. Universal Binary（推荐）⭐

```bash
make build-universal
```

生成：
- `NeuraDock_0.1.0_universal.dmg` - 同时支持 Intel + Apple Silicon

**优势**：
- ✅ 一个安装包适配所有 Mac
- ✅ 用户体验最佳
- ✅ 推荐用于发布

#### 3. 构建所有架构

```bash
make build-all-macos
```

生成：
- `NeuraDock_0.1.0_aarch64.dmg` - Apple Silicon
- `NeuraDock_0.1.0_x64.dmg` - Intel
- `NeuraDock_0.1.0_universal.dmg` - Universal Binary

#### 4. 指定架构

```bash
# Intel Mac
make build-arch ARCH=x86_64-apple-darwin

# Apple Silicon
make build-arch ARCH=aarch64-apple-darwin
```

### Windows/Linux 构建

**本地无法交叉编译**，需要使用 GitHub Actions。

---

## 🔄 CI/CD 配置

### GitHub Actions Workflows

我们配置了 2 个 workflow：

#### 1. Test Build - 测试构建

**文件**: `.github/workflows/test-build.yml`

**用途**: 验证构建流程，不创建 Release

**触发**: 手动

**选项**: 可选择单个平台或所有平台测试

**输出**: 构建产物保留 7 天

#### 2. Release - 正式发布

**文件**: `.github/workflows/release.yml`

**用途**: 构建所有平台并创建 GitHub Release

**触发**:
- 自动：推送 `v*` tag
- 手动：在 Actions 页面手动触发

**输出**:
- ✅ macOS Universal Binary (.dmg)
- ✅ Windows x64 (.msi)
- ✅ Linux (.deb, .AppImage)
- ✅ GitHub Release（草稿状态）

### 提交前验证

#### 方法 1：本地验证（推荐）

```bash
# 安装验证工具
brew install actionlint yamllint

# 运行验证
make validate-actions
```

#### 方法 2：测试分支验证

```bash
# 创建测试分支
git checkout -b test-ci
git push origin test-ci

# 在 GitHub 手动触发 Test Build
# Actions → Test Build → Run workflow → 选择平台
```

#### 方法 3：在线验证

访问 https://rhysd.github.io/actionlint/ 粘贴 workflow 文件验证

---

## 🚀 发布流程

### 完整发布步骤

#### 1. 准备发布

```bash
# 1. 更新版本号
# 编辑以下文件中的 version:
# - apps/desktop/src-tauri/tauri.conf.json
# - apps/desktop/package.json

# 2. 提交版本更新
git add .
git commit -m "chore: bump version to v0.1.0"
```

#### 2. 创建并推送 Tag

```bash
git tag v0.1.0
git push origin main
git push origin v0.1.0
```

#### 3. 自动构建

GitHub Actions 会自动：
- ✅ 在 3 个平台并行构建（约 20 分钟）
- ✅ 创建 GitHub Release（草稿）
- ✅ 上传所有平台的安装包

#### 4. 发布 Release

1. 打开 **Releases** 页面
2. 找到草稿 Release
3. 编辑 Release Notes（可选）
4. 点击 **Publish release**

### 手动触发发布

不想创建 tag？可以手动触发：

1. 打开 **Actions** 标签
2. 选择 **Release** workflow
3. 点击 **Run workflow**
4. 输入版本号（如 `v0.1.0`）
5. 选择是否创建 Release
6. 点击 **Run workflow**

---

## 📦 构建产物说明

### macOS

```
NeuraDock_0.1.0_universal.dmg          (约 11MB)
├─ 支持 Intel Mac (x86_64)
└─ 支持 Apple Silicon Mac (aarch64)
```

**为什么选择 Universal Binary？**
- ✅ 一个文件适配所有 Mac
- ✅ 用户无需选择架构
- ✅ 苹果官方推荐格式

### Windows

```
NeuraDock_0.1.0_x64_en-US.msi          (约 9MB)
└─ 64 位安装程序
```

### Linux

```
neuradock_0.1.0_amd64.deb              (约 10MB)  - Debian/Ubuntu
neuradock_0.1.0_amd64.AppImage         (约 15MB)  - 通用格式（推荐）
```

**推荐 AppImage**：
- ✅ 无需安装，直接运行
- ✅ 兼容所有 Linux 发行版
- ✅ 自包含所有依赖

---

## 🛠️ 常用命令

### 开发命令

```bash
make help               # 显示所有命令
make dev                # 启动开发服务器
make dev-debug          # DEBUG 模式
make test               # 运行测试
make clean              # 清理构建产物
make clean-all          # 深度清理（包括依赖）
```

### 构建命令

```bash
make build-release      # 构建当前架构
make build-universal    # 构建 Universal Binary（推荐）
make build-all-macos    # 构建所有 macOS 架构
make show-targets       # 显示所有构建选项
```

### 验证命令

```bash
make validate-actions   # 验证 GitHub Actions 配置
make check              # 检查代码格式
make fix                # 自动修复代码格式
```

---

## 🧪 测试

### 运行测试

```bash
# 运行所有测试
make test

# 仅后端测试
make test-backend

# 生成覆盖率报告
make test-coverage
make coverage-report  # 打开 HTML 报告
```

### 测试覆盖率

测试覆盖率报告位于：
- HTML: `apps/desktop/src-tauri/coverage/tarpaulin-report.html`
- JSON: `apps/desktop/src-tauri/coverage/tarpaulin-report.json`
- LCOV: `apps/desktop/src-tauri/coverage/lcov.info`

---

## 📝 代码规范

### Rust

- 遵循官方 Rust 风格指南
- 使用 `snake_case` 命名函数、变量
- 使用 `PascalCase` 命名类型、结构体
- 运行 `make fix` 自动格式化

### TypeScript

- 启用严格模式
- 使用 `camelCase` 命名变量、函数
- 使用 `PascalCase` 命名组件、类型
- React 函数组件 + Hooks
- 运行 `npm run build` 检查类型

---

## 🐛 故障排查

### 构建失败

**问题**: TypeScript 编译错误

```bash
# 解决方案
cd apps/desktop
npm run build  # 查看详细错误
```

**问题**: Rust 编译错误

```bash
# 解决方案
cd apps/desktop/src-tauri
cargo build --release  # 查看详细错误
```

**问题**: 依赖安装失败

```bash
# 解决方案
make clean-all
make setup
```

### GitHub Actions 失败

1. 查看 Actions 日志
2. 本地运行 `make validate-actions`
3. 在测试分支验证
4. 检查依赖版本

### 性能问题

**构建时间过长**：

第一次构建会下载所有依赖（20-30 分钟），后续构建有缓存（5-10 分钟）。

**优化建议**：
- 使用 `make build-release`（不要 `build-all-macos`）
- 启用 Rust 增量编译（已默认开启）
- 使用 SSD 存储

---

## 🤝 贡献指南

### 提交 Pull Request

1. Fork 仓库
2. 创建功能分支：`git checkout -b feature/amazing-feature`
3. 提交更改：`git commit -m 'feat: add amazing feature'`
4. 推送分支：`git push origin feature/amazing-feature`
5. 创建 Pull Request

### Commit 规范

使用 [Conventional Commits](https://www.conventionalcommits.org/)：

```
feat: 新功能
fix: 修复 bug
docs: 文档更新
style: 代码格式（不影响功能）
refactor: 重构
test: 测试相关
chore: 构建/工具链更新
```

### 代码审查

所有 PR 需要：
- ✅ 通过 CI 检查
- ✅ 代码审查通过
- ✅ 测试覆盖新功能

---

## 📚 参考资料

### 官方文档

- [Tauri 文档](https://tauri.app/v1/guides/)
- [Rust 文档](https://doc.rust-lang.org/)
- [React 文档](https://react.dev/)

### 项目文档

- [用户手册](../../README.md)
- [架构设计](../../CLAUDE.md)
- [English Version](../en/development.md)

---

## 🆘 获取帮助

遇到问题？

1. 查看 [Issues](https://github.com/你的用户名/NeuraDock/issues)
2. 查看文档：`make help`
3. 提交新 Issue
4. 查看构建日志

---

**最后更新**: 2025-12-08
