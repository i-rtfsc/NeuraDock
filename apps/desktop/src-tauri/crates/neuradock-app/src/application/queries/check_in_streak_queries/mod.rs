use std::sync::Arc;

use crate::application::dtos::{
    CheckInCalendarDto, CheckInDayDto, CheckInStreakDto, CheckInTrendDto,
};
use neuradock_domain::account::AccountRepository;
use neuradock_domain::balance_history::BalanceHistoryRepository;
use neuradock_domain::check_in::ProviderRepository;
use neuradock_domain::shared::DomainError;

mod calendar;
mod helpers;
mod streak;
mod trend;
mod types;

pub struct CheckInStreakQueries {
    account_repo: Arc<dyn AccountRepository>,
    provider_repo: Arc<dyn ProviderRepository>,
    balance_history_repo: Arc<dyn BalanceHistoryRepository>,
}

impl CheckInStreakQueries {
    pub fn new(
        account_repo: Arc<dyn AccountRepository>,
        provider_repo: Arc<dyn ProviderRepository>,
        balance_history_repo: Arc<dyn BalanceHistoryRepository>,
    ) -> Self {
        Self {
            account_repo,
            provider_repo,
            balance_history_repo,
        }
    }

    /// Get streak statistics for a single account
    pub async fn get_streak_stats(
        &self,
        account_id: &str,
    ) -> Result<CheckInStreakDto, DomainError> {
        streak::get_streak_stats(
            self.account_repo.as_ref(),
            self.provider_repo.as_ref(),
            self.balance_history_repo.as_ref(),
            account_id,
        )
        .await
    }

    /// Get streak statistics for all accounts
    pub async fn get_all_streaks(&self) -> Result<Vec<CheckInStreakDto>, DomainError> {
        streak::get_all_streaks(
            self.account_repo.as_ref(),
            self.provider_repo.as_ref(),
            self.balance_history_repo.as_ref(),
        )
        .await
    }

    /// Get check-in calendar for a specific month
    pub async fn get_calendar(
        &self,
        account_id: &str,
        year: i32,
        month: u32,
    ) -> Result<CheckInCalendarDto, DomainError> {
        calendar::get_calendar(self.balance_history_repo.as_ref(), account_id, year, month).await
    }

    /// Get check-in trend data (last N days)
    pub async fn get_trend(
        &self,
        account_id: &str,
        days: u32,
    ) -> Result<CheckInTrendDto, DomainError> {
        trend::get_trend(self.balance_history_repo.as_ref(), account_id, days).await
    }

    /// Get details for a specific day
    pub async fn get_day_detail(
        &self,
        account_id: &str,
        date: &str,
    ) -> Result<CheckInDayDto, DomainError> {
        trend::get_day_detail(self.balance_history_repo.as_ref(), account_id, date).await
    }
}
