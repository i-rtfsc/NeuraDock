use sqlx::SqlitePool;

#[derive(sqlx::FromRow)]
struct AccountRow {
    id: String,
    name: String,
    cookies: String,
    api_user: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 连接数据库
    let db_path = format!(
        "{}/Library/Application Support/com.rtfsc.neuradock/neuradock.db",
        std::env::var("HOME")?
    );

    let pool = SqlitePool::connect(&format!("sqlite:{}", db_path)).await?;

    println!("🔍 检查账户凭证加密状态...\n");

    // 查询所有账户
    let rows: Vec<AccountRow> = sqlx::query_as("SELECT id, name, cookies, api_user FROM accounts")
        .fetch_all(&pool)
        .await?;

    println!("总账户数: {}\n", rows.len());

    for row in rows {
        println!("账户: {} ({})", row.name, row.id);

        // 检查 cookies 是否加密
        let is_cookies_encrypted = !row.cookies.starts_with("{")
            && row
                .cookies
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '=');

        // 检查 api_user 是否加密
        let is_api_user_encrypted = row
            .api_user
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '=')
            && row.api_user.len() > 20;

        println!(
            "  Cookies: {} (长度: {})",
            if is_cookies_encrypted {
                "✅ 已加密"
            } else {
                "⚠️  明文"
            },
            row.cookies.len()
        );
        println!(
            "  API User: {} (长度: {})",
            if is_api_user_encrypted {
                "✅ 已加密"
            } else {
                "⚠️  明文"
            },
            row.api_user.len()
        );

        if is_cookies_encrypted {
            println!(
                "  Cookies 预览: {}...",
                &row.cookies[..50.min(row.cookies.len())]
            );
        } else {
            println!(
                "  Cookies 预览: {}...",
                &row.cookies[..100.min(row.cookies.len())]
            );
        }

        if is_api_user_encrypted {
            println!(
                "  API User 预览: {}...",
                &row.api_user[..50.min(row.api_user.len())]
            );
        } else {
            println!("  API User: {}", row.api_user);
        }

        println!();
    }

    Ok(())
}
