# ADR-005: WAF 绕过策略

## 状态
已采纳

## 背景
某些服务商（如 AnyRouter）使用 Cloudflare WAF 保护。直接 HTTP 请求会失败，返回 403 或挑战页面。我们需要：
- 在通过 WAF 挑战后获取有效 cookies
- 缓存会话以避免重复浏览器自动化
- 优雅地处理 cookie 过期

## 决策
我们使用 **chromiumoxide** crate 进行**浏览器自动化**：
1. 启动无头 Chromium 浏览器
2. 导航到服务商网站
3. 等待 Cloudflare 挑战完成
4. 从浏览器提取 cookies
5. 使用 cookies 进行后续 API 请求
6. 带过期时间缓存会话

### 实现

```rust
// infrastructure/http/waf_bypass.rs

pub async fn bypass_waf(url: &str) -> Result<HashMap<String, String>> {
    // 1. 查找已安装的浏览器
    let browser_path = find_chromium_browser()?;

    // 2. 使用临时配置文件启动无头浏览器
    let browser = Browser::new(LaunchOptions {
        headless: true,
        path: Some(browser_path),
        user_data_dir: Some(temp_dir()),
        ..Default::default()
    })?;

    // 3. 导航并等待 Cloudflare
    let page = browser.new_page(url).await?;
    page.wait_for_navigation().await?;

    // 4. 提取 cookies
    let cookies = page.get_cookies().await?;

    // 5. 返回 HashMap
    Ok(cookies_to_hashmap(cookies))
}
```

### 会话缓存
```rust
// Account 聚合存储会话
pub struct Account {
    // ...
    session_token: Option<String>,
    session_expiry: Option<DateTime<Utc>>,
}

impl Account {
    pub fn is_session_valid(&self) -> bool {
        match (&self.session_token, &self.session_expiry) {
            (Some(_), Some(expiry)) => Utc::now() < *expiry,
            _ => false,
        }
    }
}
```

## 后果

### 优点
- 绕过 Cloudflare 和类似的 WAF 保护
- 会话缓存减少浏览器自动化开销
- 与现有浏览器安装配合使用
- 无需专门的反机器人服务

### 缺点
- 需要用户系统安装 Chromium 浏览器
- 浏览器自动化较慢（每次绕过 2-5 秒）
- 无头检测可能在未来 Cloudflare 更新中失败
- 资源密集（启动浏览器进程）

### 浏览器检测
支持的浏览器（按顺序检查）：
1. Google Chrome
2. Microsoft Edge
3. Brave 浏览器
4. Chromium

平台特定路径：
- **macOS**: `/Applications/Google Chrome.app/Contents/MacOS/Google Chrome`
- **Windows**: 注册表 + `C:\Program Files\Google\Chrome\Application\chrome.exe`
- **Linux**: `/usr/bin/google-chrome`, `/usr/bin/chromium`

### 已知限制
1. 浏览器路径未缓存（每次重新发现）
2. 如果所有浏览器都不可用则无回退
3. 浏览器不支持代理
4. Cookie 提取可能遗漏 HttpOnly cookies

### 未来改进
- 缓存浏览器路径
- 支持自定义浏览器路径配置
- 实现使用不同浏览器的重试
- 添加代理支持

## 考虑过的替代方案
- **FlareSolverr**: 外部服务，增加部署复杂性
- **Puppeteer**: 基于 Node.js，不适合 Rust 技术栈
- **直接 cookie 注入**: 需要手动用户干预
- **API 令牌认证**: 并非所有服务商都支持

## 相关
- [架构概览](../architecture_overview.md)
- [故障排除：WAF 问题](../../user_guide/troubleshooting.md#waf-绕过问题)
