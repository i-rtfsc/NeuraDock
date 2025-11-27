# ADR-005: WAF Bypass Strategy

## Status
Accepted

## Context
Some service providers (e.g., AnyRouter) use Cloudflare WAF protection. Direct HTTP requests fail with 403 or challenge pages. We need to:
- Obtain valid cookies after passing WAF challenges
- Cache sessions to avoid repeated browser automation
- Handle cookie expiration gracefully

## Decision
We use **browser automation** with **chromiumoxide** crate to:
1. Launch headless Chromium browser
2. Navigate to provider's website
3. Wait for Cloudflare challenge to complete
4. Extract cookies from browser
5. Use cookies for subsequent API requests
6. Cache session with expiration

### Implementation

```rust
// infrastructure/http/waf_bypass.rs

pub async fn bypass_waf(url: &str) -> Result<HashMap<String, String>> {
    // 1. Find installed browser
    let browser_path = find_chromium_browser()?;

    // 2. Launch headless browser with temp profile
    let browser = Browser::new(LaunchOptions {
        headless: true,
        path: Some(browser_path),
        user_data_dir: Some(temp_dir()),
        ..Default::default()
    })?;

    // 3. Navigate and wait for Cloudflare
    let page = browser.new_page(url).await?;
    page.wait_for_navigation().await?;

    // 4. Extract cookies
    let cookies = page.get_cookies().await?;

    // 5. Return as HashMap
    Ok(cookies_to_hashmap(cookies))
}
```

### Session Caching
```rust
// Account aggregate stores session
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

## Consequences

### Positive
- Bypasses Cloudflare and similar WAF protections
- Session caching reduces browser automation overhead
- Works with existing browser installations
- No need for specialized anti-bot services

### Negative
- Requires Chromium browser installed on user's system
- Browser automation is slow (2-5 seconds per bypass)
- Headless detection may fail with future Cloudflare updates
- Resource intensive (spawns browser process)

### Browser Detection
Supported browsers (checked in order):
1. Google Chrome
2. Microsoft Edge
3. Brave Browser
4. Chromium

Platform-specific paths:
- **macOS**: `/Applications/Google Chrome.app/Contents/MacOS/Google Chrome`
- **Windows**: Registry + `C:\Program Files\Google\Chrome\Application\chrome.exe`
- **Linux**: `/usr/bin/google-chrome`, `/usr/bin/chromium`

### Known Limitations
1. Browser path not cached (rediscovered each time)
2. No fallback if all browsers unavailable
3. No proxy support for browser
4. Cookie extraction may miss HttpOnly cookies

### Future Improvements
- Cache browser path
- Support custom browser path configuration
- Implement retry with different browsers
- Add proxy support

## Alternatives Considered
- **FlareSolverr**: External service, adds deployment complexity
- **Puppeteer**: Node.js based, doesn't fit Rust stack
- **Direct cookie injection**: Requires manual user intervention
- **API token authentication**: Not all providers support this

## Related
- [Architecture Overview](../architecture_overview.md)
- [Troubleshooting: WAF Issues](../../user_guide/troubleshooting.md#waf-bypass-issues)
