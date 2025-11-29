use serde::{Deserialize, Serialize};
use specta::Type;

pub trait DomainEvent: Send + Sync {}
