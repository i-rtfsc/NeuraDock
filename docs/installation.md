# 安装指南

## 预编译二进制文件

安装 NeuraDock 最简单的方式是从 [Releases](https://github.com/neuradock/neuradock/releases) 页面下载预编译的二进制文件。

### macOS

1. 下载适合你架构的 `.dmg` 文件：
   - `NeuraDock_x.x.x_x64.dmg` 适用于 Intel Mac
   - `NeuraDock_x.x.x_aarch64.dmg` 适用于 Apple Silicon (M1/M2/M3)

2. 打开 `.dmg` 文件

3. 将 NeuraDock 拖入 Applications 文件夹

4. 首次启动：右键 → 打开（未签名应用需要此步骤）

### Windows

1. 下载 `NeuraDock_x.x.x_x64_en-US.msi`

2. 运行安装程序并按照提示操作

3. 从开始菜单启动 NeuraDock

### Linux

1. 下载 `NeuraDock_x.x.x_amd64.AppImage`

2. 添加执行权限：
   ```bash
   chmod +x NeuraDock_x.x.x_amd64.AppImage
   ```

3. 运行 AppImage：
   ```bash
   ./NeuraDock_x.x.x_amd64.AppImage
   ```

## 从源码构建

用于开发或构建最新版本：

### 前置要求

- **Node.js**: >= 20.0.0
- **Rust**: >= 1.70.0（通过 [rustup](https://rustup.rs/) 安装）
- **npm**: 最新版本
- **Chromium 内核浏览器**: Chrome、Edge 或 Brave（用于 WAF 绕过）

### 构建步骤

1. **克隆仓库**：
   ```bash
   git clone https://github.com/neuradock/neuradock.git
   cd neuradock
   ```

2. **安装依赖**：
   ```bash
   npm install
   ```

3. **启动开发服务器**（可选，用于测试）：
   ```bash
   npm run dev
   ```

4. **构建生产版本**：
   ```bash
   npm run build
   ```

5. **找到构建的应用**：
   - macOS: `apps/desktop/src-tauri/target/release/bundle/dmg/`
   - Windows: `apps/desktop/src-tauri/target/release/bundle/msi/`
   - Linux: `apps/desktop/src-tauri/target/release/bundle/appimage/`

## 验证安装

安装完成后，验证 NeuraDock 正常工作：

1. 启动应用
2. 检查仪表盘是否正确加载
3. 导航到 设置 → 关于 以验证版本

## 安装故障排除

| 问题 | 解决方案 |
|-----|---------|
| macOS: "应用已损坏" | 运行 `xattr -cr /Applications/NeuraDock.app` |
| Windows: SmartScreen 警告 | 点击"更多信息" → "仍要运行" |
| Linux: AppImage 无法运行 | 确保安装了 FUSE: `sudo apt install fuse` |
| 依赖构建失败 | 运行 `npm install --legacy-peer-deps` |

更多解决方案请参阅[故障排除](./user_guide/troubleshooting.md)。
