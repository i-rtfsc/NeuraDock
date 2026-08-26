use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use reqwest::{Client, RequestBuilder, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{FromRow, SqlitePool};
use url::Url;
use uuid::Uuid;

use neuradock_infrastructure::security::EncryptionService;

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct DeveloperToolConnectionDto {
    pub id: String,
    pub name: String,
    pub service: String,
    pub base_url: String,
    pub deployment: String,
    pub auth_method: String,
    pub username: Option<String>,
    pub has_secret: bool,
    pub profile_json: Option<String>,
}

#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct SaveDeveloperToolConnectionInput {
    pub id: Option<String>,
    pub name: String,
    pub service: String,
    pub base_url: String,
    pub deployment: String,
    pub auth_method: String,
    pub username: Option<String>,
    pub secret: Option<String>,
    pub profile_json: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct DeveloperToolCapabilityDto {
    pub service: String,
    pub deployment: String,
    pub version: Option<String>,
    pub current_user: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct JiraMetadataDto {
    pub fields: Vec<JiraFieldDto>,
    pub statuses: Vec<JiraStatusDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct JiraFieldDto {
    pub id: String,
    pub name: String,
    pub custom: bool,
    pub field_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct JiraStatusDto {
    pub id: String,
    pub name: String,
    pub category: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct JiraIssueDto {
    pub key: String,
    pub summary: String,
    pub project: String,
    pub issue_type: String,
    pub priority: String,
    pub status: String,
    pub status_category: String,
    pub assignee: String,
    pub updated: String,
    pub url: String,
    pub semantic_fields_json: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct GerritChangeDto {
    pub id: String,
    pub project: String,
    pub branch: String,
    pub subject: String,
    pub owner: String,
    pub status: String,
    pub updated: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct SavedDeveloperToolQueryDto {
    pub id: String,
    pub connection_id: String,
    pub name: String,
    pub query: String,
}

#[derive(FromRow)]
struct ConnectionRow {
    id: String,
    name: String,
    service: String,
    base_url: String,
    deployment: String,
    auth_method: String,
    username: Option<String>,
    encrypted_secret: Option<String>,
    profile_json: Option<String>,
}

#[derive(FromRow)]
struct SavedQueryRow {
    id: String,
    connection_id: String,
    name: String,
    query: String,
}

#[derive(Debug, Default, Deserialize)]
struct JiraSemanticProfile {
    #[serde(default)]
    field_mappings: std::collections::HashMap<String, FieldMapping>,
    #[serde(default)]
    status_mappings: HashMap<String, String>,
}

#[derive(Debug, Default, Deserialize)]
struct FieldMapping {
    field_id: Option<String>,
    #[serde(default)]
    fields: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DeveloperToolsConfigFile {
    schema_version: u64,
    connections: Vec<ConfigConnection>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ConfigConnection {
    id: String,
    name: String,
    service: String,
    base_url: String,
    deployment: String,
    auth_method: String,
    #[serde(default)]
    username: Option<String>,
    #[serde(default)]
    profile: Option<Value>,
    #[serde(default)]
    saved_queries: Vec<ConfigSavedQuery>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ConfigSavedQuery {
    id: String,
    name: String,
    query: String,
}

pub struct DeveloperToolsService {
    pool: Arc<SqlitePool>,
    encryption: Arc<EncryptionService>,
    client: Client,
    metadata_cache: Mutex<HashMap<String, (Instant, JiraMetadataDto)>>,
    config_directory: PathBuf,
}

impl DeveloperToolsService {
    pub fn new(
        pool: Arc<SqlitePool>,
        encryption: Arc<EncryptionService>,
        app_data_dir: PathBuf,
    ) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .context("Failed to build developer tools HTTP client")?;
        Ok(Self {
            pool,
            encryption,
            client,
            metadata_cache: Mutex::new(HashMap::new()),
            config_directory: std::env::var_os("HOME")
                .map(PathBuf::from)
                .unwrap_or(app_data_dir)
                .join(".neuradock")
                .join("developer-tools"),
        })
    }

    pub async fn list_connections(&self) -> Result<Vec<DeveloperToolConnectionDto>> {
        self.sync_config_files().await?;
        let rows: Vec<ConnectionRow> = sqlx::query_as(
            "SELECT id,name,service,base_url,deployment,auth_method,username,encrypted_secret,profile_json FROM developer_tool_connections ORDER BY name",
        )
        .fetch_all(&*self.pool)
        .await?;
        Ok(rows.into_iter().map(connection_dto).collect())
    }

    pub async fn save_connection(
        &self,
        input: SaveDeveloperToolConnectionInput,
    ) -> Result<DeveloperToolConnectionDto> {
        validate_connection(&input)?;
        let id = input.id.unwrap_or_else(|| Uuid::new_v4().to_string());
        let existing = self.connection(&id).await.ok();
        if existing
            .as_ref()
            .is_some_and(|row| row.service != input.service)
        {
            return Err(anyhow!("A connection service cannot be changed"));
        }
        let encrypted_secret = match input.secret.as_deref().filter(|value| !value.is_empty()) {
            Some(secret) => Some(self.encryption.encrypt(secret)?),
            None => existing
                .as_ref()
                .and_then(|row| row.encrypted_secret.clone()),
        };
        if let Some(profile) = input
            .profile_json
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        {
            validate_profile(profile)?;
        }
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            r#"INSERT INTO developer_tool_connections
               (id,name,service,base_url,deployment,auth_method,username,encrypted_secret,profile_json,created_at,updated_at)
               VALUES (?,?,?,?,?,?,?,?,?,?,?)
               ON CONFLICT(id) DO UPDATE SET name=excluded.name,service=excluded.service,
               base_url=excluded.base_url,deployment=excluded.deployment,auth_method=excluded.auth_method,
               username=excluded.username,encrypted_secret=excluded.encrypted_secret,
               profile_json=excluded.profile_json,updated_at=excluded.updated_at"#,
        )
        .bind(&id).bind(input.name.trim()).bind(&input.service).bind(input.base_url.trim_end_matches('/'))
        .bind(&input.deployment).bind(&input.auth_method).bind(input.username).bind(encrypted_secret)
        .bind(input.profile_json).bind(&now).bind(&now).execute(&*self.pool).await?;
        self.metadata_cache
            .lock()
            .expect("metadata cache poisoned")
            .remove(&id);
        Ok(connection_dto(self.connection(&id).await?))
    }

    pub async fn delete_connection(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM developer_tool_connections WHERE id=?")
            .bind(id)
            .execute(&*self.pool)
            .await?;
        self.metadata_cache
            .lock()
            .expect("metadata cache poisoned")
            .remove(id);
        Ok(())
    }

    pub async fn test_connection(&self, id: &str) -> Result<DeveloperToolCapabilityDto> {
        let row = self.connection(id).await?;
        let secret = self.secret(&row)?;
        match row.service.as_str() {
            "jira" => self.test_jira(&row, &secret).await,
            "gerrit" => self.test_gerrit(&row, &secret).await,
            _ => Err(anyhow!("Unsupported developer tool service")),
        }
    }

    pub async fn jira_metadata(&self, id: &str) -> Result<JiraMetadataDto> {
        if let Some(metadata) = self.cached_metadata(id) {
            return Ok(metadata);
        }
        let row = self.connection(id).await?;
        ensure_service(&row, "jira")?;
        let secret = self.secret(&row)?;
        let fields: Vec<Value> = self
            .jira_json(&row, &secret, "/rest/api/2/field", &[])
            .await?;
        let statuses: Vec<Value> = self
            .jira_json(&row, &secret, "/rest/api/2/status", &[])
            .await?;
        let metadata = JiraMetadataDto {
            fields: fields
                .into_iter()
                .filter_map(|field| {
                    Some(JiraFieldDto {
                        id: field.get("id")?.as_str()?.to_string(),
                        name: field.get("name")?.as_str()?.to_string(),
                        custom: field
                            .get("custom")
                            .and_then(Value::as_bool)
                            .unwrap_or(false),
                        field_type: field
                            .pointer("/schema/type")
                            .and_then(Value::as_str)
                            .map(str::to_string),
                    })
                })
                .collect(),
            statuses: statuses
                .into_iter()
                .filter_map(|status| {
                    Some(JiraStatusDto {
                        id: status.get("id")?.as_str()?.to_string(),
                        name: status.get("name")?.as_str()?.to_string(),
                        category: normalize_status_category(
                            status
                                .pointer("/statusCategory/key")
                                .and_then(Value::as_str),
                        ),
                    })
                })
                .collect(),
        };
        self.metadata_cache
            .lock()
            .expect("metadata cache poisoned")
            .insert(id.to_string(), (Instant::now(), metadata.clone()));
        Ok(metadata)
    }

    pub async fn query_jira(&self, id: &str, jql: &str) -> Result<Vec<JiraIssueDto>> {
        let row = self.connection(id).await?;
        ensure_service(&row, "jira")?;
        let secret = self.secret(&row)?;
        let profile = row
            .profile_json
            .as_deref()
            .and_then(|raw| serde_json::from_str::<JiraSemanticProfile>(raw).ok())
            .unwrap_or_default();
        let mut fields = vec![
            "summary",
            "project",
            "issuetype",
            "priority",
            "status",
            "assignee",
            "updated",
        ];
        let semantic_ids = profile
            .field_mappings
            .values()
            .flat_map(mapping_fields)
            .collect::<Vec<_>>();
        fields.extend(semantic_ids.iter().copied().map(String::as_str));
        let query = vec![
            ("jql", jql.to_string()),
            ("fields", fields.join(",")),
            ("maxResults", "100".to_string()),
        ];
        let result: Value = self
            .jira_json(&row, &secret, "/rest/api/2/search", &query)
            .await?;
        Ok(result
            .get("issues")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|issue| {
                let key = issue.get("key")?.as_str()?.to_string();
                let data = issue.get("fields")?;
                let mut semantic = serde_json::Map::new();
                for (slot, mapping) in &profile.field_mappings {
                    let value = mapping_fields(mapping).find_map(|field| {
                        data.get(field)
                            .filter(|v| !v.is_null())
                            .map(compact_json_value)
                    });
                    if let Some(value) = value {
                        semantic.insert(slot.clone(), value);
                    }
                }
                Some(JiraIssueDto {
                    url: format!("{}/browse/{key}", row.base_url),
                    key,
                    summary: string_at(data, "/summary"),
                    project: string_at(data, "/project/name"),
                    issue_type: string_at(data, "/issuetype/name"),
                    priority: string_at(data, "/priority/name"),
                    status: string_at(data, "/status/name"),
                    status_category: mapped_status_category(
                        &profile,
                        data.pointer("/status/id").and_then(Value::as_str),
                        data.pointer("/status/name").and_then(Value::as_str),
                        data.pointer("/status/statusCategory/key")
                            .and_then(Value::as_str),
                    ),
                    assignee: string_at(data, "/assignee/displayName"),
                    updated: string_at(data, "/updated"),
                    semantic_fields_json: Value::Object(semantic).to_string(),
                })
            })
            .collect())
    }

    pub async fn query_gerrit(&self, id: &str, query: &str) -> Result<Vec<GerritChangeDto>> {
        let row = self.connection(id).await?;
        ensure_service(&row, "gerrit")?;
        let secret = self.secret(&row)?;
        let auth_path = row.auth_method != "anonymous";
        let path = if auth_path {
            "/a/changes/"
        } else {
            "/changes/"
        };
        let request = self
            .auth(
                self.client.get(format!("{}{}", row.base_url, path)),
                &row,
                &secret,
            )
            .query(&[("q", query), ("o", "DETAILED_ACCOUNTS"), ("n", "100")]);
        let text = self.send_text(request, "Gerrit").await?;
        let json_text = text.strip_prefix(")]}'").unwrap_or(&text).trim();
        let values: Vec<Value> =
            serde_json::from_str(json_text).context("Invalid Gerrit response")?;
        Ok(values
            .into_iter()
            .filter_map(|change| {
                let number = change.get("_number")?.as_u64()?;
                Some(GerritChangeDto {
                    id: number.to_string(),
                    project: string_at(&change, "/project"),
                    branch: string_at(&change, "/branch"),
                    subject: string_at(&change, "/subject"),
                    owner: string_at(&change, "/owner/name"),
                    status: string_at(&change, "/status"),
                    updated: string_at(&change, "/updated"),
                    url: format!(
                        "{}/c/{}/+/{}",
                        row.base_url,
                        urlencoding::encode(&string_at(&change, "/project")),
                        number
                    ),
                })
            })
            .collect())
    }

    pub async fn list_saved_queries(
        &self,
        connection_id: &str,
    ) -> Result<Vec<SavedDeveloperToolQueryDto>> {
        let rows: Vec<SavedQueryRow> = sqlx::query_as("SELECT id,connection_id,name,query FROM developer_tool_saved_queries WHERE connection_id=? ORDER BY updated_at DESC")
            .bind(connection_id).fetch_all(&*self.pool).await?;
        Ok(rows
            .into_iter()
            .map(|r| SavedDeveloperToolQueryDto {
                id: r.id,
                connection_id: r.connection_id,
                name: r.name,
                query: r.query,
            })
            .collect())
    }

    pub async fn save_query(
        &self,
        connection_id: &str,
        id: Option<String>,
        name: &str,
        query: &str,
    ) -> Result<SavedDeveloperToolQueryDto> {
        if name.trim().is_empty() || query.trim().is_empty() {
            return Err(anyhow!("Saved query name and query are required"));
        }
        self.connection(connection_id).await?;
        if let Some(existing_id) = id.as_deref() {
            let owner: Option<String> = sqlx::query_scalar(
                "SELECT connection_id FROM developer_tool_saved_queries WHERE id=?",
            )
            .bind(existing_id)
            .fetch_optional(&*self.pool)
            .await?;
            if owner.as_deref().is_some_and(|owner| owner != connection_id) {
                return Err(anyhow!("Saved query does not belong to this connection"));
            }
        }
        let id = id.unwrap_or_else(|| Uuid::new_v4().to_string());
        let now = Utc::now().to_rfc3339();
        sqlx::query("INSERT INTO developer_tool_saved_queries(id,connection_id,name,query,created_at,updated_at) VALUES(?,?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET name=excluded.name,query=excluded.query,updated_at=excluded.updated_at")
            .bind(&id).bind(connection_id).bind(name.trim()).bind(query.trim()).bind(&now).bind(&now).execute(&*self.pool).await?;
        Ok(SavedDeveloperToolQueryDto {
            id,
            connection_id: connection_id.to_string(),
            name: name.trim().to_string(),
            query: query.trim().to_string(),
        })
    }

    pub async fn delete_query(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM developer_tool_saved_queries WHERE id=?")
            .bind(id)
            .execute(&*self.pool)
            .await?;
        Ok(())
    }

    async fn connection(&self, id: &str) -> Result<ConnectionRow> {
        sqlx::query_as("SELECT id,name,service,base_url,deployment,auth_method,username,encrypted_secret,profile_json FROM developer_tool_connections WHERE id=?")
            .bind(id).fetch_optional(&*self.pool).await?.ok_or_else(|| anyhow!("Developer tool connection not found"))
    }

    async fn sync_config_files(&self) -> Result<()> {
        std::fs::create_dir_all(&self.config_directory)
            .context("Failed to create developer tools configuration directory")?;
        for entry in std::fs::read_dir(&self.config_directory)
            .context("Failed to read developer tools configuration directory")?
        {
            let entry = entry.context("Failed to read developer tools configuration entry")?;
            let path = entry.path();
            if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
                continue;
            }
            let raw = std::fs::read_to_string(&path).with_context(|| {
                format!(
                    "Failed to read developer tools configuration {}",
                    path.display()
                )
            })?;
            let config: DeveloperToolsConfigFile =
                serde_json::from_str(&raw).with_context(|| {
                    format!("Invalid developer tools configuration {}", path.display())
                })?;
            if config.schema_version != 1 {
                return Err(anyhow!(
                    "Developer tools configuration {} must use schema_version 1",
                    path.display()
                ));
            }
            for connection in config.connections {
                let connection_id = connection.id.clone();
                let existing = self.connection(&connection_id).await.ok();
                let username = connection
                    .username
                    .or_else(|| existing.as_ref().and_then(|row| row.username.clone()));
                let profile_json = connection
                    .profile
                    .map(|profile| profile.to_string())
                    .or_else(|| existing.as_ref().and_then(|row| row.profile_json.clone()));
                self.save_connection(SaveDeveloperToolConnectionInput {
                    id: Some(connection_id.clone()),
                    name: connection.name,
                    service: connection.service,
                    base_url: connection.base_url,
                    deployment: connection.deployment,
                    auth_method: connection.auth_method,
                    username,
                    secret: None,
                    profile_json,
                })
                .await?;
                for query in connection.saved_queries {
                    self.save_query(&connection_id, Some(query.id), &query.name, &query.query)
                        .await?;
                }
            }
        }
        Ok(())
    }

    fn cached_metadata(&self, id: &str) -> Option<JiraMetadataDto> {
        const METADATA_TTL: Duration = Duration::from_secs(15 * 60);
        self.metadata_cache
            .lock()
            .expect("metadata cache poisoned")
            .get(id)
            .filter(|(created, _)| created.elapsed() < METADATA_TTL)
            .map(|(_, metadata)| metadata.clone())
    }

    fn secret(&self, row: &ConnectionRow) -> Result<String> {
        match row.encrypted_secret.as_deref() {
            Some(secret) => self
                .encryption
                .decrypt(secret)
                .context("Failed to decrypt connection credential"),
            None => Ok(String::new()),
        }
    }

    fn auth(&self, request: RequestBuilder, row: &ConnectionRow, secret: &str) -> RequestBuilder {
        match row.auth_method.as_str() {
            "basic" | "api_token" | "http_password" => {
                request.basic_auth(row.username.as_deref().unwrap_or(""), Some(secret))
            }
            "bearer" | "pat" => request.bearer_auth(secret),
            _ => request,
        }
    }

    async fn jira_json<T: serde::de::DeserializeOwned>(
        &self,
        row: &ConnectionRow,
        secret: &str,
        path: &str,
        query: &[(&str, String)],
    ) -> Result<T> {
        let request = self
            .auth(
                self.client.get(format!("{}{}", row.base_url, path)),
                row,
                secret,
            )
            .query(query);
        let text = self.send_text(request, "Jira").await?;
        serde_json::from_str(&text).context("Invalid Jira response")
    }

    async fn send_text(&self, request: RequestBuilder, service: &str) -> Result<String> {
        let response = request
            .send()
            .await
            .with_context(|| format!("{service} request failed"))?;
        let status = response.status();
        if status == StatusCode::TOO_MANY_REQUESTS {
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|value| value.parse::<u64>().ok());
            let message = match retry_after {
                Some(seconds) if seconds <= 300 => {
                    format!("{service} rate limited the request; retry after {seconds} seconds")
                }
                _ => format!(
                    "{service} rejected the request due to its rate-limit policy; retry later or contact the service administrator"
                ),
            };
            return Err(anyhow!(message));
        }
        if !status.is_success() {
            return Err(anyhow!(
                "{service} request failed with HTTP {}",
                status.as_u16()
            ));
        }
        const MAX_RESPONSE_BYTES: usize = 5 * 1024 * 1024;
        if response
            .content_length()
            .is_some_and(|length| length > MAX_RESPONSE_BYTES as u64)
        {
            return Err(anyhow!("{service} response exceeded the size limit"));
        }
        let bytes = response
            .bytes()
            .await
            .with_context(|| format!("Failed to read {service} response"))?;
        if bytes.len() > MAX_RESPONSE_BYTES {
            return Err(anyhow!("{service} response exceeded the size limit"));
        }
        String::from_utf8(bytes.to_vec())
            .with_context(|| format!("Invalid {service} text response"))
    }

    async fn test_jira(
        &self,
        row: &ConnectionRow,
        secret: &str,
    ) -> Result<DeveloperToolCapabilityDto> {
        let info: Value = self
            .jira_json(row, secret, "/rest/api/2/serverInfo", &[])
            .await?;
        let myself: Value = self
            .jira_json(row, secret, "/rest/api/2/myself", &[])
            .await
            .unwrap_or(Value::Null);
        let detected = if info
            .get("deploymentType")
            .and_then(Value::as_str)
            .is_some_and(|v| v.eq_ignore_ascii_case("cloud"))
        {
            "jira_cloud"
        } else {
            "jira_data_center"
        };
        Ok(DeveloperToolCapabilityDto {
            service: "jira".into(),
            deployment: detected.into(),
            version: info
                .get("version")
                .and_then(Value::as_str)
                .map(str::to_string),
            current_user: myself
                .get("displayName")
                .and_then(Value::as_str)
                .map(str::to_string),
        })
    }

    async fn test_gerrit(
        &self,
        row: &ConnectionRow,
        secret: &str,
    ) -> Result<DeveloperToolCapabilityDto> {
        let request = self.auth(
            self.client
                .get(format!("{}/config/server/version", row.base_url)),
            row,
            secret,
        );
        let text = self.send_text(request, "Gerrit").await?;
        Ok(DeveloperToolCapabilityDto {
            service: "gerrit".into(),
            deployment: "gerrit".into(),
            version: serde_json::from_str::<String>(
                text.strip_prefix(")]}'").unwrap_or(&text).trim(),
            )
            .ok(),
            current_user: None,
        })
    }
}

fn validate_connection(input: &SaveDeveloperToolConnectionInput) -> Result<()> {
    if input.name.trim().is_empty() {
        return Err(anyhow!("Connection name is required"));
    }
    if !matches!(input.service.as_str(), "jira" | "gerrit") {
        return Err(anyhow!("Unsupported service"));
    }
    match input.service.as_str() {
        "jira"
            if !matches!(
                input.deployment.as_str(),
                "auto" | "jira_cloud" | "jira_data_center"
            ) =>
        {
            return Err(anyhow!("Unsupported Jira deployment"))
        }
        "gerrit" if !matches!(input.deployment.as_str(), "auto" | "gerrit") => {
            return Err(anyhow!("Unsupported Gerrit deployment"))
        }
        _ => {}
    }
    match input.service.as_str() {
        "jira" if !matches!(input.auth_method.as_str(), "basic" | "api_token" | "pat") => {
            return Err(anyhow!("Unsupported Jira authentication method"))
        }
        "gerrit"
            if !matches!(
                input.auth_method.as_str(),
                "http_password" | "bearer" | "anonymous"
            ) =>
        {
            return Err(anyhow!("Unsupported Gerrit authentication method"))
        }
        _ => {}
    }
    let requires_username = matches!(
        (input.service.as_str(), input.auth_method.as_str()),
        ("jira", "basic") | ("gerrit", "http_password")
    );
    if requires_username
        && input
            .username
            .as_deref()
            .is_none_or(|value| value.trim().is_empty())
    {
        return Err(anyhow!("This authentication method requires a username"));
    }
    if input.auth_method == "anonymous"
        && input
            .secret
            .as_deref()
            .is_some_and(|value| !value.is_empty())
    {
        return Err(anyhow!("Anonymous connections cannot include a secret"));
    }
    if input.id.is_none()
        && input.auth_method != "anonymous"
        && input
            .secret
            .as_deref()
            .is_none_or(|value| value.trim().is_empty())
    {
        return Err(anyhow!("This authentication method requires a secret"));
    }
    let url = Url::parse(&input.base_url).context("Invalid base URL")?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || url.username() != ""
        || url.password().is_some()
    {
        return Err(anyhow!("Base URL must be HTTP or HTTPS"));
    }
    Ok(())
}

fn validate_profile(raw: &str) -> Result<()> {
    let value: Value = serde_json::from_str(raw).context("Profile is not valid JSON")?;
    if value.get("schema_version").and_then(Value::as_u64) != Some(1) {
        return Err(anyhow!("Profile schema_version must be 1"));
    }
    reject_profile_secrets(&value)?;
    if let Some(mappings) = value.get("status_mappings").and_then(Value::as_object) {
        for category in mappings.values().filter_map(Value::as_str) {
            if !matches!(category, "todo" | "in_progress" | "done" | "ignored") {
                return Err(anyhow!(
                    "Profile status mappings must use todo, in_progress, done, or ignored"
                ));
            }
        }
    }
    serde_json::from_value::<JiraSemanticProfile>(value)
        .context("Invalid Jira semantic profile")?;
    Ok(())
}

fn reject_profile_secrets(value: &Value) -> Result<()> {
    const SECRET_KEYS: &[&str] = &[
        "password",
        "secret",
        "token",
        "api_key",
        "authorization",
        "cookie",
    ];
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                if SECRET_KEYS
                    .iter()
                    .any(|candidate| key.eq_ignore_ascii_case(candidate))
                {
                    return Err(anyhow!(
                        "Credentials are not allowed in a Jira semantic profile"
                    ));
                }
                reject_profile_secrets(child)?;
            }
        }
        Value::Array(items) => {
            for item in items {
                reject_profile_secrets(item)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn connection_dto(row: ConnectionRow) -> DeveloperToolConnectionDto {
    DeveloperToolConnectionDto {
        id: row.id,
        name: row.name,
        service: row.service,
        base_url: row.base_url,
        deployment: row.deployment,
        auth_method: row.auth_method,
        username: row.username,
        has_secret: row.encrypted_secret.is_some(),
        profile_json: row.profile_json,
    }
}

fn ensure_service(row: &ConnectionRow, expected: &str) -> Result<()> {
    if row.service == expected {
        Ok(())
    } else {
        Err(anyhow!("Connection service does not match this operation"))
    }
}

fn mapping_fields(mapping: &FieldMapping) -> impl Iterator<Item = &String> {
    mapping.field_id.iter().chain(mapping.fields.iter())
}

fn string_at(value: &Value, pointer: &str) -> String {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

fn compact_json_value(value: &Value) -> Value {
    match value {
        Value::Object(map) => map
            .get("displayName")
            .or_else(|| map.get("name"))
            .or_else(|| map.get("value"))
            .cloned()
            .unwrap_or_else(|| value.clone()),
        Value::Array(items) => Value::Array(items.iter().map(compact_json_value).collect()),
        _ => value.clone(),
    }
}

fn normalize_status_category(category: Option<&str>) -> String {
    match category.unwrap_or_default() {
        "new" | "to-do" => "todo",
        "indeterminate" | "in-progress" => "in_progress",
        "done" | "complete" => "done",
        _ => "unknown",
    }
    .to_string()
}

fn mapped_status_category(
    profile: &JiraSemanticProfile,
    id: Option<&str>,
    name: Option<&str>,
    default_category: Option<&str>,
) -> String {
    id.and_then(|value| profile.status_mappings.get(value))
        .or_else(|| name.and_then(|value| profile.status_mappings.get(value)))
        .cloned()
        .unwrap_or_else(|| normalize_status_category(default_category))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_requires_version_and_accepts_fallback_fields() {
        let raw = r#"{"schema_version":1,"field_mappings":{"slot":{"fields":["field_alpha","assignee"]}}}"#;
        validate_profile(raw).unwrap();
        assert!(validate_profile(r#"{"field_mappings":{}}"#).is_err());
    }

    #[test]
    fn status_categories_keep_unknown_workflows_visible() {
        assert_eq!(normalize_status_category(Some("done")), "done");
        assert_eq!(
            normalize_status_category(Some("unknown-category")),
            "unknown"
        );
    }

    #[test]
    fn profile_status_mapping_is_data_driven_and_rejects_credentials() {
        let profile: JiraSemanticProfile =
            serde_json::from_str(r#"{"status_mappings":{"42":"ignored"}}"#).unwrap();
        assert_eq!(
            mapped_status_category(&profile, Some("42"), Some("Local status"), Some("done")),
            "ignored"
        );
        assert!(validate_profile(r#"{"schema_version":1,"token":"must-not-live-here"}"#).is_err());
        assert!(
            validate_profile(r#"{"schema_version":1,"status_mappings":{"42":"invalid"}}"#).is_err()
        );
    }

    #[test]
    fn connection_base_url_rejects_embedded_credentials() {
        let input = SaveDeveloperToolConnectionInput {
            id: None,
            name: "Jira".into(),
            service: "jira".into(),
            base_url: "https://user:secret@example.com".into(),
            deployment: "jira_cloud".into(),
            auth_method: "api_token".into(),
            username: None,
            secret: None,
            profile_json: None,
        };

        assert!(validate_connection(&input).is_err());
    }

    #[test]
    fn connection_authentication_fields_are_consistent() {
        let mut input = SaveDeveloperToolConnectionInput {
            id: None,
            name: "Jira".into(),
            service: "jira".into(),
            base_url: "https://example.com".into(),
            deployment: "jira_cloud".into(),
            auth_method: "basic".into(),
            username: None,
            secret: Some("secret".into()),
            profile_json: None,
        };
        assert!(validate_connection(&input).is_err());

        input.username = Some("   ".into());
        assert!(validate_connection(&input).is_err());

        input.auth_method = "pat".into();
        input.username = None;
        assert!(validate_connection(&input).is_ok());

        input.secret = None;
        assert!(validate_connection(&input).is_err());
        input.id = Some("existing".into());
        assert!(validate_connection(&input).is_ok());

        input.service = "gerrit".into();
        input.deployment = "gerrit".into();
        input.auth_method = "anonymous".into();
        input.secret = Some("unexpected".into());
        assert!(validate_connection(&input).is_err());
    }
}
