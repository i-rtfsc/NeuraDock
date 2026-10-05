mod aggregate;
mod domain_service;
mod provider;
mod quota_day;
mod repository;
mod state;
mod value_objects;

#[cfg(test)]
mod aggregate_test;
#[cfg(test)]
mod value_objects_test;

pub use aggregate::CheckInJob;
pub use domain_service::CheckInDomainService;
pub use provider::{CheckInMode, Provider, ProviderConfig};
pub use quota_day::{
    next_quota_day_start, quota_day_of, quota_day_start, today as today_quota_day,
    DEFAULT_DAY_OFFSET_MINUTES,
};
pub use repository::{CheckInJobRepository, ProviderRepository};
pub use state::{evaluate_check_in_state, CheckInPolicy, CheckInState};
pub use value_objects::Balance;
#[allow(unused_imports)]
pub use value_objects::{CheckInResult, CheckInStatus};
