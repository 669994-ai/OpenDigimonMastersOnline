use odmo_application::portal::{PortalStore, SocialNotification};
use odmo_types::{AccountId, CharacterId, CharacterSummary, GameSessionTicket, TransferTicket};

use super::PgRepository;

// ---------------------------------------------------------------------------
// PostgreSQL implementation of PortalStore
// ---------------------------------------------------------------------------

impl PortalStore for PgRepository {
    // ----- Transfer tickets -----

    fn store_transfer_ticket(&self, ticket: &TransferTicket) -> anyhow::Result<()> {
        self.block_on(async {
            sqlx::query(
                "INSERT INTO transfer_tickets (account_id, token, server_id)
                 VALUES ($1, $2, $3)
                 ON CONFLICT (account_id) DO UPDATE SET token = $2, server_id = $3, created_at = NOW()",
            )
            .bind(ticket.account_id as i64)
            .bind(&ticket.token)
            .bind(ticket.server_id as i32)
            .execute(&self.pool)
            .await?;
            Ok(())
        })
    }

    fn consume_transfer_ticket(
        &self,
        account_id: AccountId,
    ) -> anyhow::Result<Option<TransferTicket>> {
        self.block_on(async {
            let row: Option<(String, i32)> = sqlx::query_as(
                "SELECT token, server_id FROM transfer_tickets WHERE account_id = $1",
            )
            .bind(account_id as i64)
            .fetch_optional(&self.pool)
            .await?;

            if let Some((token, server_id)) = row {
                sqlx::query("DELETE FROM transfer_tickets WHERE account_id = $1")
                    .bind(account_id as i64)
                    .execute(&self.pool)
                    .await?;
                Ok(Some(TransferTicket {
                    token,
                    account_id,
                    server_id: server_id as u32,
                }))
            } else {
                Ok(None)
            }
        })
    }

    // ----- Game session tickets -----

    fn store_game_session_ticket(&self, ticket: &GameSessionTicket) -> anyhow::Result<()> {
        self.block_on(async {
            sqlx::query(
                "INSERT INTO game_session_tickets (account_id, token, character_id)
                 VALUES ($1, $2, $3)
                 ON CONFLICT (account_id) DO UPDATE SET token = $2, character_id = $3, created_at = NOW()",
            )
            .bind(ticket.account_id as i64)
            .bind(&ticket.token)
            .bind(ticket.character_id as i64)
            .execute(&self.pool)
            .await?;
            Ok(())
        })
    }

    fn consume_game_session_ticket(
        &self,
        account_id: AccountId,
    ) -> anyhow::Result<Option<GameSessionTicket>> {
        self.block_on(async {
            let row: Option<(String, i64)> = sqlx::query_as(
                "SELECT token, character_id FROM game_session_tickets WHERE account_id = $1",
            )
            .bind(account_id as i64)
            .fetch_optional(&self.pool)
            .await?;

            if let Some((token, character_id)) = row {
                Ok(Some(GameSessionTicket {
                    token,
                    account_id,
                    character_id: character_id as u64,
                }))
            } else {
                Ok(None)
            }
        })
    }

    // ----- Social notifications -----

    fn enqueue_social_notification(
        &self,
        character_id: CharacterId,
        notification: SocialNotification,
    ) -> anyhow::Result<()> {
        self.block_on(async {
            let payload = serde_json::to_value(&notification)?;
            sqlx::query(
                "INSERT INTO social_notifications (character_id, kind, payload) VALUES ($1, $2, $3)",
            )
            .bind(character_id as i64)
            .bind(format!("{:?}", notification.kind))
            .bind(payload)
            .execute(&self.pool)
            .await?;
            Ok(())
        })
    }

    fn consume_social_notifications(
        &self,
        character_id: CharacterId,
    ) -> anyhow::Result<Vec<SocialNotification>> {
        self.block_on(async {
            let rows: Vec<(String, serde_json::Value)> = sqlx::query_as(
                "SELECT kind, payload FROM social_notifications WHERE character_id = $1 ORDER BY created_at",
            )
            .bind(character_id as i64)
            .fetch_all(&self.pool)
            .await?;

            // Delete consumed notifications
            sqlx::query("DELETE FROM social_notifications WHERE character_id = $1")
                .bind(character_id as i64)
                .execute(&self.pool)
                .await?;

            let mut notifications = Vec::new();
            for (_kind, payload) in rows {
                if let Ok(notification) = serde_json::from_value::<SocialNotification>(payload) {
                    notifications.push(notification);
                }
            }
            Ok(notifications)
        })
    }

    // ----- Map presence -----

    fn load_map_presence(&self, map_id: i16, channel: u8) -> anyhow::Result<Vec<CharacterSummary>> {
        self.block_on(async {
            let rows: Vec<(i64, String, i32, i32, i32, i32)> = sqlx::query_as(
                "SELECT character_id, name, model, partner_model, x, y FROM map_presence WHERE map_id = $1 AND channel = $2",
            )
            .bind(map_id)
            .bind(channel as i16)
            .fetch_all(&self.pool)
            .await?;

            Ok(rows
                .into_iter()
                .map(|(cid, name, model, partner_model, x, y)| CharacterSummary {
                    id: cid as u64,
                    name,
                    model,
                    partner_model,
                    x,
                    y,
                    ..CharacterSummary::default()
                })
                .collect())
        })
    }

    fn upsert_map_presence(&self, character: &CharacterSummary) -> anyhow::Result<()> {
        self.block_on(async {
            sqlx::query(
                "INSERT INTO map_presence (character_id, map_id, channel, name, model, partner_model, x, y, updated_at)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW())
                 ON CONFLICT (character_id) DO UPDATE SET
                   map_id = $2, channel = $3, name = $4, model = $5,
                   partner_model = $6, x = $7, y = $8, updated_at = NOW()",
            )
            .bind(character.id as i64)
            .bind(character.map_id)
            .bind(character.channel as i16)
            .bind(&character.name)
            .bind(character.model)
            .bind(character.partner_model)
            .bind(character.x)
            .bind(character.y)
            .execute(&self.pool)
            .await?;
            Ok(())
        })
    }

    fn remove_map_presence(
        &self,
        map_id: i16,
        channel: u8,
        character_id: CharacterId,
    ) -> anyhow::Result<Vec<CharacterSummary>> {
        self.block_on(async {
            sqlx::query("DELETE FROM map_presence WHERE character_id = $1")
                .bind(character_id as i64)
                .execute(&self.pool)
                .await?;

            // Return remaining entries for this map/channel
            self.load_map_presence(map_id, channel)
        })
    }
}
