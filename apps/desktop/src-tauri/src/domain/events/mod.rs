use serde::{Serialize, Deserialize};
use specta::Type;

pub trait DomainEvent: Send + Sync {}
