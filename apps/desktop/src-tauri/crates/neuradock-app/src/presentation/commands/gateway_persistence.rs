use serde::{Deserialize, Serialize};

use super::gateway::{GatewayClientDto, GatewaySourceDto, UpstreamKindDto};

#[derive(Debug, Clone)]
pub(super) struct BindingMeta {
    pub(super) label: String,
    pub(super) source: GatewaySourceDto,
    pub(super) kind: UpstreamKindDto,
    pub(super) model: Option<String>,
    pub(super) context1m: bool,
    pub(super) impersonate: bool,
    pub(super) isolated: Option<IsolatedTarget>,
    pub(super) enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct IsolatedTarget {
    pub(super) slug: String,
    pub(super) display: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct PersistedBinding {
    pub(super) client: GatewayClientDto,
    pub(super) source: GatewaySourceDto,
    pub(super) kind: UpstreamKindDto,
    pub(super) model: Option<String>,
    #[serde(default)]
    pub(super) context1m: bool,
    #[serde(default)]
    pub(super) impersonate: bool,
    pub(super) label: String,
    #[serde(default)]
    pub(super) isolated: Option<IsolatedTarget>,
    #[serde(default = "default_true")]
    pub(super) enabled: bool,
}

fn default_true() -> bool {
    true
}
