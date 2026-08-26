# NeuraDock Documentation

欢迎查看 NeuraDock 文档！Welcome to NeuraDock Documentation!

## 📚 文档索引 / Document Index

### 中文文档 (Chinese)

- **[开发者文档](zh/development.md)** - 构建、发布和贡献指南
  - 本地构建（支持 Intel + Apple Silicon）
  - GitHub Actions CI/CD
  - 发布流程
  - **v0.5.0 变更说明 & 批量更新指南**

### English Documentation

- **[Developer Documentation](en/development.md)** - Build, release, and contribution guide
  - Local build (Intel + Apple Silicon support)
  - GitHub Actions CI/CD
  - Release process
  - **v0.5.0 highlights & bulk update workflow**

---

## 🚀 快速链接 / Quick Links

### 开发者 / For Developers

| 中文 | English | 说明 / Description |
|------|---------|-------------------|
| [开发者文档](zh/development.md) | [Developer Docs](en/development.md) | 构建、CI/CD、发布 |
| [项目架构](../CLAUDE.md) | [Architecture](../CLAUDE.md) | DDD + CQRS 架构设计 |

---

## 🛠️ 快速命令 / Quick Commands

```bash
make help                # 查看所有命令
make dev                 # 启动开发服务器
make doctor              # 检查开发环境
make package-all-macos   # 构建 Intel + Apple Silicon 安装包
```

---

## 📦 构建平台支持 / Build Platform Support

### macOS ✅

- ✅ Intel Mac (x86_64)
- ✅ Apple Silicon Mac (aarch64)
- ✅ **Universal Binary (推荐 / Recommended)**

### Windows ✅

- ✅ Windows x64 (via GitHub Actions)

### Linux ✅

- ✅ Debian/Ubuntu (.deb)
- ✅ AppImage (推荐 / Recommended)

---

**最后更新 / Last Updated**: 2025-12-21
