/// Manages reading and writing ~/.codex/auth.json
///
/// Format (chatgpt mode):
/// {
///   "auth_mode": "chatgpt",
///   "OPENAI_API_KEY": null,
///   "tokens": { "id_token": "", "access_token": "", "refresh_token": "", "account_id": "" },
///   "last_refresh": "2025-03-26T08:23:00Z"
/// }

use crate::http::openai::quota::CodexUsageQuota;
use serde::{Deserialize, Serialize};
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodexAuthTokens {
    pub id_token: String,
    pub access_token: String,
    pub refresh_token: String,
    pub account_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodexAuthJson {
    pub auth_mode: String,
    #[serde(rename = "OPENAI_API_KEY")]
    pub openai_api_key: Option<String>,
    pub tokens: Option<CodexAuthTokens>,
    pub last_refresh: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct CodexAuthQuotaCacheStore {
    entries: Vec<CodexAuthQuotaCacheEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CodexAuthQuotaCacheEntry {
    fingerprint: String,
    last_refresh: String,
    quota: CodexUsageQuota,
}

impl CodexAuthJson {
    pub fn chatgpt(
        id_token: String,
        access_token: String,
        refresh_token: String,
        account_id: String,
    ) -> Self {
        let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        Self {
            auth_mode: "chatgpt".to_string(),
            openai_api_key: None,
            tokens: Some(CodexAuthTokens {
                id_token,
                access_token,
                refresh_token,
                account_id,
            }),
            last_refresh: now,
        }
    }

    pub fn api_key(key: String) -> Self {
        Self {
            auth_mode: "api_key".to_string(),
            openai_api_key: Some(key),
            tokens: None,
            last_refresh: String::new(),
        }
    }

    pub fn email(&self) -> Option<String> {
        // Try to extract email from id_token JWT payload (no signature verification)
        let id_token = self.tokens.as_ref()?.id_token.as_str();
        let parts: Vec<&str> = id_token.splitn(3, '.').collect();
        if parts.len() < 2 {
            return None;
        }
        let payload = parts[1];
        // Pad base64
        let padded = match payload.len() % 4 {
            2 => format!("{}==", payload),
            3 => format!("{}=", payload),
            _ => payload.to_string(),
        };
        use base64::Engine;
        let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(payload)
            .or_else(|_| base64::engine::general_purpose::STANDARD.decode(&padded))
            .ok()?;
        let claims: serde_json::Value = serde_json::from_slice(&decoded).ok()?;
        claims.get("email")?.as_str().map(|s| s.to_string())
    }
}

fn auth_json_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".codex")
        .join("auth.json")
}

fn quota_cache_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".codex")
        .join("neuradock-active-auth-quota.json")
}

fn ensure_parent_dir(path: &Path) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    Ok(())
}

#[cfg(unix)]
fn set_private_permissions(path: &Path) -> anyhow::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let perms = std::fs::Permissions::from_mode(0o600);
    std::fs::set_permissions(path, perms)?;
    Ok(())
}

#[cfg(not(unix))]
fn set_private_permissions(_path: &Path) -> anyhow::Result<()> { Ok(()) }

fn auth_fingerprint(auth: &CodexAuthJson) -> String {
    let email = auth.email().filter(|value| !value.trim().is_empty());
    let account_id = auth
        .tokens
        .as_ref()
        .and_then(|tokens| (!tokens.account_id.trim().is_empty()).then_some(tokens.account_id.as_str()));

    let mut hasher = DefaultHasher::new();
    auth.auth_mode.hash(&mut hasher);
    email.hash(&mut hasher);
    account_id.hash(&mut hasher);

    if email.is_none() && account_id.is_none() {
        auth.openai_api_key.hash(&mut hasher);

        if let Some(tokens) = auth.tokens.as_ref() {
            tokens.access_token.hash(&mut hasher);
        }
    }

    format!("{:016x}", hasher.finish())
}

fn read_quota_cache_store() -> anyhow::Result<CodexAuthQuotaCacheStore> {
    let path = quota_cache_path();
    if !path.exists() {
        return Ok(CodexAuthQuotaCacheStore::default());
    }

    let content = std::fs::read_to_string(&path)?;
    if content.trim().is_empty() {
        return Ok(CodexAuthQuotaCacheStore::default());
    }

    Ok(serde_json::from_str(&content)?)
}

pub struct CodexAuthFile;

impl CodexAuthFile {
    pub fn read() -> anyhow::Result<Option<CodexAuthJson>> {
        let path = auth_json_path();
        if !path.exists() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(&path)?;
        if content.trim().is_empty() {
            return Ok(None);
        }
        let auth: CodexAuthJson = serde_json::from_str(&content)?;
        Ok(Some(auth))
    }

    pub fn write(auth: &CodexAuthJson) -> anyhow::Result<()> {
        let path = auth_json_path();
        ensure_parent_dir(&path)?;
        let content = serde_json::to_string_pretty(auth)?;
        std::fs::write(&path, content)?;
        set_private_permissions(&path)?;
        Ok(())
    }

    pub fn read_quota_cache(auth: &CodexAuthJson) -> anyhow::Result<Option<(CodexUsageQuota, String)>> {
        let fingerprint = auth_fingerprint(auth);
        let store = read_quota_cache_store()?;

        Ok(store
            .entries
            .into_iter()
            .rev()
            .find(|entry| entry.fingerprint == fingerprint)
            .map(|entry| (entry.quota, entry.last_refresh)))
    }

    pub fn write_quota_cache(
        auth: &CodexAuthJson,
        quota: &CodexUsageQuota,
        last_refresh: &str,
    ) -> anyhow::Result<()> {
        let path = quota_cache_path();
        ensure_parent_dir(&path)?;

        let fingerprint = auth_fingerprint(auth);
        let mut store = read_quota_cache_store()?;
        store.entries.retain(|entry| entry.fingerprint != fingerprint);
        store.entries.push(CodexAuthQuotaCacheEntry {
            fingerprint,
            last_refresh: last_refresh.to_string(),
            quota: quota.clone(),
        });

        if store.entries.len() > 20 {
            let overflow = store.entries.len() - 20;
            store.entries.drain(0..overflow);
        }

        let content = serde_json::to_string_pretty(&store)?;
        std::fs::write(&path, content)?;
        set_private_permissions(&path)?;
        Ok(())
    }

    pub fn clear() -> anyhow::Result<()> {
        let path = auth_json_path();
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        Ok(())
    }

    pub fn active_email() -> Option<String> {
        Self::read().ok().flatten().and_then(|a| a.email())
    }
}
