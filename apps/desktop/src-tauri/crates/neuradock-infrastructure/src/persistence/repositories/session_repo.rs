use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, SqlitePool};
use std::sync::Arc;

use neuradock_domain::session::{Session, SessionRepository};
use neuradock_domain::shared::{AccountId, DomainError};
use crate::persistence::RepositoryErrorMapper;

#[derive(FromRow)]
struct SessionRow {
    account_id: String,
    token: String,
    expires_at: DateTime<Utc>,
    last_login_at: DateTime<Utc>,
}

impl SessionRow {
    fn to_session(self) -> Session {
        Session::restore(
            AccountId::from_string(&self.account_id),
            self.token,
            self.expires_at,
            self.last_login_at,
        )
    }
}

pub struct SqliteSessionRepository {
    pool: Arc<SqlitePool>,
}

impl SqliteSessionRepository {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SessionRepository for SqliteSessionRepository {
    async fn save(&self, session: &Session) -> Result<(), DomainError> {
        let query = r#"
            INSERT INTO sessions (account_id, token, expires_at, last_login_at)
            VALUES (?1, ?2, ?3, ?4)
            ON CONFLICT(account_id) DO UPDATE SET
                token = ?2,
                expires_at = ?3,
                last_login_at = ?4
        "#;

        sqlx::query(query)
            .bind(session.account_id().as_str())
            .bind(session.token())
            .bind(session.expires_at())
            .bind(session.last_login_at())
            .execute(&*self.pool)
            .await
            .map_err(|e| RepositoryErrorMapper::map_sqlx_error(e, "Save session"))?;

        Ok(())
    }

    async fn find_by_account_id(&self, account_id: &AccountId) -> Result<Option<Session>, DomainError> {
        let query = "SELECT account_id, token, expires_at, last_login_at FROM sessions WHERE account_id = ?1";

        let row: Option<SessionRow> = sqlx::query_as(query)
            .bind(account_id.as_str())
            .fetch_optional(&*self.pool)
            .await
            .map_err(|e| RepositoryErrorMapper::map_sqlx_error(e, "Find session by account ID"))?;

        Ok(row.map(|r| r.to_session()))
    }

    async fn delete(&self, account_id: &AccountId) -> Result<(), DomainError> {
        let query = "DELETE FROM sessions WHERE account_id = ?1";

        sqlx::query(query)
            .bind(account_id.as_str())
            .execute(&*self.pool)
            .await
            .map_err(|e| RepositoryErrorMapper::map_sqlx_error(e, "Delete session"))?;

        Ok(())
    }

    async fn find_valid_sessions(&self) -> Result<Vec<Session>, DomainError> {
        let query = "SELECT account_id, token, expires_at, last_login_at FROM sessions WHERE expires_at > ?1 ORDER BY last_login_at DESC";

        let rows: Vec<SessionRow> = sqlx::query_as(query)
            .bind(Utc::now())
            .fetch_all(&*self.pool)
            .await
            .map_err(|e| RepositoryErrorMapper::map_sqlx_error(e, "Find valid sessions"))?;

        Ok(rows.into_iter().map(|r| r.to_session()).collect())
    }
}
