use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        Arc, RwLock,
        atomic::{AtomicI16, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

use thiserror::Error;

use odmo_protocol::{
    AvailableChannelsPacket, CashShopCoinsPacket, DigimonEvolutionFailPacket, DigimonWalkPacket,
    GameConnectionPacket, GameInitialInfoPacket, GameRequest, GuildHistoricPacket,
    GuildInformationPacket, InventoryType, ItemConsumeFailPacket, ItemMoveFailPacket,
    ItemMoveSuccessPacket, LoadBuffsPacket, LoadDropsPacket, LoadInventoryPacket,
    LoadMobBuffsPacket, LoadMobsPacket, LoadTamerPacket, LocalMapSwapPacket, MapSwapPacket,
    MembershipPacket, NpcPurchaseResultPacket, NpcSellResultPacket, PartnerSwitchFailurePacket,
    PartnerSwitchPacket, PartyChangeLootTypePacket, PartyCreatedPacket, PartyInvitePacket,
    PartyInviteResultPacket, PartyJoinPacket, PartyKickPacket, PartyLeaderChangedPacket,
    PartyLeavePacket, PartyMemberBuffChangePacket, PartyMemberBuffEntry,
    PartyMemberDisconnectedPacket, PartyMemberInfoPacket, PartyMemberListEntry,
    PartyMemberListPacket, PartyMemberMapChangePacket, PartyMemberPositionPacket, PickBitsPacket,
    PickItemFailPacket, PickItemFailReason, PickItemPacket, SealsPacket, ServerExperiencePacket,
    SplitItemPacket, TamerAttendancePacket, TamerRelationsPacket, TamerWalkPacket,
    TamerXaiResourcesPacket, TimeRewardPacket, UnloadDropsPacket, UnloadMobsPacket,
    UnloadTamerPacket, UpdateMovementSpeedPacket, UpdateStatusPacket, XaiInfoPacket,
    game::{FriendConnectPacket, GuildRankPacket, SkillUpdateCooldownPacket},
};
use odmo_types::{AccountId, ItemRecord};

use crate::{
    character::CharacterRepository,
    portal::{PortalBridge, SocialNotification, SocialNotificationKind},
};

const HANDSHAKE_DEGREE: i16 = 32321;
const START_TO_SEE_DISTANCE: i64 = 18_000;
const STOP_SEEING_DISTANCE: i64 = 18_001;
const PARTY_INVITE_IMPOSSIBLE: i32 = -3;
const PARTY_INVITE_OFFLINE: i32 = -2;
const PARTY_INVITE_REJECTED: i32 = -1;
const PARTY_INVITE_ALREADY_IN_PARTY: i32 = 0;
const PARTY_INVITE_ACCEPTED: i32 = 1;

#[derive(Debug, Clone)]
struct PendingPartyInvite {
    inviter_id: u64,
    target_id: u64,
}

#[derive(Debug, Clone)]
struct PartyRuntimeState {
    next_party_id: u32,
    pending_invites: HashMap<u64, PendingPartyInvite>,
    parties: HashMap<u32, PartyGroup>,
    party_by_member: HashMap<u64, u32>,
}

impl Default for PartyRuntimeState {
    fn default() -> Self {
        Self {
            next_party_id: 1,
            pending_invites: HashMap::new(),
            parties: HashMap::new(),
            party_by_member: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
struct PartyGroup {
    id: u32,
    leader_id: u64,
    loot_type: u32,
    rare_rate: u8,
    disp_rare_grade: u8,
    members: Vec<u64>,
}

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
    party_runtime: Arc<RwLock<PartyRuntimeState>>,
    game_server_address: String,
    game_server_port: i32,
}

impl GameApplication {
    pub fn new(config: GameServiceConfig, repository: Arc<dyn GameRepository>) -> Self {
        let portal_bridge = PortalBridge::from_json(config.portal_state_dir)
            .expect("portal bridge should initialize");
        Self {
            portal_bridge,
            repository,
            broadcast: None,
            party_runtime: Arc::new(RwLock::new(PartyRuntimeState::default())),
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

                // Send skill cooldowns for partner digimon (empty until skill system is implemented)
                responses.push(
                    SkillUpdateCooldownPacket {
                        handler: character.partner_handler as i32,
                        current_type: character.partner_model as i32,
                        cooldowns: vec![],
                    }
                    .encode(),
                );

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

                // Unlock map region (non-blocking, best-effort)
                let _ = self.repository.update_character_map_region(
                    character_id,
                    character.map_id,
                    true,
                );

                // Mark character state as Ready (non-blocking)
                let _ = self.repository.update_character_state(character_id, 1);

                // Update welcome flag to false (non-blocking)
                let _ = self
                    .repository
                    .update_welcome_flag(session.account_id.unwrap_or(0), false);

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
                self.broadcast_party_member_info(&updated_character);
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
                fn tab_class(sid: u16) -> u16 {
                    sid / 1000 * 1000
                }
                fn tab_index(sid: u16) -> usize {
                    (sid % 1000) as usize
                }

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
                    if a >= items.len() || b >= items.len() {
                        return;
                    }
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
                                &mut character.inventory.items,
                                src_idx,
                                &mut character.warehouse.items,
                                dst_idx,
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
                                &mut character.warehouse.items,
                                src_idx,
                                &mut character.inventory.items,
                                dst_idx,
                            );
                            true
                        } else {
                            false
                        }
                    }
                    (TAB_INVENTORY, TAB_SHARESTASH) => {
                        let aw = character.account_warehouse.as_mut();
                        match aw {
                            Some(aw)
                                if src_idx < character.inventory.items.len()
                                    && dst_idx < aw.items.len() =>
                            {
                                transfer_between(
                                    &mut character.inventory.items,
                                    src_idx,
                                    &mut aw.items,
                                    dst_idx,
                                );
                                true
                            }
                            _ => false,
                        }
                    }
                    (TAB_SHARESTASH, TAB_INVENTORY) => {
                        let aw = character.account_warehouse.as_mut();
                        match aw {
                            Some(aw)
                                if src_idx < aw.items.len()
                                    && dst_idx < character.inventory.items.len() =>
                            {
                                transfer_between(
                                    &mut aw.items,
                                    src_idx,
                                    &mut character.inventory.items,
                                    dst_idx,
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
                                character
                                    .account_warehouse
                                    .as_ref()
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

                fn tab_class(sid: u16) -> u16 {
                    sid / 1000 * 1000
                }
                fn tab_index(sid: u16) -> usize {
                    (sid % 1000) as usize
                }

                let src_tab = tab_class(origin_slot);
                let dst_tab = tab_class(destination_slot);
                let src_idx = tab_index(origin_slot);
                let dst_idx = tab_index(destination_slot);

                fn split_within(
                    items: &mut [ItemRecord],
                    src: usize,
                    dst: usize,
                    amt: i32,
                ) -> bool {
                    if src >= items.len() || dst >= items.len() {
                        return false;
                    }
                    let source = items[src].clone();
                    let dest = items[dst].clone();
                    if source.item_id <= 0 || source.amount < amt {
                        return false;
                    }
                    if dest.item_id > 0 && dest.item_id != source.item_id {
                        return false;
                    }

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
                    src_items: &mut [ItemRecord],
                    src_idx: usize,
                    dst_items: &mut [ItemRecord],
                    dst_idx: usize,
                    amt: i32,
                ) -> bool {
                    if src_idx >= src_items.len() || dst_idx >= dst_items.len() {
                        return false;
                    }
                    let source = src_items[src_idx].clone();
                    let dest = dst_items[dst_idx].clone();
                    if source.item_id <= 0 || source.amount < amt {
                        return false;
                    }
                    if dest.item_id > 0 && dest.item_id != source.item_id {
                        return false;
                    }

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
                    (0, 0) => split_within(
                        &mut character.inventory.items,
                        src_idx,
                        dst_idx,
                        amount as i32,
                    ),
                    (2000, 2000) => split_within(
                        &mut character.warehouse.items,
                        src_idx,
                        dst_idx,
                        amount as i32,
                    ),
                    (0, 2000) => split_cross(
                        &mut character.inventory.items,
                        src_idx,
                        &mut character.warehouse.items,
                        dst_idx,
                        amount as i32,
                    ),
                    (2000, 0) => split_cross(
                        &mut character.warehouse.items,
                        src_idx,
                        &mut character.inventory.items,
                        dst_idx,
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
                let updated_character = self
                    .repository
                    .character_by_id(character_id)
                    .map_err(|error| GameFlowError::Storage(error.to_string()))?
                    .ok_or(GameFlowError::CharacterNotFound(character_id))?;
                self.broadcast_party_member_position(&updated_character);

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
                let updated_character = self
                    .repository
                    .character_by_id(character_id)
                    .map_err(|error| GameFlowError::Storage(error.to_string()))?
                    .ok_or(GameFlowError::CharacterNotFound(character_id))?;
                self.broadcast_party_member_map_change(&updated_character);

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
                let character_id = session.character_id.ok_or(GameFlowError::Unauthenticated)?;
                let character = self
                    .repository
                    .character_by_id(character_id)
                    .map_err(|error| GameFlowError::Storage(error.to_string()))?
                    .ok_or(GameFlowError::CharacterNotFound(character_id))?;

                // Echo the emoticon back to the client.
                let mut writer =
                    odmo_protocol::writer::PacketWriter::new(odmo_protocol::opcode::game::EMOTICON);
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
                let character_id = session.character_id.ok_or(GameFlowError::Unauthenticated)?;
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
                let character_id = session.character_id.ok_or(GameFlowError::Unauthenticated)?;
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
                let character_id = session.character_id.ok_or(GameFlowError::Unauthenticated)?;
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
                let character_id = session.character_id.ok_or(GameFlowError::Unauthenticated)?;
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
                let character_id = session.character_id.ok_or(GameFlowError::Unauthenticated)?;
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
                let character_id = session.character_id.ok_or(GameFlowError::Unauthenticated)?;
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
            // PartnerSwitch — simplified roster-backed switch without battle-tag restrictions yet
            GameRequest::PartnerSwitch { slot } => {
                let character_id = session.character_id.ok_or(GameFlowError::Unauthenticated)?;
                let character = self
                    .repository
                    .character_by_id(character_id)
                    .map_err(|error| GameFlowError::Storage(error.to_string()))?
                    .ok_or(GameFlowError::CharacterNotFound(character_id))?;
                let old_partner_type = character.partner_current_type;
                let Some(updated_character) = self
                    .repository
                    .switch_partner(character_id, slot)
                    .map_err(|error| GameFlowError::Storage(error.to_string()))?
                else {
                    return Ok(vec![PartnerSwitchFailurePacket.encode()]);
                };
                if updated_character.partner_current_slot == character.partner_current_slot {
                    return Ok(vec![PartnerSwitchFailurePacket.encode()]);
                }

                let active_partner = updated_character
                    .partner_slots
                    .iter()
                    .find(|partner| partner.slot == updated_character.partner_current_slot)
                    .cloned()
                    .ok_or_else(|| {
                        GameFlowError::Storage(
                            "active partner slot missing after switch".to_string(),
                        )
                    })?;
                let active_partner_buffs = active_partner.active_buffs.clone();

                let switch_packet = PartnerSwitchPacket {
                    handler: updated_character.partner_handler,
                    old_partner_current_type: old_partner_type,
                    slot,
                    partner: active_partner,
                }
                .encode();

                if let Some(broadcast) = &self.broadcast {
                    let _ = broadcast.send_to_visible(
                        updated_character.map_id,
                        updated_character.channel,
                        updated_character.id,
                        &switch_packet,
                    );
                    let _ = broadcast.send_to(updated_character.id, &switch_packet);
                }

                self.broadcast_party_member_digimon_change(&updated_character);
                self.broadcast_party_member_buff_change(&updated_character, &active_partner_buffs);

                Ok(vec![
                    UpdateStatusPacket {
                        character: updated_character,
                    }
                    .encode(),
                ])
            }
            GameRequest::PartnerEvolution {
                digimon_handler: _,
                evolution_slot: _,
            } => Ok(vec![DigimonEvolutionFailPacket.encode()]),
            // PartnerDelete — stub: respond with failure (needs secondary password validation)
            GameRequest::PartnerDelete {
                slot: _,
                validation: _,
            } => {
                let character_id = session.character_id.ok_or(GameFlowError::Unauthenticated)?;
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
            GameRequest::EvolutionUnlock {
                evolution_type: _,
                inven_idx: _,
            } => {
                let mut writer = odmo_protocol::writer::PacketWriter::new(
                    odmo_protocol::opcode::game::EVOLUTION_UNLOCK_RESPONSE,
                );
                writer.write_u8(0); // failure
                Ok(vec![writer.finalize()])
            }
            // RideModeStart — stub: respond with failure
            GameRequest::RideModeStart {
                evolution_type: _,
                item_type: _,
            } => {
                let mut writer = odmo_protocol::writer::PacketWriter::new(
                    odmo_protocol::opcode::game::RIDE_MODE_START,
                );
                writer.write_u8(0); // failure
                Ok(vec![writer.finalize()])
            }
            // RideModeStop — stub: no response needed
            GameRequest::RideModeStop => Ok(vec![]),
            // DigimonChangeName — stub: respond with failure
            GameRequest::DigimonChangeName {
                inven_slot: _,
                new_name: _,
            } => {
                let mut writer = odmo_protocol::writer::PacketWriter::new(
                    odmo_protocol::opcode::game::DIGIMON_CHANGE_NAME,
                );
                writer.write_u8(0); // failure
                Ok(vec![writer.finalize()])
            }
            // HatchInsertEgg — stub: respond with failure
            GameRequest::HatchInsertEgg {
                vip: _,
                inven_slot: _,
                npc_idx: _,
            } => {
                let mut writer = odmo_protocol::writer::PacketWriter::new(
                    odmo_protocol::opcode::game::HATCH_FAILURE,
                );
                writer.write_u8(0); // failure
                Ok(vec![writer.finalize()])
            }
            // HatchIncrease — stub: respond with failure
            GameRequest::HatchIncrease {
                vip: _,
                npc_idx: _,
                data_level: _,
            } => {
                let mut writer = odmo_protocol::writer::PacketWriter::new(
                    odmo_protocol::opcode::game::HATCH_FAILURE,
                );
                writer.write_u8(0); // failure
                Ok(vec![writer.finalize()])
            }
            // HatchFinish — stub: respond with failure
            GameRequest::HatchFinish {
                vip: _,
                portable_pos: _,
                name: _,
                npc_idx: _,
            } => {
                let mut writer = odmo_protocol::writer::PacketWriter::new(
                    odmo_protocol::opcode::game::HATCH_FAILURE,
                );
                writer.write_u8(0); // failure
                Ok(vec![writer.finalize()])
            }
            // HatchRemoveEgg — stub: no response needed
            GameRequest::HatchRemoveEgg { vip: _, npc_idx: _ } => Ok(vec![]),
            // HatchBackupInsert — stub: no response needed
            GameRequest::HatchBackupInsert {
                vip: _,
                inven_slot: _,
                npc_idx: _,
            } => Ok(vec![]),
            // HatchBackupCancel — stub: no response needed
            GameRequest::HatchBackupCancel { vip: _, npc_idx: _ } => Ok(vec![]),
            // IncubatorClose — stub: no response needed
            GameRequest::IncubatorClose => Ok(vec![]),
            // DigimonArchiveMove — stub: no response needed
            GameRequest::DigimonArchiveMove {
                vip: _,
                slot1: _,
                slot2: _,
                npc_type: _,
            } => Ok(vec![]),
            // DigimonArchiveList — stub: no response needed
            GameRequest::DigimonArchiveList {
                vip: _,
                inven_idx: _,
                npc_type: _,
            } => Ok(vec![]),
            // DigimonArchiveSwap — stub: no response needed
            GameRequest::DigimonArchiveSwap {
                npc_idx: _,
                archive_type: _,
                src_arr: _,
                dst_arr: _,
            } => Ok(vec![]),
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
            GameRequest::ItemSocketIn {
                item_slot: _,
                socket_slot: _,
                chip_item_id: _,
            } => {
                let mut writer = odmo_protocol::writer::PacketWriter::new(
                    odmo_protocol::opcode::game::ITEM_SOCKET_IN,
                );
                writer.write_u8(0); // failure
                Ok(vec![writer.finalize()])
            }
            // ItemSocketOut — stub: respond with failure
            GameRequest::ItemSocketOut {
                item_slot: _,
                socket_slot: _,
            } => {
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
            GameRequest::TamerShopBuy {
                item_id: _,
                amount: _,
            } => {
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
            GameRequest::ConsignedShopPurchase {
                item_id: _,
                amount: _,
            } => Ok(vec![]),
            // ConsignedShopRetrieve — stub: respond with failure
            GameRequest::ConsignedShopRetrieve { item_slot: _ } => Ok(vec![]),
            // CashShopOpen — stub: respond with empty
            GameRequest::CashShopOpen => Ok(vec![]),
            // CashShopBuy — stub: respond with failure
            GameRequest::CashShopBuy {
                amount: _,
                total_price: _,
                order_id: _,
                product_ids: _,
            } => Ok(vec![]),
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
            GameRequest::QuestUpdate {
                quest_id: _,
                progress: _,
            } => Ok(vec![]),
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
            GameRequest::SealSetFavorite {
                card_code: _,
                bookmark: _,
            } => Ok(vec![]),
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
            GameRequest::ArenaDailyPoints {
                item_slot: _,
                points: _,
                item_id: _,
            } => Ok(vec![]),
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
            GameRequest::RareMachineRun {
                npc_idx: _,
                inven_idx: _,
                reset_count: _,
            } => Ok(vec![]),
            // Party stubs
            GameRequest::PartyInvite { target_name } => {
                self.handle_party_invite(session, target_name)
            }
            GameRequest::PartyInviteResponse {
                result_type,
                inviter_name,
            } => self.handle_party_invite_response(session, result_type, inviter_name),
            GameRequest::PartyChat { message: _ } => Ok(vec![]),
            GameRequest::PartyKick { target_name } => self.handle_party_kick(session, target_name),
            GameRequest::PartyLeave => self.handle_party_leave(session),
            GameRequest::PartyChangeMaster { new_leader_slot } => {
                self.handle_party_change_master(session, new_leader_slot)
            }
            GameRequest::PartyChangeLoot {
                loot_type,
                rare_type,
                disp_rare_grade,
            } => self.handle_party_change_loot(session, loot_type, rare_type, disp_rare_grade),
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
            GameRequest::GuildSetTitle {
                member_id: _,
                title: _,
            } => Ok(vec![]),
            // Trade stubs
            GameRequest::TradeRequest { target_handler: _ } => Ok(vec![]),
            GameRequest::TradeAccept {
                accepter_handler: _,
            } => Ok(vec![]),
            GameRequest::TradeCancel => Ok(vec![]),
            GameRequest::TradeAddItem {
                item_slot: _,
                trade_slot: _,
            } => Ok(vec![]),
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
            GameRequest::HatchSpiritEvolution {
                model_id: _,
                name: _,
                npc_id: _,
            } => Ok(vec![]),
            GameRequest::DigiSummonPurchase { npc_idx: _ } => Ok(vec![]),
            // Warehouse stubs
            GameRequest::LoadAccountWarehouse => Ok(vec![]),
            GameRequest::RetrieveAccountWarehouse { item_slot: _ } => Ok(vec![]),
            // Extra inventory stubs
            GameRequest::ExtraInventoryCategoryRefresh { category: _ } => Ok(vec![]),
            GameRequest::ExtraInventoryMove {
                category: _,
                extra_slot: _,
                inventory_slot: _,
            } => Ok(vec![]),
            GameRequest::ExtraInventorySort { category: _ } => Ok(vec![]),
            // Party extra stubs
            GameRequest::PartyConfigChange { loot_type: _ } => Ok(vec![]),
            GameRequest::PartyMemberDisconnect => Ok(vec![]),
            // Combat/Tamer stubs
            GameRequest::MonsterRespawnTimer => Ok(vec![]),
            GameRequest::JumpBooster => Ok(vec![]),
            GameRequest::SkillLevelUp {
                uid: _,
                evo_unit_idx: _,
                skill_idx: _,
            } => Ok(vec![]),
            GameRequest::TamerChargeXCrystal => Ok(vec![]),
            GameRequest::TamerConsumeXCrystal { amount: _ } => Ok(vec![]),
            GameRequest::TamerSummon { target_name: _ } => Ok(vec![]),
            GameRequest::TamerSkillRequest {
                skill_idx: _,
                target_uid: _,
            } => Ok(vec![]),
            GameRequest::TranscendenceReceiveExp => Ok(vec![]),
            GameRequest::TranscendenceSuccess => Ok(vec![]),
            GameRequest::TimeChargeResult { charge_type: _ } => Ok(vec![]),
            GameRequest::WarpGateDungeon => Ok(vec![]),
            GameRequest::SpiritCraft {
                model_id: _,
                name: _,
                npc_id: _,
            } => Ok(vec![]),
        }?;

        responses.extend(request_responses);
        Ok(responses)
    }

    fn handle_party_invite(
        &self,
        session: &GameSession,
        target_name: String,
    ) -> Result<Vec<Vec<u8>>, GameFlowError> {
        let inviter_id = session.character_id.ok_or(GameFlowError::Unauthenticated)?;
        let inviter = self
            .repository
            .character_by_id(inviter_id)
            .map_err(|error| GameFlowError::Storage(error.to_string()))?
            .ok_or(GameFlowError::CharacterNotFound(inviter_id))?;

        let Some(target) = self
            .repository
            .character_by_name(&target_name)
            .map_err(|error| GameFlowError::Storage(error.to_string()))?
        else {
            return Ok(vec![
                PartyInviteResultPacket {
                    result_type: PARTY_INVITE_OFFLINE,
                    target_name,
                }
                .encode(),
            ]);
        };

        let Some(broadcast) = &self.broadcast else {
            return Ok(vec![
                PartyInviteResultPacket {
                    result_type: PARTY_INVITE_OFFLINE,
                    target_name: target.name,
                }
                .encode(),
            ]);
        };

        if !broadcast.is_online(target.id) || target.id == inviter.id {
            return Ok(vec![
                PartyInviteResultPacket {
                    result_type: PARTY_INVITE_OFFLINE,
                    target_name: target.name,
                }
                .encode(),
            ]);
        }

        {
            let mut runtime = self.party_runtime.write().expect("party runtime poisoned");
            if runtime.party_by_member.contains_key(&target.id) {
                return Ok(vec![
                    PartyInviteResultPacket {
                        result_type: PARTY_INVITE_ALREADY_IN_PARTY,
                        target_name: target.name,
                    }
                    .encode(),
                ]);
            }

            runtime.pending_invites.insert(
                target.id,
                PendingPartyInvite {
                    inviter_id: inviter.id,
                    target_id: target.id,
                },
            );
        }

        broadcast
            .send_to(
                target.id,
                &PartyInvitePacket {
                    inviter_name: inviter.name,
                }
                .encode(),
            )
            .map_err(|error| GameFlowError::Storage(error.to_string()))?;

        Ok(vec![])
    }

    fn handle_party_invite_response(
        &self,
        session: &GameSession,
        result_type: i32,
        inviter_name: String,
    ) -> Result<Vec<Vec<u8>>, GameFlowError> {
        let invitee_id = session.character_id.ok_or(GameFlowError::Unauthenticated)?;
        let invitee = self
            .repository
            .character_by_id(invitee_id)
            .map_err(|error| GameFlowError::Storage(error.to_string()))?
            .ok_or(GameFlowError::CharacterNotFound(invitee_id))?;
        let Some(inviter) = self
            .repository
            .character_by_name(&inviter_name)
            .map_err(|error| GameFlowError::Storage(error.to_string()))?
        else {
            return Ok(vec![]);
        };

        let pending = {
            let mut runtime = self.party_runtime.write().expect("party runtime poisoned");
            match runtime.pending_invites.remove(&invitee.id) {
                Some(pending)
                    if pending.inviter_id == inviter.id && pending.target_id == invitee.id =>
                {
                    Some(pending)
                }
                Some(other) => {
                    runtime.pending_invites.insert(other.target_id, other);
                    None
                }
                None => None,
            }
        };
        if pending.is_none() {
            return Ok(vec![]);
        }

        let Some(broadcast) = &self.broadcast else {
            return Ok(vec![]);
        };

        if result_type != PARTY_INVITE_ACCEPTED {
            if broadcast.is_online(inviter.id) {
                let _ = broadcast.send_to(
                    inviter.id,
                    &PartyInviteResultPacket {
                        result_type,
                        target_name: invitee.name,
                    }
                    .encode(),
                );
            }
            return Ok(vec![]);
        }

        let (party, created_new, existing_members_before_add) = {
            let mut runtime = self.party_runtime.write().expect("party runtime poisoned");
            if runtime.party_by_member.contains_key(&invitee.id) {
                drop(runtime);
                if broadcast.is_online(inviter.id) {
                    let _ = broadcast.send_to(
                        inviter.id,
                        &PartyInviteResultPacket {
                            result_type: PARTY_INVITE_ALREADY_IN_PARTY,
                            target_name: invitee.name,
                        }
                        .encode(),
                    );
                }
                return Ok(vec![]);
            }

            let existing_party_id = runtime.party_by_member.get(&inviter.id).copied();
            if let Some(party_id) = existing_party_id {
                let snapshot = {
                    let party = runtime
                        .parties
                        .get_mut(&party_id)
                        .expect("party should exist for member mapping");
                    let existing_members = party.members.iter().copied().collect::<Vec<_>>();
                    party.members.push(invitee.id);
                    (party.clone(), false, existing_members)
                };
                runtime.party_by_member.insert(invitee.id, party_id);
                snapshot
            } else {
                let party_id = runtime.next_party_id;
                runtime.next_party_id = runtime.next_party_id.saturating_add(1);
                let party = PartyGroup {
                    id: party_id,
                    leader_id: inviter.id,
                    loot_type: 0,
                    rare_rate: 0,
                    disp_rare_grade: 0,
                    members: vec![inviter.id, invitee.id],
                };
                runtime.parties.insert(party_id, party.clone());
                runtime.party_by_member.insert(inviter.id, party_id);
                runtime.party_by_member.insert(invitee.id, party_id);
                (party, true, vec![inviter.id])
            }
        };

        let invitee_list_packet = self.build_party_member_list_packet(&party, invitee.id)?;

        if broadcast.is_online(inviter.id) {
            if created_new {
                let _ = broadcast.send_to(
                    inviter.id,
                    &PartyCreatedPacket {
                        party_id: party.id,
                        loot_type: party.loot_type,
                    }
                    .encode(),
                );
            }

            let _ = broadcast.send_to(
                inviter.id,
                &PartyInviteResultPacket {
                    result_type: PARTY_INVITE_ACCEPTED,
                    target_name: invitee.name.clone(),
                }
                .encode(),
            );
        }

        let join_packet = self.build_party_join_packet(&invitee, &inviter)?;
        for member_id in existing_members_before_add {
            if member_id != invitee.id && broadcast.is_online(member_id) {
                let _ = broadcast.send_to(member_id, &join_packet);
            }
        }

        Ok(vec![invitee_list_packet])
    }

    fn handle_party_leave(&self, session: &GameSession) -> Result<Vec<Vec<u8>>, GameFlowError> {
        let character_id = session.character_id.ok_or(GameFlowError::Unauthenticated)?;
        let Some(broadcast) = &self.broadcast else {
            return Ok(vec![]);
        };

        let (party_member_ids, leaving_slot, new_leader_slot, destroy_party) = {
            let mut runtime = self.party_runtime.write().expect("party runtime poisoned");
            let Some(party_id) = runtime.party_by_member.get(&character_id).copied() else {
                return Ok(vec![]);
            };
            let Some(party) = runtime.parties.get_mut(&party_id) else {
                runtime.party_by_member.remove(&character_id);
                return Ok(vec![]);
            };
            let Some(leaving_slot) = party
                .members
                .iter()
                .position(|member_id| *member_id == character_id)
            else {
                runtime.party_by_member.remove(&character_id);
                return Ok(vec![]);
            };

            let recipients = party.members.clone();
            party.members.remove(leaving_slot);

            let mut new_leader_slot = None;
            if party.leader_id == character_id {
                if let Some(new_leader_id) = party.members.first().copied() {
                    party.leader_id = new_leader_id;
                    new_leader_slot = party
                        .members
                        .iter()
                        .position(|member_id| *member_id == new_leader_id)
                        .map(|slot| slot as i32);
                }
            }

            let remaining_members = party.members.clone();
            let destroy_party = party.members.len() < 2;
            let _ = party;
            runtime.party_by_member.remove(&character_id);
            if destroy_party {
                runtime.parties.remove(&party_id);
                for member_id in &remaining_members {
                    runtime.party_by_member.remove(member_id);
                }
            }

            (
                recipients,
                leaving_slot as u8,
                new_leader_slot,
                destroy_party,
            )
        };

        let leave_packet = PartyLeavePacket {
            member_slot: leaving_slot,
        }
        .encode();
        for member_id in party_member_ids {
            if broadcast.is_online(member_id) {
                let _ = broadcast.send_to(member_id, &leave_packet);
                if !destroy_party {
                    if let Some(new_leader_slot) = new_leader_slot {
                        let _ = broadcast.send_to(
                            member_id,
                            &PartyLeaderChangedPacket { new_leader_slot }.encode(),
                        );
                    }
                }
            }
        }

        Ok(vec![])
    }

    fn handle_party_kick(
        &self,
        session: &GameSession,
        target_name: String,
    ) -> Result<Vec<Vec<u8>>, GameFlowError> {
        let requester_id = session.character_id.ok_or(GameFlowError::Unauthenticated)?;
        let Some(target) = self
            .repository
            .character_by_name(&target_name)
            .map_err(|error| GameFlowError::Storage(error.to_string()))?
        else {
            return Ok(vec![]);
        };
        let Some(broadcast) = &self.broadcast else {
            return Ok(vec![]);
        };

        let (party_member_ids, target_slot, destroy_party) = {
            let mut runtime = self.party_runtime.write().expect("party runtime poisoned");
            let Some(party_id) = runtime.party_by_member.get(&requester_id).copied() else {
                return Ok(vec![]);
            };
            let Some(target_party_id) = runtime.party_by_member.get(&target.id).copied() else {
                return Ok(vec![]);
            };
            if party_id != target_party_id {
                return Ok(vec![]);
            }
            let Some(party) = runtime.parties.get_mut(&party_id) else {
                return Ok(vec![]);
            };
            if party.leader_id != requester_id {
                return Ok(vec![]);
            }
            let Some(target_slot) = party
                .members
                .iter()
                .position(|member_id| *member_id == target.id)
            else {
                return Ok(vec![]);
            };
            let recipients = party.members.clone();
            party.members.remove(target_slot);

            let remaining_members = party.members.clone();
            let destroy_party = party.members.len() < 2;
            let _ = party;
            runtime.party_by_member.remove(&target.id);
            if destroy_party {
                runtime.parties.remove(&party_id);
                for member_id in &remaining_members {
                    runtime.party_by_member.remove(member_id);
                }
            }

            (recipients, target_slot as u8, destroy_party)
        };

        let kick_packet = PartyKickPacket {
            member_slot: target_slot,
        }
        .encode();
        for member_id in &party_member_ids {
            if broadcast.is_online(*member_id) {
                let _ = broadcast.send_to(*member_id, &kick_packet);
            }
        }

        let _ = destroy_party;
        Ok(vec![])
    }

    fn handle_party_change_master(
        &self,
        session: &GameSession,
        new_leader_slot: i32,
    ) -> Result<Vec<Vec<u8>>, GameFlowError> {
        let requester_id = session.character_id.ok_or(GameFlowError::Unauthenticated)?;
        let Some(broadcast) = &self.broadcast else {
            return Ok(vec![]);
        };

        let recipients = {
            let mut runtime = self.party_runtime.write().expect("party runtime poisoned");
            let Some(party_id) = runtime.party_by_member.get(&requester_id).copied() else {
                return Ok(vec![]);
            };
            let Some(party) = runtime.parties.get_mut(&party_id) else {
                return Ok(vec![]);
            };
            if party.leader_id != requester_id {
                return Ok(vec![]);
            }
            let Some(new_leader_id) = party.members.get(new_leader_slot.max(0) as usize).copied()
            else {
                return Ok(vec![]);
            };
            party.leader_id = new_leader_id;
            party.members.clone()
        };

        let packet = PartyLeaderChangedPacket { new_leader_slot }.encode();
        for member_id in recipients {
            if broadcast.is_online(member_id) {
                let _ = broadcast.send_to(member_id, &packet);
            }
        }

        Ok(vec![])
    }

    fn handle_party_change_loot(
        &self,
        session: &GameSession,
        loot_type: i32,
        rare_type: u8,
        disp_rare_grade: u8,
    ) -> Result<Vec<Vec<u8>>, GameFlowError> {
        let requester_id = session.character_id.ok_or(GameFlowError::Unauthenticated)?;
        let Some(broadcast) = &self.broadcast else {
            return Ok(vec![]);
        };

        let recipients = {
            let mut runtime = self.party_runtime.write().expect("party runtime poisoned");
            let Some(party_id) = runtime.party_by_member.get(&requester_id).copied() else {
                return Ok(vec![]);
            };
            let Some(party) = runtime.parties.get_mut(&party_id) else {
                return Ok(vec![]);
            };
            if party.leader_id != requester_id {
                return Ok(vec![]);
            }
            party.loot_type = loot_type.max(0) as u32;
            party.rare_rate = rare_type;
            party.disp_rare_grade = disp_rare_grade;
            party.members.clone()
        };

        let packet = PartyChangeLootTypePacket {
            loot_type,
            rare_type,
            disp_rare_grade,
        }
        .encode();
        for member_id in recipients {
            if broadcast.is_online(member_id) {
                let _ = broadcast.send_to(member_id, &packet);
            }
        }

        Ok(vec![])
    }

    pub fn handle_disconnect(&self, session: &GameSession) -> Result<(), GameFlowError> {
        let Some(character_id) = session.character_id else {
            return Ok(());
        };

        let character = self
            .repository
            .character_by_id(character_id)
            .map_err(|error| GameFlowError::Storage(error.to_string()))?
            .ok_or(GameFlowError::CharacterNotFound(character_id))?;

        if session.registered_map_presence {
            self.portal_bridge
                .remove_map_presence(character.map_id, character.channel, character.id)
                .map_err(|error| GameFlowError::PortalBridge(error.to_string()))?;
        }

        self.broadcast_party_member_disconnected(character_id);

        let mut runtime = self.party_runtime.write().expect("party runtime poisoned");
        runtime.pending_invites.retain(|_, invite| {
            invite.inviter_id != character_id && invite.target_id != character_id
        });
        if let Some(party_id) = runtime.party_by_member.remove(&character_id) {
            if let Some(party) = runtime.parties.get_mut(&party_id) {
                party.members.retain(|member_id| *member_id != character_id);
                if party.leader_id == character_id {
                    if let Some(new_leader) = party.members.first().copied() {
                        party.leader_id = new_leader;
                    }
                }
                if party.members.len() < 2 {
                    let members_to_clear = party.members.clone();
                    runtime.parties.remove(&party_id);
                    for member_id in members_to_clear {
                        runtime.party_by_member.remove(&member_id);
                    }
                }
            }
        }

        Ok(())
    }

    fn build_party_member_list_packet(
        &self,
        party: &PartyGroup,
        receiver_id: u64,
    ) -> Result<Vec<u8>, GameFlowError> {
        let receiver = self
            .repository
            .character_by_id(receiver_id)
            .map_err(|error| GameFlowError::Storage(error.to_string()))?
            .ok_or(GameFlowError::CharacterNotFound(receiver_id))?;
        let my_slot = party
            .members
            .iter()
            .position(|member_id| *member_id == receiver_id)
            .map(|index| index as i32)
            .ok_or_else(|| GameFlowError::Storage("receiver not found in party".to_string()))?;
        let leader_slot = party
            .members
            .iter()
            .position(|member_id| *member_id == party.leader_id)
            .map(|index| index as i32)
            .unwrap_or(0);

        let mut members = Vec::new();
        for (index, member_id) in party.members.iter().enumerate() {
            if *member_id == receiver_id {
                continue;
            }
            let member = self
                .repository
                .character_by_id(*member_id)
                .map_err(|error| GameFlowError::Storage(error.to_string()))?
                .ok_or(GameFlowError::CharacterNotFound(*member_id))?;
            members.push(PartyMemberListEntry {
                party_slot: index as i32,
                character: self.party_visible_character(&receiver, &member),
            });
        }

        Ok(PartyMemberListPacket {
            party_id: party.id,
            my_slot,
            leader_slot,
            loot_type: party.loot_type,
            rare_rate: party.rare_rate,
            disp_rare_grade: party.disp_rare_grade,
            members,
        }
        .encode())
    }

    fn build_party_join_packet(
        &self,
        joined_member: &odmo_types::CharacterSummary,
        viewer: &odmo_types::CharacterSummary,
    ) -> Result<Vec<u8>, GameFlowError> {
        let runtime = self.party_runtime.read().expect("party runtime poisoned");
        let Some(party_id) = runtime.party_by_member.get(&joined_member.id).copied() else {
            return Err(GameFlowError::Storage(
                "joined member is not mapped to a party".to_string(),
            ));
        };
        let Some(party) = runtime.parties.get(&party_id) else {
            return Err(GameFlowError::Storage(
                "party mapping exists without party".to_string(),
            ));
        };
        let Some(slot) = party
            .members
            .iter()
            .position(|member_id| *member_id == joined_member.id)
        else {
            return Err(GameFlowError::Storage(
                "joined member missing from party".to_string(),
            ));
        };
        Ok(PartyJoinPacket {
            member: PartyMemberListEntry {
                party_slot: slot as i32,
                character: self.party_visible_character(viewer, joined_member),
            },
        }
        .encode())
    }

    fn party_visible_character(
        &self,
        viewer: &odmo_types::CharacterSummary,
        member: &odmo_types::CharacterSummary,
    ) -> odmo_types::CharacterSummary {
        let mut visible = member.clone();
        if viewer.map_id != member.map_id || viewer.channel != member.channel {
            visible.general_handler = 0;
            visible.partner_handler = 0;
        }
        visible
    }

    fn party_context_for_member(&self, character_id: u64) -> Option<(PartyGroup, usize)> {
        let runtime = self.party_runtime.read().expect("party runtime poisoned");
        let party_id = runtime.party_by_member.get(&character_id).copied()?;
        let party = runtime.parties.get(&party_id)?.clone();
        let slot = party
            .members
            .iter()
            .position(|member_id| *member_id == character_id)?;
        Some((party, slot))
    }

    fn party_other_members(&self, character_id: u64) -> Vec<u64> {
        self.party_context_for_member(character_id)
            .map(|(party, _)| {
                party
                    .members
                    .into_iter()
                    .filter(|member_id| *member_id != character_id)
                    .collect()
            })
            .unwrap_or_default()
    }

    fn broadcast_party_packet(&self, recipients: &[u64], packet: &[u8]) {
        let Some(broadcast) = &self.broadcast else {
            return;
        };
        for member_id in recipients {
            if broadcast.is_online(*member_id) {
                let _ = broadcast.send_to(*member_id, packet);
            }
        }
    }

    fn broadcast_party_member_info(&self, character: &odmo_types::CharacterSummary) {
        let Some((_, slot)) = self.party_context_for_member(character.id) else {
            return;
        };
        let packet = PartyMemberInfoPacket {
            member_slot: slot as u8,
            digimon_type: character.partner_current_type,
            tamer_hp: character.current_hp,
            tamer_max_hp: character.hp,
            tamer_ds: character.current_ds,
            tamer_max_ds: character.ds,
            digimon_hp: character.partner_current_hp,
            digimon_max_hp: character.partner_hp,
            digimon_ds: character.partner_current_ds,
            digimon_max_ds: character.partner_ds,
            tamer_level: u16::from(character.level),
            digimon_level: u16::from(character.partner_level),
        }
        .encode();
        let recipients = self.party_other_members(character.id);
        self.broadcast_party_packet(&recipients, &packet);
    }

    fn broadcast_party_member_position(&self, character: &odmo_types::CharacterSummary) {
        let Some((_, slot)) = self.party_context_for_member(character.id) else {
            return;
        };
        let packet = PartyMemberPositionPacket {
            member_slot: slot as u8,
            tamer_x: character.x,
            tamer_y: character.y,
            digimon_x: character.partner_x,
            digimon_y: character.partner_y,
        }
        .encode();
        let recipients = self.party_other_members(character.id);
        self.broadcast_party_packet(&recipients, &packet);
    }

    fn broadcast_party_member_map_change(&self, character: &odmo_types::CharacterSummary) {
        let Some((_, slot)) = self.party_context_for_member(character.id) else {
            return;
        };
        let recipients = self.party_other_members(character.id);
        let Some(broadcast) = &self.broadcast else {
            return;
        };

        for member_id in recipients {
            if !broadcast.is_online(member_id) {
                continue;
            }
            let Ok(Some(viewer)) = self.repository.character_by_id(member_id) else {
                continue;
            };
            let packet = self.party_member_map_change_packet_for_viewer(&viewer, character, slot);
            let _ = broadcast.send_to(member_id, &packet);
        }
    }

    fn broadcast_party_member_digimon_change(&self, character: &odmo_types::CharacterSummary) {
        let Some((_, slot)) = self.party_context_for_member(character.id) else {
            return;
        };
        let packet = odmo_protocol::PartyMemberDigimonChangePacket {
            member_slot: slot as u8,
            digimon_type: character.partner_current_type,
            digimon_name: character.partner_name.clone(),
            digimon_hp: character.partner_current_hp.clamp(0, u16::MAX as i32) as u16,
            digimon_max_hp: character.partner_hp.clamp(0, u16::MAX as i32) as u16,
            digimon_ds: character.partner_current_ds.clamp(0, u16::MAX as i32) as u16,
            digimon_max_ds: character.partner_ds.clamp(0, u16::MAX as i32) as u16,
        }
        .encode();
        let recipients = self.party_other_members(character.id);
        self.broadcast_party_packet(&recipients, &packet);
    }

    fn broadcast_party_member_buff_change(
        &self,
        character: &odmo_types::CharacterSummary,
        active_buffs: &[odmo_types::ActiveBuffSnapshot],
    ) {
        let Some((_, slot)) = self.party_context_for_member(character.id) else {
            return;
        };
        let packet = PartyMemberBuffChangePacket {
            member_slot: slot as u8,
            buffs: active_buffs
                .iter()
                .map(|buff| PartyMemberBuffEntry {
                    // The modern client packet is delta-shaped. For partner switch we
                    // publish the new active set as present entries and let an empty
                    // list represent "no active party-visible buffs".
                    status: 1,
                    buff_code: buff.buff_id,
                })
                .collect(),
        }
        .encode();
        let recipients = self.party_other_members(character.id);
        self.broadcast_party_packet(&recipients, &packet);
    }

    fn broadcast_party_member_disconnected(&self, character_id: u64) {
        let Some((_, slot)) = self.party_context_for_member(character_id) else {
            return;
        };
        let packet = PartyMemberDisconnectedPacket {
            member_slot: slot as i32,
        }
        .encode();
        let recipients = self.party_other_members(character_id);
        self.broadcast_party_packet(&recipients, &packet);
    }

    fn party_member_map_change_packet_for_viewer(
        &self,
        viewer: &odmo_types::CharacterSummary,
        member: &odmo_types::CharacterSummary,
        member_slot: usize,
    ) -> Vec<u8> {
        let same_map = viewer.map_id == member.map_id && viewer.channel == member.channel;
        PartyMemberMapChangePacket {
            member_slot: member_slot as u8,
            map_id: i32::from(member.map_id),
            channel: i32::from(member.channel),
            tamer_handler: if same_map { member.general_handler } else { 0 },
            digimon_handler: if same_map { member.partner_handler } else { 0 },
        }
        .encode()
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
            return Ok(vec![
                LoadInventoryPacket {
                    inventory: character.extra_inventory,
                    inventory_type: InventoryType::ExtraInventory,
                }
                .encode(),
            ]);
        }

        let source_item = character.extra_inventory.items[ext_idx].clone();
        if source_item.item_id <= 0 || source_item.amount <= 0 {
            return Ok(vec![
                LoadInventoryPacket {
                    inventory: character.extra_inventory,
                    inventory_type: InventoryType::ExtraInventory,
                }
                .encode(),
            ]);
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

        Ok(vec![
            LoadInventoryPacket {
                inventory: updated.extra_inventory,
                inventory_type: InventoryType::ExtraInventory,
            }
            .encode(),
        ])
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

        Ok(vec![
            LoadInventoryPacket {
                inventory: character.extra_inventory,
                inventory_type: InventoryType::ExtraInventory,
            }
            .encode(),
        ])
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
            return Ok(vec![
                LoadInventoryPacket {
                    inventory: character.extra_inventory,
                    inventory_type: InventoryType::ExtraInventory,
                }
                .encode(),
            ]);
        }

        let item = &character.extra_inventory.items[ext_idx];
        if item.item_id <= 0 || item.amount <= 0 {
            return Ok(vec![
                LoadInventoryPacket {
                    inventory: character.extra_inventory,
                    inventory_type: InventoryType::ExtraInventory,
                }
                .encode(),
            ]);
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

        let _ = self
            .repository
            .character_by_id(character_id)?
            .ok_or_else(|| {
                anyhow::anyhow!("character {character_id} not found during mob reconciliation")
            })?;

        let character = self
            .repository
            .character_by_id(character_id)?
            .ok_or_else(|| {
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

        let _ = self
            .repository
            .character_by_id(character_id)?
            .ok_or_else(|| {
                anyhow::anyhow!("character {character_id} not found during drop reconciliation")
            })?;

        let character = self
            .repository
            .character_by_id(character_id)?
            .ok_or_else(|| {
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
        DEFAULT_GM_TAMER_MODEL_ID, DEFAULT_PARTNER_MODEL_ID, DEFAULT_START_MAP_ID, DEFAULT_START_X,
        DEFAULT_START_Y, DEFAULT_TAMER_MODEL_ID, DailyRewardStatus, DropSummary, GameSessionTicket,
        GuildHistoricEntry, GuildMemberSnapshot, GuildSnapshot, MobSummary, RelationEntry,
        SealListSnapshot, SealRecord, XaiSnapshot,
    };

    #[derive(Debug)]
    struct InMemoryCharacterRepository {
        characters: RwLock<HashMap<u64, CharacterSummary>>,
        mobs_by_map: RwLock<HashMap<(i16, u8), Vec<MobSummary>>>,
        drops_by_map: RwLock<HashMap<(i16, u8), Vec<DropSummary>>>,
    }

    #[derive(Debug, Default)]
    struct RecordingBroadcast {
        online: RwLock<std::collections::HashSet<u64>>,
        packets: RwLock<HashMap<u64, Vec<Vec<u8>>>>,
    }

    impl RecordingBroadcast {
        fn with_online(online: impl IntoIterator<Item = u64>) -> Self {
            Self {
                online: RwLock::new(online.into_iter().collect::<std::collections::HashSet<_>>()),
                packets: RwLock::new(HashMap::new()),
            }
        }

        fn packets_for(&self, character_id: u64) -> Vec<Vec<u8>> {
            self.packets
                .read()
                .expect("broadcast poisoned")
                .get(&character_id)
                .cloned()
                .unwrap_or_default()
        }
    }

    impl crate::BroadcastSink for RecordingBroadcast {
        fn send_to(&self, character_id: u64, packet: &[u8]) -> anyhow::Result<()> {
            self.packets
                .write()
                .expect("broadcast poisoned")
                .entry(character_id)
                .or_default()
                .push(packet.to_vec());
            Ok(())
        }

        fn is_online(&self, character_id: u64) -> bool {
            self.online
                .read()
                .expect("broadcast poisoned")
                .contains(&character_id)
        }

        fn send_to_visible(
            &self,
            _map_id: i16,
            _channel: u8,
            _exclude_character_id: u64,
            _packet: &[u8],
        ) -> anyhow::Result<()> {
            Ok(())
        }

        fn update_location(&self, _character_id: u64, _map_id: i16, _channel: u8) {}
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
                                buff_class: 1,
                                skill_id: 8_001_001,
                                remaining_seconds: 120,
                            }],
                            current_xgauge: 500,
                            current_xcrystals: 2,
                            partner_current_slot: 1,
                            partner_slots: vec![
                                odmo_types::PartnerSlotSnapshot {
                                    slot: 1,
                                    digimon_type: DEFAULT_PARTNER_MODEL_ID,
                                    model: DEFAULT_PARTNER_MODEL_ID,
                                    name: "Agumon".to_string(),
                                    ..odmo_types::PartnerSlotSnapshot::default()
                                },
                                odmo_types::PartnerSlotSnapshot {
                                    slot: 2,
                                    digimon_type: 31_002,
                                    model: 31_002,
                                    level: 11,
                                    name: "Greymon".to_string(),
                                    size: 13_000,
                                    hatch_grade: 4,
                                    hp: 1_400,
                                    ds: 1_200,
                                    current_hp: 1_400,
                                    current_ds: 1_200,
                                    de: 120,
                                    at: 150,
                                    fs: 120,
                                    ev: 8,
                                    cc: 5,
                                    ms: 260,
                                    as_value: 950,
                                    ht: 3,
                                    ar: 1,
                                    bl: 2,
                                    clone_level: 3,
                                    clone_at_value: 1,
                                    clone_bl_value: 1,
                                    clone_hp_value: 1,
                                    clone_at_level: 1,
                                    clone_bl_level: 1,
                                    clone_hp_level: 1,
                                    ..odmo_types::PartnerSlotSnapshot::default()
                                },
                            ],
                            partner_active_buffs: vec![ActiveBuffSnapshot {
                                buff_id: 600,
                                buff_class: 1,
                                skill_id: 8_002_001,
                                remaining_seconds: 90,
                            }],
                            partner_active_debuffs: vec![ActiveBuffSnapshot {
                                buff_id: 700,
                                buff_class: 1,
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
                                buff_class: 1,
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
        fn character_by_name(&self, name: &str) -> anyhow::Result<Option<CharacterSummary>> {
            Ok(self
                .characters
                .read()
                .expect("repo poisoned")
                .values()
                .find(|character| character.name.eq_ignore_ascii_case(name))
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
            character_id: u64,
            x: i32,
            y: i32,
            _z: f32,
        ) -> anyhow::Result<()> {
            let mut characters = self.characters.write().expect("repo poisoned");
            if let Some(character) = characters.get_mut(&character_id) {
                character.x = x;
                character.y = y;
            }
            Ok(())
        }
        fn update_partner_position(
            &self,
            character_id: u64,
            x: i32,
            y: i32,
            _z: f32,
        ) -> anyhow::Result<()> {
            let mut characters = self.characters.write().expect("repo poisoned");
            if let Some(character) = characters.get_mut(&character_id) {
                character.partner_x = x;
                character.partner_y = y;
            }
            Ok(())
        }
        fn update_character_map(
            &self,
            character_id: u64,
            map_id: i16,
            x: i32,
            y: i32,
        ) -> anyhow::Result<()> {
            let mut characters = self.characters.write().expect("repo poisoned");
            if let Some(character) = characters.get_mut(&character_id) {
                character.map_id = map_id;
                character.x = x;
                character.y = y;
            }
            Ok(())
        }
        fn switch_partner(
            &self,
            character_id: u64,
            slot: u8,
        ) -> anyhow::Result<Option<CharacterSummary>> {
            let mut characters = self.characters.write().expect("repo poisoned");
            let Some(character) = characters.get_mut(&character_id) else {
                return Ok(None);
            };
            let Some(target) = character
                .partner_slots
                .iter()
                .find(|partner| partner.slot == slot)
                .cloned()
            else {
                return Ok(None);
            };
            if character.partner_current_slot == slot {
                return Ok(Some(character.clone()));
            }
            character.partner_current_slot = slot;
            character.partner_current_type = target.digimon_type;
            character.partner_model = target.model;
            character.partner_name = target.name;
            character.partner_level = target.level;
            character.partner_size = target.size;
            character.partner_hatch_grade = target.hatch_grade;
            character.partner_hp = target.hp;
            character.partner_ds = target.ds;
            character.partner_current_hp = target.current_hp;
            character.partner_current_ds = target.current_ds;
            character.partner_de = target.de;
            character.partner_at = target.at;
            character.partner_fs = target.fs;
            character.partner_ev = target.ev;
            character.partner_cc = target.cc;
            character.partner_ms = target.ms;
            character.partner_as = target.as_value;
            character.partner_ht = target.ht;
            character.partner_ar = target.ar;
            character.partner_bl = target.bl;
            character.partner_clone_level = target.clone_level;
            character.partner_clone_at_value = target.clone_at_value;
            character.partner_clone_bl_value = target.clone_bl_value;
            character.partner_clone_ct_value = target.clone_ct_value;
            character.partner_clone_ev_value = target.clone_ev_value;
            character.partner_clone_hp_value = target.clone_hp_value;
            character.partner_clone_at_level = target.clone_at_level;
            character.partner_clone_bl_level = target.clone_bl_level;
            character.partner_clone_ct_level = target.clone_ct_level;
            character.partner_clone_ev_level = target.clone_ev_level;
            character.partner_clone_hp_level = target.clone_hp_level;
            character.partner_active_buffs = target.active_buffs;
            Ok(Some(character.clone()))
        }
        fn update_inventory(
            &self,
            character_id: u64,
            inventory: odmo_types::InventorySnapshot,
        ) -> anyhow::Result<()> {
            let mut characters = self.characters.write().expect("repo poisoned");
            if let Some(character) = characters.get_mut(&character_id) {
                character.inventory = inventory;
            }
            Ok(())
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
        fn update_welcome_flag(&self, _account_id: u64, _welcome: bool) -> anyhow::Result<()> {
            Ok(())
        }
    }

    impl PortalRepository for InMemoryCharacterRepository {
        fn portal_by_id(&self, portal_id: i32) -> anyhow::Result<Option<PortalDefinition>> {
            Ok(match portal_id {
                10001 => Some(PortalDefinition {
                    id: 10001,
                    is_local: false,
                    destination_map_id: 102,
                    destination_x: 32615,
                    destination_y: 14930,
                }),
                10002 => Some(PortalDefinition {
                    id: 10002,
                    is_local: false,
                    destination_map_id: 3,
                    destination_x: 18086,
                    destination_y: 18874,
                }),
                _ => None,
            })
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

    fn establish_party(
        app: &GameApplication,
        inviter_id: u64,
        invitee_id: u64,
        invitee_name: &str,
        inviter_name: &str,
    ) {
        let mut inviter_session = GameSession::new(1);
        inviter_session.character_id = Some(inviter_id);
        app.handle_request(
            &mut inviter_session,
            GameRequest::PartyInvite {
                target_name: invitee_name.to_string(),
            },
        )
        .expect("invite should be delivered");

        let mut invitee_session = GameSession::new(2);
        invitee_session.character_id = Some(invitee_id);
        app.handle_request(
            &mut invitee_session,
            GameRequest::PartyInviteResponse {
                result_type: PARTY_INVITE_ACCEPTED,
                inviter_name: inviter_name.to_string(),
            },
        )
        .expect("accept should create party");
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

    #[test]
    fn party_invite_accept_bootstraps_party_contract() {
        let repo = Arc::new(InMemoryCharacterRepository::demo());
        let broadcast = Arc::new(RecordingBroadcast::with_online([100, 200]));
        let app = GameApplication::new(
            GameServiceConfig {
                portal_state_dir: unique_test_dir("party-accept"),
            },
            repo,
        )
        .with_broadcast(broadcast.clone());

        let mut inviter_session = GameSession::new(1);
        inviter_session.character_id = Some(100);

        let invite_responses = app
            .handle_request(
                &mut inviter_session,
                GameRequest::PartyInvite {
                    target_name: "Matt".to_string(),
                },
            )
            .expect("invite should be accepted for online target");
        assert!(
            invite_responses.is_empty(),
            "invite sender should not receive immediate local packets on success",
        );

        let invite_packets = broadcast.packets_for(200);
        assert_eq!(
            invite_packets.len(),
            1,
            "target should receive one invite packet"
        );
        assert_eq!(
            PacketReader::from_frame(&invite_packets[0])
                .expect("invite frame should decode")
                .packet_type,
            odmo_protocol::opcode::game::PARTY_INVITE,
        );

        let mut target_session = GameSession::new(2);
        target_session.character_id = Some(200);

        let responses = app
            .handle_request(
                &mut target_session,
                GameRequest::PartyInviteResponse {
                    result_type: PARTY_INVITE_ACCEPTED,
                    inviter_name: "AdminTamer".to_string(),
                },
            )
            .expect("accept should create party");
        assert_eq!(
            responses.len(),
            1,
            "invitee should receive party member list"
        );
        assert_eq!(
            PacketReader::from_frame(&responses[0])
                .expect("member-list frame should decode")
                .packet_type,
            odmo_protocol::opcode::game::PARTY_MEMBER_LIST,
        );

        let inviter_packets = broadcast.packets_for(100);
        let inviter_types: Vec<i16> = inviter_packets
            .iter()
            .map(|frame| {
                PacketReader::from_frame(frame)
                    .expect("leader frame should decode")
                    .packet_type
            })
            .collect();
        assert!(
            inviter_types.contains(&odmo_protocol::opcode::game::PARTY_CREATED),
            "leader should receive party-created packet",
        );
        assert!(
            inviter_types.contains(&odmo_protocol::opcode::game::PARTY_INVITE_RESPONSE),
            "leader should receive invite-result packet",
        );
        assert!(
            inviter_types.contains(&odmo_protocol::opcode::game::PARTY_JOIN),
            "leader should receive join packet for the new member",
        );
    }

    #[test]
    fn party_invite_reject_notifies_inviter() {
        let repo = Arc::new(InMemoryCharacterRepository::demo());
        let broadcast = Arc::new(RecordingBroadcast::with_online([100, 200]));
        let app = GameApplication::new(
            GameServiceConfig {
                portal_state_dir: unique_test_dir("party-reject"),
            },
            repo,
        )
        .with_broadcast(broadcast.clone());

        let mut inviter_session = GameSession::new(1);
        inviter_session.character_id = Some(100);
        app.handle_request(
            &mut inviter_session,
            GameRequest::PartyInvite {
                target_name: "Matt".to_string(),
            },
        )
        .expect("invite should be delivered");

        let mut target_session = GameSession::new(2);
        target_session.character_id = Some(200);
        let responses = app
            .handle_request(
                &mut target_session,
                GameRequest::PartyInviteResponse {
                    result_type: PARTY_INVITE_REJECTED,
                    inviter_name: "AdminTamer".to_string(),
                },
            )
            .expect("reject should notify inviter");
        assert!(
            responses.is_empty(),
            "reject should not send local packets to invitee"
        );

        let inviter_packets = broadcast.packets_for(100);
        let inviter_types: Vec<i16> = inviter_packets
            .iter()
            .map(|frame| {
                PacketReader::from_frame(frame)
                    .expect("leader frame should decode")
                    .packet_type
            })
            .collect();
        assert!(
            inviter_types.contains(&odmo_protocol::opcode::game::PARTY_INVITE_RESPONSE),
            "inviter should receive reject result packet",
        );
    }

    #[test]
    fn party_leave_notifies_members() {
        let repo = Arc::new(InMemoryCharacterRepository::demo());
        let broadcast = Arc::new(RecordingBroadcast::with_online([100, 200]));
        let app = GameApplication::new(
            GameServiceConfig {
                portal_state_dir: unique_test_dir("party-leave"),
            },
            repo,
        )
        .with_broadcast(broadcast.clone());

        establish_party(&app, 100, 200, "Matt", "AdminTamer");

        let mut invitee_session = GameSession::new(2);
        invitee_session.character_id = Some(200);
        app.handle_request(&mut invitee_session, GameRequest::PartyLeave)
            .expect("leave should succeed");

        let inviter_types: Vec<i16> = broadcast
            .packets_for(100)
            .iter()
            .map(|frame| PacketReader::from_frame(frame).expect("frame").packet_type)
            .collect();
        assert!(
            inviter_types.contains(&odmo_protocol::opcode::game::PARTY_LEAVE),
            "remaining member should receive leave packet",
        );
    }

    #[test]
    fn party_kick_notifies_target_and_members() {
        let repo = Arc::new(InMemoryCharacterRepository::demo());
        let broadcast = Arc::new(RecordingBroadcast::with_online([100, 200]));
        let app = GameApplication::new(
            GameServiceConfig {
                portal_state_dir: unique_test_dir("party-kick"),
            },
            repo,
        )
        .with_broadcast(broadcast.clone());

        establish_party(&app, 100, 200, "Matt", "AdminTamer");

        let mut leader_session = GameSession::new(1);
        leader_session.character_id = Some(100);
        app.handle_request(
            &mut leader_session,
            GameRequest::PartyKick {
                target_name: "Matt".to_string(),
            },
        )
        .expect("kick should succeed");

        let target_types: Vec<i16> = broadcast
            .packets_for(200)
            .iter()
            .map(|frame| PacketReader::from_frame(frame).expect("frame").packet_type)
            .collect();
        assert!(
            target_types.contains(&odmo_protocol::opcode::game::PARTY_KICK),
            "kicked member should receive kick packet",
        );
    }

    #[test]
    fn party_change_master_broadcasts_slot() {
        let repo = Arc::new(InMemoryCharacterRepository::demo());
        let broadcast = Arc::new(RecordingBroadcast::with_online([100, 200]));
        let app = GameApplication::new(
            GameServiceConfig {
                portal_state_dir: unique_test_dir("party-master"),
            },
            repo,
        )
        .with_broadcast(broadcast.clone());

        establish_party(&app, 100, 200, "Matt", "AdminTamer");

        let mut leader_session = GameSession::new(1);
        leader_session.character_id = Some(100);
        app.handle_request(
            &mut leader_session,
            GameRequest::PartyChangeMaster { new_leader_slot: 1 },
        )
        .expect("leader change should succeed");

        let invitee_types: Vec<i16> = broadcast
            .packets_for(200)
            .iter()
            .map(|frame| PacketReader::from_frame(frame).expect("frame").packet_type)
            .collect();
        assert!(
            invitee_types.contains(&odmo_protocol::opcode::game::PARTY_CHANGE_MASTER),
            "members should receive leader-changed packet",
        );
    }

    #[test]
    fn party_change_loot_broadcasts_rule() {
        let repo = Arc::new(InMemoryCharacterRepository::demo());
        let broadcast = Arc::new(RecordingBroadcast::with_online([100, 200]));
        let app = GameApplication::new(
            GameServiceConfig {
                portal_state_dir: unique_test_dir("party-loot"),
            },
            repo,
        )
        .with_broadcast(broadcast.clone());

        establish_party(&app, 100, 200, "Matt", "AdminTamer");

        let mut leader_session = GameSession::new(1);
        leader_session.character_id = Some(100);
        app.handle_request(
            &mut leader_session,
            GameRequest::PartyChangeLoot {
                loot_type: 2,
                rare_type: 3,
                disp_rare_grade: 4,
            },
        )
        .expect("loot change should succeed");

        let invitee_types: Vec<i16> = broadcast
            .packets_for(200)
            .iter()
            .map(|frame| PacketReader::from_frame(frame).expect("frame").packet_type)
            .collect();
        assert!(
            invitee_types.contains(&odmo_protocol::opcode::game::PARTY_CHANGE_LOOT),
            "members should receive loot-change packet",
        );
    }

    #[test]
    fn consume_item_broadcasts_party_member_info() {
        let repo = Arc::new(InMemoryCharacterRepository::demo());
        {
            let mut characters = repo.characters.write().expect("repo poisoned");
            let character = characters.get_mut(&100).expect("leader should exist");
            if character.inventory.items.is_empty() {
                character
                    .inventory
                    .items
                    .resize(1, odmo_types::ItemRecord::default());
            }
            character.inventory.items[0] = odmo_types::ItemRecord {
                item_id: 5101,
                amount: 1,
                ..odmo_types::ItemRecord::default()
            };
            character.inventory.items[0].sync_record();
        }
        let broadcast = Arc::new(RecordingBroadcast::with_online([100, 200]));
        let app = GameApplication::new(
            GameServiceConfig {
                portal_state_dir: unique_test_dir("party-info"),
            },
            repo,
        )
        .with_broadcast(broadcast.clone());

        establish_party(&app, 100, 200, "Matt", "AdminTamer");

        let mut leader_session = GameSession::new(1);
        leader_session.character_id = Some(100);
        app.handle_request(
            &mut leader_session,
            GameRequest::ConsumeItem {
                target_handler: 0,
                slot: 0,
            },
        )
        .expect("consume should succeed");

        let info_packet = broadcast
            .packets_for(200)
            .into_iter()
            .find(|frame| {
                PacketReader::from_frame(frame)
                    .map(|raw| raw.packet_type == odmo_protocol::opcode::game::PARTY_MEMBER_INFO)
                    .unwrap_or(false)
            })
            .expect("party member info should be broadcast");
        let raw = PacketReader::from_frame(&info_packet).expect("frame");
        let mut payload = PacketReader::new(raw.payload);
        assert_eq!(payload.read_u8().expect("slot"), 0);
        assert_eq!(payload.read_i32().expect("digimon type"), 31_001);
    }

    #[test]
    fn movement_broadcasts_party_member_position() {
        let repo = Arc::new(InMemoryCharacterRepository::demo());
        let broadcast = Arc::new(RecordingBroadcast::with_online([100, 200]));
        let app = GameApplication::new(
            GameServiceConfig {
                portal_state_dir: unique_test_dir("party-position"),
            },
            repo,
        )
        .with_broadcast(broadcast.clone());

        establish_party(&app, 100, 200, "Matt", "AdminTamer");

        let mut leader_session = GameSession::new(1);
        leader_session.character_id = Some(100);
        app.handle_request(
            &mut leader_session,
            GameRequest::TamerMovimentation {
                ticks: 0,
                handler: 32_767,
                x: 4321,
                y: 5432,
                z: 0.0,
            },
        )
        .expect("movement should succeed");

        let position_packet = broadcast
            .packets_for(200)
            .into_iter()
            .find(|frame| {
                PacketReader::from_frame(frame)
                    .map(|raw| {
                        raw.packet_type == odmo_protocol::opcode::game::PARTY_MEMBER_POSITION
                    })
                    .unwrap_or(false)
            })
            .expect("party member position should be broadcast");
        let raw = PacketReader::from_frame(&position_packet).expect("frame");
        let mut payload = PacketReader::new(raw.payload);
        assert_eq!(payload.read_u8().expect("slot"), 0);
        assert_eq!(payload.read_i32().expect("x"), 4321);
        assert_eq!(payload.read_i32().expect("y"), 5432);
    }

    #[test]
    fn warp_gate_broadcasts_party_member_map_change() {
        let repo = Arc::new(InMemoryCharacterRepository::demo());
        let broadcast = Arc::new(RecordingBroadcast::with_online([100, 200]));
        let app = GameApplication::new(
            GameServiceConfig {
                portal_state_dir: unique_test_dir("party-map-change"),
            },
            repo,
        )
        .with_broadcast(broadcast.clone());

        establish_party(&app, 100, 200, "Matt", "AdminTamer");

        let mut leader_session = GameSession::new(1);
        leader_session.character_id = Some(100);
        app.handle_request(
            &mut leader_session,
            GameRequest::WarpGate { portal_id: 10001 },
        )
        .expect("warp should succeed");

        let map_packet = broadcast
            .packets_for(200)
            .into_iter()
            .find(|frame| {
                PacketReader::from_frame(frame)
                    .map(|raw| {
                        raw.packet_type == odmo_protocol::opcode::game::PARTY_MEMBER_MAP_CHANGE
                    })
                    .unwrap_or(false)
            })
            .expect("party member map change should be broadcast");
        let raw = PacketReader::from_frame(&map_packet).expect("frame");
        let mut payload = PacketReader::new(raw.payload);
        assert_eq!(payload.read_u8().expect("slot"), 0);
        assert_eq!(payload.read_i32().expect("map"), 102);
    }

    #[test]
    fn disconnect_broadcasts_party_member_disconnected_even_without_map_presence() {
        let repo = Arc::new(InMemoryCharacterRepository::demo());
        let broadcast = Arc::new(RecordingBroadcast::with_online([100, 200]));
        let app = GameApplication::new(
            GameServiceConfig {
                portal_state_dir: unique_test_dir("party-disconnected"),
            },
            repo,
        )
        .with_broadcast(broadcast.clone());

        establish_party(&app, 100, 200, "Matt", "AdminTamer");

        let mut invitee_session = GameSession::new(2);
        invitee_session.character_id = Some(200);
        app.handle_disconnect(&invitee_session)
            .expect("disconnect should clean party runtime");

        let disconnect_packet = broadcast
            .packets_for(100)
            .into_iter()
            .find(|frame| {
                PacketReader::from_frame(frame)
                    .map(|raw| {
                        raw.packet_type == odmo_protocol::opcode::game::PARTY_MEMBER_DISCONNECTED
                    })
                    .unwrap_or(false)
            })
            .expect("party member disconnected should be broadcast");
        let raw = PacketReader::from_frame(&disconnect_packet).expect("frame");
        let mut payload = PacketReader::new(raw.payload);
        assert_eq!(payload.read_i32().expect("slot"), 1);
    }

    #[test]
    fn partner_switch_broadcasts_party_member_digimon_change() {
        let repo = Arc::new(InMemoryCharacterRepository::demo());
        let broadcast = Arc::new(RecordingBroadcast::with_online([100, 200]));
        let app = GameApplication::new(
            GameServiceConfig {
                portal_state_dir: unique_test_dir("party-partner-switch"),
            },
            repo,
        )
        .with_broadcast(broadcast.clone());

        establish_party(&app, 100, 200, "Matt", "AdminTamer");

        let mut leader_session = GameSession::new(1);
        leader_session.character_id = Some(100);
        let responses = app
            .handle_request(&mut leader_session, GameRequest::PartnerSwitch { slot: 2 })
            .expect("partner switch should succeed");

        assert!(
            responses.iter().any(|frame| {
                PacketReader::from_frame(frame)
                    .map(|raw| raw.packet_type == odmo_protocol::opcode::game::UPDATE_STATUS)
                    .unwrap_or(false)
            }),
            "partner switch should refresh local status",
        );

        let change_packet = broadcast
            .packets_for(200)
            .into_iter()
            .find(|frame| {
                PacketReader::from_frame(frame)
                    .map(|raw| {
                        raw.packet_type == odmo_protocol::opcode::game::PARTY_MEMBER_DIGIMON_CHANGE
                    })
                    .unwrap_or(false)
            })
            .expect("party digimon change should be broadcast");
        let raw = PacketReader::from_frame(&change_packet).expect("frame");
        let mut payload = PacketReader::new(raw.payload);
        assert_eq!(payload.read_u8().expect("slot"), 0);
        assert_eq!(payload.read_i32().expect("type"), 31_002);

        let buff_packet = broadcast
            .packets_for(200)
            .into_iter()
            .find(|frame| {
                PacketReader::from_frame(frame)
                    .map(|raw| {
                        raw.packet_type == odmo_protocol::opcode::game::PARTY_MEMBER_BUFF_CHANGE
                    })
                    .unwrap_or(false)
            })
            .expect("party buff change should be broadcast");
        let raw = PacketReader::from_frame(&buff_packet).expect("frame");
        let mut payload = PacketReader::new(raw.payload);
        assert_eq!(payload.read_u8().expect("slot"), 0);
        assert_eq!(payload.read_u16().expect("buff count"), 0);
    }

    #[test]
    fn partner_switch_invalid_slot_uses_modern_failure_contract() {
        let repo = Arc::new(InMemoryCharacterRepository::demo());
        let app = GameApplication::new(
            GameServiceConfig {
                portal_state_dir: unique_test_dir("partner-switch-fail"),
            },
            repo,
        );

        let mut session = GameSession::new(1);
        session.character_id = Some(100);
        let responses = app
            .handle_request(&mut session, GameRequest::PartnerSwitch { slot: 9 })
            .expect("failure packet should still be returned");

        assert_eq!(responses.len(), 1);
        let raw = PacketReader::from_frame(&responses[0]).expect("frame");
        assert_eq!(
            raw.packet_type,
            odmo_protocol::opcode::game::PARTNER_SWITCH_RESPONSE
        );
        let mut payload = PacketReader::new(raw.payload);
        assert_eq!(payload.read_u32().expect("uid"), 0);
    }

    #[test]
    fn partner_evolution_currently_returns_modern_failure_packet() {
        let repo = Arc::new(InMemoryCharacterRepository::demo());
        let app = GameApplication::new(
            GameServiceConfig {
                portal_state_dir: unique_test_dir("partner-evolution-fail"),
            },
            repo,
        );

        let mut session = GameSession::new(1);
        session.character_id = Some(100);
        let responses = app
            .handle_request(
                &mut session,
                GameRequest::PartnerEvolution {
                    digimon_handler: 21_000,
                    evolution_slot: 4,
                },
            )
            .expect("request should complete");

        assert_eq!(responses.len(), 1);
        let raw = PacketReader::from_frame(&responses[0]).expect("frame");
        assert_eq!(
            raw.packet_type,
            odmo_protocol::opcode::game::EVOLUTION_FAILURE
        );
    }
}
