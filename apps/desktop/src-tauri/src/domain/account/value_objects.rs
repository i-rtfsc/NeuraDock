use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct Credentials {
    cookies: HashMap<String, String>,
    api_user: String,
}

impl Credentials {
    pub fn new(cookies: HashMap<String, String>, api_user: String) -> Self {
        Self { cookies, api_user }
    }

    pub fn cookies(&self) -> &HashMap<String, String> {
        &self.cookies
    }

    pub fn api_user(&self) -> &str {
        &self.api_user
    }

    pub fn is_valid(&self) -> bool {
        !self.cookies.is_empty()
    }

    pub fn cookie_string(&self) -> String {
        self.cookies
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("; ")
    }
}
