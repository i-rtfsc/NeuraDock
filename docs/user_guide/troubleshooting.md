# 故障排除

## 常见问题

### 签到失败

| 错误 | 原因 | 解决方案 |
|-----|------|---------|
| "Session expired" | Cookies 已失效 | 在账号设置中更新 cookies |
| "WAF challenge failed" | 无法绕过 Cloudflare 保护 | 确保安装了 Chrome/Edge/Brave |
| "Network error" | 连接问题 | 检查网络连接 |
| "Invalid credentials" | Cookies 或 API 用户错误 | 从浏览器验证凭证 |
| "Account not found" | 账号已删除或服务商已更改 | 重新添加账号 |

### WAF 绕过问题

**问题**：签到失败，提示"WAF bypass failed"

**解决方案**：
1. 确保安装了 Chromium 内核浏览器：
   - Google Chrome
   - Microsoft Edge
   - Brave 浏览器

2. 检查浏览器路径：
   - **macOS**: `/Applications/Google Chrome.app`
   - **Windows**: `C:\Program Files\Google\Chrome\Application\chrome.exe`
   - **Linux**: `/usr/bin/google-chrome` 或 `/usr/bin/chromium`

3. 关闭所有浏览器实例后重试

4. 清除浏览器缓存和 cookies，然后重试

**问题**：检测到浏览器但 WAF 仍然失败

**解决方案**：
1. 将浏览器更新到最新版本
2. 禁用可能干扰的浏览器扩展
3. 检查服务商网站是否可以手动访问

### 余额不更新

**问题**：余额显示过期数据

**解决方案**：
1. 点击账号卡片上的刷新图标
2. 检查 设置 → 余额缓存时间 设置
3. 验证账号凭证是否有效
4. 检查服务商 API 是否可访问

### 自动签到不工作

**问题**：计划的签到未执行

**解决方案**：
1. 验证账号已启用自动签到
2. 检查计划时间是否正确
3. 确保应用正在运行（不只是在托盘中）
4. 检查系统时间是否正确
5. 查看签到历史中的错误消息

### 应用无法启动

**问题**：应用启动时崩溃

**解决方案**：

| 平台 | 解决方案 |
|-----|---------|
| macOS | 运行 `xattr -cr /Applications/NeuraDock.app` |
| Windows | 以管理员身份运行 |
| Linux | 检查是否安装了 WebKit2GTK |

**如果数据库损坏**：
1. 备份你的账号（如果可能导出 JSON）
2. 删除数据库文件（位置见[配置指南](../configuration.md)）
3. 重启应用
4. 重新导入账号

## 已知问题

### 严重（修复中）

1. **凭证未加密存储** - 安全问题
   - 状态：开发中
   - 临时方案：暂无

2. **HTTP 响应体重复读取** - 可能导致签到失败
   - 状态：待修复
   - 临时方案：重试签到

3. **调度器在无效时间时崩溃**
   - 状态：待修复
   - 临时方案：确保有效的小时（0-23）和分钟（0-59）

### 次要

1. **余额可能显示过期数据** - 如果 API 失败
   - 临时方案：手动刷新

2. **通知铃铛是占位符** - 功能未实现
   - 状态：尚未实现

3. **部分命令未实现**：
   - `get_check_in_history`
   - `get_check_in_stats`
   - `stop_check_in`
   - `add_provider`

## 获取调试日志

获取详细日志用于故障排除：

**macOS/Linux**：
```bash
# 从终端运行，启用调试日志
RUST_LOG=debug /Applications/NeuraDock.app/Contents/MacOS/NeuraDock 2>&1 | tee neuradock.log
```

**Windows**：
```powershell
# 从 PowerShell 运行
$env:RUST_LOG="debug"
& "C:\Program Files\NeuraDock\NeuraDock.exe" 2>&1 | Tee-Object -FilePath neuradock.log
```

## 报告问题

报告问题时，请包含：

1. **NeuraDock 版本**（设置 → 关于）
2. **操作系统**及版本
3. **复现步骤**
4. **错误消息**或截图
5. **调试日志**（如有）

报告问题：https://github.com/neuradock/neuradock/issues
