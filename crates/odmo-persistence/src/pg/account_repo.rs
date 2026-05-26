use odmo_application::account::AccountRepository;
use odmo_types::{AccessLevel, Account, AccountSuspension, ServerDescriptor};

use super::PgRepository;

impl AccountRepository for PgRepository {
    fn account_by_username(&self, username: &str) -> anyhow::Result<Option<Account>> {
        let pool = self.pool().clone();
        let username = username.to_string();
        self.block_on(async move {
            let row: Option<(i64, String, String, String, i16, Option<String>, Option<i32>, Option<String>)> =
                sqlx::query_as(
                    "SELECT id, username, password_hash, email, access_level, secondary_password, suspension_remaining_seconds, suspension_reason FROM accounts WHERE username = $1",
                )
                .bind(&username)
                .fetch_optional(&pool)
                .await?;

            Ok(row.map(map_account))
        })
    }

    fn account_by_id(&self, account_id: odmo_types::AccountId) -> anyhow::Result<Option<Account>> {
        let pool = self.pool().clone();
        self.block_on(async move {
            let row: Option<(i64, String, String, String, i16, Option<String>, Option<i32>, Option<String>)> =
                sqlx::query_as(
                    "SELECT id, username, password_hash, email, access_level, secondary_password, suspension_remaining_seconds, suspension_reason FROM accounts WHERE id = $1",
                )
                .bind(account_id as i64)
                .fetch_optional(&pool)
                .await?;

            Ok(row.map(map_account))
        })
    }

    fn update_secondary_password(
        &self,
        account_id: odmo_types::AccountId,
        password: String,
    ) -> anyhow::Result<()> {
        let pool = self.pool().clone();
        self.block_on(async move {
            sqlx::query("UPDATE accounts SET secondary_password = $1 WHERE id = $2")
                .bind(&password)
                .bind(account_id as i64)
                .execute(&pool)
                .await?;
            Ok(())
        })
    }

    fn list_servers(&self) -> anyhow::Result<Vec<ServerDescriptor>> {
        let pool = self.pool().clone();
        self.block_on(async move {
            let rows: Vec<(i32, String, bool, bool, bool, i16)> = sqlx::query_as(
                "SELECT id, name, maintenance, overloaded, is_new, character_count FROM servers ORDER BY id",
            )
            .fetch_all(&pool)
            .await?;

            Ok(rows
                .into_iter()
                .map(|(id, name, maintenance, overloaded, is_new, character_count)| {
                    ServerDescriptor {
                        id: id as u32,
                        name,
                        maintenance,
                        overloaded,
                        is_new,
                        character_count: character_count as u8,
                    }
                })
                .collect())
        })
    }

    fn resource_hash_hex(&self) -> anyhow::Result<Option<String>> {
        let pool = self.pool().clone();
        self.block_on(async move {
            let row: Option<(String,)> =
                sqlx::query_as("SELECT value FROM server_config WHERE key = 'resource_hash_hex'")
                    .fetch_optional(&pool)
                    .await?;

            Ok(row.map(|(v,)| v))
        })
    }
}

type AccountRow = (
    i64,
    String,
    String,
    String,
    i16,
    Option<String>,
    Option<i32>,
    Option<String>,
);

fn map_account(row: AccountRow) -> Account {
    let (
        id,
        username,
        password_hash,
        email,
        access_level,
        secondary_password,
        susp_secs,
        susp_reason,
    ) = row;
    Account {
        id: id as u64,
        username,
        password_hash,
        email,
        access_level: match access_level {
            0 => AccessLevel::Player,
            1 => AccessLevel::GameMaster,
            _ => AccessLevel::Administrator,
        },
        secondary_password,
        suspension: susp_secs.map(|secs| AccountSuspension {
            remaining_seconds: secs as u32,
            reason: susp_reason.unwrap_or_default(),
        }),
    }
}
