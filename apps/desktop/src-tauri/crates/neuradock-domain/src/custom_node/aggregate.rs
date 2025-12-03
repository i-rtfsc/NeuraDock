use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::shared::ProviderId;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
pub struct CustomNodeId(i64);

impl CustomNodeId {
    pub fn new(id: i64) -> Self {
        Self(id)
    }

    pub fn value(&self) -> i64 {
        self.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct CustomProviderNode {
    id: CustomNodeId,
    provider_id: ProviderId,
    name: String,
    base_url: String,
    created_at: DateTime<Utc>,
}

impl CustomProviderNode {
    pub fn new(
        id: CustomNodeId,
        provider_id: ProviderId,
        name: String,
        base_url: String,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            provider_id,
            name,
            base_url,
            created_at,
        }
    }

    pub fn create(provider_id: ProviderId, name: String, base_url: String) -> Self {
        Self {
            id: CustomNodeId::new(0), // Will be set by database
            provider_id,
            name,
            base_url,
            created_at: Utc::now(),
        }
    }

    // Getters
    pub fn id(&self) -> &CustomNodeId {
        &self.id
    }

    pub fn provider_id(&self) -> &ProviderId {
        &self.provider_id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    // Business logic
    pub fn update_name(&mut self, name: String) {
        self.name = name;
    }

    pub fn update_base_url(&mut self, base_url: String) {
        self.base_url = base_url;
    }
}
