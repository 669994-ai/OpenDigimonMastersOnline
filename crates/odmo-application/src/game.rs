use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicI16, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

use thiserror::Error;

use odmo_protocol::{
    AvailableChannelsPacket, CashShopCoinsPacket, DigimonWalkPacket, FriendConnectPacket,
    GameConnectionPacket, GameInitialInfoPacket, GameRequest, GuildHistoricPacket,
    GuildInformationPacket, GuildRankPacket, InventoryType, ItemConsumeFailPacket,
    ItemMoveFailPacket, ItemMoveSuccessPacket, LoadBuffsPacket, LoadDropsPacket,
    LoadInventoryPacket, LoadMobBuffsPacket, LoadMobsPacket, LoadTamerPacket, LocalMapSwapPacket,
    MapSwapPacket, MembershipPacket, NpcPurchaseResultPacket, NpcSellResultPacket, PickBitsPacket,
    PickItemFailPacket, PickItemFailReason, PickItemPacket, SealsPacket, ServerExperiencePacket,
    SplitItemPacket, TamerAttendancePacket, TamerRelationsPacket, TamerWalkPacket,
    TamerXaiResourcesPacket, TimeRewardPacket, UnloadDropsPacket, UnloadMobsPacket,
    UnloadTamerPacket, UpdateMovementSpeedPacket, UpdateStatusPacket, XaiInfoPacket,
};
use odmo_types::{AccountId, ItemRecord};

use crate::{
    character::CharacterRepository,
    portal::{PortalBridge, SocialNotification, SocialNotificationKind},
};

const HANDSHAKE_DEGREE: i16 = 32321;
const START_TO_SEE_DISTANCE: i64 = 18_000;
const STOP_SEEING_DISTANCE: i64 = 18_001;

#[derive(Debug, Clone)]
pub struct GameServiceConfig {
    pub portal_state_dir: PathBuf,
}

#[derive(Debug, Clone)]
pub struct GameSession {
    pub handshake_seed: i16,
    pub account_id: Option<AccountId>,
    pub character_id: Option<u64>,
    pub announced_friend_connect: bool,
    pub registered_map_presence: bool,
    pub viewed_characters: HashMap<u64, odmo_types::CharacterSummary>,
    pub viewed_mobs: HashMap<u64, odmo_types::MobSummary>,
    pub viewed_drops: HashMap<u64, odmo_types::DropSummary>,
}

impl GameSession {
    pub fn new(handshake_seed: i16) -> Self {
        Self {
            handshake_seed,
            account_id: None,
            character_id: None,
            announced_friend_connect: false,
            registered_map_presence: false,
            viewed_characters: HashMap::new(),
            viewed_mobs: HashMap::new(),
            viewed_drops: HashMap::new(),
        }
    }
}

pub trait MapMobRepository: Send + Sync {
    fn mobs_by_map(&self, map_id: i16, channel: u8) -> anyhow::Result<Vec<odmo_types::MobSummary>>;
}

pub trait MapDropRepository: Send + Sync {
    fn drops_by_map(
        &self,
        map_id: i16,
        channel: u8,
    ) -> anyhow::Result<Vec<odmo_types::DropSummary>>;
    fn collect_drop(
        &self,
        character_id: u64,
        map_id: i16,
        channel: u8,
        drop_handler: u32,
    ) -> anyhow::Result<DropCollectionResult>;
}

#[derive(Debug, Clone, PartialEq)]
pub enum DropCollectionResult {
    Missing,
    NotTheOwner,
    InventoryFull,
    TooFarAway,
    BitsCollected {
        drop: odmo_types::DropSummary,
        amount: i32,
        character: odmo_types::CharacterSummary,
    },
    ItemCollected {
        drop: odmo_types::DropSummary,
        item_id: i32,
        amount: i16,
        character: odmo_types::CharacterSummary,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct PortalDefinition {
    pub id: i32,
    pub is_local: bool,
    pub destination_map_id: i16,
    pub destination_x: i32,
    pub destination_y: i32,
}

pub trait PortalRepository: Send + Sync {
    fn portal_by_id(&self, portal_id: i32) -> anyhow::Result<Option<PortalDefinition>>;
}

#[derive(Debug, Clone, PartialEq)]
pub struct NpcShopItem {
    pub item_id: i32,
    pub buy_price: i32,
    pub sell_price: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NpcShopDefinition {
    pub npc_id: i32,
    pub map_id: i16,
    pub items: Vec<NpcShopItem>,
}

pub trait NpcShopRepository: Send + Sync {
    fn shop_by_npc(&self, npc_id: i32, map_id: i16) -> anyhow::Result<Option<NpcShopDefinition>>;
}

pub trait GameRepository:
    CharacterRepository + MapMobRepository + MapDropRepository + PortalRepository + NpcShopRepository
{
}

impl<T> GameRepository for T where
    T: CharacterRepository
        + MapMobRepository
        + MapDropRepository
        + PortalRepository
        + NpcShopRepository
{
}

#[derive(Clone)]
pub struct GameApplication {
    portal_bridge: PortalBridge,
    repository: Arc<dyn GameRepository>,
    broadcast: Option<Arc<dyn crate::BroadcastSink>>,
    game_server_address: String,
    game_server_port: i32,
}

impl GameApplication {
    pub fn new(config: GameServiceConfig, repository: Arc<dyn GameRepository>) -> Self {
        let portal_bridge =
            PortalBridge::from_json(config.portal_state_dir).expect("portal bridge should initialize");
        Self {
            portal_bridge,
            repository,
            broadcast: None,
            game_server_address: "127.0.0.1".to_string(),
            game_server_port: 7003,
        }
    }

    pub fn with_broadcast(mut self, broadcast: Arc<dyn crate::BroadcastSink>) -> Self {
        self.broadcast = Some(broadcast);
        self
    }

    pub fn with_game_server(mut self, address: String, port: i32) -> Self {
        self.game_server_address = address;
        self.game_server_port = port;
        self
    }

    pub fn handle_request(
        &self,
        session: &mut GameSession,
        request: GameRequest,
    ) -> Result<Vec<Vec<u8>>, GameFlowError> {
        let mut responses = self
            .drain_social_notifications(session)
            .map_err(|error| GameFlowError::PortalBridge(error.to_string()))?;
        responses.extend(
            self.reconcile_map_visibility(session)
                .map_err(|error| GameFlowError::PortalBridge(error.to_string()))?,
        );
        responses.extend(
            self.reconcile_mob_visibility(session)
                .map_err(|error| GameFlowError::Storage(error.to_string()))?,
        );
        responses.extend(
            self.reconcile_drop_visibility(session)
                .map_err(|error| GameFlowError::Storage(error.to_string()))?,
        );

        let request_responses = match request {
            GameRequest::Connection { .. } => Ok(vec![
                GameConnectionPacket {
                    handshake: session.handshake_seed ^ HANDSHAKE_DEGREE,
                }
                .encode(),
            ]),
            GameRequest::KeepConnection => Ok(Vec::new()),
            GameRequest::InitialInformation {
                account_id,
                access_code: _,
            } => {
                let ticket = self
                    .portal_bridge
                    .consume_game_session_ticket(account_id)
                    .map_err(|error| GameFlowError::PortalBridge(error.to_string()))?
                    .ok_or(GameFlowError::MissingSessionTicket(account_id))?;

                let character = self
                    .repository
                    .character_by_id(ticket.character_id)
                    .map_err(|error| GameFlowError::Storage(error.to_string()))?
                    .ok_or(GameFlowError::CharacterNotFound(ticket.character_id))?;

                session.account_id = Some(account_id);
                session.character_id = Some(character.id);

                Ok(vec![GameInitialInfoPacket { character }.encode()])
            }
            GameRequest::ComplementarInformation => {
                let character_id = session.character_id.ok_or(GameFlowError::Unauthenticated)?;
                let character = self
                    .repository
                    .character_by_id(character_id)
                    .map_err(|error| GameFlowError::Storage(error.to_string()))?
                    .ok_or(GameFlowError::CharacterNotFound(character_id))?;
                let mut responses = vec![
                    SealsPacket {
                        seal_list: character.seal_list.clone(),
                    }
                    .encode(),
                    LoadInventoryPacket {
                        inventory: character.inventory.clone(),
                        inventory_type: InventoryType::Inventory,
                    }
                    .encode(),
                    LoadInventoryPacket {
                        inventory: character.warehouse.clone(),
                        inventory_type: InventoryType::Warehouse,
                    }
                    .encode(),
                    LoadInventoryPacket {
                        inventory: character.extra_inventory.clone(),
                        inventory_type: InventoryType::ExtraInventory,
                    }
                    .encode(),
                    ServerExperiencePacket {
                        experience: character.server_experience,
                    }
                    .encode(),
                    MembershipPacket {
                        remaining_seconds: character.membership_seconds,
                    }
                    .encode(),
                    CashShopCoinsPacket {
                        premium: character.premium,
                        silk: character.silk,
                    }
                    .encode(),
                    TimeRewardPacket {
                        reward: character.daily_reward.clone(),
                    }
                    .encode(),
                    TamerRelationsPacket {
                        friends: character.friends.clone(),
                        foes: character.foes.clone(),
                    }
                    .encode(),
                    AvailableChannelsPacket {
                        channels: character.available_channels.clone(),
                    }
                    .encode(),
                    TamerAttendancePacket {
                        attendance: character.attendance.clone(),
                    }
                    .encode(),
                    UpdateStatusPacket {
                        character: character.clone(),
                    }
                    .encode(),
                    UpdateMovementSpeedPacket {
                        character: character.clone(),
                    }
                    .encode(),
                ];

                if let Some(account_warehouse) = &character.account_warehouse {
                    responses.push(
                        LoadInventoryPacket {
                            inventory: account_warehouse.clone(),
                            inventory_type: InventoryType::AccountWarehouse,
                        }
                        .encode(),
                    );
                }

                if character.xai.as_ref().is_some_and(|xai| xai.item_id > 0) {
                    responses.push(
                        XaiInfoPacket {
                            xai: character.xai.clone(),
                        }
                        .encode(),
                    );
                    responses.push(
                        TamerXaiResourcesPacket {
                            current_xgauge: character.current_xgauge,
                            current_xcrystals: character.current_xcrystals,
                        }
                        .encode(),
                    );
                }

                if let Some(guild) = &character.guild {
                    responses.push(
                        GuildInformationPacket {
                            guild: guild.clone(),
                        }
                        .encode(),
                    );
                    responses.push(
                        GuildHistoricPacket {
                            entries: guild.historic.clone(),
                        }
                        .encode(),
                    );

                    if (1..=100).contains(&guild.rank_position) {
                        responses.push(
                            GuildRankPacket {
                                position: guild.rank_position,
                            }
                            .encode(),
                        );
                    }
                }

                if !session.announced_friend_connect {
                    self.announce_friend_connect(&character)
                        .map_err(|error| GameFlowError::PortalBridge(error.to_string()))?;
                    session.announced_friend_connect = true;
                }

                responses.extend(
                    self.register_map_presence(session, &character)
                        .map_err(|error| GameFlowError::PortalBridge(error.to_string()))?,
                );
                responses.extend(
                    self.reconcile_mob_visibility(session)
                        .map_err(|error| GameFlowError::Storage(error.to_string()))?,
                );
                responses.extend(
                    self.reconcile_drop_visibility(session)
                        .map_err(|error| GameFlowError::Storage(error.to_string()))?,
                );

                Ok(responses)
            }
            GameRequest::ConsumeItem {
                target_handler: _,
                slot,
            } => {
                let character_id = session.character_id.ok_or(GameFlowError::Unauthenticated)?;
                let mut character = self
                    .repository
                    .character_by_id(character_id)
                    .map_err(|error| GameFlowError::Storage(error.to_string()))?
                    .ok_or(GameFlowError::CharacterNotFound(character_id))?;

                let inventory_slot = slot as usize;
                if inventory_slot >= character.inventory.items.len() {
                    return Ok(vec![
                        ItemConsumeFailPacket {
                            slot,
                            item_id: 0,
                            result: 1,
                        }
                        .encode(),
                    ]);
                }

                let item = &character.inventory.items[inventory_slot];
                if item.item_id <= 0 || item.amount <= 0 {
                    return Ok(vec![
                        ItemConsumeFailPacket {
                            slot,
                            item_id: 0,
                            result: 1,
                        }
                        .encode(),
                    ]);
                }

                let new_amount = item.amount - 1;
                if new_amount <= 0 {
                    // Remove item completely
                    character.inventory.items[inventory_slot] = ItemRecord::default();
                } else {
                    // Reduce amount
                    let mut updated = item.clone();
                    updated.amount = new_amount;
                    updated.sync_record();
                    character.inventory.items[inventory_slot] = updated;
                }

                self.repository
                    .update_inventory(character_id, character.inventory)
                    .map_err(|error| GameFlowError::Storage(error.to_string()))?;

                // Re-read character for the response
                let updated_character = self
                    .repository
                    .character_by_id(character_id)
                    .map_err(|error| GameFlowError::Storage(error.to_string()))?
                    .ok_or(GameFlowError::CharacterNotFound(character_id))?;

                let mut responses = Vec::new();
                responses.push(
                    LoadInventoryPacket {
                        inventory: updated_character.inventory.clone(),
                        inventory_type: InventoryType::Inventory,
                    }
                    .encode(),
                );

                // If item was a consumable that affects HP/DS (type check would need asset data),
                // send UpdateStatusPacket
                responses.push(
                    UpdateStatusPacket {
                        character: updated_character,
                    }
                    .encode(),
                );

                Ok(responses)
            }
            GameRequest::MoveItem {
                origin_slot,
                destination_slot,
            } => {
                let character_id = session.character_id.ok_or(GameFlowError::Unauthenticated)?;
                let mut character = self
                    .repository
                    .character_by_id(character_id)
                    .map_err(|error| GameFlowError::Storage(error.to_string()))?
                    .ok_or(GameFlowError::CharacterNotFound(character_id))?;

                const TAB_INVENTORY: u16 = 0;
                const TAB_WAREHOUSE: u16 = 2000;
                const TAB_SHARESTASH: u16 = 9000;
                fn tab_class(sid: u16) -> u16 { sid / 1000 * 1000 }
                fn tab_index(sid: u16) -> usize { (sid % 1000) as usize }

                let src_tab = tab_class(origin_slot);
                let dst_tab = tab_class(destination_slot);
                let src_idx = tab_index(origin_slot);
                let dst_idx = tab_index(destination_slot);

                // Helper: swap/merge items between two mutable inventory slices
                fn transfer_between(
                    src_items: &mut [ItemRecord],
                    src_idx: usize,
                    dst_items: &mut [ItemRecord],
                    dst_idx: usize,
                ) {
                    let origin_item = src_items[src_idx].clone();
                    let dest_item = dst_items[dst_idx].clone();

                    if dest_item.item_id > 0
                        && origin_item.item_id > 0
                        && dest_item.item_id == origin_item.item_id
                    {
                        let mut merged = dest_item;
                        merged.amount += origin_item.amount;
                        dst_items[dst_idx] = merged;
                        src_items[src_idx] = ItemRecord::default();
                    } else {
                        src_items[src_idx] = dest_item;
                        dst_items[dst_idx] = origin_item;
                    }
                }

                // Same-tab move: swap via indices without double borrow
                fn swap_within(items: &mut [ItemRecord], a: usize, b: usize) {
                    if a >= items.len() || b >= items.len() { return; }
                    let origin = items[a].clone();
                    let dest = items[b].clone();
                    if dest.item_id > 0 && origin.item_id > 0 && dest.item_id == origin.item_id {
                        let mut merged = dest;
                        merged.amount += origin.amount;
                        items[b] = merged;
                        items[a] = ItemRecord::default();
                    } else {
                        items[a] = dest;
                        items[b] = origin;
                    }
                }

                let success = match (src_tab, dst_tab) {
                    (TAB_INVENTORY, TAB_INVENTORY) => {
                        let len = character.inventory.items.len();
                        if src_idx < len && dst_idx < len {
                            swap_within(&mut character.inventory.items, src_idx, dst_idx);
                            true
                        } else {
                            false
                        }
                    }
                    (TAB_WAREHOUSE, TAB_WAREHOUSE) => {
                        let len = character.warehouse.items.len();
                        if src_idx < len && dst_idx < len {
                            swap_within(&mut character.warehouse.items, src_idx, dst_idx);
                            true
                        } else {
                            false
                        }
                    }
                    (TAB_INVENTORY, TAB_WAREHOUSE) => {
                        let i_len = character.inventory.items.len();
                        let w_len = character.warehouse.items.len();
                        if src_idx < i_len && dst_idx < w_len {
                            transfer_between(
                                &mut character.inventory.items, src_idx,
                                &mut character.warehouse.items, dst_idx,
                            );
                            true
                        } else {
                            false
                        }
                    }
                    (TAB_WAREHOUSE, TAB_INVENTORY) => {
                        let w_len = character.warehouse.items.len();
                        let i_len = character.inventory.items.len();
                        if src_idx < w_len && dst_idx < i_len {
                            transfer_between(
                                &mut character.warehouse.items, src_idx,
                                &mut character.inventory.items, dst_idx,
                            );
                            true
                        } else {
                            false
                        }
                    }
                    (TAB_INVENTORY, TAB_SHARESTASH) => {
                        let aw = character.account_warehouse.as_mut();
                        match aw {
                            Some(aw) if src_idx < character.inventory.items.len() && dst_idx < aw.items.len() => {
                                transfer_between(
                                    &mut character.inventory.items, src_idx,
                                    &mut aw.items, dst_idx,
                                );
                                true
                            }
                            _ => false,
                        }
                    }
                    (TAB_SHARESTASH, TAB_INVENTORY) => {
                        let aw = character.account_warehouse.as_mut();
                        match aw {
                            Some(aw) if src_idx < aw.items.len() && dst_idx < character.inventory.items.len() => {
                                transfer_between(
                                    &mut aw.items, src_idx,
                                    &mut character.inventory.items, dst_idx,
                                );
                                true
                            }
                            _ => false,
                        }
                    }
                    _ => false,
                };

                if success {
                    // Persist all involved inventories unconditionally
                    self.repository
                        .update_inventory(character_id, character.inventory.clone())
                        .map_err(|error| GameFlowError::Storage(error.to_string()))?;
                    self.repository
                        .update_warehouse(character_id, character.warehouse.clone())
                        .map_err(|error| GameFlowError::Storage(error.to_string()))?;
                    if let Some(aw) = &character.account_warehouse {
                        self.repository
                            .update_account_warehouse(character_id, aw.clone())
                            .map_err(|error| GameFlowError::Storage(error.to_string()))?;
                    }

                    let mut responses = Vec::new();
                    responses.push(
                        ItemMoveSuccessPacket {
                            origin_slot,
                            destination_slot,
                        }
                        .encode(),
                    );
                    for tab in [src_tab, dst_tab] {
                        let (inv, ty) = match tab {
                            TAB_INVENTORY => (&character.inventory, InventoryType::Inventory),
                            TAB_WAREHOUSE => (&character.warehouse, InventoryType::Warehouse),
                            TAB_SHARESTASH => (
                                character.account_warehouse.as_ref()
                                    .unwrap_or_else(|| unreachable!()),
                                InventoryType::AccountWarehouse,
                            ),
                            _ => continue,
                        };
                        responses.push(
                            LoadInventoryPacket {
                                inventory: inv.clone(),
                                inventory_type: ty,
                            }
                            .encode(),
                        );
                    }
                    Ok(responses)
                } else {
                    Ok(vec![
                        ItemMoveFailPacket {
                            origin_slot,
                            destination_slot,
                        }
                        .encode(),
                    ])
                }
            }
            GameRequest::SplitItem {
                origin_slot,
                destination_slot,
                amount,
            } => {
                let character_id = session.character_id.ok_or(GameFlowError::Unauthenticated)?;
                let mut character = self
                    .repository
                    .character_by_id(character_id)
                    .map_err(|error| GameFlowError::Storage(error.to_string()))?
                    .ok_or(GameFlowError::CharacterNotFound(character_id))?;

                fn tab_class(sid: u16) -> u16 { sid / 1000 * 1000 }
                fn tab_index(sid: u16) -> usize { (sid % 1000) as usize }

                let src_tab = tab_class(origin_slot);
                let dst_tab = tab_class(destination_slot);
                let src_idx = tab_index(origin_slot);
                let dst_idx = tab_index(destination_slot);

                fn split_within(items: &mut [ItemRecord], src: usize, dst: usize, amt: i32) -> bool {
                    if src >= items.len() || dst >= items.len() { return false; }
                    let source = items[src].clone();
                    let dest = items[dst].clone();
                    if source.item_id <= 0 || source.amount < amt { return false; }
                    if dest.item_id > 0 && dest.item_id != source.item_id { return false; }

                    if dest.item_id > 0 {
                        let mut updated = dest;
                        updated.amount += amt;
                        updated.sync_record();
                        items[dst] = updated;
                    } else {
                        let mut new_item = source.clone();
                        new_item.amount = amt;
                        new_item.sync_record();
                        items[dst] = new_item;
                    }

                    let remaining = source.amount - amt;
                    if remaining <= 0 {
                        items[src] = ItemRecord::default();
                    } else {
                        let mut updated = source;
                        updated.amount = remaining;
                        updated.sync_record();
                        items[src] = updated;
                    }
                    true
                }

                fn split_cross(
                    src_items: &mut [ItemRecord], src_idx: usize,
                    dst_items: &mut [ItemRecord], dst_idx: usize,
                    amt: i32,
                ) -> bool {
                    if src_idx >= src_items.len() || dst_idx >= dst_items.len() { return false; }
                    let source = src_items[src_idx].clone();
                    let dest = dst_items[dst_idx].clone();
                    if source.item_id <= 0 || source.amount < amt { return false; }
                    if dest.item_id > 0 && dest.item_id != source.item_id { return false; }

                    if dest.item_id > 0 {
                        let mut updated = dest;
                        updated.amount += amt;
                        updated.sync_record();
                        dst_items[dst_idx] = updated;
                    } else {
                        let mut new_item = source.clone();
                        new_item.amount = amt;
                        new_item.sync_record();
                        dst_items[dst_idx] = new_item;
                    }

                    let remaining = source.amount - amt;
                    if remaining <= 0 {
                        src_items[src_idx] = ItemRecord::default();
                    } else {
                        let mut updated = source;
                        updated.amount = remaining;
                        updated.sync_record();
                        src_items[src_idx] = updated;
                    }
                    true
                }

                let success = match (src_tab, dst_tab) {
                    (0, 0) => split_within(&mut character.inventory.items, src_idx, dst_idx, amount as i32),
                    (2000, 2000) => split_within(&mut character.warehouse.items, src_idx, dst_idx, amount as i32),
                    (0, 2000) => split_cross(
                        &mut character.inventory.items, src_idx,
                        &mut character.warehouse.items, dst_idx,
                        amount as i32,
                    ),
                    (2000, 0) => split_cross(
                        &mut character.warehouse.items, src_idx,
                        &mut character.inventory.items, dst_idx,
                        amount as i32,
                    ),
                    _ => false,
                };

                if !success {
                    return Ok(vec![
                        SplitItemPacket {
                            origin_slot,
                            destination_slot,
                            amount: 0,
                        }
                        .encode(),
                    ]);
                }

                // Persist all involved inventories
                self.repository
                    .update_inventory(character_id, character.inventory.clone())
                    .map_err(|error| GameFlowError::Storage(error.to_string()))?;
                self.repository
                    .update_warehouse(character_id, character.warehouse.clone())
                    .map_err(|error| GameFlowError::Storage(error.to_string()))?;

                let mut responses = Vec::new();
                responses.push(
                    SplitItemPacket {
                        origin_slot,
                        destination_slot,
                        amount,
                    }
                    .encode(),
                );
                for tab in [src_tab, dst_tab] {
                    let (inv, ty) = match tab {
                        0 => (&character.inventory, InventoryType::Inventory),
                        2000 => (&character.warehouse, InventoryType::Warehouse),
                        _ => continue,
                    };
                    responses.push(
                        LoadInventoryPacket {
                            inventory: inv.clone(),
                            inventory_type: ty,
                        }
                        .encode(),
                    );
                }
                Ok(responses)
            }
            GameRequest::RemoveItem {
                slot,
                x: _,
                y: _,
                amount,
            } => {
                let character_id = session.character_id.ok_or(GameFlowError::Unauthenticated)?;
                let mut character = self
                    .repository
                    .character_by_id(character_id)
                    .map_err(|error| GameFlowError::Storage(error.to_string()))?
                    .ok_or(GameFlowError::CharacterNotFound(character_id))?;

                let slot_idx = slot as usize;
                if slot_idx >= character.inventory.items.len() {
                    return Ok(Vec::new());
                }

                let item = &character.inventory.items[slot_idx];
                if item.item_id <= 0 || item.amount <= 0 {
                    return Ok(Vec::new());
                }

                let new_amount = item.amount - amount as i32;
                if new_amount <= 0 {
                    character.inventory.items[slot_idx] = ItemRecord::default();
                } else {
                    let mut updated = item.clone();
                    updated.amount = new_amount;
                    updated.sync_record();
                    character.inventory.items[slot_idx] = updated;
                }

                self.repository
                    .update_inventory(character_id, character.inventory.clone())
                    .map_err(|error| GameFlowError::Storage(error.to_string()))?;

                Ok(vec![
                    LoadInventoryPacket {
                        inventory: character.inventory,
                        inventory_type: InventoryType::Inventory,
                    }
                    .encode(),
                ])
            }
            GameRequest::NpcPurchase {
                npc_id,
                unk: _,
                shop_slot,
                purchase_count,
            } => {
                let character_id = session.character_id.ok_or(GameFlowError::Unauthenticated)?;
                let mut character = self
                    .repository
                    .character_by_id(character_id)
                    .map_err(|error| GameFlowError::Storage(error.to_string()))?
                    .ok_or(GameFlowError::CharacterNotFound(character_id))?;

                let shop = self
                    .repository
                    .shop_by_npc(npc_id, character.map_id)
                    .map_err(|error| GameFlowError::Storage(error.to_string()))?
                    .ok_or(GameFlowError::Storage(format!(
                        "NPC shop {npc_id} not found on map {}",
                        character.map_id
                    )))?;

                let slot_idx = shop_slot as usize;
                if slot_idx >= shop.items.len() {
                    return Ok(vec![
                        NpcPurchaseResultPacket {
                            success: false,
                            remaining_bits: character.inventory.bits,
                        }
                        .encode(),
                    ]);
                }

                let npc_item = &shop.items[slot_idx];
                let total_cost = npc_item.buy_price * purchase_count as i32;

                if character.inventory.bits < total_cost as i64 {
                    return Ok(vec![
                        NpcPurchaseResultPacket {
                            success: false,
                            remaining_bits: character.inventory.bits,
                        }
                        .encode(),
                    ]);
                }

                // Deduct bits
                character.inventory.bits -= total_cost as i64;

                // Find empty slot or existing stack of same item
                let mut placed = false;
                for i in 0..character
                    .inventory
                    .items
                    .len()
                    .min(character.inventory.size as usize)
                {
                    let existing = &character.inventory.items[i];
                    if existing.item_id == npc_item.item_id && existing.amount > 0 {
                        // Stack onto existing
                        let mut updated = existing.clone();
                        updated.amount += purchase_count as i32;
                        updated.sync_record();
                        character.inventory.items[i] = updated;
                        placed = true;
                        break;
                    }
                }

                if !placed {
                    // Find first empty slot
                    for i in 0..character
                        .inventory
                        .items
                        .len()
                        .min(character.inventory.size as usize)
                    {
                        if character.inventory.items[i].item_id <= 0
                            || character.inventory.items[i].amount <= 0
                        {
                            character.inventory.items[i] =
                                ItemRecord::new(npc_item.item_id, purchase_count as i32);
                            placed = true;
                            break;
                        }
                    }
                }

                if !placed {
                    // No space - refund bits
                    character.inventory.bits += total_cost as i64;
                    return Ok(vec![
                        NpcPurchaseResultPacket {
                            success: false,
                            remaining_bits: character.inventory.bits,
                        }
                        .encode(),
                    ]);
                }

                self.repository
                    .update_inventory(character_id, character.inventory.clone())
                    .map_err(|error| GameFlowError::Storage(error.to_string()))?;

                let mut responses = Vec::new();
                responses.push(
                    NpcPurchaseResultPacket {
                        success: true,
                        remaining_bits: character.inventory.bits,
                    }
                    .encode(),
                );
                responses.push(
                    LoadInventoryPacket {
                        inventory: character.inventory,
                        inventory_type: InventoryType::Inventory,
                    }
                    .encode(),
                );
                Ok(responses)
            }
            GameRequest::NpcSell {
                npc_id,
                unk: _,
                item_slot,
                sell_amount,
            } => {
                let character_id = session.character_id.ok_or(GameFlowError::Unauthenticated)?;
                let mut character = self
                    .repository
                    .character_by_id(character_id)
                    .map_err(|error| GameFlowError::Storage(error.to_string()))?
                    .ok_or(GameFlowError::CharacterNotFound(character_id))?;

                let slot_idx = item_slot as usize;
                if slot_idx >= character.inventory.items.len() {
                    return Ok(Vec::new());
                }

                let item = &character.inventory.items[slot_idx];
                if item.item_id <= 0 || item.amount <= 0 {
                    return Ok(Vec::new());
                }

                if item.amount < sell_amount as i32 {
                    return Ok(Vec::new());
                }

                // Look up actual sell price from NPC shop definition
                let sell_per_item =
                    if let Ok(Some(shop)) = self.repository.shop_by_npc(npc_id, character.map_id) {
                        shop.items
                            .iter()
                            .find(|si| si.item_id == item.item_id)
                            .map(|si| si.sell_price as i64)
                            .unwrap_or(100)
                    } else {
                        100 // Fallback default if shop not found
                    };
                let sell_price = sell_per_item * sell_amount as i64;

                character.inventory.bits += sell_price;

                // Reduce or remove item
                let remaining = item.amount - sell_amount as i32;
                if remaining <= 0 {
                    character.inventory.items[slot_idx] = ItemRecord::default();
                } else {
                    let mut updated = item.clone();
                    updated.amount = remaining;
                    updated.sync_record();
                    character.inventory.items[slot_idx] = updated;
                }

                self.repository
                    .update_inventory(character_id, character.inventory.clone())
                    .map_err(|error| GameFlowError::Storage(error.to_string()))?;

                let mut responses = Vec::new();
                responses.push(
                    NpcSellResultPacket {
                        remaining_bits: character.inventory.bits,
                    }
                    .encode(),
                );
                responses.push(
                    LoadInventoryPacket {
                        inventory: character.inventory,
                        inventory_type: InventoryType::Inventory,
                    }
                    .encode(),
                );
                Ok(responses)
            }
            GameRequest::LootItem { drop_handler } => {
                let character_id = session.character_id.ok_or(GameFlowError::Unauthenticated)?;
                let character = self
                    .repository
                    .character_by_id(character_id)
                    .map_err(|error| GameFlowError::Storage(error.to_string()))?
                    .ok_or(GameFlowError::CharacterNotFound(character_id))?;

                let responses = match self
                    .repository
                    .collect_drop(
                        character_id,
                        character.map_id,
                        character.channel,
                        drop_handler,
                    )
                    .map_err(|error| GameFlowError::Storage(error.to_string()))?
                {
                    DropCollectionResult::Missing => vec![
                        UnloadDropsPacket {
                            drop: odmo_types::DropSummary {
                                handler: drop_handler,
                                ..odmo_types::DropSummary::default()
                            },
                        }
                        .encode(),
                    ],
                    DropCollectionResult::NotTheOwner => vec![
                        PickItemFailPacket {
                            reason: PickItemFailReason::NotTheOwner,
                        }
                        .encode(),
                    ],
                    DropCollectionResult::InventoryFull => vec![
                        PickItemFailPacket {
                            reason: PickItemFailReason::InventoryFull,
                        }
                        .encode(),
                    ],
                    DropCollectionResult::TooFarAway => vec![
                        PickItemFailPacket {
                            reason: PickItemFailReason::TooFarAway,
                        }
                        .encode(),
                    ],
                    DropCollectionResult::BitsCollected {
                        drop,
                        amount,
                        character,
                    } => {
                        session.viewed_drops.remove(&drop.id);
                        vec![
                            PickBitsPacket {
                                appearance_handler: character.general_handler,
                                value: amount,
                            }
                            .encode(),
                            UnloadDropsPacket { drop }.encode(),
                            LoadInventoryPacket {
                                inventory: character.inventory.clone(),
                                inventory_type: InventoryType::Inventory,
                            }
                            .encode(),
                        ]
                    }
                    DropCollectionResult::ItemCollected {
                        drop,
                        item_id,
                        amount,
                        character,
                    } => {
                        session.viewed_drops.remove(&drop.id);
                        vec![
                            PickItemPacket {
                                appearance_handler: character.general_handler,
                                item_id,
                                amount,
                            }
                            .encode(),
                            UnloadDropsPacket { drop }.encode(),
                            LoadInventoryPacket {
                                inventory: character.inventory.clone(),
                                inventory_type: InventoryType::Inventory,
                            }
                            .encode(),
                        ]
                    }
                };

                Ok(responses)
            }
            GameRequest::TamerMovimentation {
                ticks: _,
                handler,
                x,
                y,
                z,
            } => {
                let character_id = session.character_id.ok_or(GameFlowError::Unauthenticated)?;
                let character = self
                    .repository
                    .character_by_id(character_id)
                    .map_err(|error| GameFlowError::Storage(error.to_string()))?
                    .ok_or(GameFlowError::CharacterNotFound(character_id))?;

                // Handler >= 0x7FFF (32767) means tamer movement
                // Handler < 0x7FFF means partner (digimon) movement
                let is_tamer = handler >= 0x7FFF;
                // Condition bit 0x01 = Ride (ConditionEnum.Ride = 1)
                let is_riding = (character.current_condition & 0x01) != 0;

                if is_tamer {
                    self.repository
                        .update_character_position(character_id, x, y, z)
                        .map_err(|error| GameFlowError::Storage(error.to_string()))?;

                    // If riding, also move partner to same position
                    if is_riding {
                        self.repository
                            .update_partner_position(character_id, x, y, z)
                            .map_err(|error| GameFlowError::Storage(error.to_string()))?;
                    }
                } else {
                    self.repository
                        .update_partner_position(character_id, x, y, z)
                        .map_err(|error| GameFlowError::Storage(error.to_string()))?;
                }

                // Broadcast walk packets to other visible tamers
                if let Some(broadcast) = &self.broadcast {
                    if is_tamer {
                        let walk = TamerWalkPacket {
                            handler: character.general_handler,
                            x,
                            y,
                        };
                        let _ = broadcast.send_to_visible(
                            character.map_id,
                            character.channel,
                            character.id,
                            &walk.encode(),
                        );
                        // Also send back to self for client consistency
                        let _ = broadcast.send_to(character.id, &walk.encode());
                    }

                    if !is_tamer || is_riding {
                        let digimon_walk = DigimonWalkPacket {
                            handler: character.partner_handler,
                            x,
                            y,
                        };
                        let _ = broadcast.send_to_visible(
                            character.map_id,
                            character.channel,
                            character.id,
                            &digimon_walk.encode(),
                        );
                        let _ = broadcast.send_to(character.id, &digimon_walk.encode());
                    }
                }

                Ok(Vec::new())
            }
            GameRequest::WarpGate { portal_id } => {
                let character_id = session.character_id.ok_or(GameFlowError::Unauthenticated)?;
                let character = self
                    .repository
                    .character_by_id(character_id)
                    .map_err(|error| GameFlowError::Storage(error.to_string()))?
                    .ok_or(GameFlowError::CharacterNotFound(character_id))?;

                let portal = self
                    .repository
                    .portal_by_id(portal_id)
                    .map_err(|error| GameFlowError::Storage(error.to_string()))?
                    .ok_or(GameFlowError::Storage(format!(
                        "portal {portal_id} not found"
                    )))?;

                // Remove from current map presence
                self.portal_bridge
                    .remove_map_presence(character.map_id, character.channel, character.id)
                    .map_err(|error| GameFlowError::PortalBridge(error.to_string()))?;

                // Update position in database
                self.repository
                    .update_character_map(
                        character_id,
                        portal.destination_map_id,
                        portal.destination_x,
                        portal.destination_y,
                    )
                    .map_err(|error| GameFlowError::Storage(error.to_string()))?;

                // Update partner position too
                self.repository
                    .update_partner_position(
                        character_id,
                        portal.destination_x,
                        portal.destination_y,
                        0.0,
                    )
                    .map_err(|error| GameFlowError::Storage(error.to_string()))?;

                // Reset session state for new map
                session.registered_map_presence = false;
                session.viewed_characters.clear();
                session.viewed_mobs.clear();
                session.viewed_drops.clear();

                if portal.is_local {
                    // Local teleport: same map, send LocalMapSwapPacket (opcode 1711)
                    Ok(vec![
                        LocalMapSwapPacket {
                            tamer_handler: character.general_handler as i32,
                            partner_handler: character.partner_handler as i32,
                            x: portal.destination_x,
                            y: portal.destination_y,
                        }
                        .encode(),
                    ])
                } else {
                    // Cross-map teleport: send MapSwapPacket (opcode 1709) - client reconnects
                    Ok(vec![
                        MapSwapPacket {
                            address: self.game_server_address.clone(),
                            port: self.game_server_port,
                            map_id: portal.destination_map_id,
                            x: portal.destination_x,
                            y: portal.destination_y,
                        }
                        .encode(),
                    ])
                }
            }
            // DigiSummonSyncRequest — empty request, respond with empty sync response
            GameRequest::DigiSummonSyncRequest => {
                // The client expects a DigiSummonSyncResponse (opcode 3702) with result=0 and count=0.
                let mut writer = odmo_protocol::writer::PacketWriter::new(
                    odmo_protocol::opcode::game::DIGI_SUMMON_SYNC_RESPONSE,
                );
                writer.write_u8(0); // result = 0 (success)
                writer.write_u16(0); // product count = 0
                Ok(vec![writer.finalize()])
            }
            // ChannelInfo — client echoes back the channel info sent during ComplementarInformation.
            // No second response needed.
            GameRequest::ChannelInfo => Ok(vec![]),
            // Membership — client echoes back the membership packet sent during ComplementarInformation.
            // No second response needed.
            GameRequest::Membership => Ok(vec![]),
            // Emoticon — client sends an emote to display
            GameRequest::Emoticon {
                emoticon_type,
                value: _,
            } => {
                let character_id = session
                    .character_id
                    .ok_or(GameFlowError::Unauthenticated)?;
                let character = self
                    .repository
                    .character_by_id(character_id)
                    .map_err(|error| GameFlowError::Storage(error.to_string()))?
                    .ok_or(GameFlowError::CharacterNotFound(character_id))?;

                // Echo the emoticon back to the client.
                let mut writer = odmo_protocol::writer::PacketWriter::new(
                    odmo_protocol::opcode::game::EMOTICON,
                );
                writer.write_u32(character.general_handler);
                writer.write_i32(emoticon_type);
                Ok(vec![writer.finalize()])
            }
            // FriendlyInfo — client requests friendship info with a target
            GameRequest::FriendlyInfo { target_handler: _ } => {
                // Respond with empty friendship data — no friendship system implemented yet.
                let mut writer = odmo_protocol::writer::PacketWriter::new(
                    odmo_protocol::opcode::game::FRIENDLY_INFO,
                );
                writer.write_u32(0); // target handler
                writer.write_i32(0); // friendship level
                Ok(vec![writer.finalize()])
            }
            // FriendlyMark — client echoes back the relations packet sent during ComplementarInformation.
            // No second response needed.
            GameRequest::FriendlyMark => Ok(vec![]),
            // ExtraInventory — move item from extra inventory to regular inventory
            GameRequest::ExtraInventoryMove {
                category: _,
                extra_slot,
                inventory_slot,
            } => self.handle_extra_inventory_move(session, extra_slot, inventory_slot),
            // ExtraInventory — batch move all items from extra to regular
            GameRequest::ExtraInventoryBatchMove { category: _ } => {
                self.handle_extra_inventory_batch_move(session)
            }
            // ExtraInventory — sort items in extra inventory
            GameRequest::ExtraInventorySort { category: _ } => {
                self.handle_extra_inventory_sort(session)
            }
            // ExtraInventory — use/consume item from extra inventory
            GameRequest::ExtraInventoryUse {
                category: _,
                extra_slot,
            } => self.handle_extra_inventory_use(session, extra_slot),
            // ChatMessage — echo chat back to client (opcode 1006)
            GameRequest::ChatMessage { message } => {
                let character_id = session
                    .character_id
                    .ok_or(GameFlowError::Unauthenticated)?;
                let character = self
                    .repository
                    .character_by_id(character_id)
                    .map_err(|error| GameFlowError::Storage(error.to_string()))?
                    .ok_or(GameFlowError::CharacterNotFound(character_id))?;

                let mut writer = odmo_protocol::writer::PacketWriter::new(
                    odmo_protocol::opcode::game::CHAT_MESSAGE_RESPONSE,
                );
                writer.write_u8(0); // chatType: general
                writer.write_u8(1); // flag
                writer.write_u32(character.general_handler); // source handler
                writer.write_string(&message); // message
                writer.write_u8(0); // terminator
                Ok(vec![writer.finalize()])
            }
            // WhisperMessage — echo whisper back to client
            GameRequest::WhisperMessage {
                target_name,
                message,
            } => {
                let character_id = session
                    .character_id
                    .ok_or(GameFlowError::Unauthenticated)?;
                let character = self
                    .repository
                    .character_by_id(character_id)
                    .map_err(|error| GameFlowError::Storage(error.to_string()))?
                    .ok_or(GameFlowError::CharacterNotFound(character_id))?;

                let mut writer = odmo_protocol::writer::PacketWriter::new(
                    odmo_protocol::opcode::game::CHAT_MESSAGE_RESPONSE,
                );
                writer.write_u8(1); // chatType: whisper
                writer.write_u8(1); // flag
                writer.write_u8(0); // whisperResult: success
                writer.write_string(&character.name); // sender name
                writer.write_string(&target_name); // receiver name
                writer.write_string(&message); // message
                writer.write_u8(0); // terminator
                Ok(vec![writer.finalize()])
            }
            // ShoutMessage — echo shout back to client
            GameRequest::ShoutMessage { message } => {
                let character_id = session
                    .character_id
                    .ok_or(GameFlowError::Unauthenticated)?;
                let character = self
                    .repository
                    .character_by_id(character_id)
                    .map_err(|error| GameFlowError::Storage(error.to_string()))?
                    .ok_or(GameFlowError::CharacterNotFound(character_id))?;

                let mut writer = odmo_protocol::writer::PacketWriter::new(
                    odmo_protocol::opcode::game::CHAT_MESSAGE_RESPONSE,
                );
                writer.write_u8(2); // chatType: shout
                writer.write_u8(1); // flag
                writer.write_u32(character.general_handler); // source handler
                writer.write_string(&message); // message
                writer.write_u8(0); // terminator
                Ok(vec![writer.finalize()])
            }
            // MegaphoneMessage — echo megaphone back to client
            GameRequest::MegaphoneMessage {
                message,
                item_slot: _,
            } => {
                let character_id = session
                    .character_id
                    .ok_or(GameFlowError::Unauthenticated)?;
                let character = self
                    .repository
                    .character_by_id(character_id)
                    .map_err(|error| GameFlowError::Storage(error.to_string()))?
                    .ok_or(GameFlowError::CharacterNotFound(character_id))?;

                let mut writer = odmo_protocol::writer::PacketWriter::new(
                    odmo_protocol::opcode::game::CHAT_MESSAGE_RESPONSE,
                );
                writer.write_u8(3); // chatType: megaphone
                writer.write_u8(1); // flag
                writer.write_u32(character.general_handler); // source handler
                writer.write_string(&message); // message
                writer.write_u8(0); // terminator
                Ok(vec![writer.finalize()])
            }
            // TamerReaction — echo reaction back to client
            GameRequest::TamerReaction { reaction_type } => {
                let character_id = session
                    .character_id
                    .ok_or(GameFlowError::Unauthenticated)?;
                let character = self
                    .repository
                    .character_by_id(character_id)
                    .map_err(|error| GameFlowError::Storage(error.to_string()))?
                    .ok_or(GameFlowError::CharacterNotFound(character_id))?;

                let mut writer = odmo_protocol::writer::PacketWriter::new(
                    odmo_protocol::opcode::game::TAMER_REACTION,
                );
                writer.write_u32(character.general_handler); // tamer handler
                writer.write_i32(reaction_type); // reaction type
                Ok(vec![writer.finalize()])
            }
            // PartnerStop — echo partner stop to client
            GameRequest::PartnerStop { uid: _ } => {
                let character_id = session
                    .character_id
                    .ok_or(GameFlowError::Unauthenticated)?;
                let character = self
                    .repository
                    .character_by_id(character_id)
                    .map_err(|error| GameFlowError::Storage(error.to_string()))?
                    .ok_or(GameFlowError::CharacterNotFound(character_id))?;

                let mut writer = odmo_protocol::writer::PacketWriter::new(
                    odmo_protocol::opcode::game::PARTNER_STOP_RESPONSE,
                );
                writer.write_u32(character.partner_handler);
                Ok(vec![writer.finalize()])
            }
            // PartnerSwitch — stub: respond with failure (full implementation needs battle tag system)
            GameRequest::PartnerSwitch { slot: _ } => {
                let character_id = session
                    .character_id
                    .ok_or(GameFlowError::Unauthenticated)?;
                let _ = self
                    .repository
                    .character_by_id(character_id)
                    .map_err(|error| GameFlowError::Storage(error.to_string()))?
                    .ok_or(GameFlowError::CharacterNotFound(character_id))?;

                let mut writer = odmo_protocol::writer::PacketWriter::new(
                    odmo_protocol::opcode::game::PARTNER_SWITCH_RESPONSE,
                );
                writer.write_u8(0); // failure result
                Ok(vec![writer.finalize()])
            }
            // PartnerDelete — stub: respond with failure (needs secondary password validation)
            GameRequest::PartnerDelete { slot: _, validation: _ } => {
                let character_id = session
                    .character_id
                    .ok_or(GameFlowError::Unauthenticated)?;
                let _ = self
                    .repository
                    .character_by_id(character_id)
                    .map_err(|error| GameFlowError::Storage(error.to_string()))?
                    .ok_or(GameFlowError::CharacterNotFound(character_id))?;

                let mut writer = odmo_protocol::writer::PacketWriter::new(
                    odmo_protocol::opcode::game::PARTNER_DELETE_RESPONSE,
                );
                writer.write_u8(0); // failure result
                Ok(vec![writer.finalize()])
            }
            // EvolutionUnlock — stub: respond with failure (needs evolution data)
            GameRequest::EvolutionUnlock { evolution_type: _, inven_idx: _ } => {
                let mut writer = odmo_protocol::writer::PacketWriter::new(
                    odmo_protocol::opcode::game::EVOLUTION_UNLOCK_RESPONSE,
                );
                writer.write_u8(0); // failure
                Ok(vec![writer.finalize()])
            }
            // RideModeStart — stub: respond with failure
            GameRequest::RideModeStart { evolution_type: _, item_type: _ } => {
                let mut writer = odmo_protocol::writer::PacketWriter::new(
                    odmo_protocol::opcode::game::RIDE_MODE_START,
                );
                writer.write_u8(0); // failure
                Ok(vec![writer.finalize()])
            }
            // RideModeStop — stub: no response needed
            GameRequest::RideModeStop => Ok(vec![]),
            // DigimonChangeName — stub: respond with failure
            GameRequest::DigimonChangeName { inven_slot: _, new_name: _ } => {
                let mut writer = odmo_protocol::writer::PacketWriter::new(
                    odmo_protocol::opcode::game::DIGIMON_CHANGE_NAME,
                );
                writer.write_u8(0); // failure
                Ok(vec![writer.finalize()])
            }
            // HatchInsertEgg — stub: respond with failure
            GameRequest::HatchInsertEgg { vip: _, inven_slot: _, npc_idx: _ } => {
                let mut writer = odmo_protocol::writer::PacketWriter::new(
                    odmo_protocol::opcode::game::HATCH_FAILURE,
                );
                writer.write_u8(0); // failure
                Ok(vec![writer.finalize()])
            }
            // HatchIncrease — stub: respond with failure
            GameRequest::HatchIncrease { vip: _, npc_idx: _, data_level: _ } => {
                let mut writer = odmo_protocol::writer::PacketWriter::new(
                    odmo_protocol::opcode::game::HATCH_FAILURE,
                );
                writer.write_u8(0); // failure
                Ok(vec![writer.finalize()])
            }
            // HatchFinish — stub: respond with failure
            GameRequest::HatchFinish { vip: _, portable_pos: _, name: _, npc_idx: _ } => {
                let mut writer = odmo_protocol::writer::PacketWriter::new(
                    odmo_protocol::opcode::game::HATCH_FAILURE,
                );
                writer.write_u8(0); // failure
                Ok(vec![writer.finalize()])
            }
            // HatchRemoveEgg — stub: no response needed
            GameRequest::HatchRemoveEgg { vip: _, npc_idx: _ } => Ok(vec![]),
            // HatchBackupInsert — stub: no response needed
            GameRequest::HatchBackupInsert { vip: _, inven_slot: _, npc_idx: _ } => Ok(vec![]),
            // HatchBackupCancel — stub: no response needed
            GameRequest::HatchBackupCancel { vip: _, npc_idx: _ } => Ok(vec![]),
            // IncubatorClose — stub: no response needed
            GameRequest::IncubatorClose => Ok(vec![]),
            // DigimonArchiveMove — stub: no response needed
            GameRequest::DigimonArchiveMove { vip: _, slot1: _, slot2: _, npc_type: _ } => Ok(vec![]),
            // DigimonArchiveList — stub: no response needed
            GameRequest::DigimonArchiveList { vip: _, inven_idx: _, npc_type: _ } => Ok(vec![]),
            // DigimonArchiveSwap — stub: no response needed
            GameRequest::DigimonArchiveSwap { npc_idx: _, archive_type: _, src_arr: _, dst_arr: _ } => Ok(vec![]),
            // InventorySort — stub: no response needed
            GameRequest::InventorySort { sort_type: _ } => Ok(vec![]),
            // ItemIdentify — stub: respond with failure
            GameRequest::ItemIdentify { item_slot: _ } => {
                let mut writer = odmo_protocol::writer::PacketWriter::new(
                    odmo_protocol::opcode::game::ITEM_IDENTIFY,
                );
                writer.write_u8(0); // failure
                Ok(vec![writer.finalize()])
            }
            // ItemCraft — stub: respond with failure
            GameRequest::ItemCraft { recipe_slot: _ } => {
                let mut writer = odmo_protocol::writer::PacketWriter::new(
                    odmo_protocol::opcode::game::ITEM_CRAFT,
                );
                writer.write_u8(0); // failure
                Ok(vec![writer.finalize()])
            }
            // ItemReroll — stub: respond with failure
            GameRequest::ItemReroll { item_slot: _ } => {
                let mut writer = odmo_protocol::writer::PacketWriter::new(
                    odmo_protocol::opcode::game::ITEM_REROLL,
                );
                writer.write_u8(0); // failure
                Ok(vec![writer.finalize()])
            }
            // ItemSocketIn — stub: respond with failure
            GameRequest::ItemSocketIn { item_slot: _, socket_slot: _, chip_item_id: _ } => {
                let mut writer = odmo_protocol::writer::PacketWriter::new(
                    odmo_protocol::opcode::game::ITEM_SOCKET_IN,
                );
                writer.write_u8(0); // failure
                Ok(vec![writer.finalize()])
            }
            // ItemSocketOut — stub: respond with failure
            GameRequest::ItemSocketOut { item_slot: _, socket_slot: _ } => {
                let mut writer = odmo_protocol::writer::PacketWriter::new(
                    odmo_protocol::opcode::game::ITEM_SOCKET_OUT,
                );
                writer.write_u8(0); // failure
                Ok(vec![writer.finalize()])
            }
            // ItemSocketIdentify — stub: respond with failure
            GameRequest::ItemSocketIdentify { item_slot: _ } => {
                let mut writer = odmo_protocol::writer::PacketWriter::new(
                    odmo_protocol::opcode::game::ITEM_SOCKET_IDENTIFY,
                );
                writer.write_u8(0); // failure
                Ok(vec![writer.finalize()])
            }
            // ItemReturn — stub: no response needed
            GameRequest::ItemReturn { item_slot: _ } => Ok(vec![]),
            // ItemScan — stub: no response needed
            GameRequest::ItemScan { item_slot: _ } => Ok(vec![]),
            // LoadGiftStorage — stub: respond with empty storage
            GameRequest::LoadGiftStorage => {
                let mut writer = odmo_protocol::writer::PacketWriter::new(
                    odmo_protocol::opcode::game::LOAD_GIFT_STORAGE,
                );
                writer.write_u16(0); // count = 0
                Ok(vec![writer.finalize()])
            }
            // GiftStorageRetrieve — stub: respond with failure
            GameRequest::GiftStorageRetrieve { item_slot: _ } => {
                let mut writer = odmo_protocol::writer::PacketWriter::new(
                    odmo_protocol::opcode::game::GIFT_STORAGE_RETRIEVE,
                );
                writer.write_u8(0); // failure
                Ok(vec![writer.finalize()])
            }
            // LoadRewardStorage — stub: respond with empty storage
            GameRequest::LoadRewardStorage => {
                let mut writer = odmo_protocol::writer::PacketWriter::new(
                    odmo_protocol::opcode::game::LOAD_REWARD_STORAGE,
                );
                writer.write_u16(0); // count = 0
                Ok(vec![writer.finalize()])
            }
            // RecompenseGain — stub: respond with failure
            GameRequest::RecompenseGain { reward_id: _ } => {
                let mut writer = odmo_protocol::writer::PacketWriter::new(
                    odmo_protocol::opcode::game::RECOMPENSE_GAIN,
                );
                writer.write_u8(0); // failure
                Ok(vec![writer.finalize()])
            }
            // TamerShopOpen — stub: no response needed
            GameRequest::TamerShopOpen => Ok(vec![]),
            // TamerShopClose — stub: no response needed
            GameRequest::TamerShopClose => Ok(vec![]),
            // TamerShopBuy — stub: respond with failure
            GameRequest::TamerShopBuy { item_id: _, amount: _ } => {
                let mut writer = odmo_protocol::writer::PacketWriter::new(
                    odmo_protocol::opcode::game::TAMER_SHOP_BUY,
                );
                writer.write_u8(0); // failure
                Ok(vec![writer.finalize()])
            }
            // ConsignedShopOpen — stub: respond with empty
            GameRequest::ConsignedShopOpen => Ok(vec![]),
            // ConsignedShopView — stub: respond with empty
            GameRequest::ConsignedShopView { shop_id: _ } => Ok(vec![]),
            // ConsignedShopPurchase — stub: respond with failure
            GameRequest::ConsignedShopPurchase { item_id: _, amount: _ } => Ok(vec![]),
            // ConsignedShopRetrieve — stub: respond with failure
            GameRequest::ConsignedShopRetrieve { item_slot: _ } => Ok(vec![]),
            // CashShopOpen — stub: respond with empty
            GameRequest::CashShopOpen => Ok(vec![]),
            // CashShopBuy — stub: respond with failure
            GameRequest::CashShopBuy { amount: _, total_price: _, order_id: _, product_ids: _ } => Ok(vec![]),
            // CashShopReload — stub: respond with empty
            GameRequest::CashShopReload => Ok(vec![]),
            // QuestAvailableList — stub: respond with empty list
            GameRequest::QuestAvailableList => {
                let mut writer = odmo_protocol::writer::PacketWriter::new(
                    odmo_protocol::opcode::game::QUEST_AVAILABLE_LIST,
                );
                writer.write_u16(0); // count = 0
                Ok(vec![writer.finalize()])
            }
            // QuestAccept — stub: respond with failure
            GameRequest::QuestAccept { quest_id: _ } => Ok(vec![]),
            // QuestDeliver — stub: respond with failure
            GameRequest::QuestDeliver { quest_id: _ } => Ok(vec![]),
            // QuestGiveUp — stub: respond with failure
            GameRequest::QuestGiveUp { quest_id: _ } => Ok(vec![]),
            // QuestUpdate — stub: respond with failure
            GameRequest::QuestUpdate { quest_id: _, progress: _ } => Ok(vec![]),
            // DieConfirm — stub: no response needed
            GameRequest::DieConfirm => Ok(vec![]),
            // RemoveBuff — stub: no response needed
            GameRequest::RemoveBuff { buff_id: _ } => Ok(vec![]),
            // DamageSkinChange — stub: no response needed
            GameRequest::DamageSkinChange { skin_id: _ } => Ok(vec![]),
            // SealOpen — stub: no response needed
            GameRequest::SealOpen { seal_idx: _ } => Ok(vec![]),
            // SealClose — stub: no response needed
            GameRequest::SealClose { seal_idx: _ } => Ok(vec![]),
            // SealSetLeader — stub: no response needed
            GameRequest::SealSetLeader { card_code: _ } => Ok(vec![]),
            // SealRemoveLeader — stub: no response needed
            GameRequest::SealRemoveLeader => Ok(vec![]),
            // SealSetFavorite — stub: no response needed
            GameRequest::SealSetFavorite { card_code: _, bookmark: _ } => Ok(vec![]),
            // EncyclopediaLoad — stub: respond with empty
            GameRequest::EncyclopediaLoad => {
                let mut writer = odmo_protocol::writer::PacketWriter::new(
                    odmo_protocol::opcode::game::ENCYCLOPEDIA_LOAD,
                );
                writer.write_u8(0); // count = 0
                Ok(vec![writer.finalize()])
            }
            // EncyclopediaGetReward — stub: no response needed
            GameRequest::EncyclopediaGetReward { digimon_id: _ } => Ok(vec![]),
            // EncyclopediaDeckBuff — stub: no response needed
            GameRequest::EncyclopediaDeckBuff { deck_idx: _ } => Ok(vec![]),
            // ArenaDailyPoints — stub: no response needed
            GameRequest::ArenaDailyPoints { item_slot: _, points: _, item_id: _ } => Ok(vec![]),
            // ArenaDailyRanking — stub: no response needed
            GameRequest::ArenaDailyRanking => Ok(vec![]),
            // ArenaRankingAll — stub: no response needed
            GameRequest::ArenaRankingAll { ranking_type: _ } => Ok(vec![]),
            // ArenaRequestRank — stub: no response needed
            GameRequest::ArenaRequestRank { ranking_type: _ } => Ok(vec![]),
            // ArenaRequestOldRank — stub: no response needed
            GameRequest::ArenaRequestOldRank { ranking_type: _ } => Ok(vec![]),
            // DungeonNextStage — stub: no response needed
            GameRequest::DungeonNextStage => Ok(vec![]),
            // DungeonSurrender — stub: no response needed
            GameRequest::DungeonSurrender => Ok(vec![]),
            // BurningEvent — stub: no response needed
            GameRequest::BurningEvent => Ok(vec![]),
            // DailyCheckEvent — stub: no response needed
            GameRequest::DailyCheckEvent => Ok(vec![]),
            // DailyCheckEventRequest — stub: no response needed
            GameRequest::DailyCheckEventRequest { event_no: _ } => Ok(vec![]),
            // JoinEventQueue — stub: no response needed
            GameRequest::JoinEventQueue { event_id: _ } => Ok(vec![]),
            // RegionUnlock — stub: no response needed
            GameRequest::RegionUnlock { region_idx: _ } => Ok(vec![]),
            // SetTitle — stub: no response needed
            GameRequest::SetTitle { title_id: _ } => Ok(vec![]),
            // ChangeTamerModel — stub: no response needed
            GameRequest::ChangeTamerModel { model_id: _ } => Ok(vec![]),
            // TamerNameChange — stub: no response needed
            GameRequest::TamerNameChange { new_name: _ } => Ok(vec![]),
            // RareMachineOpen — stub: no response needed
            GameRequest::RareMachineOpen { npc_idx: _ } => Ok(vec![]),
            // RareMachineRun — stub: no response needed
            GameRequest::RareMachineRun { npc_idx: _, inven_idx: _, reset_count: _ } => Ok(vec![]),
            // Party stubs
            GameRequest::PartyInvite { target_handler: _ } => Ok(vec![]),
            GameRequest::PartyInviteResponse { inviter_handler: _, accepted: _ } => Ok(vec![]),
            GameRequest::PartyChat { message: _ } => Ok(vec![]),
            GameRequest::PartyKick { member_slot: _ } => Ok(vec![]),
            GameRequest::PartyLeave => Ok(vec![]),
            GameRequest::PartyChangeMaster { new_leader_slot: _ } => Ok(vec![]),
            GameRequest::PartyChangeLoot { loot_type: _ } => Ok(vec![]),
            GameRequest::PartyDismiss => Ok(vec![]),
            // Guild stubs
            GameRequest::GuildCreate { guild_name: _ } => Ok(vec![]),
            GameRequest::GuildDelete => Ok(vec![]),
            GameRequest::GuildInvite { target_name: _ } => Ok(vec![]),
            GameRequest::GuildInviteAccept { guild_id: _ } => Ok(vec![]),
            GameRequest::GuildInviteDeny { guild_id: _ } => Ok(vec![]),
            GameRequest::GuildKick { member_id: _ } => Ok(vec![]),
            GameRequest::GuildLeave => Ok(vec![]),
            GameRequest::GuildMessage { message: _ } => Ok(vec![]),
            GameRequest::GuildNotice { notice: _ } => Ok(vec![]),
            GameRequest::GuildHistory => Ok(vec![]),
            GameRequest::GuildSetTitle { member_id: _, title: _ } => Ok(vec![]),
            // Trade stubs
            GameRequest::TradeRequest { target_handler: _ } => Ok(vec![]),
            GameRequest::TradeAccept { accepter_handler: _ } => Ok(vec![]),
            GameRequest::TradeCancel => Ok(vec![]),
            GameRequest::TradeAddItem { item_slot: _, trade_slot: _ } => Ok(vec![]),
            GameRequest::TradeRemoveItem { trade_slot: _ } => Ok(vec![]),
            GameRequest::TradeAddMoney { amount: _ } => Ok(vec![]),
            GameRequest::TradeConfirm => Ok(vec![]),
            GameRequest::TradeLock => Ok(vec![]),
            GameRequest::TradeUnlock => Ok(vec![]),
            // Season Pass stubs
            GameRequest::SeasonPassDetails => Ok(vec![]),
            GameRequest::SeasonPassPurchaseExp { purchase_count: _ } => Ok(vec![]),
            GameRequest::SeasonPassMissionReward { mission_id: _ } => Ok(vec![]),
            GameRequest::SeasonPassSeasonReward { level: _ } => Ok(vec![]),
            // Channel stubs
            GameRequest::ChangeChannel { channel: _ } => Ok(vec![]),
            GameRequest::ChannelSwitchConfirm => Ok(vec![]),
            // Shop stubs
            GameRequest::TamerShopList => Ok(vec![]),
            GameRequest::ConsignedWarehouse => Ok(vec![]),
            GameRequest::ConsignedWarehouseRetrieve { item_slot: _ } => Ok(vec![]),
            GameRequest::CashShopBuyHistory => Ok(vec![]),
            // Friend stubs
            GameRequest::AddFriend { friend_name: _ } => Ok(vec![]),
            GameRequest::FriendList => Ok(vec![]),
            // Guild authority stubs
            GameRequest::GuildAuthorityMaster { member_id: _ } => Ok(vec![]),
            GameRequest::GuildAuthoritySubMaster { member_id: _ } => Ok(vec![]),
            GameRequest::GuildAuthorityMember { member_id: _ } => Ok(vec![]),
            GameRequest::GuildAuthorityNewMember { member_id: _ } => Ok(vec![]),
            GameRequest::GuildAuthorityDats { member_id: _ } => Ok(vec![]),
            // Hatch/Digimon stubs
            GameRequest::HatchSpiritEvolution { model_id: _, name: _, npc_id: _ } => Ok(vec![]),
            GameRequest::DigiSummonPurchase { npc_idx: _ } => Ok(vec![]),
            // Warehouse stubs
            GameRequest::LoadAccountWarehouse => Ok(vec![]),
            GameRequest::RetrieveAccountWarehouse { item_slot: _ } => Ok(vec![]),
            // Extra inventory stubs
            GameRequest::ExtraInventoryCategoryRefresh { category: _ } => Ok(vec![]),
            GameRequest::ExtraInventoryMove { category: _, extra_slot: _, inventory_slot: _ } => Ok(vec![]),
            GameRequest::ExtraInventorySort { category: _ } => Ok(vec![]),
            // Party extra stubs
            GameRequest::PartyConfigChange { loot_type: _ } => Ok(vec![]),
            GameRequest::PartyMemberDisconnect => Ok(vec![]),
            // Combat/Tamer stubs
            GameRequest::MonsterRespawnTimer => Ok(vec![]),
            GameRequest::JumpBooster => Ok(vec![]),
            GameRequest::SkillLevelUp { uid: _, evo_unit_idx: _, skill_idx: _ } => Ok(vec![]),
            GameRequest::TamerChargeXCrystal => Ok(vec![]),
            GameRequest::TamerConsumeXCrystal { amount: _ } => Ok(vec![]),
            GameRequest::TamerSummon { target_name: _ } => Ok(vec![]),
            GameRequest::TamerSkillRequest { skill_idx: _, target_uid: _ } => Ok(vec![]),
            GameRequest::TranscendenceReceiveExp => Ok(vec![]),
            GameRequest::TranscendenceSuccess => Ok(vec![]),
            GameRequest::TimeChargeResult { charge_type: _ } => Ok(vec![]),
            GameRequest::WarpGateDungeon => Ok(vec![]),
            GameRequest::SpiritCraft { model_id: _, name: _, npc_id: _ } => Ok(vec![])
        }?;

        responses.extend(request_responses);
        Ok(responses)
    }

    pub fn handle_disconnect(&self, session: &GameSession) -> Result<(), GameFlowError> {
        let Some(character_id) = session.character_id else {
            return Ok(());
        };
        if !session.registered_map_presence {
            return Ok(());
        }

        let character = self
            .repository
            .character_by_id(character_id)
            .map_err(|error| GameFlowError::Storage(error.to_string()))?
            .ok_or(GameFlowError::CharacterNotFound(character_id))?;

        self.portal_bridge
            .remove_map_presence(character.map_id, character.channel, character.id)
            .map_err(|error| GameFlowError::PortalBridge(error.to_string()))?;

        Ok(())
    }

    fn handle_extra_inventory_move(
        &self,
        session: &GameSession,
        extra_slot: u16,
        inventory_slot: u16,
    ) -> Result<Vec<Vec<u8>>, GameFlowError> {
        let character_id = session.character_id.ok_or(GameFlowError::Unauthenticated)?;
        let mut character = self
            .repository
            .character_by_id(character_id)
            .map_err(|error| GameFlowError::Storage(error.to_string()))?
            .ok_or(GameFlowError::CharacterNotFound(character_id))?;

        let ext_idx = extra_slot as usize;
        let inv_idx = inventory_slot as usize;

        if ext_idx >= character.extra_inventory.items.len()
            || inv_idx >= character.inventory.items.len()
        {
            return Ok(vec![LoadInventoryPacket {
                inventory: character.extra_inventory,
                inventory_type: InventoryType::ExtraInventory,
            }
            .encode()]);
        }

        let source_item = character.extra_inventory.items[ext_idx].clone();
        if source_item.item_id <= 0 || source_item.amount <= 0 {
            return Ok(vec![LoadInventoryPacket {
                inventory: character.extra_inventory,
                inventory_type: InventoryType::ExtraInventory,
            }
            .encode()]);
        }

        let dest_item = character.inventory.items[inv_idx].clone();
        if dest_item.item_id > 0 && dest_item.item_id == source_item.item_id {
            let mut merged = dest_item.clone();
            merged.amount = merged.amount.saturating_add(source_item.amount);
            merged.sync_record();
            character.inventory.items[inv_idx] = merged;
            character.extra_inventory.items[ext_idx] = ItemRecord::default();
        } else if dest_item.item_id > 0 {
            character.inventory.items[inv_idx] = source_item;
            character.extra_inventory.items[ext_idx] = dest_item;
        } else {
            character.inventory.items[inv_idx] = source_item;
            character.extra_inventory.items[ext_idx] = ItemRecord::default();
        }

        self.repository
            .update_extra_inventory(character_id, character.extra_inventory.clone())
            .map_err(|error| GameFlowError::Storage(error.to_string()))?;
        self.repository
            .update_inventory(character_id, character.inventory.clone())
            .map_err(|error| GameFlowError::Storage(error.to_string()))?;

        let updated = self
            .repository
            .character_by_id(character_id)
            .map_err(|error| GameFlowError::Storage(error.to_string()))?
            .ok_or(GameFlowError::CharacterNotFound(character_id))?;

        Ok(vec![
            LoadInventoryPacket {
                inventory: updated.inventory,
                inventory_type: InventoryType::Inventory,
            }
            .encode(),
            LoadInventoryPacket {
                inventory: updated.extra_inventory,
                inventory_type: InventoryType::ExtraInventory,
            }
            .encode(),
        ])
    }

    fn handle_extra_inventory_batch_move(
        &self,
        session: &GameSession,
    ) -> Result<Vec<Vec<u8>>, GameFlowError> {
        let character_id = session.character_id.ok_or(GameFlowError::Unauthenticated)?;
        let mut character = self
            .repository
            .character_by_id(character_id)
            .map_err(|error| GameFlowError::Storage(error.to_string()))?
            .ok_or(GameFlowError::CharacterNotFound(character_id))?;

        for i in 0..character.extra_inventory.items.len() {
            let item = character.extra_inventory.items[i].clone();
            if item.item_id <= 0 || item.amount <= 0 {
                continue;
            }

            let mut placed = false;
            for j in 0..character.inventory.items.len() {
                let existing = &character.inventory.items[j];
                if existing.item_id == item.item_id && existing.amount > 0 {
                    let mut merged = existing.clone();
                    merged.amount = merged.amount.saturating_add(item.amount);
                    merged.sync_record();
                    character.inventory.items[j] = merged;
                    character.extra_inventory.items[i] = ItemRecord::default();
                    placed = true;
                    break;
                }
            }

            if !placed {
                for j in 0..character.inventory.items.len() {
                    if character.inventory.items[j].item_id <= 0
                        || character.inventory.items[j].amount <= 0
                    {
                        character.inventory.items[j] = item;
                        character.extra_inventory.items[i] = ItemRecord::default();
                        break;
                    }
                }
            }
        }

        self.repository
            .update_extra_inventory(character_id, character.extra_inventory.clone())
            .map_err(|error| GameFlowError::Storage(error.to_string()))?;
        self.repository
            .update_inventory(character_id, character.inventory.clone())
            .map_err(|error| GameFlowError::Storage(error.to_string()))?;

        let updated = self
            .repository
            .character_by_id(character_id)
            .map_err(|error| GameFlowError::Storage(error.to_string()))?
            .ok_or(GameFlowError::CharacterNotFound(character_id))?;

        Ok(vec![LoadInventoryPacket {
            inventory: updated.extra_inventory,
            inventory_type: InventoryType::ExtraInventory,
        }
        .encode()])
    }

    fn handle_extra_inventory_sort(
        &self,
        session: &GameSession,
    ) -> Result<Vec<Vec<u8>>, GameFlowError> {
        let character_id = session.character_id.ok_or(GameFlowError::Unauthenticated)?;
        let mut character = self
            .repository
            .character_by_id(character_id)
            .map_err(|error| GameFlowError::Storage(error.to_string()))?
            .ok_or(GameFlowError::CharacterNotFound(character_id))?;

        let mut items: Vec<ItemRecord> = character
            .extra_inventory
            .items
            .iter()
            .filter(|item| item.item_id > 0 && item.amount > 0)
            .cloned()
            .collect();
        items.sort_by_key(|item| item.item_id);

        let empty_count = character.extra_inventory.items.len() - items.len();
        items.resize(items.len() + empty_count, ItemRecord::default());

        character.extra_inventory.items = items;

        self.repository
            .update_extra_inventory(character_id, character.extra_inventory.clone())
            .map_err(|error| GameFlowError::Storage(error.to_string()))?;

        Ok(vec![LoadInventoryPacket {
            inventory: character.extra_inventory,
            inventory_type: InventoryType::ExtraInventory,
        }
        .encode()])
    }

    fn handle_extra_inventory_use(
        &self,
        session: &GameSession,
        extra_slot: u16,
    ) -> Result<Vec<Vec<u8>>, GameFlowError> {
        let character_id = session.character_id.ok_or(GameFlowError::Unauthenticated)?;
        let mut character = self
            .repository
            .character_by_id(character_id)
            .map_err(|error| GameFlowError::Storage(error.to_string()))?
            .ok_or(GameFlowError::CharacterNotFound(character_id))?;

        let ext_idx = extra_slot as usize;
        if ext_idx >= character.extra_inventory.items.len() {
            return Ok(vec![LoadInventoryPacket {
                inventory: character.extra_inventory,
                inventory_type: InventoryType::ExtraInventory,
            }
            .encode()]);
        }

        let item = &character.extra_inventory.items[ext_idx];
        if item.item_id <= 0 || item.amount <= 0 {
            return Ok(vec![LoadInventoryPacket {
                inventory: character.extra_inventory,
                inventory_type: InventoryType::ExtraInventory,
            }
            .encode()]);
        }

        let new_amount = item.amount - 1;
        if new_amount <= 0 {
            character.extra_inventory.items[ext_idx] = ItemRecord::default();
        } else {
            let mut updated = item.clone();
            updated.amount = new_amount;
            updated.sync_record();
            character.extra_inventory.items[ext_idx] = updated;
        }

        self.repository
            .update_extra_inventory(character_id, character.extra_inventory.clone())
            .map_err(|error| GameFlowError::Storage(error.to_string()))?;

        let updated = self
            .repository
            .character_by_id(character_id)
            .map_err(|error| GameFlowError::Storage(error.to_string()))?
            .ok_or(GameFlowError::CharacterNotFound(character_id))?;

        Ok(vec![
            LoadInventoryPacket {
                inventory: updated.extra_inventory,
                inventory_type: InventoryType::ExtraInventory,
            }
            .encode(),
        ])
    }

    fn drain_social_notifications(&self, session: &GameSession) -> anyhow::Result<Vec<Vec<u8>>> {
        let Some(character_id) = session.character_id else {
            return Ok(Vec::new());
        };

        let notifications = self
            .portal_bridge
            .consume_social_notifications(character_id)?;
        Ok(notifications
            .into_iter()
            .map(|notification| match notification.kind {
                SocialNotificationKind::FriendConnect { name } => {
                    FriendConnectPacket { name }.encode()
                }
                SocialNotificationKind::MapTamerSpawn { .. }
                | SocialNotificationKind::MapTamerUnload { .. } => Vec::new(),
            })
            .filter(|packet| !packet.is_empty())
            .collect())
    }

    fn announce_friend_connect(
        &self,
        character: &odmo_types::CharacterSummary,
    ) -> anyhow::Result<()> {
        for target_character_id in &character.friended_character_ids {
            self.portal_bridge.enqueue_social_notification(
                *target_character_id,
                SocialNotification {
                    kind: SocialNotificationKind::FriendConnect {
                        name: character.name.clone(),
                    },
                },
            )?;
        }

        Ok(())
    }

    fn register_map_presence(
        &self,
        session: &mut GameSession,
        character: &odmo_types::CharacterSummary,
    ) -> anyhow::Result<Vec<Vec<u8>>> {
        if session.registered_map_presence {
            return Ok(Vec::new());
        }

        let existing = self
            .portal_bridge
            .load_map_presence(character.map_id, character.channel)?;
        let mut responses = Vec::new();
        for occupant in existing
            .iter()
            .filter(|occupant| occupant.id != character.id)
            .filter(|occupant| can_see_each_other(character, occupant))
        {
            responses.push(
                LoadTamerPacket {
                    character: occupant.clone(),
                }
                .encode(),
            );
            responses.push(
                LoadBuffsPacket {
                    character: occupant.clone(),
                }
                .encode(),
            );
            session
                .viewed_characters
                .insert(occupant.id, occupant.clone());
        }

        self.portal_bridge.upsert_map_presence(character)?;
        session.registered_map_presence = true;

        // Update broadcast sink with the character's map location
        if let Some(broadcast) = &self.broadcast {
            broadcast.update_location(character.id, character.map_id, character.channel);
        }

        Ok(responses)
    }

    fn reconcile_map_visibility(&self, session: &mut GameSession) -> anyhow::Result<Vec<Vec<u8>>> {
        let Some(character_id) = session.character_id else {
            return Ok(Vec::new());
        };
        if !session.registered_map_presence {
            return Ok(Vec::new());
        }

        let character = self
            .repository
            .character_by_id(character_id)?
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "character {character_id} not found during visibility reconciliation"
                )
            })?;
        let occupants = self
            .portal_bridge
            .load_map_presence(character.map_id, character.channel)?;

        let mut responses = Vec::new();
        let mut next_viewed_characters = HashMap::new();

        for occupant in occupants
            .iter()
            .filter(|occupant| occupant.id != character.id)
            .filter(|occupant| {
                map_distance(character.x, character.y, occupant.x, occupant.y)
                    <= START_TO_SEE_DISTANCE
            })
        {
            next_viewed_characters.insert(occupant.id, occupant.clone());
            if !session.viewed_characters.contains_key(&occupant.id) {
                responses.push(
                    LoadTamerPacket {
                        character: occupant.clone(),
                    }
                    .encode(),
                );
                responses.push(
                    LoadBuffsPacket {
                        character: occupant.clone(),
                    }
                    .encode(),
                );
            }
        }

        let previously_visible: Vec<u64> = session.viewed_characters.keys().copied().collect();
        for viewed_id in previously_visible {
            let should_hide = if let Some(occupant) =
                occupants.iter().find(|occupant| occupant.id == viewed_id)
            {
                map_distance(character.x, character.y, occupant.x, occupant.y)
                    >= STOP_SEEING_DISTANCE
            } else {
                true
            };

            if should_hide {
                let character_to_unload = session
                    .viewed_characters
                    .get(&viewed_id)
                    .cloned()
                    .or_else(|| {
                        occupants
                            .iter()
                            .find(|occupant| occupant.id == viewed_id)
                            .cloned()
                    })
                    .unwrap_or_else(|| {
                        let mut placeholder = odmo_types::CharacterSummary {
                            id: viewed_id,
                            ..odmo_types::CharacterSummary::default()
                        };
                        placeholder.general_handler = viewed_id.min(u32::MAX as u64) as u32;
                        placeholder.partner_handler =
                            viewed_id.saturating_add(10_000).min(u32::MAX as u64) as u32;
                        placeholder
                    });
                responses.push(
                    UnloadTamerPacket {
                        character: character_to_unload,
                    }
                    .encode(),
                );
            }
        }

        session.viewed_characters = next_viewed_characters;
        Ok(responses)
    }

    fn reconcile_mob_visibility(&self, session: &mut GameSession) -> anyhow::Result<Vec<Vec<u8>>> {
        let Some(character_id) = session.character_id else {
            return Ok(Vec::new());
        };
        if !session.registered_map_presence {
            return Ok(Vec::new());
        }

        let _ = self.repository.character_by_id(character_id)?.ok_or_else(|| {
            anyhow::anyhow!("character {character_id} not found during mob reconciliation")
        })?;

        let character = self.repository.character_by_id(character_id)?.ok_or_else(|| {
            anyhow::anyhow!("character {character_id} not found during mob reconciliation")
        })?;
        let current = self
            .repository
            .mobs_by_map(character.map_id, character.channel)?;
        let mut responses = Vec::new();
        let mut next_viewed_mobs = HashMap::new();

        for mob in current {
            if map_distance(character.x, character.y, mob.x, mob.y) > START_TO_SEE_DISTANCE {
                continue;
            }

            if !session.viewed_mobs.contains_key(&mob.id) {
                responses.push(LoadMobsPacket { mob: mob.clone() }.encode());
                if !mob.active_debuffs.is_empty() {
                    responses.push(LoadMobBuffsPacket { mob: mob.clone() }.encode());
                }
            }

            next_viewed_mobs.insert(mob.id, mob);
        }

        for mob in session.viewed_mobs.values() {
            if next_viewed_mobs.contains_key(&mob.id) {
                continue;
            }

            responses.push(UnloadMobsPacket { mob: mob.clone() }.encode());
        }

        session.viewed_mobs = next_viewed_mobs;
        Ok(responses)
    }

    fn reconcile_drop_visibility(&self, session: &mut GameSession) -> anyhow::Result<Vec<Vec<u8>>> {
        let Some(character_id) = session.character_id else {
            return Ok(Vec::new());
        };
        if !session.registered_map_presence {
            return Ok(Vec::new());
        }

        let _ = self.repository.character_by_id(character_id)?.ok_or_else(|| {
            anyhow::anyhow!("character {character_id} not found during drop reconciliation")
        })?;

        let character = self.repository.character_by_id(character_id)?.ok_or_else(|| {
            anyhow::anyhow!("character {character_id} not found during drop reconciliation")
        })?;
        let current = self
            .repository
            .drops_by_map(character.map_id, character.channel)?;
        let mut responses = Vec::new();
        let mut next_viewed_drops = HashMap::new();

        for drop in current {
            if map_distance(character.x, character.y, drop.x, drop.y) > START_TO_SEE_DISTANCE {
                continue;
            }

            if !session.viewed_drops.contains_key(&drop.id) {
                let viewer_handler = if character.general_handler == 0 {
                    character.id.min(u32::MAX as u64) as u32
                } else {
                    character.general_handler
                };
                responses.push(
                    LoadDropsPacket {
                        drop: drop.clone(),
                        viewer_handler,
                    }
                    .encode(),
                );
            }

            next_viewed_drops.insert(drop.id, drop);
        }

        for drop in session.viewed_drops.values() {
            if next_viewed_drops.contains_key(&drop.id) {
                continue;
            }

            responses.push(UnloadDropsPacket { drop: drop.clone() }.encode());
        }

        session.viewed_drops = next_viewed_drops;
        Ok(responses)
    }
}

#[derive(Debug)]
pub struct GameSessionFactory {
    next_seed: AtomicI16,
}

impl GameSessionFactory {
    pub fn new() -> Self {
        Self {
            next_seed: AtomicI16::new(3_000),
        }
    }

    pub fn create(&self) -> GameSession {
        let seed = self.next_seed.fetch_add(1, Ordering::Relaxed);
        GameSession::new(seed)
    }
}

#[derive(Debug, Error)]
pub enum GameFlowError {
    #[error("missing game session ticket for account {0}")]
    MissingSessionTicket(AccountId),
    #[error("character {0} not found for bootstrap")]
    CharacterNotFound(u64),
    #[error("game request requires initialized session")]
    Unauthenticated,
    #[error("portal bridge error: {0}")]
    PortalBridge(String),
    #[error("storage error: {0}")]
    Storage(String),
}

fn unix_timestamp() -> u32 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as u32
}

fn map_distance(xa: i32, ya: i32, xb: i32, yb: i32) -> i64 {
    let distance_x = (xb as i64 - xa as i64).pow(2);
    let distance_y = (yb as i64 - ya as i64).pow(2);
    ((distance_x + distance_y) as f64).sqrt() as i64
}

fn can_see_each_other(
    left: &odmo_types::CharacterSummary,
    right: &odmo_types::CharacterSummary,
) -> bool {
    map_distance(left.x, left.y, right.x, right.y) <= START_TO_SEE_DISTANCE
}

fn current_unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn apply_runtime_drop_state(mut drop: odmo_types::DropSummary) -> odmo_types::DropSummary {
    let now = current_unix_timestamp();
    if drop.expires_at_unix > 0 && now >= drop.expires_at_unix {
        drop.collected = true;
    }
    if !drop.collected && drop.owner_expires_at_unix > 0 && now >= drop.owner_expires_at_unix {
        drop.no_owner = true;
    }
    drop
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashMap,
        path::PathBuf,
        sync::{Arc, RwLock},
    };

    use super::*;
    use crate::{character::CharacterRepository, portal::PortalBridge};
    use odmo_protocol::PacketReader;
    use odmo_types::{
        ActiveBuffSnapshot, AttendanceStatus, CharacterConnectionState, CharacterSummary,
        DEFAULT_ALT_PARTNER_MODEL_ID, DEFAULT_ALT_TAMER_MODEL_ID, DEFAULT_GM_PARTNER_MODEL_ID,
        DEFAULT_GM_TAMER_MODEL_ID, DEFAULT_PARTNER_MODEL_ID, DEFAULT_START_MAP_ID,
        DEFAULT_START_X, DEFAULT_START_Y,
        DEFAULT_TAMER_MODEL_ID, DailyRewardStatus, DropSummary, GameSessionTicket,
        GuildHistoricEntry, GuildMemberSnapshot, GuildSnapshot, MobSummary, RelationEntry,
        SealListSnapshot, SealRecord, XaiSnapshot,
    };

    #[derive(Debug)]
    struct InMemoryCharacterRepository {
        characters: RwLock<HashMap<u64, CharacterSummary>>,
        mobs_by_map: RwLock<HashMap<(i16, u8), Vec<MobSummary>>>,
        drops_by_map: RwLock<HashMap<(i16, u8), Vec<DropSummary>>>,
    }

    impl InMemoryCharacterRepository {
        fn demo() -> Self {
            Self {
                characters: RwLock::new(HashMap::from([
                    (
                        100,
                        CharacterSummary {
                            id: 100,
                            account_id: 1,
                            slot: 0,
                            name: "AdminTamer".to_string(),
                            partner_name: "Agumon".to_string(),
                            general_handler: 11_000,
                            partner_handler: 21_000,
                            model: DEFAULT_TAMER_MODEL_ID,
                            partner_model: DEFAULT_PARTNER_MODEL_ID,
                            seal_list: SealListSnapshot {
                                seal_leader_id: 1,
                                seals: vec![SealRecord {
                                    seal_id: 5001,
                                    amount: 3,
                                    sequential_id: 9,
                                    favorite: true,
                                }],
                            },
                            daily_reward: DailyRewardStatus {
                                event_no: 2001,
                                remaining_seconds: 120,
                                total_seconds: 600,
                                week: 3,
                            },
                            attendance: AttendanceStatus {
                                event_no: 0xFF,
                                attendance_count: 0,
                                notify: false,
                            },
                            friends: vec![RelationEntry {
                                name: "Matt".to_string(),
                                connected: true,
                                annotation: "Party".to_string(),
                            }],
                            foes: vec![RelationEntry {
                                name: "Devimon".to_string(),
                                connected: false,
                                annotation: String::new(),
                            }],
                            friended_character_ids: vec![200],
                            guild: Some(GuildSnapshot {
                                id: 77,
                                name: "Tamers".to_string(),
                                level: 4,
                                current_experience: 12_345,
                                notice: "Digital world first.".to_string(),
                                extra_slots: 10,
                                members: vec![
                                    GuildMemberSnapshot {
                                        character_id: 100,
                                        authority: 1,
                                        contribution: 900,
                                        character_name: "AdminTamer".to_string(),
                                        character_level: 70,
                                        character_model: 80_001,
                                        map_id: DEFAULT_START_MAP_ID,
                                        channel: 1,
                                        state: CharacterConnectionState::Ready,
                                    },
                                    GuildMemberSnapshot {
                                        character_id: 200,
                                        authority: 5,
                                        contribution: 100,
                                        character_name: "Rookie".to_string(),
                                        character_level: 20,
                                        character_model: 80_002,
                                        map_id: 0,
                                        channel: 0,
                                        state: CharacterConnectionState::Disconnected,
                                    },
                                ],
                                historic: vec![GuildHistoricEntry {
                                    historic_type: 101,
                                    date_utc_seconds: 1_700_000_000,
                                    master_class: 1,
                                    master_name: "AdminTamer".to_string(),
                                    member_class: 5,
                                    member_name: "Rookie".to_string(),
                                }],
                                rank_position: 9,
                                ..GuildSnapshot::default()
                            }),
                            xai: Some(XaiSnapshot {
                                item_id: 131063,
                                max_xgauge: 2000,
                                max_xcrystals: 3,
                            }),
                            active_buffs: vec![ActiveBuffSnapshot {
                                buff_id: 500,
                                skill_id: 8_001_001,
                                remaining_seconds: 120,
                            }],
                            current_xgauge: 500,
                            current_xcrystals: 2,
                            partner_active_buffs: vec![ActiveBuffSnapshot {
                                buff_id: 600,
                                skill_id: 8_002_001,
                                remaining_seconds: 90,
                            }],
                            partner_active_debuffs: vec![ActiveBuffSnapshot {
                                buff_id: 700,
                                skill_id: 8_003_001,
                                remaining_seconds: 30,
                            }],
                            ..CharacterSummary::default()
                        },
                    ),
                    (
                        200,
                        CharacterSummary {
                            id: 200,
                            account_id: 2,
                            slot: 0,
                            name: "Matt".to_string(),
                            partner_name: "Gabumon".to_string(),
                            model: DEFAULT_GM_TAMER_MODEL_ID,
                            partner_model: DEFAULT_GM_PARTNER_MODEL_ID,
                            general_handler: 12_000,
                            partner_handler: 22_000,
                            ..CharacterSummary::default()
                        },
                    ),
                    (
                        300,
                        CharacterSummary {
                            id: 300,
                            account_id: 3,
                            slot: 0,
                            name: "FarAway".to_string(),
                            partner_name: "Patamon".to_string(),
                            model: DEFAULT_ALT_TAMER_MODEL_ID,
                            partner_model: DEFAULT_ALT_PARTNER_MODEL_ID,
                            general_handler: 13_000,
                            partner_handler: 23_000,
                            x: 99_999,
                            y: 99_999,
                            partner_x: 99_999,
                            partner_y: 99_999,
                            ..CharacterSummary::default()
                        },
                    ),
                ])),
                mobs_by_map: RwLock::new(HashMap::from([(
                    (DEFAULT_START_MAP_ID, 0),
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
                            x: DEFAULT_START_X,
                            y: DEFAULT_START_Y,
                            previous_x: DEFAULT_START_X,
                            previous_y: DEFAULT_START_Y,
                            active_debuffs: vec![ActiveBuffSnapshot {
                                buff_id: 901,
                                skill_id: 7_001_001,
                                remaining_seconds: 45,
                            }],
                            ..MobSummary::default()
                        },
                        MobSummary {
                            id: 401,
                            map_id: DEFAULT_START_MAP_ID,
                            channel: 0,
                            handler: 34_001,
                            type_id: 51_002,
                            model: 51_002,
                            name: "FarMob".to_string(),
                            level: 40,
                            x: 99_999,
                            y: 99_999,
                            previous_x: 99_950,
                            previous_y: 99_950,
                            ..MobSummary::default()
                        },
                    ],
                )])),
                drops_by_map: RwLock::new(HashMap::from([(
                    (DEFAULT_START_MAP_ID, 0),
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
                            x: DEFAULT_START_X,
                            y: DEFAULT_START_Y,
                            owner_expires_at_unix: current_unix_timestamp() + 60,
                            expires_at_unix: current_unix_timestamp() + 90,
                            bits_drop: true,
                            ..DropSummary::default()
                        },
                        DropSummary {
                            id: 501,
                            map_id: DEFAULT_START_MAP_ID,
                            channel: 0,
                            handler: 49_201,
                            item_id: 5101,
                            amount: 1,
                            x: 99_999,
                            y: 99_999,
                            owner_expires_at_unix: current_unix_timestamp().saturating_sub(5),
                            expires_at_unix: current_unix_timestamp() + 30,
                            no_owner: true,
                            ..DropSummary::default()
                        },
                    ],
                )])),
            }
        }
    }

    impl CharacterRepository for InMemoryCharacterRepository {
        fn list_characters_by_account(
            &self,
            _account_id: AccountId,
        ) -> anyhow::Result<Vec<CharacterSummary>> {
            unreachable!()
        }
        fn character_by_slot(
            &self,
            _account_id: AccountId,
            _slot: u8,
        ) -> anyhow::Result<Option<CharacterSummary>> {
            unreachable!()
        }
        fn character_by_id(&self, character_id: u64) -> anyhow::Result<Option<CharacterSummary>> {
            Ok(self
                .characters
                .read()
                .expect("repo poisoned")
                .get(&character_id)
                .cloned())
        }
        fn is_name_available(&self, _name: &str) -> anyhow::Result<bool> {
            unreachable!()
        }
        fn create_character(
            &self,
            _account_id: AccountId,
            _slot: u8,
            _tamer_name: String,
            _tamer_model: i32,
            _partner_name: String,
            _partner_model: i32,
        ) -> anyhow::Result<CharacterSummary> {
            unreachable!()
        }
        fn delete_character(&self, _account_id: AccountId, _slot: u8) -> anyhow::Result<bool> {
            unreachable!()
        }
        fn update_character_position(
            &self,
            _character_id: u64,
            _x: i32,
            _y: i32,
            _z: f32,
        ) -> anyhow::Result<()> {
            unreachable!()
        }
        fn update_partner_position(
            &self,
            _character_id: u64,
            _x: i32,
            _y: i32,
            _z: f32,
        ) -> anyhow::Result<()> {
            unreachable!()
        }
        fn update_character_map(
            &self,
            _character_id: u64,
            _map_id: i16,
            _x: i32,
            _y: i32,
        ) -> anyhow::Result<()> {
            unreachable!()
        }
        fn update_inventory(
            &self,
            _character_id: u64,
            _inventory: odmo_types::InventorySnapshot,
        ) -> anyhow::Result<()> {
            unreachable!()
        }
        fn update_extra_inventory(
            &self,
            _character_id: u64,
            _extra_inventory: odmo_types::InventorySnapshot,
        ) -> anyhow::Result<()> {
            unreachable!()
        }
        fn update_warehouse(
            &self,
            _character_id: u64,
            _warehouse: odmo_types::InventorySnapshot,
        ) -> anyhow::Result<()> {
            unreachable!()
        }
        fn update_account_warehouse(
            &self,
            _character_id: u64,
            _account_warehouse: odmo_types::InventorySnapshot,
        ) -> anyhow::Result<()> {
            unreachable!()
        }
    }

    impl PortalRepository for InMemoryCharacterRepository {
        fn portal_by_id(&self, _portal_id: i32) -> anyhow::Result<Option<PortalDefinition>> {
            Ok(None)
        }
    }

    impl NpcShopRepository for InMemoryCharacterRepository {
        fn shop_by_npc(
            &self,
            _npc_id: i32,
            _map_id: i16,
        ) -> anyhow::Result<Option<NpcShopDefinition>> {
            Ok(None)
        }
    }

    impl MapMobRepository for InMemoryCharacterRepository {
        fn mobs_by_map(&self, map_id: i16, channel: u8) -> anyhow::Result<Vec<MobSummary>> {
            Ok(self
                .mobs_by_map
                .read()
                .expect("repo poisoned")
                .get(&(map_id, channel))
                .cloned()
                .unwrap_or_default())
        }
    }

    impl MapDropRepository for InMemoryCharacterRepository {
        fn drops_by_map(&self, map_id: i16, channel: u8) -> anyhow::Result<Vec<DropSummary>> {
            Ok(self
                .drops_by_map
                .read()
                .expect("repo poisoned")
                .get(&(map_id, channel))
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
            let mut characters = self.characters.write().expect("repo poisoned");
            let Some(character) = characters.get_mut(&character_id) else {
                return Ok(DropCollectionResult::Missing);
            };

            let mut drops = self.drops_by_map.write().expect("repo poisoned");
            let Some(entries) = drops.get_mut(&(map_id, channel)) else {
                return Ok(DropCollectionResult::Missing);
            };
            let Some(index) = entries.iter().position(|drop| drop.handler == drop_handler) else {
                return Ok(DropCollectionResult::Missing);
            };
            let drop = apply_runtime_drop_state(entries[index].clone());

            if drop.collected {
                entries.remove(index);
                return Ok(DropCollectionResult::Missing);
            }
            if map_distance(character.x, character.y, drop.x, drop.y) >= STOP_SEEING_DISTANCE {
                return Ok(DropCollectionResult::TooFarAway);
            }
            if drop.owner_id != 0 && drop.owner_id != character_id && !drop.no_owner {
                return Ok(DropCollectionResult::NotTheOwner);
            }

            if drop.bits_drop {
                character.inventory.bits += i64::from(drop.amount.max(0));
                character.inventory_bits += i64::from(drop.amount.max(0));
                entries.remove(index);
                return Ok(DropCollectionResult::BitsCollected {
                    drop: drop.clone(),
                    amount: drop.amount,
                    character: character.clone(),
                });
            }

            if !test_add_inventory_item(
                &mut character.inventory.items,
                character.inventory.size,
                &drop,
            ) {
                return Ok(DropCollectionResult::InventoryFull);
            }

            entries.remove(index);
            Ok(DropCollectionResult::ItemCollected {
                drop: drop.clone(),
                item_id: drop.item_id,
                amount: drop.amount.clamp(i16::MIN as i32, i16::MAX as i32) as i16,
                character: character.clone(),
            })
        }
    }

    fn unique_test_dir(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("odmo-game-{name}-{}", uuid::Uuid::new_v4()))
    }

    fn test_add_inventory_item(
        items: &mut Vec<odmo_types::ItemRecord>,
        size: u16,
        drop: &DropSummary,
    ) -> bool {
        if let Some(existing) = items.iter_mut().find(|item| item.item_id == drop.item_id) {
            existing.amount = existing.amount.saturating_add(drop.amount.max(0));
            existing.record.resize(69, 0);
            existing.record[0..4].copy_from_slice(&existing.item_id.to_le_bytes());
            existing.record[4..8].copy_from_slice(&existing.amount.to_le_bytes());
            return true;
        }

        if items.len() >= size as usize {
            return false;
        }

        let mut record = odmo_types::ItemRecord {
            item_id: drop.item_id,
            amount: drop.amount.max(1),
            ..odmo_types::ItemRecord::default()
        };
        record.record[0..4].copy_from_slice(&record.item_id.to_le_bytes());
        record.record[4..8].copy_from_slice(&record.amount.to_le_bytes());
        items.push(record);
        true
    }

    #[test]
    fn initial_information_requires_ticket() {
        let app = GameApplication::new(
            GameServiceConfig {
                portal_state_dir: unique_test_dir("missing-ticket"),
            },
            Arc::new(InMemoryCharacterRepository::demo()),
        );
        let mut session = GameSession::new(1);
        let error = app
            .handle_request(
                &mut session,
                GameRequest::InitialInformation {
                    account_id: 1,
                    access_code: 0,
                },
            )
            .expect_err("ticket should be required");
        assert!(matches!(error, GameFlowError::MissingSessionTicket(1)));
    }

    #[test]
    fn initial_information_returns_bootstrap_packet() {
        let portal_state_dir = unique_test_dir("with-ticket");
        let bridge = PortalBridge::from_json(portal_state_dir.clone()).expect("bridge");
        bridge
            .store_game_session_ticket(&GameSessionTicket {
                token: "demo".to_string(),
                account_id: 1,
                character_id: 100,
            })
            .expect("store ticket");

        let app = GameApplication::new(
            GameServiceConfig { portal_state_dir },
            Arc::new(InMemoryCharacterRepository::demo()),
        );
        let mut session = GameSession::new(1);
        let responses = app
            .handle_request(
                &mut session,
                GameRequest::InitialInformation {
                    account_id: 1,
                    access_code: 0,
                },
            )
            .expect("bootstrap should succeed");
        assert_eq!(responses.len(), 1);
    }

    #[test]
    fn complementar_information_returns_follow_up_packets() {
        let portal_state_dir = unique_test_dir("complementary");
        let bridge = PortalBridge::from_json(portal_state_dir.clone()).expect("bridge");
        bridge
            .store_game_session_ticket(&GameSessionTicket {
                token: "demo".to_string(),
                account_id: 1,
                character_id: 100,
            })
            .expect("store ticket");
        bridge
            .store_game_session_ticket(&GameSessionTicket {
                token: "friend".to_string(),
                account_id: 2,
                character_id: 200,
            })
            .expect("store friend ticket");

        let app = GameApplication::new(
            GameServiceConfig { portal_state_dir },
            Arc::new(InMemoryCharacterRepository::demo()),
        );
        let mut existing_session = GameSession::new(2);
        app.handle_request(
            &mut existing_session,
            GameRequest::InitialInformation {
                account_id: 2,
                access_code: 0,
            },
        )
        .expect("existing occupant bootstrap should succeed");
        app.handle_request(&mut existing_session, GameRequest::ComplementarInformation)
            .expect("existing occupant should register map presence");

        let mut session = GameSession::new(1);
        app.handle_request(
            &mut session,
            GameRequest::InitialInformation {
                account_id: 1,
                access_code: 0,
            },
        )
        .expect("bootstrap should succeed");

        let responses = app
            .handle_request(&mut session, GameRequest::ComplementarInformation)
            .expect("follow-up should succeed");
        let packet_types: Vec<i16> = responses
            .iter()
            .map(|frame| {
                PacketReader::from_frame(frame)
                    .expect("frame should decode")
                    .packet_type
            })
            .collect();

        assert!(
            packet_types.contains(&odmo_protocol::opcode::game::SEALS),
            "complementary flow should include seals",
        );
        assert!(
            packet_types.contains(&odmo_protocol::opcode::game::TIME_REWARD),
            "complementary flow should include daily time reward",
        );
        assert!(
            packet_types.contains(&odmo_protocol::opcode::game::AVAILABLE_RELATIONS),
            "complementary flow should include tamer relations",
        );
        assert!(
            packet_types.contains(&odmo_protocol::opcode::game::TAMER_ATTENDANCE),
            "complementary flow should include attendance",
        );
        assert!(
            packet_types.contains(&odmo_protocol::opcode::game::GUILD_INFORMATION),
            "complementary flow should include guild information",
        );
        assert!(
            packet_types.contains(&odmo_protocol::opcode::game::LOAD_UNLOAD_ENTITY),
            "complementary flow should include load/unload spawn packets for existing occupants",
        );
        assert!(
            packet_types.contains(&odmo_protocol::opcode::game::LOAD_BUFFS),
            "map visibility should include load-buffs packets",
        );
        assert!(
            packet_types.contains(&odmo_protocol::opcode::game::XAI_INFO),
            "complementary flow should include xai info when equipped",
        );
        assert!(
            packet_types.contains(&odmo_protocol::opcode::game::TAMER_XAI_RESOURCES),
            "complementary flow should include tamer xai resources when equipped",
        );
        assert!(
            packet_types.contains(&odmo_protocol::opcode::game::GUILD_HISTORIC),
            "complementary flow should include guild historic",
        );
        assert!(
            packet_types.contains(&odmo_protocol::opcode::game::GUILD_RANK),
            "complementary flow should include guild rank when present",
        );
        assert!(
            responses.len() >= 18,
            "complementary flow should emit the expanded legacy-like follow-up set",
        );
    }

    #[test]
    fn complementar_information_enqueues_friend_connect_for_friended_characters() {
        let portal_state_dir = unique_test_dir("friend-connect");
        let bridge = PortalBridge::from_json(portal_state_dir.clone()).expect("bridge");
        bridge
            .store_game_session_ticket(&GameSessionTicket {
                token: "demo".to_string(),
                account_id: 1,
                character_id: 100,
            })
            .expect("store ticket");
        bridge
            .store_game_session_ticket(&GameSessionTicket {
                token: "demo-friend".to_string(),
                account_id: 2,
                character_id: 200,
            })
            .expect("store friend ticket");

        let app = GameApplication::new(
            GameServiceConfig {
                portal_state_dir: portal_state_dir.clone(),
            },
            Arc::new(InMemoryCharacterRepository::demo()),
        );

        let mut friend_session = GameSession::new(2);
        app.handle_request(
            &mut friend_session,
            GameRequest::InitialInformation {
                account_id: 2,
                access_code: 0,
            },
        )
        .expect("friend bootstrap should succeed");
        app.handle_request(&mut friend_session, GameRequest::ComplementarInformation)
            .expect("friend follow-up should register map presence");

        let mut announcer_session = GameSession::new(1);
        app.handle_request(
            &mut announcer_session,
            GameRequest::InitialInformation {
                account_id: 1,
                access_code: 0,
            },
        )
        .expect("bootstrap should succeed");
        app.handle_request(&mut announcer_session, GameRequest::ComplementarInformation)
            .expect("follow-up should succeed");

        let responses = app
            .handle_request(&mut friend_session, GameRequest::KeepConnection)
            .expect("friend keep-connection should flush notifications");
        let packet_types: Vec<i16> = responses
            .iter()
            .map(|frame| {
                PacketReader::from_frame(frame)
                    .expect("frame should decode")
                    .packet_type
            })
            .collect();

        assert!(
            packet_types.contains(&odmo_protocol::opcode::game::FRIEND_CONNECT),
            "friend should receive friend-connect notification after announcer login",
        );
        assert!(
            packet_types.contains(&odmo_protocol::opcode::game::LOAD_UNLOAD_ENTITY),
            "friend should receive map spawn notification after announcer login",
        );
        assert!(
            packet_types.contains(&odmo_protocol::opcode::game::LOAD_BUFFS),
            "friend visibility should include load-buffs packets",
        );
    }

    #[test]
    fn disconnect_enqueues_unload_for_remaining_map_occupants() {
        let portal_state_dir = unique_test_dir("map-unload");
        let bridge = PortalBridge::from_json(portal_state_dir.clone()).expect("bridge");
        bridge
            .store_game_session_ticket(&GameSessionTicket {
                token: "demo".to_string(),
                account_id: 1,
                character_id: 100,
            })
            .expect("store ticket");
        bridge
            .store_game_session_ticket(&GameSessionTicket {
                token: "friend".to_string(),
                account_id: 2,
                character_id: 200,
            })
            .expect("store friend ticket");

        let app = GameApplication::new(
            GameServiceConfig {
                portal_state_dir: portal_state_dir.clone(),
            },
            Arc::new(InMemoryCharacterRepository::demo()),
        );

        let mut occupant_session = GameSession::new(1);
        app.handle_request(
            &mut occupant_session,
            GameRequest::InitialInformation {
                account_id: 1,
                access_code: 0,
            },
        )
        .expect("occupant bootstrap should succeed");
        app.handle_request(&mut occupant_session, GameRequest::ComplementarInformation)
            .expect("occupant should register map presence");

        let mut leaving_session = GameSession::new(2);
        app.handle_request(
            &mut leaving_session,
            GameRequest::InitialInformation {
                account_id: 2,
                access_code: 0,
            },
        )
        .expect("leaving bootstrap should succeed");
        app.handle_request(&mut leaving_session, GameRequest::ComplementarInformation)
            .expect("leaving should register map presence");

        app.handle_request(&mut occupant_session, GameRequest::KeepConnection)
            .expect("occupant should first observe the leaving tamer");

        app.handle_disconnect(&leaving_session)
            .expect("disconnect cleanup should succeed");

        let responses = app
            .handle_request(&mut occupant_session, GameRequest::KeepConnection)
            .expect("occupant keep-connection should flush unload");
        let packet_types: Vec<i16> = responses
            .iter()
            .map(|frame| {
                PacketReader::from_frame(frame)
                    .expect("frame should decode")
                    .packet_type
            })
            .collect();

        assert!(
            packet_types.contains(&odmo_protocol::opcode::game::LOAD_UNLOAD_ENTITY),
            "remaining occupant should receive unload notification after disconnect",
        );
    }

    #[test]
    fn keep_connection_unloads_tamers_that_moved_out_of_visibility_range() {
        let portal_state_dir = unique_test_dir("distance-hide");
        let bridge = PortalBridge::from_json(portal_state_dir.clone()).expect("bridge");
        bridge
            .upsert_map_presence(&CharacterSummary {
                id: 300,
                account_id: 3,
                name: "FarAway".to_string(),
                partner_name: "Patamon".to_string(),
                general_handler: 13_000,
                partner_handler: 23_000,
                x: 99_999,
                y: 99_999,
                partner_x: 99_999,
                partner_y: 99_999,
                ..CharacterSummary::default()
            })
            .expect("store far occupant");

        let app = GameApplication::new(
            GameServiceConfig {
                portal_state_dir: portal_state_dir.clone(),
            },
            Arc::new(InMemoryCharacterRepository::demo()),
        );

        let mut session = GameSession::new(1);
        session.character_id = Some(100);
        session.registered_map_presence = true;
        session.viewed_characters.insert(
            300,
            CharacterSummary {
                id: 300,
                account_id: 3,
                name: "FarAway".to_string(),
                partner_name: "Patamon".to_string(),
                general_handler: 13_000,
                partner_handler: 23_000,
                x: 99_999,
                y: 99_999,
                partner_x: 99_999,
                partner_y: 99_999,
                ..CharacterSummary::default()
            },
        );

        let responses = app
            .handle_request(&mut session, GameRequest::KeepConnection)
            .expect("visibility reconciliation should succeed");
        let packet_types: Vec<i16> = responses
            .iter()
            .map(|frame| {
                PacketReader::from_frame(frame)
                    .expect("frame should decode")
                    .packet_type
            })
            .collect();

        assert!(
            packet_types.contains(&odmo_protocol::opcode::game::LOAD_UNLOAD_ENTITY),
            "keep-connection should unload tamers outside visibility range",
        );
    }

    #[test]
    fn complementar_information_loads_visible_mobs() {
        let portal_state_dir = unique_test_dir("mob-load");
        let bridge = PortalBridge::from_json(portal_state_dir.clone()).expect("bridge");
        bridge
            .store_game_session_ticket(&GameSessionTicket {
                token: "demo".to_string(),
                account_id: 1,
                character_id: 100,
            })
            .expect("store ticket");

        let app = GameApplication::new(
            GameServiceConfig { portal_state_dir },
            Arc::new(InMemoryCharacterRepository::demo()),
        );

        let mut session = GameSession::new(1);
        app.handle_request(
            &mut session,
            GameRequest::InitialInformation {
                account_id: 1,
                access_code: 0,
            },
        )
        .expect("bootstrap should succeed");

        // ComplementarInformation handler internally calls register_map_presence then reconcile.
        let responses = app
            .handle_request(&mut session, GameRequest::ComplementarInformation)
            .expect("follow-up should succeed");
        let load_unload_payloads: Vec<Vec<u8>> = responses
            .iter()
            .filter_map(|frame| {
                let raw = PacketReader::from_frame(frame).ok()?;
                (raw.packet_type == odmo_protocol::opcode::game::LOAD_UNLOAD_ENTITY)
                    .then_some(raw.payload)
            })
            .collect();
        let load_buffs_payloads: Vec<Vec<u8>> = responses
            .iter()
            .filter_map(|frame| {
                let raw = PacketReader::from_frame(frame).ok()?;
                (raw.packet_type == odmo_protocol::opcode::game::LOAD_BUFFS).then_some(raw.payload)
            })
            .collect();

        assert!(
            load_unload_payloads
                .iter()
                .any(|payload| payload.first() == Some(&3)),
            "complementary flow should include a load-mob entity packet",
        );
        assert!(
            load_buffs_payloads
                .iter()
                .any(|payload| payload.first() == Some(&16)),
            "complementary flow should include a load-mob buffs packet",
        );
    }

    #[test]
    fn keep_connection_unloads_mobs_that_moved_out_of_visibility_range() {
        let app = GameApplication::new(
            GameServiceConfig {
                portal_state_dir: unique_test_dir("mob-distance-hide"),
            },
            Arc::new(InMemoryCharacterRepository::demo()),
        );

        let mut session = GameSession::new(1);
        session.character_id = Some(100);
        session.registered_map_presence = true;
        session.viewed_mobs.insert(
            401,
            MobSummary {
                id: 401,
                map_id: DEFAULT_START_MAP_ID,
                channel: 0,
                handler: 34_001,
                type_id: 51_002,
                x: 99_999,
                y: 99_999,
                previous_x: 99_950,
                previous_y: 99_950,
                ..MobSummary::default()
            },
        );

        let responses = app
            .handle_request(&mut session, GameRequest::KeepConnection)
            .expect("mob visibility reconciliation should succeed");
        let unload_payloads: Vec<Vec<u8>> = responses
            .iter()
            .filter_map(|frame| {
                let raw = PacketReader::from_frame(frame).ok()?;
                (raw.packet_type == odmo_protocol::opcode::game::LOAD_UNLOAD_ENTITY
                    && raw.payload.first() == Some(&4))
                .then_some(raw.payload)
            })
            .collect();

        assert!(
            !unload_payloads.is_empty(),
            "keep-connection should unload mobs outside visibility range",
        );
    }

    #[test]
    fn complementar_information_loads_visible_drops() {
        let portal_state_dir = unique_test_dir("drop-load");
        let bridge = PortalBridge::from_json(portal_state_dir.clone()).expect("bridge");
        bridge
            .store_game_session_ticket(&GameSessionTicket {
                token: "demo".to_string(),
                account_id: 1,
                character_id: 100,
            })
            .expect("store ticket");

        let app = GameApplication::new(
            GameServiceConfig { portal_state_dir },
            Arc::new(InMemoryCharacterRepository::demo()),
        );

        let mut session = GameSession::new(1);
        app.handle_request(
            &mut session,
            GameRequest::InitialInformation {
                account_id: 1,
                access_code: 0,
            },
        )
        .expect("bootstrap should succeed");

        // ComplementarInformation handler internally calls register_map_presence then reconcile.
        let responses = app
            .handle_request(&mut session, GameRequest::ComplementarInformation)
            .expect("follow-up should succeed");
        let load_unload_payloads: Vec<Vec<u8>> = responses
            .iter()
            .filter_map(|frame| {
                let raw = PacketReader::from_frame(frame).ok()?;
                (raw.packet_type == odmo_protocol::opcode::game::LOAD_UNLOAD_ENTITY)
                    .then_some(raw.payload)
            })
            .collect();

        assert!(
            load_unload_payloads
                .iter()
                .any(|payload| payload.first() == Some(&3)),
            "complementary flow should include a load-drop entity packet",
        );
    }

    #[test]
    fn keep_connection_unloads_drops_that_moved_out_of_visibility_range() {
        let app = GameApplication::new(
            GameServiceConfig {
                portal_state_dir: unique_test_dir("drop-distance-hide"),
            },
            Arc::new(InMemoryCharacterRepository::demo()),
        );

        let mut session = GameSession::new(1);
        session.character_id = Some(100);
        session.registered_map_presence = true;
        session.viewed_drops.insert(
            501,
            DropSummary {
                id: 501,
                map_id: DEFAULT_START_MAP_ID,
                channel: 0,
                handler: 49_201,
                item_id: 5101,
                amount: 1,
                x: 99_999,
                y: 99_999,
                no_owner: true,
                ..DropSummary::default()
            },
        );

        let responses = app
            .handle_request(&mut session, GameRequest::KeepConnection)
            .expect("drop visibility reconciliation should succeed");
        let unload_payloads: Vec<Vec<u8>> = responses
            .iter()
            .filter_map(|frame| {
                let raw = PacketReader::from_frame(frame).ok()?;
                (raw.packet_type == odmo_protocol::opcode::game::LOAD_UNLOAD_ENTITY
                    && raw.payload.first() == Some(&4))
                .then_some(raw.payload)
            })
            .collect();

        assert!(
            !unload_payloads.is_empty(),
            "keep-connection should unload drops outside visibility range",
        );
    }

    #[test]
    fn loot_item_collects_bits_drop_and_reloads_inventory() {
        let app = GameApplication::new(
            GameServiceConfig {
                portal_state_dir: unique_test_dir("loot-bits"),
            },
            Arc::new(InMemoryCharacterRepository::demo()),
        );

        let mut session = GameSession::new(1);
        session.character_id = Some(100);
        session.registered_map_presence = true;

        let responses = app
            .handle_request(
                &mut session,
                GameRequest::LootItem {
                    drop_handler: 49_200,
                },
            )
            .expect("bits loot should succeed");
        let packet_types: Vec<i16> = responses
            .iter()
            .map(|frame| {
                PacketReader::from_frame(frame)
                    .expect("frame should decode")
                    .packet_type
            })
            .collect();

        assert!(
            packet_types.contains(&odmo_protocol::opcode::game::PICK_BITS),
            "bits loot should emit pick-bits packet",
        );
        assert!(
            packet_types.contains(&odmo_protocol::opcode::game::LOAD_UNLOAD_ENTITY),
            "bits loot should unload the collected drop",
        );
        assert!(
            packet_types.contains(&odmo_protocol::opcode::game::LOAD_INVENTORY),
            "bits loot should reload inventory after updating bits",
        );
    }

    #[test]
    fn loot_item_collects_item_drop_when_owner_is_lost() {
        let repo = InMemoryCharacterRepository::demo();
        repo.drops_by_map
            .write()
            .expect("repo poisoned")
            .entry((DEFAULT_START_MAP_ID, 0))
            .or_default()
            .push(DropSummary {
                id: 502,
                map_id: DEFAULT_START_MAP_ID,
                channel: 0,
                handler: 49_202,
                owner_id: 0,
                owner_handler: 0,
                item_id: 6001,
                amount: 2,
                x: DEFAULT_START_X,
                y: DEFAULT_START_Y,
                owner_expires_at_unix: current_unix_timestamp().saturating_sub(2),
                expires_at_unix: current_unix_timestamp() + 30,
                no_owner: true,
                ..DropSummary::default()
            });

        let app = GameApplication::new(
            GameServiceConfig {
                portal_state_dir: unique_test_dir("loot-item"),
            },
            Arc::new(repo),
        );

        let mut session = GameSession::new(1);
        session.character_id = Some(100);
        session.registered_map_presence = true;

        let responses = app
            .handle_request(
                &mut session,
                GameRequest::LootItem {
                    drop_handler: 49_202,
                },
            )
            .expect("item loot should succeed");
        let packet_types: Vec<i16> = responses
            .iter()
            .map(|frame| {
                PacketReader::from_frame(frame)
                    .expect("frame should decode")
                    .packet_type
            })
            .collect();

        assert!(
            packet_types.contains(&odmo_protocol::opcode::game::LOOT_ITEM),
            "item loot should emit pick-item packet",
        );
        assert!(
            packet_types.contains(&odmo_protocol::opcode::game::LOAD_INVENTORY),
            "item loot should reload inventory after pickup",
        );
    }

    #[test]
    fn loot_item_fails_for_foreign_owned_drop() {
        let repo = InMemoryCharacterRepository::demo();
        repo.drops_by_map
            .write()
            .expect("repo poisoned")
            .entry((DEFAULT_START_MAP_ID, 0))
            .or_default()
            .push(DropSummary {
                id: 503,
                map_id: DEFAULT_START_MAP_ID,
                channel: 0,
                handler: 49_203,
                owner_id: 200,
                owner_handler: 12_000,
                item_id: 7001,
                amount: 1,
                x: 14_925,
                y: 9_965,
                owner_expires_at_unix: current_unix_timestamp() + 60,
                expires_at_unix: current_unix_timestamp() + 90,
                ..DropSummary::default()
            });

        let app = GameApplication::new(
            GameServiceConfig {
                portal_state_dir: unique_test_dir("loot-owner-fail"),
            },
            Arc::new(repo),
        );

        let mut session = GameSession::new(1);
        session.character_id = Some(100);
        session.registered_map_presence = true;

        let responses = app
            .handle_request(
                &mut session,
                GameRequest::LootItem {
                    drop_handler: 49_203,
                },
            )
            .expect("owner failure should still respond cleanly");
        let packet_types: Vec<i16> = responses
            .iter()
            .map(|frame| {
                PacketReader::from_frame(frame)
                    .expect("frame should decode")
                    .packet_type
            })
            .collect();

        assert!(
            packet_types.contains(&odmo_protocol::opcode::game::PICK_ITEM_FAIL),
            "foreign owned drop should return pick-item-fail",
        );
    }
}
