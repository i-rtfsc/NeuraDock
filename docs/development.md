# 开发指南

本文档提供 NeuraDock 开发的完整命令参考和最佳实践。

## 目录

- [快速开始](#快速开始)
- [完整命令参考](#完整命令参考)
- [常用工作流](#常用工作流)
- [故障排除](#故障排除)

---

## 快速开始

### 首次设置

```bash
# 1. 克隆仓库
git clone https://github.com/i-rtfsc/NeuraDock.git
cd NeuraDock

# 2. 安装依赖
make install

# 3. 启动开发服务器
make dev
```

### 日常开发

```bash
# 启动开发服务器
make dev

# 重启服务器
make kill dev
```

---

## 完整命令参考

Makefile 只保留少量常用 target，具体以 `make help` 输出为准。

### 常用命令

| 命令 | 说明 |
|------|------|
| `make install` | 安装前端依赖 |
| `make doctor` | 检查开发环境 |
| `make dev` | 启动开发（默认 `RUST_LOG=info`） |
| `make bindings` | 生成 TypeScript 绑定 |
| `make build` | 构建前端 + 后端（Release，不打包） |
| `make package` | 构建并打包（生成安装包） |
| `make test` | 运行前后端测试 |
| `make check` | 后端 clippy + 前端 typecheck |
| `make clean` | 清理构建产物（保留依赖） |
| `make purge` | 深度清理（构建产物 + 依赖 + db） |

### 日志级别

- `make dev-debug`
- `make dev-trace`
- `make dev-warn`

### macOS 打包

- `make package-universal`
- `make package-arch ARCH=x86_64-apple-darwin`
- `make package-all-macos`

---

## 常用工作流

### 1. 日常开发流程

```bash
# 1. 启动开发
make dev

# 2. 编写代码...

# 3. 测试
make test-backend

# 4. 检查格式
make check

# 5. 自动修复
make fmt

# 6. 提交代码
git add .
git commit -m "feat: ..."
```

### 2. 发布流程

```bash
# 1. 更新版本号
# 编辑 apps/desktop/src-tauri/Cargo.toml

# 2. 清理旧构建
make purge

# 3. 重新安装依赖
make install

# 4. 运行测试
make test-backend

# 5. 构建 Release 版本
make package

# 7. 查看构建产物
ls -lh apps/desktop/src-tauri/target/release/bundle/*/
```

### 3. 调试流程

```bash
# 1. 使用 debug 日志启动
make dev-debug

# 2. 如果需要性能分析
make kill
make dev-trace

# 4. 运行测试定位问题
make test-backend

# 4. 运行测试定位问题
make test-backend
```

### 4. 完全重置流程

```bash
# 1. 深度清理
make purge

# 2. 重新安装依赖
make install

# 3. 验证环境
make doctor

# 4. 启动开发
make dev
```

---

## 故障排除

### 问题：依赖安装失败

```bash
# 解决方案 1: 清理后重新安装
make purge
make install

# 解决方案 2: 检查网络和 Node 版本
make doctor
node --version  # 需要 >= 20.0.0

# 解决方案 3: 手动安装
cd apps/desktop
rm -rf node_modules
npm install --legacy-peer-deps
```

### 问题：开发服务器无法启动

```bash
# 解决方案 1: 杀掉旧进程
make kill
make dev

# 解决方案 2: 检查端口占用
lsof -ti:1420  # Tauri 端口
lsof -ti:5173  # Vite 端口

# 解决方案 3: 重启并查看详细日志
make dev-debug
```

### 问题：构建失败

```bash
# 解决方案 1: 清理后重新构建
make clean
make build

# 解决方案 2: 完全重置
make purge
make install
make build

# 解决方案 3: 检查 Rust 版本
rustc --version  # 需要 >= 1.70.0
cargo --version
```

### 问题：测试失败

```bash
# 解决方案 1: 运行单个测试
cd apps/desktop/src-tauri
cargo test <test_name> -- --nocapture

# 解决方案 2: 清理测试缓存
make clean
make test-backend

# 解决方案 3: 查看详细输出
cd apps/desktop/src-tauri
RUST_LOG=debug cargo test -- --nocapture
```

### 问题：权限错误

```bash
# 解决方案: 手动修复
chmod +x apps/desktop/src-tauri/target/release/neuradock
chmod -R u+w apps/desktop/node_modules
```

### 问题：数据库错误

```bash
# 解决方案 1: 重新运行迁移
make migrate

# 解决方案 2: 删除数据库重新创建
rm *.db *.db-shm *.db-wal
make dev  # 会自动创建数据库

# 解决方案 3: 使用开发数据库
# 开发环境会使用 neuradock_dev.db
```

---

## 环境变量

### 日志级别

```bash
# 通过 RUST_LOG 控制日志级别
RUST_LOG=debug make dev      # 详细日志
RUST_LOG=trace make dev      # 追踪级别（最详细）
RUST_LOG=warn make dev       # 仅警告
RUST_LOG=info make dev       # 标准日志（默认）

# 或使用预设命令
make dev-debug               # 相当于 RUST_LOG=debug
make dev-trace               # 相当于 RUST_LOG=trace
make dev-warn                # 相当于 RUST_LOG=warn
```

### 数据库位置

- **开发环境**: `neuradock_dev.db`
- **生产环境**:
  - macOS: `~/Library/Application Support/com.neuradock.app/neuradock.db`
  - Windows: `%APPDATA%\com.neuradock.app\neuradock.db`
  - Linux: `~/.local/share/com.neuradock.app/neuradock.db`

### 日志位置

- macOS: `~/Library/Logs/neuradock/logs/`
- Windows: `%APPDATA%\neuradock\logs\`
- Linux: `~/.local/share/neuradock/logs/`

---

## 性能优化建议

### 加快开发启动速度

```bash
# 1. 使用 Rust 的增量编译（默认启用）
# 已在 Cargo.toml 中配置

# 2. 使用更少的日志
make dev-warn  # 只显示警告
```

### 加快构建速度

```bash
# 1. 仅构建需要的部分
make build-frontend  # 仅前端
make build-backend   # 仅后端

# 2. 使用多核编译
# Rust 默认使用所有 CPU 核心
```

### 减少磁盘占用

```bash
# 1. 定期清理
make clean

# 2. 深度清理（重置环境时）
make purge

# 3. 清理 Rust 缓存
cargo cache --autoclean  # 需要安装 cargo-cache
```

---

## 相关文档

- [贡献指南](./contributing.md) - 完整的贡献流程
- [架构概览](./architecture/architecture_overview.md) - 系统架构
- [技术实现细节](./architecture/technical_details.md) - 技术深入文档
- [API 参考](./api/api_reference.md) - API 文档

---

## 获取帮助

```bash
# 查看所有命令
make help

# 查看开发环境状态
make doctor
```

如有问题，请参考：
- [故障排除文档](./user_guide/troubleshooting.md)
- [GitHub Issues](https://github.com/i-rtfsc/NeuraDock/issues)
