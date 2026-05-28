mod account_repo;
mod character_repo;
mod drop_repo;
mod mob_repo;
mod npc_shop_repo;
mod portal_bridge;
mod portal_repo;

use std::future::Future;

use odmo_types::{
    DEFAULT_GM_PARTNER_MODEL_ID, DEFAULT_GM_TAMER_MODEL_ID, DEFAULT_PARTNER_MODEL_ID,
    DEFAULT_START_MAP_ID, DEFAULT_START_X, DEFAULT_START_Y, DEFAULT_TAMER_MODEL_ID,
};
use sqlx::PgPool;

#[derive(Debug, Clone)]
pub struct PgRepository {
    pool: PgPool,
}

impl PgRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn open(database_url: &str) -> anyhow::Result<Self> {
        let pool = PgPool::connect(database_url).await?;
        Ok(Self { pool })
    }

    pub async fn migrate(&self) -> anyhow::Result<()> {
        sqlx::migrate!("./migrations").run(&self.pool).await?;
        Ok(())
    }

    pub async fn seed_demo(&self) -> anyhow::Result<()> {
        let existing: Option<(i64,)> =
            sqlx::query_as("SELECT id FROM accounts WHERE username = 'admin'")
                .fetch_optional(&self.pool)
                .await?;
        if existing.is_some() {
            return Ok(());
        }

        // Accounts
        sqlx::query(
            "INSERT INTO accounts (id, username, password_hash, email, access_level) VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(1i64)
        .bind("admin")
        .bind("admin")
        .bind("admin@odmo.local")
        .bind(2i16) // Administrator
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "INSERT INTO accounts (id, username, password_hash, email, access_level, secondary_password) VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(2i64)
        .bind("gm")
        .bind("gm")
        .bind("gm@odmo.local")
        .bind(1i16) // GameMaster
        .bind("4321")
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "INSERT INTO accounts (id, username, password_hash, email, access_level, suspension_remaining_seconds, suspension_reason) VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(3i64)
        .bind("banned")
        .bind("banned")
        .bind("banned@odmo.local")
        .bind(0i16) // Player
        .bind(3600i32)
        .bind("Policy violation")
        .execute(&self.pool)
        .await?;

        // Servers
        sqlx::query(
            "INSERT INTO servers (id, name, maintenance, overloaded, is_new, character_count) VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(1i32)
        .bind("ODMO Alpha")
        .bind(false)
        .bind(false)
        .bind(true)
        .bind(0i16)
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "INSERT INTO servers (id, name, maintenance, overloaded, is_new, character_count) VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(2i32)
        .bind("ODMO Beta")
        .bind(false)
        .bind(false)
        .bind(false)
        .bind(0i16)
        .execute(&self.pool)
        .await?;

        // Characters
        let default_inv = serde_json::json!({"bits": 0, "size": 30, "items": []});
        let default_warehouse = serde_json::json!({"bits": 0, "size": 21, "items": []});
        let default_account_warehouse = serde_json::json!({"bits": 0, "size": 14, "items": []});
        let default_seals = serde_json::json!({"seal_leader_id": 0, "seals": []});
        let default_channels = serde_json::json!([{"channel": 0, "load": 1}]);
        let admin_partner_slots = serde_json::json!([
            {
                "slot": 1, "digimon_type": DEFAULT_PARTNER_MODEL_ID, "model": DEFAULT_PARTNER_MODEL_ID,
                "level": 1, "name": "Agumon", "size": 12000, "hatch_grade": 3,
                "hp": 1000, "ds": 1000, "current_hp": 1000, "current_ds": 1000,
                "de": 100, "at": 100, "fs": 100, "ev": 0, "cc": 0, "ms": 250, "as_value": 1000,
                "ht": 0, "ar": 0, "bl": 0, "clone_level": 0,
                "clone_at_value": 0, "clone_bl_value": 0, "clone_ct_value": 0, "clone_ev_value": 0, "clone_hp_value": 0,
                "clone_at_level": 0, "clone_bl_level": 0, "clone_ct_level": 0, "clone_ev_level": 0, "clone_hp_level": 0,
                "active_buffs": []
            },
            {
                "slot": 2, "digimon_type": 31002, "model": 31002,
                "level": 11, "name": "Greymon", "size": 13000, "hatch_grade": 4,
                "hp": 1400, "ds": 1200, "current_hp": 1400, "current_ds": 1200,
                "de": 120, "at": 150, "fs": 120, "ev": 8, "cc": 5, "ms": 260, "as_value": 950,
                "ht": 3, "ar": 1, "bl": 2, "clone_level": 3,
                "clone_at_value": 1, "clone_bl_value": 1, "clone_ct_value": 0, "clone_ev_value": 0, "clone_hp_value": 1,
                "clone_at_level": 1, "clone_bl_level": 1, "clone_ct_level": 0, "clone_ev_level": 0, "clone_hp_level": 1,
                "active_buffs": []
            }
        ]);
        let gm_partner_slots = serde_json::json!([
            {
                "slot": 1, "digimon_type": DEFAULT_GM_PARTNER_MODEL_ID, "model": DEFAULT_GM_PARTNER_MODEL_ID,
                "level": 1, "name": "Gabumon", "size": 12000, "hatch_grade": 3,
                "hp": 1000, "ds": 1000, "current_hp": 1000, "current_ds": 1000,
                "de": 100, "at": 100, "fs": 100, "ev": 0, "cc": 0, "ms": 250, "as_value": 1000,
                "ht": 0, "ar": 0, "bl": 0, "clone_level": 0,
                "clone_at_value": 0, "clone_bl_value": 0, "clone_ct_value": 0, "clone_ev_value": 0, "clone_hp_value": 0,
                "clone_at_level": 0, "clone_bl_level": 0, "clone_ct_level": 0, "clone_ev_level": 0, "clone_hp_level": 0,
                "active_buffs": []
            }
        ]);

        sqlx::query(
            "INSERT INTO characters (id, account_id, slot, name, model, level, current_x, current_y, current_map_id, partner_current_x, partner_current_y, partner_current_slot, general_handler, partner_handler, partner_name, partner_model, bits, inventory, warehouse, extra_inventory, account_warehouse, seal_list, available_channels, partner_slots) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,$22,$23,$24)",
        )
        .bind(100i64)
        .bind(1i64)
        .bind(0i16)
        .bind("AdminTamer")
        .bind(DEFAULT_TAMER_MODEL_ID)
        .bind(1i16)
        .bind(DEFAULT_START_X)
        .bind(DEFAULT_START_Y)
        .bind(DEFAULT_START_MAP_ID)
        .bind(DEFAULT_START_X)
        .bind(DEFAULT_START_Y)
        .bind(1i16)
        .bind(11000i32)
        .bind(21000i32)
        .bind("Agumon")
        .bind(DEFAULT_PARTNER_MODEL_ID)
        .bind(0i64)
        .bind(&default_inv)
        .bind(&default_warehouse)
        .bind(&default_inv)
        .bind(&default_account_warehouse)
        .bind(&default_seals)
        .bind(&default_channels)
        .bind(&admin_partner_slots)
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "INSERT INTO characters (id, account_id, slot, name, model, level, current_x, current_y, current_map_id, partner_current_x, partner_current_y, partner_current_slot, general_handler, partner_handler, partner_name, partner_model, bits, inventory, warehouse, extra_inventory, account_warehouse, seal_list, available_channels, partner_slots) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,$22,$23,$24)",
        )
        .bind(200i64)
        .bind(2i64)
        .bind(0i16)
        .bind("GmTamer")
        .bind(DEFAULT_GM_TAMER_MODEL_ID)
        .bind(1i16)
        .bind(DEFAULT_START_X)
        .bind(DEFAULT_START_Y)
        .bind(DEFAULT_START_MAP_ID)
        .bind(DEFAULT_START_X)
        .bind(DEFAULT_START_Y)
        .bind(1i16)
        .bind(12000i32)
        .bind(22000i32)
        .bind("Gabumon")
        .bind(DEFAULT_GM_PARTNER_MODEL_ID)
        .bind(0i64)
        .bind(&default_inv)
        .bind(&default_warehouse)
        .bind(&default_inv)
        .bind(&default_account_warehouse)
        .bind(&default_seals)
        .bind(&default_channels)
        .bind(&gm_partner_slots)
        .execute(&self.pool)
        .await?;

        // Mobs
        sqlx::query(
            "INSERT INTO map_mobs (map_id, channel, handler, type_id, model, name, level, x, y, previous_x, previous_y, current_hp, max_hp) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)",
        )
        .bind(DEFAULT_START_MAP_ID)
        .bind(0i16)
        .bind(34000i32)
        .bind(51001i32)
        .bind(51001i32)
        .bind("Goblimon")
        .bind(12i16)
        .bind(DEFAULT_START_X + 66)
        .bind(DEFAULT_START_Y + 45)
        .bind(DEFAULT_START_X + 46)
        .bind(DEFAULT_START_Y + 25)
        .bind(1000i32)
        .bind(1000i32)
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "INSERT INTO map_mobs (map_id, channel, handler, type_id, model, name, level, x, y, previous_x, previous_y, current_hp, max_hp) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)",
        )
        .bind(DEFAULT_START_MAP_ID)
        .bind(0i16)
        .bind(34001i32)
        .bind(51002i32)
        .bind(51002i32)
        .bind("Woodmon")
        .bind(18i16)
        .bind(DEFAULT_START_X + 416)
        .bind(DEFAULT_START_Y + 225)
        .bind(DEFAULT_START_X + 386)
        .bind(DEFAULT_START_Y + 195)
        .bind(1000i32)
        .bind(1000i32)
        .execute(&self.pool)
        .await?;

        // Drops
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        sqlx::query(
            "INSERT INTO map_drops (map_id, channel, handler, owner_id, owner_handler, item_id, amount, x, y, owner_expires_at_unix, expires_at_unix, bits_drop) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)",
        )
        .bind(DEFAULT_START_MAP_ID)
        .bind(0i16)
        .bind(49200i32)
        .bind(100i64)
        .bind(11000i32)
        .bind(90600i32)
        .bind(123i32)
        .bind(DEFAULT_START_X + 76)
        .bind(DEFAULT_START_Y + 55)
        .bind(now + 60)
        .bind(now + 90)
        .bind(true)
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "INSERT INTO map_drops (map_id, channel, handler, item_id, amount, x, y, owner_expires_at_unix, expires_at_unix, no_owner) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)",
        )
        .bind(DEFAULT_START_MAP_ID)
        .bind(0i16)
        .bind(49201i32)
        .bind(5101i32)
        .bind(1i32)
        .bind(DEFAULT_START_X + 286)
        .bind(DEFAULT_START_Y + 145)
        .bind(now.saturating_sub(5))
        .bind(now + 30)
        .bind(true)
        .execute(&self.pool)
        .await?;

        // Resource hash
        sqlx::query(
            "INSERT INTO server_config (key, value) VALUES ($1, $2) ON CONFLICT (key) DO NOTHING",
        )
        .bind("resource_hash_hex")
        .bind("0123456789ABCDEF")
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Bridge sync trait methods to async sqlx calls.
    /// Uses `tokio::task::block_in_place` to safely block inside a tokio runtime.
    pub(crate) fn block_on<F: Future>(&self, f: F) -> F::Output {
        tokio::task::block_in_place(move || {
            let handle = tokio::runtime::Handle::current();
            handle.block_on(f)
        })
    }
}
