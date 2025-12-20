use serde::{Deserialize, Serialize};

/// Request configuration for fetching tokens
#[derive(Debug, Clone)]
pub struct FetchTokensRequest<'a> {
    pub base_url: &'a str,
    pub token_api_path: &'a str,
    pub cookie_string: &'a str,
    pub api_user_header: Option<&'a str>,
    pub api_user: Option<&'a str>,
    pub page: u32,
    pub size: u32,
}

#[derive(Debug, Deserialize)]
pub struct TokenResponse {
    pub success: bool,
    pub message: String,
    #[serde(flatten)]
    pub data: TokenResponseData,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum TokenResponseData {
    /// AgentRouter format: {"data": {"page": 1, "items": [...]}}
    Paginated { data: TokenDataWrapper },
    /// AnyRouter format: {"data": [...]}
    Direct { data: Vec<TokenData> },
}

#[derive(Debug, Deserialize)]
pub struct TokenDataWrapper {
    pub page: u32,
    pub page_size: u32,
    pub total: u32,
    pub items: Vec<TokenData>,
}

impl TokenResponseData {
    pub fn items(&self) -> &[TokenData] {
        match self {
            TokenResponseData::Paginated { data } => &data.items,
            TokenResponseData::Direct { data } => data,
        }
    }

    pub fn page(&self) -> u32 {
        match self {
            TokenResponseData::Paginated { data } => data.page,
            TokenResponseData::Direct { .. } => 1,
        }
    }

    pub fn total(&self) -> u32 {
        match self {
            TokenResponseData::Paginated { data } => data.total,
            TokenResponseData::Direct { data } => data.len() as u32,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TokenData {
    pub id: i64,
    pub user_id: i64,
    pub key: String,
    pub status: i32,
    pub name: String,
    pub created_time: i64,
    pub accessed_time: i64,
    pub expired_time: i64,
    pub remain_quota: i64,
    pub unlimited_quota: bool,
    pub used_quota: i64,
    pub model_limits_enabled: bool,
    pub model_limits: serde_json::Value,
}

/// Response format for provider models API
#[derive(Debug, Deserialize)]
pub struct ProviderModelsResponse {
    pub success: bool,
    pub message: String,
    pub data: Vec<String>, // Changed: data is a simple string array, not objects
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProviderModelData {
    pub id: String,
    #[serde(default)]
    pub object: Option<String>,
    #[serde(default)]
    pub created: Option<i64>,
    #[serde(default)]
    pub owned_by: Option<String>,
}
