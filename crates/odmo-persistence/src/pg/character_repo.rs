use odmo_application::account::AccountRepository;
use odmo_application::character::{CharacterAccountRepository, CharacterRepository};
use odmo_types::{
    Account, AccountId, ActiveBuffSnapshot, AttendanceStatus, ChannelAvailability,
    CharacterSummary, DEFAULT_START_MAP_ID, DEFAULT_START_X, DEFAULT_START_Y, DailyRewardStatus,
    GuildSnapshot, InventorySnapshot, RelationEntry, SealListSnapshot, XaiSnapshot,
};

use super::PgRepository;

const LEGACY_START_X: i32 = 14_834;
const LEGACY_START_Y: i32 = 9_895;

/// Database row struct mapping all columns from the characters table.
#[derive(Debug, sqlx::FromRow)]
pub(crate) struct CharacterDb {
    pub id: i64,
    pub account_id: i64,
    pub slot: i16,
    pub name: String,
    pub model: i32,
    pub level: i16,
    pub current_x: i32,
    pub current_y: i32,
    pub current_map_id: i16,
    pub partner_current_x: i32,
    pub partner_current_y: i32,
    pub channel: i16,
    pub current_condition: i16,
    pub general_handler: i32,
    pub partner_handler: i32,
    pub partner_name: String,
    pub partner_model: i32,
    pub bits: i64,
    pub xgauge: i16,
    pub xcrystals: i16,
    pub inventory: serde_json::Value,
    pub warehouse: serde_json::Value,
    pub extra_inventory: serde_json::Value,
    pub account_warehouse: Option<serde_json::Value>,
    pub seal_list: serde_json::Value,
    pub guild_snapshot: Option<serde_json::Value>,
    pub xai_snapshot: Option<serde_json::Value>,
    pub active_buffs: serde_json::Value,
    pub friends: serde_json::Value,
    pub foes: serde_json::Value,
    pub friended_character_ids: serde_json::Value,
    pub map_regions: serde_json::Value,
    pub equipment: serde_json::Value,
    pub digivice: serde_json::Value,
    pub daily_reward: serde_json::Value,
    pub attendance: serde_json::Value,
    pub available_channels: serde_json::Value,
    pub server_experience: i32,
    pub premium: i32,
    pub silk: i32,
    pub membership_seconds: i32,
}

const SELECT_COLS: &str = "\
    id, account_id, slot, name, model, level, \
    current_x, current_y, current_map_id, \
    partner_current_x, partner_current_y, \
    channel, current_condition, general_handler, partner_handler, \
    partner_name, partner_model, bits, xgauge, xcrystals, \
    inventory, warehouse, extra_inventory, account_warehouse, \
    seal_list, guild_snapshot, xai_snapshot, active_buffs, \
    friends, foes, friended_character_ids, map_regions, \
    equipment, digivice, daily_reward, attendance, \
    available_channels, server_experience, premium, silk, membership_seconds";

pub(crate) fn row_to_character(row: CharacterDb) -> CharacterSummary {
    let inventory: InventorySnapshot = serde_json::from_value(row.inventory).unwrap_or_default();
    let warehouse: InventorySnapshot = serde_json::from_value(row.warehouse).unwrap_or_default();
    let extra_inventory: InventorySnapshot =
        serde_json::from_value(row.extra_inventory).unwrap_or_default();
    let account_warehouse: Option<InventorySnapshot> = row
        .account_warehouse
        .and_then(|v| serde_json::from_value(v).ok());
    let seal_list: SealListSnapshot = serde_json::from_value(row.seal_list).unwrap_or_default();
    let guild: Option<GuildSnapshot> = row
        .guild_snapshot
        .and_then(|v| serde_json::from_value(v).ok());
    let xai: Option<XaiSnapshot> = row
        .xai_snapshot
        .and_then(|v| serde_json::from_value(v).ok());
    let active_buffs: Vec<ActiveBuffSnapshot> =
        serde_json::from_value(row.active_buffs).unwrap_or_default();
    let friends: Vec<RelationEntry> = serde_json::from_value(row.friends).unwrap_or_default();
    let foes: Vec<RelationEntry> = serde_json::from_value(row.foes).unwrap_or_default();
    let friended_character_ids: Vec<u64> =
        serde_json::from_value(row.friended_character_ids).unwrap_or_default();
    let map_region: Vec<u8> =
        serde_json::from_value(row.map_regions).unwrap_or_else(|_| vec![0u8; 255]);
    let equipment: Vec<u8> =
        serde_json::from_value(row.equipment).unwrap_or_else(|_| vec![0u8; 16 * 60]);
    let digivice: Vec<u8> = serde_json::from_value(row.digivice).unwrap_or_else(|_| vec![0u8; 60]);
    let daily_reward: DailyRewardStatus =
        serde_json::from_value(row.daily_reward).unwrap_or_default();
    let attendance: AttendanceStatus = serde_json::from_value(row.attendance).unwrap_or_default();
    let available_channels: Vec<ChannelAvailability> =
        serde_json::from_value(row.available_channels).unwrap_or_else(|_| {
            vec![ChannelAvailability {
                channel: 0,
                load: 1,
            }]
        });

    let mut summary = CharacterSummary {
        id: row.id as u64,
        account_id: row.account_id as u64,
        slot: row.slot as u8,
        map_id: row.current_map_id,
        x: row.current_x,
        y: row.current_y,
        z: 0.0,
        channel: row.channel as u8,
        general_handler: row.general_handler as u32,
        model: row.model,
        level: row.level as u8,
        name: row.name,
        current_experience: 0,
        hp: 1000,
        ds: 1000,
        current_hp: 1000,
        current_ds: 1000,
        fatigue: 0,
        at: 100,
        de: 100,
        ms: 300,
        proper_ms: 300,
        current_condition: row.current_condition as i32,
        partner_condition: 0,
        inventory_bits: row.bits,
        inventory_size: inventory.size,
        warehouse_size: warehouse.size,
        account_warehouse_size: account_warehouse.as_ref().map_or(14, |w| w.size),
        extra_inventory_size: extra_inventory.size,
        inventory,
        warehouse,
        account_warehouse,
        extra_inventory,
        digimon_slots: 7,
        current_title: 0,
        map_region,
        archive_slots: 7,
        deck_buff_id: 0,
        equipment,
        digivice,
        shop_name: String::new(),
        size: 12_000,
        active_buffs,
        seal_list,
        daily_reward,
        attendance,
        server_experience: row.server_experience,
        premium: row.premium,
        silk: row.silk,
        membership_seconds: row.membership_seconds.max(0) as u32,
        available_channels,
        friends,
        foes,
        friended_character_ids,
        guild,
        xai,
        current_xgauge: row.xgauge as i32,
        current_xcrystals: row.xcrystals,
        partner_x: row.partner_current_x,
        partner_y: row.partner_current_y,
        partner_z: 0.0,
        partner_current_type: row.partner_model,
        partner_active_buffs: Vec::new(),
        partner_active_debuffs: Vec::new(),
        partner_model: row.partner_model,
        partner_level: 1,
        partner_name: row.partner_name,
        partner_handler: row.partner_handler as u32,
        partner_size: 12_000,
        partner_hatch_grade: 3,
        partner_current_experience: 0,
        partner_transcendence_experience: 0,
        partner_hp: 1000,
        partner_ds: 1000,
        partner_de: 100,
        partner_at: 100,
        partner_current_hp: 1000,
        partner_current_ds: 1000,
        partner_fs: 100,
        partner_ev: 0,
        partner_cc: 0,
        partner_ms: 250,
        partner_as: 1000,
        partner_ht: 0,
        partner_ar: 0,
        partner_bl: 0,
        partner_clone_level: 0,
        partner_clone_at_value: 0,
        partner_clone_bl_value: 0,
        partner_clone_ct_value: 0,
        partner_clone_ev_value: 0,
        partner_clone_hp_value: 0,
        partner_clone_at_level: 0,
        partner_clone_bl_level: 0,
        partner_clone_ct_level: 0,
        partner_clone_ev_level: 0,
        partner_clone_hp_level: 0,
    };

    if summary.map_id == DEFAULT_START_MAP_ID
        && summary.x == LEGACY_START_X
        && summary.y == LEGACY_START_Y
    {
        summary.x = DEFAULT_START_X;
        summary.y = DEFAULT_START_Y;
        summary.z = 0.0;
        summary.partner_x = DEFAULT_START_X;
        summary.partner_y = DEFAULT_START_Y;
        summary.partner_z = 0.0;
    }

    summary
}

impl CharacterRepository for PgRepository {
    fn list_characters_by_account(
        &self,
        account_id: AccountId,
    ) -> anyhow::Result<Vec<CharacterSummary>> {
        let pool = self.pool().clone();
        let query =
            format!("SELECT {SELECT_COLS} FROM characters WHERE account_id = $1 ORDER BY slot");
        self.block_on(async move {
            let rows: Vec<CharacterDb> = sqlx::query_as(&query)
                .bind(account_id as i64)
                .fetch_all(&pool)
                .await
                .map_err(|e| anyhow::anyhow!("list_characters_by_account: {e}"))?;

            Ok(rows.into_iter().map(row_to_character).collect())
        })
    }

    fn character_by_slot(
        &self,
        account_id: AccountId,
        slot: u8,
    ) -> anyhow::Result<Option<CharacterSummary>> {
        let pool = self.pool().clone();
        let query =
            format!("SELECT {SELECT_COLS} FROM characters WHERE account_id = $1 AND slot = $2");
        self.block_on(async move {
            let row: Option<CharacterDb> = sqlx::query_as(&query)
                .bind(account_id as i64)
                .bind(slot as i16)
                .fetch_optional(&pool)
                .await
                .map_err(|e| anyhow::anyhow!("character_by_slot: {e}"))?;

            Ok(row.map(row_to_character))
        })
    }

    fn character_by_id(&self, character_id: u64) -> anyhow::Result<Option<CharacterSummary>> {
        let pool = self.pool().clone();
        let query = format!("SELECT {SELECT_COLS} FROM characters WHERE id = $1");
        self.block_on(async move {
            let row: Option<CharacterDb> = sqlx::query_as(&query)
                .bind(character_id as i64)
                .fetch_optional(&pool)
                .await
                .map_err(|e| anyhow::anyhow!("character_by_id: {e}"))?;

            Ok(row.map(row_to_character))
        })
    }

    fn is_name_available(&self, name: &str) -> anyhow::Result<bool> {
        let pool = self.pool().clone();
        let name = name.to_string();
        self.block_on(async move {
            let count: (i64,) =
                sqlx::query_as("SELECT COUNT(*) FROM characters WHERE LOWER(name) = LOWER($1)")
                    .bind(&name)
                    .fetch_one(&pool)
                    .await?;

            Ok(count.0 == 0)
        })
    }

    fn create_character(
        &self,
        account_id: AccountId,
        slot: u8,
        tamer_name: String,
        tamer_model: i32,
        partner_name: String,
        partner_model: i32,
    ) -> anyhow::Result<CharacterSummary> {
        let pool = self.pool().clone();
        self.block_on(async move {
            let next_handler: (i32,) = sqlx::query_as(
                "SELECT COALESCE(MAX(general_handler), 10000) + 1 FROM characters",
            )
            .fetch_one(&pool)
            .await?;
            let general_handler = next_handler.0;
            let partner_handler = general_handler + 10_000;

            let default_inv = serde_json::json!({"bits": 0, "size": 30, "items": []});
            let default_seals = serde_json::json!({"seal_leader_id": 0, "seals": []});
            let default_channels = serde_json::json!([{"channel": 0, "load": 1}]);

            sqlx::query(
                "INSERT INTO characters \
                (account_id, slot, name, model, level, \
                 current_x, current_y, current_map_id, \
                 partner_current_x, partner_current_y, \
                 channel, current_condition, \
                 general_handler, partner_handler, \
                 partner_name, partner_model, bits, xgauge, xcrystals, \
                 inventory, warehouse, extra_inventory, \
                 seal_list, available_channels) \
                VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,$22,$23,$24)",
            )
            .bind(account_id as i64)
            .bind(slot as i16)
            .bind(&tamer_name)
            .bind(tamer_model)
            .bind(1i16)
            .bind(DEFAULT_START_X)
            .bind(DEFAULT_START_Y)
            .bind(DEFAULT_START_MAP_ID)
            .bind(DEFAULT_START_X)
            .bind(DEFAULT_START_Y)
            .bind(0i16)
            .bind(0i16)
            .bind(general_handler)
            .bind(partner_handler)
            .bind(&partner_name)
            .bind(partner_model)
            .bind(0i64)
            .bind(0i16)
            .bind(0i16)
            .bind(&default_inv)
            .bind(&default_inv)
            .bind(&default_inv)
            .bind(&default_seals)
            .bind(&default_channels)
            .execute(&pool)
            .await
            .map_err(|e| anyhow::anyhow!("create_character: {e}"))?;

            // Fetch back the created row
            let query = format!("SELECT {SELECT_COLS} FROM characters WHERE account_id = $1 AND slot = $2");
            let row: CharacterDb = sqlx::query_as(&query)
                .bind(account_id as i64)
                .bind(slot as i16)
                .fetch_one(&pool)
                .await
                .map_err(|e| anyhow::anyhow!("create_character fetch: {e}"))?;

            Ok(row_to_character(row))
        })
    }

    fn delete_character(&self, account_id: AccountId, slot: u8) -> anyhow::Result<bool> {
        let pool = self.pool().clone();
        self.block_on(async move {
            let result = sqlx::query("DELETE FROM characters WHERE account_id = $1 AND slot = $2")
                .bind(account_id as i64)
                .bind(slot as i16)
                .execute(&pool)
                .await?;

            Ok(result.rows_affected() > 0)
        })
    }

    fn update_character_position(
        &self,
        character_id: u64,
        x: i32,
        y: i32,
        z: f32,
    ) -> anyhow::Result<()> {
        let pool = self.pool().clone();
        self.block_on(async move {
            sqlx::query(
                "UPDATE characters SET current_x = $1, current_y = $2, current_z = $3 WHERE id = $4",
            )
            .bind(x)
            .bind(y)
            .bind(z)
            .bind(character_id as i64)
            .execute(&pool)
            .await?;
            Ok(())
        })
    }

    fn update_partner_position(
        &self,
        character_id: u64,
        x: i32,
        y: i32,
        z: f32,
    ) -> anyhow::Result<()> {
        let pool = self.pool().clone();
        self.block_on(async move {
            sqlx::query(
                "UPDATE characters SET partner_current_x = $1, partner_current_y = $2, partner_current_z = $3 WHERE id = $4",
            )
            .bind(x)
            .bind(y)
            .bind(z)
            .bind(character_id as i64)
            .execute(&pool)
            .await?;
            Ok(())
        })
    }

    fn update_character_map(
        &self,
        character_id: u64,
        map_id: i16,
        x: i32,
        y: i32,
    ) -> anyhow::Result<()> {
        let pool = self.pool().clone();
        self.block_on(async move {
            sqlx::query(
                "UPDATE characters SET current_map_id = $1, current_x = $2, current_y = $3, \
                 partner_current_x = $2, partner_current_y = $3 WHERE id = $4",
            )
            .bind(map_id)
            .bind(x)
            .bind(y)
            .bind(character_id as i64)
            .execute(&pool)
            .await?;
            Ok(())
        })
    }

    fn update_inventory(
        &self,
        character_id: u64,
        inventory: odmo_types::InventorySnapshot,
    ) -> anyhow::Result<()> {
        let inventory_json = serde_json::to_value(&inventory)?;
        let pool = self.pool().clone();
        self.block_on(async move {
            sqlx::query("UPDATE characters SET inventory = $1 WHERE id = $2")
                .bind(&inventory_json)
                .bind(character_id as i64)
                .execute(&pool)
                .await?;
            Ok(())
        })
    }
}

impl CharacterAccountRepository for PgRepository {
    fn account_by_id(&self, account_id: AccountId) -> anyhow::Result<Option<Account>> {
        <Self as AccountRepository>::account_by_id(self, account_id)
    }
}

/// Returns the query to fetch a character by ID, for use by drop_repo.
pub(crate) fn character_by_id_query() -> String {
    format!("SELECT {SELECT_COLS} FROM characters WHERE id = $1")
}
