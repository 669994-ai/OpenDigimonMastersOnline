pub mod pg;

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::Context;
use odmo_application::{
    account::AccountRepository,
    character::{CharacterAccountRepository, CharacterRepository},
    game::{
        DropCollectionResult, GameRepository, MapDropRepository, MapMobRepository,
        NpcShopDefinition, NpcShopItem, NpcShopRepository, PortalDefinition, PortalRepository,
    },
};
use odmo_types::{
    AccessLevel, Account, AccountId, AccountSuspension, CharacterSummary,
    DEFAULT_GM_PARTNER_MODEL_ID, DEFAULT_GM_TAMER_MODEL_ID, DEFAULT_PARTNER_MODEL_ID,
    DEFAULT_START_MAP_ID, DEFAULT_START_X, DEFAULT_START_Y, DEFAULT_TAMER_MODEL_ID, DropSummary,
    ItemRecord, MobSummary, PartnerSlotSnapshot, ServerDescriptor,
};
use serde::{Deserialize, Serialize};

fn map_key(map_id: i16, channel: u8) -> String {
    format!("{map_id}:{channel}")
}

fn active_partner_snapshot(character: &CharacterSummary) -> PartnerSlotSnapshot {
    PartnerSlotSnapshot {
        slot: character.partner_current_slot,
        digimon_type: character.partner_current_type,
        model: character.partner_model,
        level: character.partner_level,
        name: character.partner_name.clone(),
        size: character.partner_size,
        hatch_grade: character.partner_hatch_grade,
        hp: character.partner_hp,
        ds: character.partner_ds,
        current_hp: character.partner_current_hp,
        current_ds: character.partner_current_ds,
        de: character.partner_de,
        at: character.partner_at,
        fs: character.partner_fs,
        ev: character.partner_ev,
        cc: character.partner_cc,
        ms: character.partner_ms,
        as_value: character.partner_as,
        ht: character.partner_ht,
        ar: character.partner_ar,
        bl: character.partner_bl,
        clone_level: character.partner_clone_level,
        clone_at_value: character.partner_clone_at_value,
        clone_bl_value: character.partner_clone_bl_value,
        clone_ct_value: character.partner_clone_ct_value,
        clone_ev_value: character.partner_clone_ev_value,
        clone_hp_value: character.partner_clone_hp_value,
        clone_at_level: character.partner_clone_at_level,
        clone_bl_level: character.partner_clone_bl_level,
        clone_ct_level: character.partner_clone_ct_level,
        clone_ev_level: character.partner_clone_ev_level,
        clone_hp_level: character.partner_clone_hp_level,
        active_buffs: character.partner_active_buffs.clone(),
    }
}

fn apply_partner_snapshot(character: &mut CharacterSummary, partner: &PartnerSlotSnapshot) {
    character.partner_current_type = partner.digimon_type;
    character.partner_model = partner.model;
    character.partner_level = partner.level;
    character.partner_name = partner.name.clone();
    character.partner_size = partner.size;
    character.partner_hatch_grade = partner.hatch_grade;
    character.partner_hp = partner.hp;
    character.partner_ds = partner.ds;
    character.partner_current_hp = partner.current_hp;
    character.partner_current_ds = partner.current_ds;
    character.partner_de = partner.de;
    character.partner_at = partner.at;
    character.partner_fs = partner.fs;
    character.partner_ev = partner.ev;
    character.partner_cc = partner.cc;
    character.partner_ms = partner.ms;
    character.partner_as = partner.as_value;
    character.partner_ht = partner.ht;
    character.partner_ar = partner.ar;
    character.partner_bl = partner.bl;
    character.partner_clone_level = partner.clone_level;
    character.partner_clone_at_value = partner.clone_at_value;
    character.partner_clone_bl_value = partner.clone_bl_value;
    character.partner_clone_ct_value = partner.clone_ct_value;
    character.partner_clone_ev_value = partner.clone_ev_value;
    character.partner_clone_hp_value = partner.clone_hp_value;
    character.partner_clone_at_level = partner.clone_at_level;
    character.partner_clone_bl_level = partner.clone_bl_level;
    character.partner_clone_ct_level = partner.clone_ct_level;
    character.partner_clone_ev_level = partner.clone_ev_level;
    character.partner_clone_hp_level = partner.clone_hp_level;
    character.partner_active_buffs = partner.active_buffs.clone();
}

/// Persistence backend selection.
pub enum PersistenceBackend {
    /// PostgreSQL — canonical production backend.
    Pg(Arc<pg::PgRepository>),
    /// JSON file — development/testing only. Requires ODMO_DEV_MODE=1.
    Json(Arc<JsonRepository>),
}

/// Initialize the persistence backend.
///
/// Priority:
/// 1. `ODMO_DATABASE_URL` → PostgreSQL (production)
/// 2. `ODMO_DEV_MODE=1` → JSON file (development)
/// 3. Neither set → error
///
/// For PostgreSQL, runs migrations and seeds demo data automatically.
pub async fn initialize_backend() -> anyhow::Result<PersistenceBackend> {
    if let Ok(database_url) = std::env::var("ODMO_DATABASE_URL") {
        tracing::info!("using PostgreSQL persistence");
        let pg = pg::PgRepository::open(&database_url)
            .await
            .context("failed to connect to PostgreSQL")?;
        pg.migrate().await.context("failed to run migrations")?;
        pg.seed_demo().await.context("failed to seed demo data")?;
        Ok(PersistenceBackend::Pg(Arc::new(pg)))
    } else if std::env::var("ODMO_DEV_MODE").is_ok() {
        tracing::warn!("using JSON file persistence (dev mode — not for production)");
        let repository_path = std::env::var("ODMO_REPOSITORY_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| std::env::temp_dir().join("odmo-data").join("world.json"));
        let repo = JsonRepository::open_or_create(repository_path)
            .context("failed to initialize JSON repository")?;
        Ok(PersistenceBackend::Json(Arc::new(repo)))
    } else {
        anyhow::bail!(
            "no persistence backend configured. \
             Set ODMO_DATABASE_URL for PostgreSQL (production) \
             or ODMO_DEV_MODE=1 for JSON file (development)."
        )
    }
}

impl PersistenceBackend {
    /// Extract an `Arc<dyn AccountRepository>` from the backend.
    pub fn account_repository(self: &Arc<Self>) -> Arc<dyn AccountRepository> {
        match self.as_ref() {
            PersistenceBackend::Pg(pg) => pg.clone() as Arc<dyn AccountRepository>,
            PersistenceBackend::Json(json) => json.clone() as Arc<dyn AccountRepository>,
        }
    }

    /// Extract an `Arc<dyn CharacterRepository>` from the backend.
    pub fn character_repository(self: &Arc<Self>) -> Arc<dyn CharacterRepository> {
        match self.as_ref() {
            PersistenceBackend::Pg(pg) => pg.clone() as Arc<dyn CharacterRepository>,
            PersistenceBackend::Json(json) => json.clone() as Arc<dyn CharacterRepository>,
        }
    }

    /// Extract an `Arc<dyn CharacterAccountRepository>` from the backend.
    pub fn character_account_repository(self: &Arc<Self>) -> Arc<dyn CharacterAccountRepository> {
        match self.as_ref() {
            PersistenceBackend::Pg(pg) => pg.clone() as Arc<dyn CharacterAccountRepository>,
            PersistenceBackend::Json(json) => json.clone() as Arc<dyn CharacterAccountRepository>,
        }
    }

    /// Extract an `Arc<dyn GameRepository>` from the backend.
    pub fn game_repository(self: &Arc<Self>) -> Arc<dyn GameRepository> {
        match self.as_ref() {
            PersistenceBackend::Pg(pg) => pg.clone() as Arc<dyn GameRepository>,
            PersistenceBackend::Json(json) => json.clone() as Arc<dyn GameRepository>,
        }
    }
}

#[derive(Debug)]
pub struct JsonRepository {
    path: PathBuf,
    state: RwLock<WorldSnapshot>,
}

impl JsonRepository {
    pub fn open_or_create(path: PathBuf) -> anyhow::Result<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("failed to create repository directory {}", parent.display())
            })?;
        }

        let mut snapshot = if path.exists() {
            let payload = fs::read(&path)
                .with_context(|| format!("failed to read repository {}", path.display()))?;
            serde_json::from_slice(&payload)
                .with_context(|| format!("failed to decode repository {}", path.display()))?
        } else {
            let seed = WorldSnapshot::demo();
            fs::write(&path, serde_json::to_vec_pretty(&seed)?)
                .with_context(|| format!("failed to seed repository {}", path.display()))?;
            seed
        };

        if normalize_legacy_snapshot(&mut snapshot) {
            fs::write(&path, serde_json::to_vec_pretty(&snapshot)?).with_context(|| {
                format!("failed to persist normalized repository {}", path.display())
            })?;
        }

        Ok(Self {
            path,
            state: RwLock::new(snapshot),
        })
    }

    fn persist(&self, snapshot: &WorldSnapshot) -> anyhow::Result<()> {
        let payload = serde_json::to_vec_pretty(snapshot)?;
        fs::write(&self.path, payload)
            .with_context(|| format!("failed to persist repository {}", self.path.display()))?;
        Ok(())
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl AccountRepository for JsonRepository {
    fn account_by_username(&self, username: &str) -> anyhow::Result<Option<Account>> {
        let state = self.state.read().expect("repository poisoned");
        Ok(state.accounts.get(username).cloned())
    }

    fn account_by_id(&self, account_id: AccountId) -> anyhow::Result<Option<Account>> {
        let state = self.state.read().expect("repository poisoned");
        Ok(state
            .accounts
            .values()
            .find(|account| account.id == account_id)
            .cloned())
    }

    fn update_secondary_password(
        &self,
        account_id: AccountId,
        password: String,
    ) -> anyhow::Result<()> {
        let mut state = self.state.write().expect("repository poisoned");
        if let Some(account) = state
            .accounts
            .values_mut()
            .find(|account| account.id == account_id)
        {
            account.secondary_password = Some(password);
        }
        self.persist(&state)
    }

    fn list_servers(&self) -> anyhow::Result<Vec<ServerDescriptor>> {
        let state = self.state.read().expect("repository poisoned");
        Ok(state.servers.clone())
    }

    fn resource_hash_hex(&self) -> anyhow::Result<Option<String>> {
        let state = self.state.read().expect("repository poisoned");
        Ok(state.resource_hash_hex.clone())
    }
}

impl CharacterRepository for JsonRepository {
    fn list_characters_by_account(
        &self,
        account_id: AccountId,
    ) -> anyhow::Result<Vec<CharacterSummary>> {
        let state = self.state.read().expect("repository poisoned");
        Ok(state
            .characters_by_account
            .get(&account_id)
            .cloned()
            .unwrap_or_default())
    }

    fn character_by_slot(
        &self,
        account_id: AccountId,
        slot: u8,
    ) -> anyhow::Result<Option<CharacterSummary>> {
        let state = self.state.read().expect("repository poisoned");
        Ok(state
            .characters_by_account
            .get(&account_id)
            .and_then(|characters| characters.iter().find(|character| character.slot == slot))
            .cloned())
    }

    fn character_by_id(&self, character_id: u64) -> anyhow::Result<Option<CharacterSummary>> {
        let state = self.state.read().expect("repository poisoned");
        Ok(state
            .characters_by_account
            .values()
            .flatten()
            .find(|character| character.id == character_id)
            .cloned())
    }

    fn character_by_name(&self, name: &str) -> anyhow::Result<Option<CharacterSummary>> {
        let state = self.state.read().expect("repository poisoned");
        Ok(state
            .characters_by_account
            .values()
            .flat_map(|characters| characters.iter())
            .find(|character| character.name.eq_ignore_ascii_case(name))
            .cloned())
    }

    fn is_name_available(&self, name: &str) -> anyhow::Result<bool> {
        let state = self.state.read().expect("repository poisoned");
        Ok(!state
            .characters_by_account
            .values()
            .flatten()
            .any(|character| character.name.eq_ignore_ascii_case(name)))
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
        let mut state = self.state.write().expect("repository poisoned");
        let next_id = state
            .characters_by_account
            .values()
            .flatten()
            .map(|character| character.id)
            .max()
            .unwrap_or(0)
            + 1;

        let character = CharacterSummary {
            id: next_id,
            account_id,
            slot,
            name: tamer_name,
            partner_current_slot: 1,
            partner_current_type: partner_model,
            partner_model,
            partner_name: partner_name.clone(),
            partner_slots: vec![PartnerSlotSnapshot {
                slot: 1,
                digimon_type: partner_model,
                model: partner_model,
                name: partner_name,
                ..PartnerSlotSnapshot::default()
            }],
            general_handler: next_id as u32 + 10_000,
            partner_handler: next_id as u32 + 20_000,
            model: tamer_model,
            ..CharacterSummary::default()
        };

        state
            .characters_by_account
            .entry(account_id)
            .or_default()
            .retain(|existing| existing.slot != slot);
        state
            .characters_by_account
            .entry(account_id)
            .or_default()
            .push(character.clone());
        state
            .characters_by_account
            .get_mut(&account_id)
            .expect("character list must exist")
            .sort_by_key(|entry| entry.slot);
        self.persist(&state)?;
        Ok(character)
    }

    fn delete_character(&self, account_id: AccountId, slot: u8) -> anyhow::Result<bool> {
        let mut state = self.state.write().expect("repository poisoned");
        let Some(characters) = state.characters_by_account.get_mut(&account_id) else {
            return Ok(false);
        };

        let original_len = characters.len();
        characters.retain(|character| character.slot != slot);
        let deleted = characters.len() != original_len;
        if deleted {
            self.persist(&state)?;
        }
        Ok(deleted)
    }

    fn update_character_position(
        &self,
        character_id: u64,
        x: i32,
        y: i32,
        z: f32,
    ) -> anyhow::Result<()> {
        let mut state = self.state.write().expect("repository poisoned");
        for characters in state.characters_by_account.values_mut() {
            if let Some(ch) = characters.iter_mut().find(|c| c.id == character_id) {
                ch.x = x;
                ch.y = y;
                ch.z = z;
                self.persist(&state)?;
                return Ok(());
            }
        }
        Ok(())
    }

    fn update_partner_position(
        &self,
        character_id: u64,
        x: i32,
        y: i32,
        z: f32,
    ) -> anyhow::Result<()> {
        let mut state = self.state.write().expect("repository poisoned");
        for characters in state.characters_by_account.values_mut() {
            if let Some(ch) = characters.iter_mut().find(|c| c.id == character_id) {
                ch.partner_x = x;
                ch.partner_y = y;
                ch.partner_z = z;
                self.persist(&state)?;
                return Ok(());
            }
        }
        Ok(())
    }

    fn switch_partner(
        &self,
        character_id: u64,
        slot: u8,
    ) -> anyhow::Result<Option<CharacterSummary>> {
        let mut state = self.state.write().expect("repository poisoned");
        let mut updated_character = None;
        for characters in state.characters_by_account.values_mut() {
            if let Some(ch) = characters.iter_mut().find(|c| c.id == character_id) {
                let current_slot = ch.partner_current_slot;
                if current_slot == slot {
                    updated_character = Some(ch.clone());
                    break;
                }

                if let Some(current_index) = ch
                    .partner_slots
                    .iter()
                    .position(|partner| partner.slot == current_slot)
                {
                    ch.partner_slots[current_index] = active_partner_snapshot(ch);
                }

                let Some(target_partner) = ch
                    .partner_slots
                    .iter()
                    .find(|partner| partner.slot == slot)
                    .cloned()
                else {
                    return Ok(None);
                };

                apply_partner_snapshot(ch, &target_partner);
                ch.partner_current_slot = slot;
                updated_character = Some(ch.clone());
                break;
            }
        }
        if updated_character.is_some() {
            self.persist(&state)?;
        }
        Ok(updated_character)
    }

    fn update_character_map(
        &self,
        character_id: u64,
        map_id: i16,
        x: i32,
        y: i32,
    ) -> anyhow::Result<()> {
        let mut state = self.state.write().expect("repository poisoned");
        for characters in state.characters_by_account.values_mut() {
            if let Some(ch) = characters.iter_mut().find(|c| c.id == character_id) {
                ch.map_id = map_id;
                ch.x = x;
                ch.y = y;
                ch.partner_x = x;
                ch.partner_y = y;
                self.persist(&state)?;
                return Ok(());
            }
        }
        Ok(())
    }

    fn update_inventory(
        &self,
        character_id: u64,
        inventory: odmo_types::InventorySnapshot,
    ) -> anyhow::Result<()> {
        let mut state = self.state.write().expect("repository poisoned");
        for characters in state.characters_by_account.values_mut() {
            if let Some(ch) = characters.iter_mut().find(|c| c.id == character_id) {
                ch.inventory = inventory;
                self.persist(&state)?;
                return Ok(());
            }
        }
        Ok(())
    }

    fn update_extra_inventory(
        &self,
        character_id: u64,
        extra_inventory: odmo_types::InventorySnapshot,
    ) -> anyhow::Result<()> {
        let mut state = self.state.write().expect("repository poisoned");
        for characters in state.characters_by_account.values_mut() {
            if let Some(ch) = characters.iter_mut().find(|c| c.id == character_id) {
                ch.extra_inventory = extra_inventory;
                self.persist(&state)?;
                return Ok(());
            }
        }
        Ok(())
    }

    fn update_warehouse(
        &self,
        character_id: u64,
        warehouse: odmo_types::InventorySnapshot,
    ) -> anyhow::Result<()> {
        let mut state = self.state.write().expect("repository poisoned");
        for characters in state.characters_by_account.values_mut() {
            if let Some(ch) = characters.iter_mut().find(|c| c.id == character_id) {
                ch.warehouse = warehouse;
                self.persist(&state)?;
                return Ok(());
            }
        }
        Ok(())
    }

    fn update_account_warehouse(
        &self,
        character_id: u64,
        account_warehouse: odmo_types::InventorySnapshot,
    ) -> anyhow::Result<()> {
        let mut state = self.state.write().expect("repository poisoned");
        for characters in state.characters_by_account.values_mut() {
            if let Some(ch) = characters.iter_mut().find(|c| c.id == character_id) {
                ch.account_warehouse = Some(account_warehouse);
                self.persist(&state)?;
                return Ok(());
            }
        }
        Ok(())
    }

    fn update_character_map_region(
        &self,
        _character_id: u64,
        _map_id: i16,
        _unlocked: bool,
    ) -> anyhow::Result<()> {
        Ok(())
    }
    fn update_character_state(&self, _character_id: u64, _state: u8) -> anyhow::Result<()> {
        Ok(())
    }
    fn update_welcome_flag(&self, _account_id: AccountId, _welcome: bool) -> anyhow::Result<()> {
        Ok(())
    }
}

impl CharacterAccountRepository for JsonRepository {
    fn account_by_id(&self, account_id: AccountId) -> anyhow::Result<Option<Account>> {
        AccountRepository::account_by_id(self, account_id)
    }
}

impl MapMobRepository for JsonRepository {
    fn mobs_by_map(&self, map_id: i16, channel: u8) -> anyhow::Result<Vec<MobSummary>> {
        let state = self.state.read().expect("repository poisoned");
        Ok(state
            .mobs_by_map
            .get(&map_key(map_id, channel))
            .cloned()
            .unwrap_or_default())
    }
}

impl MapDropRepository for JsonRepository {
    fn drops_by_map(&self, map_id: i16, channel: u8) -> anyhow::Result<Vec<DropSummary>> {
        let state = self.state.read().expect("repository poisoned");
        Ok(state
            .drops_by_map
            .get(&map_key(map_id, channel))
            .cloned()
            .unwrap_or_default())
    }

    fn collect_drop(
        &self,
        character_id: u64,
        map_id: i16,
        channel: u8,
        drop_handler: u32,
    ) -> anyhow::Result<DropCollectionResult> {
        let mut state = self.state.write().expect("repository poisoned");
        let Some((account_id, character_index)) =
            state
                .characters_by_account
                .iter()
                .find_map(|(account_id, characters)| {
                    characters
                        .iter()
                        .position(|character| character.id == character_id)
                        .map(|index| (*account_id, index))
                })
        else {
            return Ok(DropCollectionResult::Missing);
        };
        let character_snapshot = state
            .characters_by_account
            .get(&account_id)
            .and_then(|characters| characters.get(character_index))
            .cloned()
            .expect("character should exist");
        let Some(drop_index) = state
            .drops_by_map
            .get(&map_key(map_id, channel))
            .and_then(|drops| drops.iter().position(|drop| drop.handler == drop_handler))
        else {
            return Ok(DropCollectionResult::Missing);
        };

        let drop = state
            .drops_by_map
            .get(&map_key(map_id, channel))
            .and_then(|drops| drops.get(drop_index))
            .cloned()
            .map(apply_runtime_drop_state)
            .expect("drop should exist");

        if drop.collected {
            state
                .drops_by_map
                .get_mut(&map_key(map_id, channel))
                .expect("drop map should exist")
                .remove(drop_index);
            self.persist(&state)?;
            return Ok(DropCollectionResult::Missing);
        }

        if map_distance(character_snapshot.x, character_snapshot.y, drop.x, drop.y) >= 18_001 {
            return Ok(DropCollectionResult::TooFarAway);
        }

        if drop.owner_id != 0 && drop.owner_id != character_id && !drop.no_owner {
            return Ok(DropCollectionResult::NotTheOwner);
        }

        if drop.bits_drop {
            let updated_character = {
                let character = state
                    .characters_by_account
                    .get_mut(&account_id)
                    .and_then(|characters| characters.get_mut(character_index))
                    .expect("character should exist");
                character.inventory.bits += i64::from(drop.amount.max(0));
                character.inventory_bits += i64::from(drop.amount.max(0));
                character.clone()
            };
            state
                .drops_by_map
                .get_mut(&map_key(map_id, channel))
                .expect("drop map should exist")
                .remove(drop_index);
            self.persist(&state)?;
            return Ok(DropCollectionResult::BitsCollected {
                drop: drop.clone(),
                amount: drop.amount,
                character: updated_character,
            });
        }

        let updated_character = {
            let character = state
                .characters_by_account
                .get_mut(&account_id)
                .and_then(|characters| characters.get_mut(character_index))
                .expect("character should exist");
            if !add_inventory_item(
                &mut character.inventory.items,
                character.inventory.size,
                &drop,
            ) {
                return Ok(DropCollectionResult::InventoryFull);
            }
            character.clone()
        };

        state
            .drops_by_map
            .get_mut(&map_key(map_id, channel))
            .expect("drop map should exist")
            .remove(drop_index);
        self.persist(&state)?;
        Ok(DropCollectionResult::ItemCollected {
            drop: drop.clone(),
            item_id: drop.item_id,
            amount: drop.amount.clamp(i16::MIN as i32, i16::MAX as i32) as i16,
            character: updated_character,
        })
    }
}

impl PortalRepository for JsonRepository {
    fn portal_by_id(&self, portal_id: i32) -> anyhow::Result<Option<PortalDefinition>> {
        // Hardcoded portal definitions from the game's asset files
        let portals = get_portal_definitions();
        Ok(portals.into_iter().find(|p| p.id == portal_id))
    }
}

pub(crate) fn get_portal_definitions() -> Vec<PortalDefinition> {
    // Demo portals constrained to map ids known by the current client smoke build.
    vec![
        PortalDefinition {
            id: 10001,
            is_local: false,
            destination_map_id: 102,
            destination_x: 32615,
            destination_y: 14930,
        },
        PortalDefinition {
            id: 10002,
            is_local: false,
            destination_map_id: 3,
            destination_x: 18086,
            destination_y: 18874,
        },
        PortalDefinition {
            id: 20001,
            is_local: false,
            destination_map_id: DEFAULT_START_MAP_ID,
            destination_x: DEFAULT_START_X,
            destination_y: DEFAULT_START_Y,
        },
        PortalDefinition {
            id: 20002,
            is_local: false,
            destination_map_id: DEFAULT_START_MAP_ID,
            destination_x: DEFAULT_START_X,
            destination_y: DEFAULT_START_Y,
        },
    ]
}

impl NpcShopRepository for JsonRepository {
    fn shop_by_npc(&self, npc_id: i32, map_id: i16) -> anyhow::Result<Option<NpcShopDefinition>> {
        Ok(get_npc_shops()
            .into_iter()
            .find(|s| s.npc_id == npc_id && s.map_id == map_id))
    }
}

pub(crate) fn get_npc_shops() -> Vec<NpcShopDefinition> {
    vec![
        NpcShopDefinition {
            npc_id: 1001,
            map_id: DEFAULT_START_MAP_ID,
            items: vec![
                NpcShopItem {
                    item_id: 41001,
                    buy_price: 200,
                    sell_price: 100,
                },
                NpcShopItem {
                    item_id: 41002,
                    buy_price: 200,
                    sell_price: 100,
                },
                NpcShopItem {
                    item_id: 42001,
                    buy_price: 500,
                    sell_price: 250,
                },
                NpcShopItem {
                    item_id: 43001,
                    buy_price: 300,
                    sell_price: 150,
                },
            ],
        },
        NpcShopDefinition {
            npc_id: 1002,
            map_id: DEFAULT_START_MAP_ID,
            items: vec![
                NpcShopItem {
                    item_id: 51001,
                    buy_price: 1000,
                    sell_price: 500,
                },
                NpcShopItem {
                    item_id: 52001,
                    buy_price: 800,
                    sell_price: 400,
                },
            ],
        },
    ]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
struct WorldSnapshot {
    accounts: HashMap<String, Account>,
    servers: Vec<ServerDescriptor>,
    characters_by_account: HashMap<AccountId, Vec<CharacterSummary>>,
    mobs_by_map: HashMap<String, Vec<MobSummary>>,
    drops_by_map: HashMap<String, Vec<DropSummary>>,
    resource_hash_hex: Option<String>,
}

impl WorldSnapshot {
    fn demo() -> Self {
        let now = current_unix_timestamp();
        let mut accounts = HashMap::new();
        accounts.insert(
            "admin".to_string(),
            Account {
                id: 1,
                username: "admin".to_string(),
                password_hash: "admin".to_string(),
                email: "admin@odmo.local".to_string(),
                access_level: AccessLevel::Administrator,
                secondary_password: None,
                suspension: None,
            },
        );
        accounts.insert(
            "gm".to_string(),
            Account {
                id: 2,
                username: "gm".to_string(),
                password_hash: "gm".to_string(),
                email: "gm@odmo.local".to_string(),
                access_level: AccessLevel::GameMaster,
                secondary_password: Some("4321".to_string()),
                suspension: None,
            },
        );
        accounts.insert(
            "banned".to_string(),
            Account {
                id: 3,
                username: "banned".to_string(),
                password_hash: "banned".to_string(),
                email: "banned@odmo.local".to_string(),
                access_level: AccessLevel::Player,
                secondary_password: None,
                suspension: Some(AccountSuspension {
                    remaining_seconds: 3_600,
                    reason: "Policy violation".to_string(),
                }),
            },
        );

        Self {
            accounts,
            servers: vec![
                ServerDescriptor {
                    id: 1,
                    name: "ODMO Alpha".to_string(),
                    maintenance: false,
                    overloaded: false,
                    is_new: true,
                    character_count: 0,
                },
                ServerDescriptor {
                    id: 2,
                    name: "ODMO Beta".to_string(),
                    maintenance: false,
                    overloaded: false,
                    is_new: false,
                    character_count: 0,
                },
            ],
            characters_by_account: HashMap::from([
                (
                    1,
                    vec![CharacterSummary {
                        id: 100,
                        account_id: 1,
                        slot: 0,
                        name: "AdminTamer".to_string(),
                        partner_current_slot: 1,
                        partner_current_type: DEFAULT_PARTNER_MODEL_ID,
                        partner_name: "Agumon".to_string(),
                        partner_slots: vec![
                            PartnerSlotSnapshot {
                                slot: 1,
                                digimon_type: DEFAULT_PARTNER_MODEL_ID,
                                model: DEFAULT_PARTNER_MODEL_ID,
                                name: "Agumon".to_string(),
                                ..PartnerSlotSnapshot::default()
                            },
                            PartnerSlotSnapshot {
                                slot: 2,
                                digimon_type: 31_002,
                                model: 31_002,
                                name: "Greymon".to_string(),
                                level: 11,
                                hp: 1_400,
                                ds: 1_200,
                                current_hp: 1_400,
                                current_ds: 1_200,
                                at: 150,
                                de: 120,
                                fs: 120,
                                ms: 260,
                                as_value: 950,
                                ..PartnerSlotSnapshot::default()
                            },
                        ],
                        general_handler: 11_000,
                        partner_handler: 21_000,
                        model: DEFAULT_TAMER_MODEL_ID,
                        partner_model: DEFAULT_PARTNER_MODEL_ID,
                        ..CharacterSummary::default()
                    }],
                ),
                (
                    2,
                    vec![CharacterSummary {
                        id: 200,
                        account_id: 2,
                        slot: 0,
                        name: "GmTamer".to_string(),
                        partner_current_slot: 1,
                        partner_current_type: DEFAULT_GM_PARTNER_MODEL_ID,
                        partner_name: "Gabumon".to_string(),
                        partner_slots: vec![PartnerSlotSnapshot {
                            slot: 1,
                            digimon_type: DEFAULT_GM_PARTNER_MODEL_ID,
                            model: DEFAULT_GM_PARTNER_MODEL_ID,
                            name: "Gabumon".to_string(),
                            ..PartnerSlotSnapshot::default()
                        }],
                        general_handler: 12_000,
                        partner_handler: 22_000,
                        model: DEFAULT_GM_TAMER_MODEL_ID,
                        partner_model: DEFAULT_GM_PARTNER_MODEL_ID,
                        ..CharacterSummary::default()
                    }],
                ),
            ]),
            mobs_by_map: HashMap::from([(
                map_key(DEFAULT_START_MAP_ID, 0),
                vec![
                    MobSummary {
                        id: 400,
                        map_id: DEFAULT_START_MAP_ID,
                        channel: 0,
                        handler: 34_000,
                        type_id: 51_001,
                        model: 51_001,
                        name: "Goblimon".to_string(),
                        level: 12,
                        x: DEFAULT_START_X + 66,
                        y: DEFAULT_START_Y + 45,
                        previous_x: DEFAULT_START_X + 46,
                        previous_y: DEFAULT_START_Y + 25,
                        ..MobSummary::default()
                    },
                    MobSummary {
                        id: 401,
                        map_id: DEFAULT_START_MAP_ID,
                        channel: 0,
                        handler: 34_001,
                        type_id: 51_002,
                        model: 51_002,
                        name: "Woodmon".to_string(),
                        level: 18,
                        x: DEFAULT_START_X + 416,
                        y: DEFAULT_START_Y + 225,
                        previous_x: DEFAULT_START_X + 386,
                        previous_y: DEFAULT_START_Y + 195,
                        ..MobSummary::default()
                    },
                ],
            )]),
            drops_by_map: HashMap::from([(
                map_key(DEFAULT_START_MAP_ID, 0),
                vec![
                    DropSummary {
                        id: 500,
                        map_id: DEFAULT_START_MAP_ID,
                        channel: 0,
                        handler: 49_200,
                        owner_id: 100,
                        owner_handler: 11_000,
                        item_id: 90600,
                        amount: 123,
                        x: DEFAULT_START_X + 76,
                        y: DEFAULT_START_Y + 55,
                        owner_expires_at_unix: now + 60,
                        expires_at_unix: now + 90,
                        bits_drop: true,
                        ..DropSummary::default()
                    },
                    DropSummary {
                        id: 501,
                        map_id: DEFAULT_START_MAP_ID,
                        channel: 0,
                        handler: 49_201,
                        owner_id: 0,
                        owner_handler: 0,
                        item_id: 5101,
                        amount: 1,
                        x: DEFAULT_START_X + 286,
                        y: DEFAULT_START_Y + 145,
                        owner_expires_at_unix: now.saturating_sub(5),
                        expires_at_unix: now + 30,
                        no_owner: true,
                        ..DropSummary::default()
                    },
                ],
            )]),
            resource_hash_hex: Some("0123456789ABCDEF".to_string()),
        }
    }
}

impl Default for WorldSnapshot {
    fn default() -> Self {
        Self::demo()
    }
}

fn normalize_legacy_snapshot(snapshot: &mut WorldSnapshot) -> bool {
    let mut changed = false;

    for (account_id, characters) in snapshot.characters_by_account.iter_mut() {
        let fallback_model = if *account_id == 2 {
            DEFAULT_GM_TAMER_MODEL_ID
        } else {
            DEFAULT_TAMER_MODEL_ID
        };

        for character in characters.iter_mut() {
            changed |= normalize_legacy_character(character, fallback_model);
        }
    }

    if !snapshot
        .mobs_by_map
        .contains_key(&map_key(DEFAULT_START_MAP_ID, 0))
    {
        snapshot.mobs_by_map.insert(
            map_key(DEFAULT_START_MAP_ID, 0),
            WorldSnapshot::demo()
                .mobs_by_map
                .remove(&map_key(DEFAULT_START_MAP_ID, 0))
                .unwrap_or_default(),
        );
        changed = true;
    }

    if !snapshot
        .drops_by_map
        .contains_key(&map_key(DEFAULT_START_MAP_ID, 0))
    {
        snapshot.drops_by_map.insert(
            map_key(DEFAULT_START_MAP_ID, 0),
            WorldSnapshot::demo()
                .drops_by_map
                .remove(&map_key(DEFAULT_START_MAP_ID, 0))
                .unwrap_or_default(),
        );
        changed = true;
    }

    changed
}

fn normalize_legacy_character(character: &mut CharacterSummary, fallback_model: i32) -> bool {
    const LEGACY_START_X: i32 = 14_834;
    const LEGACY_START_Y: i32 = 9_895;

    let mut changed = false;

    if character.map_id == 0 || character.map_id == 8101 {
        character.map_id = DEFAULT_START_MAP_ID;
        character.x = DEFAULT_START_X;
        character.y = DEFAULT_START_Y;
        character.z = 0.0;
        character.partner_x = DEFAULT_START_X;
        character.partner_y = DEFAULT_START_Y;
        character.partner_z = 0.0;
        changed = true;
    }

    if character.map_id == DEFAULT_START_MAP_ID
        && character.x == LEGACY_START_X
        && character.y == LEGACY_START_Y
    {
        character.x = DEFAULT_START_X;
        character.y = DEFAULT_START_Y;
        character.z = 0.0;
        character.partner_x = DEFAULT_START_X;
        character.partner_y = DEFAULT_START_Y;
        character.partner_z = 0.0;
        changed = true;
    }

    if matches!(character.model, 1001 | 1002 | 1003) || character.model < 80_000 {
        character.model = fallback_model;
        changed = true;
    }

    if character.partner_model <= 0 {
        character.partner_model = DEFAULT_PARTNER_MODEL_ID;
        changed = true;
    }

    if character.general_handler == 0 {
        character.general_handler = character.id as u32 + 10_000;
        changed = true;
    }

    if character.partner_handler == 0 {
        character.partner_handler = character.id as u32 + 20_000;
        changed = true;
    }

    changed
}

fn current_unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn apply_runtime_drop_state(mut drop: DropSummary) -> DropSummary {
    let now = current_unix_timestamp();
    if drop.expires_at_unix > 0 && now >= drop.expires_at_unix {
        drop.collected = true;
    }
    if !drop.collected && drop.owner_expires_at_unix > 0 && now >= drop.owner_expires_at_unix {
        drop.no_owner = true;
    }
    drop
}

fn map_distance(xa: i32, ya: i32, xb: i32, yb: i32) -> i64 {
    let distance_x = (xb as i64 - xa as i64).pow(2);
    let distance_y = (yb as i64 - ya as i64).pow(2);
    ((distance_x + distance_y) as f64).sqrt() as i64
}

fn add_inventory_item(items: &mut Vec<ItemRecord>, size: u16, drop: &DropSummary) -> bool {
    if let Some(existing) = items.iter_mut().find(|item| item.item_id == drop.item_id) {
        existing.amount = existing.amount.saturating_add(drop.amount.max(0));
        existing.sync_record();
        return true;
    }

    if items.len() >= size as usize {
        return false;
    }

    let record = ItemRecord::new(drop.item_id, drop.amount.max(1));
    items.push(record);
    true
}
