use odmo_types::{
    ActiveBuffSnapshot, AttendanceStatus, ChannelAvailability, CharacterSummary, DailyRewardStatus,
    DropSummary, GuildHistoricEntry, GuildSnapshot, InventorySnapshot, ItemRecord, MobSummary,
    RelationEntry, SealListSnapshot, XaiSnapshot,
};

use crate::{
    error::ProtocolError,
    opcode::game,
    reader::{PacketReader, RawPacket},
    writer::PacketWriter,
};

#[derive(Debug, Clone, PartialEq)]
pub enum GameRequest {
    Connection {
        kind: u8,
    },
    KeepConnection,
    InitialInformation {
        account_id: u64,
        access_code: u32,
    },
    ComplementarInformation,
    TamerMovimentation {
        ticks: u32,
        handler: u32,
        x: i32,
        y: i32,
        z: f32,
    },
    WarpGate {
        portal_id: i32,
    },
    ConsumeItem {
        target_handler: i32,
        slot: u16,
    },
    MoveItem {
        origin_slot: u16,
        destination_slot: u16,
    },
    SplitItem {
        origin_slot: u16,
        destination_slot: u16,
        amount: u16,
    },
    RemoveItem {
        slot: u16,
        x: i32,
        y: i32,
        amount: u16,
    },
    NpcPurchase {
        npc_id: i32,
        unk: u8,
        shop_slot: i32,
        purchase_count: u16,
    },
    NpcSell {
        npc_id: i32,
        unk: u8,
        item_slot: u8,
        sell_amount: u16,
    },
    LootItem {
        drop_handler: u32,
    },
    DigiSummonSyncRequest,
    ChannelInfo,
    Membership,
    Emoticon {
        emoticon_type: i32,
        value: i32,
    },
    FriendlyInfo {
        target_handler: u32,
    },
    FriendlyMark,
    ExtraInventoryMove {
        category: u16,
        extra_slot: u16,
        inventory_slot: u16,
    },
    ExtraInventoryBatchMove {
        category: u8,
    },
    ExtraInventorySort {
        category: u8,
    },
    ExtraInventoryUse {
        category: u8,
        extra_slot: u16,
    },
    ChatMessage {
        message: String,
    },
    WhisperMessage {
        target_name: String,
        message: String,
    },
    ShoutMessage {
        message: String,
    },
    MegaphoneMessage {
        message: String,
        item_slot: i32,
    },
    TamerReaction {
        reaction_type: i32,
    },
    PartnerStop {
        uid: u32,
    },
    PartnerEvolution {
        digimon_handler: u32,
        evolution_slot: u8,
    },
    PartnerSwitch {
        slot: u8,
    },
    PartnerDelete {
        slot: u8,
        validation: String,
    },
    EvolutionUnlock {
        evolution_type: i32,
        inven_idx: Option<i16>,
    },
    RideModeStart {
        evolution_type: i32,
        item_type: i32,
    },
    RideModeStop,
    DigimonChangeName {
        inven_slot: i32,
        new_name: String,
    },
    HatchInsertEgg {
        vip: u8,
        inven_slot: u16,
        npc_idx: i32,
    },
    HatchIncrease {
        vip: u8,
        npc_idx: i32,
        data_level: i8,
    },
    HatchFinish {
        vip: u8,
        portable_pos: u32,
        name: String,
        npc_idx: i32,
    },
    HatchRemoveEgg {
        vip: u8,
        npc_idx: i32,
    },
    HatchBackupInsert {
        vip: u8,
        inven_slot: u16,
        npc_idx: i32,
    },
    HatchBackupCancel {
        vip: u8,
        npc_idx: i32,
    },
    IncubatorClose,
    DigimonArchiveMove {
        vip: u8,
        slot1: i32,
        slot2: i32,
        npc_type: u32,
    },
    DigimonArchiveList {
        vip: u8,
        inven_idx: u32,
        npc_type: u32,
    },
    DigimonArchiveSwap {
        npc_idx: u32,
        archive_type: i32,
        src_arr: u8,
        dst_arr: u8,
    },
    InventorySort {
        sort_type: u8,
    },
    ItemIdentify {
        item_slot: i16,
    },
    ItemCraft {
        recipe_slot: i16,
    },
    ItemReroll {
        item_slot: i16,
    },
    ItemSocketIn {
        item_slot: i16,
        socket_slot: u8,
        chip_item_id: i32,
    },
    ItemSocketOut {
        item_slot: i16,
        socket_slot: u8,
    },
    ItemSocketIdentify {
        item_slot: i16,
    },
    ItemReturn {
        item_slot: i16,
    },
    ItemScan {
        item_slot: i16,
    },
    LoadGiftStorage,
    GiftStorageRetrieve {
        item_slot: i16,
    },
    LoadRewardStorage,
    RecompenseGain {
        reward_id: i32,
    },
    TamerShopOpen,
    TamerShopClose,
    TamerShopBuy {
        item_id: i32,
        amount: i16,
    },
    ConsignedShopOpen,
    ConsignedShopView {
        shop_id: i32,
    },
    ConsignedShopPurchase {
        item_id: i32,
        amount: i16,
    },
    ConsignedShopRetrieve {
        item_slot: i16,
    },
    CashShopOpen,
    CashShopBuy {
        amount: u8,
        total_price: i32,
        order_id: u16,
        product_ids: Vec<i32>,
    },
    CashShopReload,
    QuestAvailableList,
    QuestAccept {
        quest_id: i32,
    },
    QuestDeliver {
        quest_id: i32,
    },
    QuestGiveUp {
        quest_id: i32,
    },
    QuestUpdate {
        quest_id: i32,
        progress: i32,
    },
    DieConfirm,
    RemoveBuff {
        buff_id: i32,
    },
    DamageSkinChange {
        skin_id: i32,
    },
    SealOpen {
        seal_idx: i16,
    },
    SealClose {
        seal_idx: i16,
    },
    SealSetLeader {
        card_code: u16,
    },
    SealRemoveLeader,
    SealSetFavorite {
        card_code: u16,
        bookmark: u8,
    },
    EncyclopediaLoad,
    EncyclopediaGetReward {
        digimon_id: u32,
    },
    EncyclopediaDeckBuff {
        deck_idx: u32,
    },
    ArenaDailyPoints {
        item_slot: i16,
        points: i16,
        item_id: i16,
    },
    ArenaDailyRanking,
    ArenaRankingAll {
        ranking_type: u8,
    },
    ArenaRequestRank {
        ranking_type: u8,
    },
    ArenaRequestOldRank {
        ranking_type: u8,
    },
    DungeonNextStage,
    DungeonSurrender,
    BurningEvent,
    DailyCheckEvent,
    DailyCheckEventRequest {
        event_no: i32,
    },
    JoinEventQueue {
        event_id: i32,
    },
    RegionUnlock {
        region_idx: i16,
    },
    SetTitle {
        title_id: i16,
    },
    ChangeTamerModel {
        model_id: i32,
    },
    TamerNameChange {
        new_name: String,
    },
    RareMachineOpen {
        npc_idx: u32,
    },
    RareMachineRun {
        npc_idx: u32,
        inven_idx: u32,
        reset_count: u32,
    },
    PartyInvite {
        target_name: String,
    },
    PartyInviteResponse {
        result_type: i32,
        inviter_name: String,
    },
    PartyChat {
        message: String,
    },
    PartyKick {
        target_name: String,
    },
    PartyLeave,
    PartyChangeMaster {
        new_leader_slot: i32,
    },
    PartyChangeLoot {
        loot_type: i32,
        rare_type: u8,
        disp_rare_grade: u8,
    },
    PartyDismiss,
    GuildCreate {
        guild_name: String,
    },
    GuildDelete,
    GuildInvite {
        target_name: String,
    },
    GuildInviteAccept {
        guild_id: i32,
    },
    GuildInviteDeny {
        guild_id: i32,
    },
    GuildKick {
        member_id: i32,
    },
    GuildLeave,
    GuildMessage {
        message: String,
    },
    GuildNotice {
        notice: String,
    },
    GuildHistory,
    GuildSetTitle {
        member_id: i32,
        title: String,
    },
    TradeRequest {
        target_handler: u32,
    },
    TradeAccept {
        accepter_handler: u32,
    },
    TradeCancel,
    TradeAddItem {
        item_slot: i16,
        trade_slot: u8,
    },
    TradeRemoveItem {
        trade_slot: u8,
    },
    TradeAddMoney {
        amount: i32,
    },
    TradeConfirm,
    TradeLock,
    TradeUnlock,
    SeasonPassDetails,
    SeasonPassPurchaseExp {
        purchase_count: i32,
    },
    SeasonPassMissionReward {
        mission_id: i32,
    },
    SeasonPassSeasonReward {
        level: i32,
    },
    ChangeChannel {
        channel: u8,
    },
    ChannelSwitchConfirm,
    TamerShopList,
    ConsignedWarehouse,
    ConsignedWarehouseRetrieve {
        item_slot: i16,
    },
    CashShopBuyHistory,
    AddFriend {
        friend_name: String,
    },
    FriendList,
    GuildAuthorityMaster {
        member_id: i32,
    },
    GuildAuthoritySubMaster {
        member_id: i32,
    },
    GuildAuthorityMember {
        member_id: i32,
    },
    GuildAuthorityNewMember {
        member_id: i32,
    },
    GuildAuthorityDats {
        member_id: i32,
    },
    HatchSpiritEvolution {
        model_id: i32,
        name: String,
        npc_id: i32,
    },
    DigiSummonPurchase {
        npc_idx: u32,
    },
    LoadAccountWarehouse,
    RetrieveAccountWarehouse {
        item_slot: i16,
    },
    ExtraInventoryCategoryRefresh {
        category: u8,
    },
    PartyConfigChange {
        loot_type: u8,
    },
    PartyMemberDisconnect,
    MonsterRespawnTimer,
    JumpBooster,
    SkillLevelUp {
        uid: u32,
        evo_unit_idx: u8,
        skill_idx: u8,
    },
    TamerChargeXCrystal,
    TamerConsumeXCrystal {
        amount: i32,
    },
    TamerSummon {
        target_name: String,
    },
    TamerSkillRequest {
        skill_idx: u32,
        target_uid: u32,
    },
    TranscendenceReceiveExp,
    TranscendenceSuccess,
    TimeChargeResult {
        charge_type: u8,
    },
    WarpGateDungeon,
    SpiritCraft {
        model_id: i32,
        name: String,
        npc_id: i32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameConnectionPacket {
    pub handshake: i16,
}

impl GameConnectionPacket {
    /// The proactive handshake sent on TCP connect.
    /// Uses opcode -1 (65535 as u16).
    /// Proactive handshake sent on TCP connect (opcode -1 / 65535).
    /// Wire: [FF,FF] [HS] [0,0] [0,0] [0,0] [0,0] [0,0] [CK] = 18 bytes
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(-1);
        writer.write_i16(self.handshake);
        writer.write_i16(0);
        writer.write_i16(0);
        writer.write_i16(0);
        writer.write_i16(0);
        writer.write_i16(0);
        writer.finalize() // adds 1 zero→checksum, total = 18 bytes
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GameInitialInfoPacket {
    pub character: CharacterSummary,
}

impl GameInitialInfoPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::INITIAL_INFO_RESPONSE);
        writer.write_i32(1);
        writer.write_i32(self.character.x);
        writer.write_i32(self.character.y);
        writer.write_i32(non_zero_handler(
            self.character.general_handler,
            self.character.id,
        ));
        writer.write_u8(1);
        writer.write_i32(self.character.model);
        writer.write_string(&self.character.name);
        writer.write_i64(self.character.current_experience.saturating_mul(100));
        writer.write_i16(self.character.level as i16);
        writer.write_i32(self.character.hp);
        writer.write_i32(self.character.ds);
        writer.write_i32(self.character.current_hp);
        writer.write_i32(self.character.current_ds);
        writer.write_i32(self.character.fatigue);
        writer.write_i32(self.character.at);
        writer.write_i32(self.character.de);
        writer.write_i32(self.character.ms);
        write_empty_item_records(&mut writer, 16);
        write_empty_item_records(&mut writer, 12);
        write_empty_item_records(&mut writer, 1);
        write_empty_item_records(&mut writer, 5);
        writer.write_zeroes(1_292);
        writer.write_i32(0);
        writer.write_i32(0);
        writer.write_i32(-1);
        writer.write_i32(0);
        writer.write_i32(-1);
        writer.write_i64(self.character.inventory_bits);
        writer.write_u16(self.character.inventory_size);
        writer.write_u16(self.character.warehouse_size);
        writer.write_i16(0);
        writer.write_u8(self.character.digimon_slots);
        writer.write_i32(non_zero_handler(
            self.character.partner_handler,
            self.character.id.saturating_add(10_000),
        ));
        writer.write_i32(self.character.partner_model);
        writer.write_string(&self.character.partner_name);
        writer.write_u8(self.character.partner_hatch_grade);
        writer.write_i16(self.character.partner_size);
        writer.write_i64(
            self.character
                .partner_current_experience
                .saturating_mul(100),
        );
        writer.write_i64(self.character.partner_transcendence_experience);
        writer.write_i16(self.character.partner_level as i16);
        writer.write_i32(self.character.partner_hp);
        writer.write_i32(self.character.partner_ds);
        writer.write_i32(self.character.partner_de);
        writer.write_i32(self.character.partner_at);
        writer.write_i32(self.character.partner_current_hp);
        writer.write_i32(self.character.partner_current_ds);
        writer.write_i32(self.character.partner_fs);
        writer.write_i32(0);
        writer.write_i32(self.character.partner_ev);
        writer.write_i32(self.character.partner_cc);
        writer.write_i32(self.character.partner_ms);
        writer.write_i32(self.character.partner_as);
        writer.write_i32(0);
        writer.write_i32(self.character.partner_ht);
        writer.write_i32(0);
        writer.write_i32(0);
        writer.write_i32(self.character.partner_ar);
        writer.write_i32(self.character.partner_bl);
        writer.write_i32(self.character.partner_model);
        writer.write_u8(0);
        write_zero_i16s(&mut writer, 15);
        writer.write_u16(0);
        write_zero_i16s(&mut writer, 13);
        writer.write_i32(non_zero_handler(
            self.character.partner_handler,
            self.character.id.saturating_add(10_000),
        ));
        writer.write_u8(0);
        writer.write_u8(99);
        writer.write_i32(0);
        writer.write_i32(self.character.channel as i32);
        write_map_region(&mut writer, &self.character.map_region);
        writer.write_i32(self.character.archive_slots);
        writer.write_i32(0);
        writer.write_i32(0);
        writer.write_u8(0);
        writer.write_u8(0);
        writer.write_u8(0);
        writer.write_u8(99);
        writer.write_i16(self.character.current_title as i16);
        for _ in 0..32 {
            writer.write_i32(0);
        }
        writer.write_i32(0);
        writer.write_i32(2);
        writer.write_i32(0);
        writer.write_i32(0);
        writer.write_u8(0);
        writer.write_i32(0);
        writer.write_i32(0);
        writer.write_i32(0);
        writer.write_u8(0);
        writer.write_i16(0);
        writer.write_u8(0);
        writer.write_u8(0);
        writer.write_i32(self.character.deck_buff_id);
        writer.write_u8(0);
        writer.write_i32(0);
        writer.write_zeroes(29);
        writer.write_u8(0);
        writer.write_u8(0);
        writer.write_i32(0);
        writer.write_u8(0);
        writer.write_i32(0);
        writer.write_u8(0);
        writer.write_u8(0);
        writer.finalize()
    }
}

// --- Item operation packets ---

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemMoveSuccessPacket {
    pub origin_slot: u16,
    pub destination_slot: u16,
}

impl ItemMoveSuccessPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::MOVE_ITEM);
        writer.write_u16(self.origin_slot);
        writer.write_u16(self.destination_slot);
        writer.finalize()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemMoveFailPacket {
    pub origin_slot: u16,
    pub destination_slot: u16,
}

impl ItemMoveFailPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::MOVE_ITEM_FAILURE);
        writer.write_u16(self.origin_slot);
        writer.write_u16(self.destination_slot);
        writer.finalize()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SplitItemPacket {
    pub origin_slot: u16,
    pub destination_slot: u16,
    pub amount: u16,
}

impl SplitItemPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::SPLIT_ITEM);
        writer.write_u16(self.origin_slot);
        writer.write_u16(self.destination_slot);
        writer.write_u16(self.amount);
        writer.finalize()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemConsumeFailPacket {
    pub slot: u16,
    pub item_id: i32,
    pub result: u8,
}

impl ItemConsumeFailPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::CONSUME_ITEM);
        writer.write_u8(0); // fail
        writer.write_u16(self.slot);
        writer.write_i32(self.item_id);
        writer.write_u8(self.result);
        writer.finalize()
    }
}

// --- NPC shop packets ---

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NpcPurchaseResultPacket {
    pub success: bool,
    pub remaining_bits: i64,
}

impl NpcPurchaseResultPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::NPC_PURCHASE);
        writer.write_u8(if self.success { 1 } else { 0 });
        writer.write_i64(self.remaining_bits);
        writer.finalize()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NpcSellResultPacket {
    pub remaining_bits: i64,
}

impl NpcSellResultPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::NPC_SELL);
        writer.write_i64(self.remaining_bits);
        writer.finalize()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InventoryType {
    Inventory = 0,
    Warehouse = 1,
    AccountWarehouse = 2,
    ExtraInventory = 3,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadInventoryPacket {
    pub inventory: InventorySnapshot,
    pub inventory_type: InventoryType,
}

impl LoadInventoryPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::LOAD_INVENTORY);
        writer.write_i32(0);
        writer.write_i64(self.inventory.bits);
        writer.write_u8(self.inventory_type as u8);
        writer.write_i16(self.inventory.size as i16);

        for slot in 0..self.inventory.size as usize {
            let record = self.inventory.items.get(slot).map_or_else(
                || ItemRecord::default().record,
                |item| normalize_item_record(item),
            );
            writer.write_bytes(&record);
        }

        writer.finalize()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LoadTamerPacket {
    pub character: CharacterSummary,
}

impl LoadTamerPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::LOAD_UNLOAD_ENTITY);
        writer.write_u8(3);
        writer.write_i16(2);

        writer.write_i32(self.character.x);
        writer.write_i32(self.character.y);
        writer
            .write_u32(non_zero_handler(self.character.general_handler, self.character.id) as u32);
        writer.write_i32(self.character.model);
        writer.write_i32(self.character.x);
        writer.write_i32(self.character.y);
        writer.write_string(&self.character.name);
        writer.write_u8(self.character.level);
        writer.write_f32(self.character.z);
        writer.write_i16(self.character.ms.clamp(i16::MIN as i32, i16::MAX as i32) as i16);
        writer.write_u8(hp_rate(self.character.current_hp, self.character.hp));
        writer.write_bytes(&normalized_visual_bytes(&self.character.equipment, 16 * 60));
        writer.write_bytes(&normalized_visual_bytes(&self.character.digivice, 60));
        writer.write_i32(self.character.current_condition);
        writer.write_i32(0);
        writer.write_i32(non_zero_handler(
            self.character.partner_handler,
            self.character.id.saturating_add(10_000),
        ));
        writer.write_i16(self.character.size);
        if let Some(guild) = &self.character.guild {
            writer.write_u8(1);
            writer.write_i32(guild.id.min(i32::MAX as u32) as i32);
            writer.write_string(&guild.name);
        } else {
            writer.write_u8(0);
        }
        writer.write_i16(self.character.current_title as i16);
        writer.write_u8(0);
        writer.write_i16(self.character.seal_list.seal_leader_id);
        if self.character.current_condition == 1 {
            writer.write_string(&self.character.shop_name);
        }
        writer.write_i32(0);

        writer.write_i32(self.character.partner_x);
        writer.write_i32(self.character.partner_y);
        writer.write_i32(non_zero_handler(
            self.character.partner_handler,
            self.character.id.saturating_add(10_000),
        ));
        writer.write_i32(self.character.partner_current_type);
        writer.write_i32(self.character.partner_x);
        writer.write_i32(self.character.partner_y);
        writer.write_string(&self.character.partner_name);
        writer.write_i16(self.character.partner_size);
        writer.write_u8(self.character.partner_level);
        writer.write_f32(self.character.partner_z);
        writer.write_i16(
            self.character
                .partner_ms
                .clamp(i16::MIN as i32, i16::MAX as i32) as i16,
        );
        writer.write_i16(
            self.character
                .partner_as
                .clamp(i16::MIN as i32, i16::MAX as i32) as i16,
        );
        writer
            .write_u32(non_zero_handler(self.character.general_handler, self.character.id) as u32);
        writer.write_u8(hp_rate(
            self.character.partner_current_hp,
            self.character.partner_hp,
        ));
        writer.write_i32(self.character.partner_condition);
        writer.write_i16(self.character.partner_clone_level as i16);
        writer.write_i16(self.character.partner_clone_at_level as i16);
        writer.write_i16(self.character.partner_clone_bl_level as i16);
        writer.write_i16(self.character.partner_clone_ct_level as i16);
        writer.write_i16(0);
        writer.write_i16(self.character.partner_clone_ev_level as i16);
        writer.write_i16(0);
        writer.write_i16(self.character.partner_clone_hp_level as i16);
        writer.write_i16(0);
        writer.write_i16(0);

        writer.write_i16(0);
        writer.finalize()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadMobsPacket {
    pub mob: MobSummary,
}

impl LoadMobsPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::LOAD_UNLOAD_ENTITY);
        writer.write_u8(if self.mob.respawn { 1 } else { 3 });
        writer.write_i16(1);
        writer.write_i32(self.mob.previous_x);
        writer.write_i32(self.mob.previous_y);
        writer.write_u32(non_zero_handler(self.mob.handler, self.mob.id) as u32);
        writer.write_i32(self.mob.type_id);
        writer.write_i32(self.mob.x);
        writer.write_i32(self.mob.y);
        writer.write_u8(hp_rate(self.mob.current_hp, self.mob.max_hp));
        writer.write_i16(self.mob.level as i16);
        writer.write_i16(2);
        writer.write_i32(self.mob.grow_stack as i32);
        writer.write_i32(0);
        writer.write_u8(self.mob.disposed_objects);
        writer.write_u8(0);
        writer.write_i32(0);
        writer.finalize()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadDropsPacket {
    pub drop: DropSummary,
    pub viewer_handler: u32,
}

impl LoadDropsPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::LOAD_UNLOAD_ENTITY);
        writer.write_u8(3);
        writer.write_i16(1);
        writer.write_i32(self.drop.x);
        writer.write_i32(self.drop.y);
        writer.write_u32(non_zero_handler(self.drop.handler, self.drop.id) as u32);
        writer.write_i32(self.drop.item_id);
        let owner_handler = if self.drop.no_owner {
            non_zero_handler(self.viewer_handler, self.drop.owner_id)
        } else {
            non_zero_handler(self.drop.owner_handler, self.drop.owner_id)
        };
        writer.write_i32(owner_handler);
        writer.write_u8(0);
        writer.write_i32(0);
        writer.finalize()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickItemPacket {
    pub appearance_handler: u32,
    pub item_id: i32,
    pub amount: i16,
}

impl PickItemPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::LOOT_ITEM);
        writer.write_u32(self.appearance_handler);
        writer.write_i32(self.item_id);
        writer.write_i16(self.amount);
        writer.write_u8(0);
        writer.write_i32(0);
        writer.finalize()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickBitsPacket {
    pub appearance_handler: u32,
    pub value: i32,
}

impl PickBitsPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::PICK_BITS);
        writer.write_u32(self.appearance_handler);
        writer.write_i32(self.value);
        writer.write_i32(0);
        writer.write_i32(0);
        writer.write_i32(0);
        writer.finalize()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickItemFailReason {
    Unknown = 1,
    NotTheOwner = 2,
    TooFarAway = 3,
    InventoryFull = 4,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickItemFailPacket {
    pub reason: PickItemFailReason,
}

impl PickItemFailPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::PICK_ITEM_FAIL);
        writer.write_i32(self.reason as i32);
        writer.finalize()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct UnloadTamerPacket {
    pub character: CharacterSummary,
}

impl UnloadTamerPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::LOAD_UNLOAD_ENTITY);
        writer.write_u8(4);
        writer.write_i16(2);
        writer.write_i32(non_zero_handler(
            self.character.general_handler,
            self.character.id,
        ));
        writer.write_i32(self.character.x);
        writer.write_i32(self.character.y);
        writer.write_i32(non_zero_handler(
            self.character.partner_handler,
            self.character.id.saturating_add(10_000),
        ));
        writer.write_i32(self.character.partner_x);
        writer.write_i32(self.character.partner_y);
        writer.write_i32(0);
        writer.finalize()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnloadMobsPacket {
    pub mob: MobSummary,
}

impl UnloadMobsPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::LOAD_UNLOAD_ENTITY);
        writer.write_u8(4);
        writer.write_i16(1);
        writer.write_u32(non_zero_handler(self.mob.handler, self.mob.id) as u32);
        writer.write_i32(self.mob.x);
        writer.write_i32(self.mob.y);
        writer.write_i32(0);
        writer.finalize()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnloadDropsPacket {
    pub drop: DropSummary,
}

impl UnloadDropsPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::LOAD_UNLOAD_ENTITY);
        writer.write_u8(4);
        writer.write_i16(1);
        writer.write_u32(non_zero_handler(self.drop.handler, self.drop.id) as u32);
        writer.write_i32(self.drop.x);
        writer.write_i32(self.drop.y);
        writer.write_i32(0);
        writer.finalize()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LoadBuffsPacket {
    pub character: CharacterSummary,
}

impl LoadBuffsPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::LOAD_BUFFS);
        writer.write_u8(16);
        writer.write_i16(1);
        writer.write_i32(non_zero_handler(
            self.character.general_handler,
            self.character.id,
        ));
        writer.write_u8(clamp_u8_len(self.character.active_buffs.len()));
        for buff in &self.character.active_buffs {
            write_active_buff(&mut writer, buff);
        }

        writer.write_i16(1);
        writer.write_i32(non_zero_handler(
            self.character.partner_handler,
            self.character.id.saturating_add(10_000),
        ));
        writer.write_u8(clamp_u8_len(
            self.character.partner_active_buffs.len() + self.character.partner_active_debuffs.len(),
        ));

        for buff in &self.character.partner_active_buffs {
            write_active_buff(&mut writer, buff);
        }
        for buff in &self.character.partner_active_debuffs {
            write_active_buff(&mut writer, buff);
        }

        writer.write_i16(0);
        writer.write_u8(0);
        writer.finalize()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadMobBuffsPacket {
    pub mob: MobSummary,
}

impl LoadMobBuffsPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::LOAD_BUFFS);
        writer.write_u8(16);
        writer.write_i16(0);
        writer.write_i16(0);
        writer.write_i16(1);
        writer.write_u32(non_zero_handler(self.mob.handler, self.mob.id) as u32);
        writer.write_u8(clamp_u8_len(self.mob.active_debuffs.len()));
        for buff in self
            .mob
            .active_debuffs
            .iter()
            .take(clamp_u8_len(self.mob.active_debuffs.len()) as usize)
        {
            write_active_buff(&mut writer, buff);
        }
        writer.write_u8(0);
        writer.finalize()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerExperiencePacket {
    pub experience: i32,
}

impl ServerExperiencePacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::SERVER_EXPERIENCE);
        writer.write_i32(1);
        writer.write_i32(self.experience);
        writer.write_i32(1);
        writer.write_i32(0);
        writer.write_i32(self.experience);
        writer.finalize()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MembershipPacket {
    pub remaining_seconds: u32,
}

impl MembershipPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::MEMBERSHIP);
        writer.write_u8((self.remaining_seconds > 0) as u8);
        writer.write_i32(self.remaining_seconds.min(i32::MAX as u32) as i32);
        writer.finalize()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CashShopCoinsPacket {
    pub premium: i32,
    pub silk: i32,
}

impl CashShopCoinsPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::CASH_SHOP_COINS);
        writer.write_i32(0);
        writer.write_i32(self.silk);
        writer.write_i32(self.premium);
        writer.finalize()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AvailableChannelsPacket {
    pub channels: Vec<ChannelAvailability>,
}

impl AvailableChannelsPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::AVAILABLE_CHANNELS);
        for channel in &self.channels {
            writer.write_u8(channel.channel);
            writer.write_u8(channel.load);
        }
        writer.write_u8(u8::MAX);
        writer.finalize()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SealsPacket {
    pub seal_list: SealListSnapshot,
}

impl SealsPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::SEALS);
        writer.write_i16(self.seal_list.seal_leader_id);
        writer.write_i16(clamp_i16_len(self.seal_list.seals.len()));
        for seal in &self.seal_list.seals {
            writer.write_i16(0);
            writer.write_i32(seal.seal_id);
            writer.write_i16(seal.amount);
        }

        writer.write_i16(clamp_i16_len(self.seal_list.seals.len()));
        for seal in &self.seal_list.seals {
            writer.write_i16(seal.sequential_id);
            writer.write_u8(seal.favorite as u8);
        }

        writer.finalize()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimeRewardPacket {
    pub reward: DailyRewardStatus,
}

impl TimeRewardPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::TIME_REWARD);
        writer.write_i32(self.reward.event_no);
        writer.write_i32(self.reward.remaining_seconds);
        writer.write_i32(self.reward.total_seconds);
        writer.write_u8(self.reward.week);
        writer.finalize()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TamerRelationsPacket {
    pub friends: Vec<RelationEntry>,
    pub foes: Vec<RelationEntry>,
}

impl TamerRelationsPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::AVAILABLE_RELATIONS);
        writer.write_i16(clamp_i16_len(self.friends.len()));
        for friend in &self.friends {
            writer.write_u8(friend.connected as u8);
            writer.write_string(non_empty_relation_name(friend));
            write_optional_legacy_string(&mut writer, &friend.annotation);
        }

        writer.write_i16(clamp_i16_len(self.foes.len()));
        for foe in &self.foes {
            writer.write_string(non_empty_relation_name(foe));
            write_optional_legacy_string(&mut writer, &foe.annotation);
        }

        writer.finalize()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FriendConnectPacket {
    pub name: String,
}

impl FriendConnectPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::FRIEND_CONNECT);
        writer.write_string(&self.name);
        writer.finalize()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartyInvitePacket {
    pub inviter_name: String,
}

impl PartyInvitePacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::PARTY_INVITE);
        writer.write_string(&self.inviter_name);
        writer.finalize()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartyInviteResultPacket {
    pub result_type: i32,
    pub target_name: String,
}

impl PartyInviteResultPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::PARTY_INVITE_RESPONSE);
        writer.write_i32(self.result_type);
        writer.write_string(&self.target_name);
        writer.finalize()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartyCreatedPacket {
    pub party_id: u32,
    pub loot_type: u32,
}

impl PartyCreatedPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::PARTY_CREATED);
        writer.write_u32(self.party_id);
        writer.write_u32(self.loot_type);
        writer.finalize()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PartyMemberListEntry {
    pub party_slot: i32,
    pub character: CharacterSummary,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PartyJoinPacket {
    pub member: PartyMemberListEntry,
}

impl PartyJoinPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::PARTY_JOIN);
        write_party_member(&mut writer, &self.member);
        writer.finalize()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PartyMemberListPacket {
    pub party_id: u32,
    pub my_slot: i32,
    pub leader_slot: i32,
    pub loot_type: u32,
    pub rare_rate: u8,
    pub disp_rare_grade: u8,
    pub members: Vec<PartyMemberListEntry>,
}

impl PartyMemberListPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::PARTY_MEMBER_LIST);
        writer.write_u32(self.party_id);
        writer.write_i32(self.my_slot);
        writer.write_i32(self.leader_slot);
        writer.write_u32(self.loot_type);
        writer.write_u8(self.rare_rate);
        writer.write_u8(self.disp_rare_grade);
        for member in &self.members {
            write_party_member(&mut writer, member);
        }
        writer.write_i32(-1);
        writer.finalize()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartyLeavePacket {
    pub member_slot: u8,
}

impl PartyLeavePacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::PARTY_LEAVE);
        writer.write_u8(self.member_slot);
        writer.finalize()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartyKickPacket {
    pub member_slot: u8,
}

impl PartyKickPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::PARTY_KICK);
        writer.write_u8(self.member_slot);
        writer.finalize()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartyLeaderChangedPacket {
    pub new_leader_slot: i32,
}

impl PartyLeaderChangedPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::PARTY_CHANGE_MASTER);
        writer.write_i32(self.new_leader_slot);
        writer.finalize()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartyChangeLootTypePacket {
    pub loot_type: i32,
    pub rare_type: u8,
    pub disp_rare_grade: u8,
}

impl PartyChangeLootTypePacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::PARTY_CHANGE_LOOT);
        writer.write_i32(self.loot_type);
        writer.write_u8(self.rare_type);
        writer.write_u8(self.disp_rare_grade);
        writer.finalize()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartyMemberInfoPacket {
    pub member_slot: u8,
    pub digimon_type: i32,
    pub tamer_hp: i32,
    pub tamer_max_hp: i32,
    pub tamer_ds: i32,
    pub tamer_max_ds: i32,
    pub digimon_hp: i32,
    pub digimon_max_hp: i32,
    pub digimon_ds: i32,
    pub digimon_max_ds: i32,
    pub tamer_level: u16,
    pub digimon_level: u16,
}

impl PartyMemberInfoPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::PARTY_MEMBER_INFO);
        writer.write_u8(self.member_slot);
        writer.write_i32(self.digimon_type);
        writer.write_i32(self.tamer_hp);
        writer.write_i32(self.tamer_max_hp);
        writer.write_i32(self.tamer_ds);
        writer.write_i32(self.tamer_max_ds);
        writer.write_i32(self.digimon_hp);
        writer.write_i32(self.digimon_max_hp);
        writer.write_i32(self.digimon_ds);
        writer.write_i32(self.digimon_max_ds);
        writer.write_u16(self.tamer_level);
        writer.write_u16(self.digimon_level);
        writer.finalize()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartyMemberPositionPacket {
    pub member_slot: u8,
    pub tamer_x: i32,
    pub tamer_y: i32,
    pub digimon_x: i32,
    pub digimon_y: i32,
}

impl PartyMemberPositionPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::PARTY_MEMBER_POSITION);
        writer.write_u8(self.member_slot);
        writer.write_i32(self.tamer_x);
        writer.write_i32(self.tamer_y);
        writer.write_i32(self.digimon_x);
        writer.write_i32(self.digimon_y);
        writer.finalize()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartyMemberMapChangePacket {
    pub member_slot: u8,
    pub map_id: i32,
    pub channel: i32,
    pub tamer_handler: u32,
    pub digimon_handler: u32,
}

impl PartyMemberMapChangePacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::PARTY_MEMBER_MAP_CHANGE);
        writer.write_u8(self.member_slot);
        writer.write_i32(self.map_id);
        writer.write_i32(self.channel);
        writer.write_u32(self.tamer_handler);
        writer.write_u32(self.digimon_handler);
        writer.finalize()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartyMemberDigimonChangePacket {
    pub member_slot: u8,
    pub digimon_type: i32,
    pub digimon_name: String,
    pub digimon_hp: u16,
    pub digimon_max_hp: u16,
    pub digimon_ds: u16,
    pub digimon_max_ds: u16,
}

impl PartyMemberDigimonChangePacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::PARTY_MEMBER_DIGIMON_CHANGE);
        writer.write_u8(self.member_slot);
        writer.write_i32(self.digimon_type);
        writer.write_string(&self.digimon_name);
        writer.write_u16(self.digimon_hp);
        writer.write_u16(self.digimon_max_hp);
        writer.write_u16(self.digimon_ds);
        writer.write_u16(self.digimon_max_ds);
        writer.finalize()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartyMemberBuffEntry {
    pub status: u8,
    pub buff_code: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartyMemberBuffChangePacket {
    pub member_slot: u8,
    pub buffs: Vec<PartyMemberBuffEntry>,
}

impl PartyMemberBuffChangePacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::PARTY_MEMBER_BUFF_CHANGE);
        writer.write_u8(self.member_slot);
        writer.write_u16(self.buffs.len() as u16);
        for buff in &self.buffs {
            writer.write_u8(buff.status);
            writer.write_u16(buff.buff_code);
        }
        writer.finalize()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartnerSwitchFailurePacket;

impl PartnerSwitchFailurePacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::PARTNER_SWITCH_RESPONSE);
        writer.write_u32(0);
        writer.finalize()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DigimonEvolutionFailPacket;

impl DigimonEvolutionFailPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::EVOLUTION_FAILURE);
        writer.write_i32(0);
        writer.finalize()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DigimonEvolutionSuccessPacket {
    pub digimon_handler: u32,
    pub tamer_handler: u32,
    pub new_type: i32,
    pub evolution_slot: u8,
    pub hp_rate: u8,
    pub parts_type: i32,
}

impl DigimonEvolutionSuccessPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::EVOLUTION);
        writer.write_u32(self.digimon_handler);
        writer.write_u32(self.tamer_handler);
        writer.write_i32(self.new_type);
        writer.write_u8(self.evolution_slot);
        writer.write_u8(self.hp_rate);
        writer.write_i32(self.parts_type);
        writer.finalize()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PartnerSwitchPacket {
    pub handler: u32,
    pub old_partner_current_type: i32,
    pub slot: u8,
    pub partner: odmo_types::PartnerSlotSnapshot,
}

impl PartnerSwitchPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::PARTNER_SWITCH_RESPONSE);
        writer.write_u32(self.handler);
        writer.write_i32(self.old_partner_current_type);
        writer.write_u8(self.slot);
        writer.write_i32(self.partner.digimon_type);
        writer.write_u8(self.partner.level);
        writer.write_fixed_wide_string(&self.partner.name, 32);
        writer.write_i16(self.partner.size);
        writer.write_i32(0);
        writer.write_u16(self.partner.clone_level);
        writer.write_u16(self.partner.clone_at_value);
        writer.write_u16(self.partner.clone_bl_value);
        writer.write_u16(self.partner.clone_ct_value);
        writer.write_u16(0);
        writer.write_u16(self.partner.clone_ev_value);
        writer.write_u16(0);
        writer.write_u16(self.partner.clone_hp_value);
        writer.write_u16(self.partner.clone_at_level);
        writer.write_u16(self.partner.clone_bl_level);
        writer.write_u16(self.partner.clone_ct_level);
        writer.write_u16(0);
        writer.write_u16(self.partner.clone_ev_level);
        writer.write_u16(0);
        writer.write_u16(self.partner.clone_hp_level);
        writer.write_u16(self.partner.active_buffs.len() as u16);
        for buff in &self.partner.active_buffs {
            writer.write_u16(buff.buff_id);
            writer.write_u16(buff.buff_class);
            writer.write_u32(buff.remaining_seconds.max(0) as u32);
            writer.write_i32(buff.skill_id);
        }
        writer.write_i32(self.partner.hp);
        writer.write_i32(self.partner.ds);
        writer.write_i32(self.partner.de);
        writer.write_i32(self.partner.at);
        writer.write_i32(self.partner.current_hp);
        writer.write_i32(self.partner.current_ds);
        writer.write_i32(self.partner.fs);
        writer.write_i32(0);
        writer.write_i32(self.partner.ev);
        writer.write_i32(self.partner.cc);
        writer.write_i32(self.partner.ms);
        writer.write_i32(self.partner.as_value);
        writer.write_i32(self.partner.ar);
        writer.write_i32(self.partner.ht);
        writer.write_i32(0);
        writer.write_i32(0);
        writer.write_i32(0);
        writer.write_i32(self.partner.bl);
        writer.finalize()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartyMemberDisconnectedPacket {
    pub member_slot: i32,
}

impl PartyMemberDisconnectedPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::PARTY_MEMBER_DISCONNECTED);
        writer.write_i32(self.member_slot);
        writer.finalize()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TamerAttendancePacket {
    pub attendance: AttendanceStatus,
}

impl TamerAttendancePacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::TAMER_ATTENDANCE);
        writer.write_u8(self.attendance.event_no);
        writer.write_u8(self.attendance.attendance_count);
        writer.write_u8(self.attendance.notify as u8);
        writer.finalize()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuildInformationPacket {
    pub guild: GuildSnapshot,
}

impl GuildInformationPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::GUILD_INFORMATION);
        writer.write_string(&self.guild.name);
        writer.write_i32(self.guild.id.min(i32::MAX as u32) as i32);
        writer.write_u8(self.guild.level);
        writer.write_i32(self.guild.current_experience);
        writer.write_string(&self.guild.notice);
        writer.write_i32(self.guild.extra_slots);

        for authority in normalized_guild_authorities(&self.guild) {
            writer.write_string(&authority.title);
            writer.write_string(&authority.duty);
        }

        let mut members = self.guild.members.clone();
        members.sort_by_key(|member| (std::cmp::Reverse(member.map_id), member.authority));
        for member in members {
            writer.write_u8(member.authority);
            writer.write_u8(guild_member_model(&member));
            writer.write_string(&member.character_name);
            writer.write_i32(member.contribution);
            writer.write_u8(member.character_level);

            if is_online_state(member.state) {
                writer.write_i16(member.map_id);
                writer.write_u8(member.channel);
            } else {
                writer.write_i16(0);
            }
        }

        writer.write_u8(0);
        writer.finalize()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuildHistoricPacket {
    pub entries: Vec<GuildHistoricEntry>,
}

impl GuildHistoricPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::GUILD_HISTORIC);
        for entry in &self.entries {
            writer.write_u8(entry.historic_type);
            writer.write_i32(entry.date_utc_seconds.min(i32::MAX as u32) as i32);
            writer.write_u8(entry.master_class);
            writer.write_string(&entry.master_name);
            writer.write_u8(entry.member_class);
            writer.write_string(&entry.member_name);
        }
        writer.write_u8(0);
        writer.finalize()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuildRankPacket {
    pub position: i16,
}

impl GuildRankPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::GUILD_RANK);
        writer.write_i16(self.position);
        writer.finalize()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XaiInfoPacket {
    pub xai: Option<XaiSnapshot>,
}

impl XaiInfoPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::XAI_INFO);
        writer.write_i32(self.xai.as_ref().map_or(0, |xai| xai.max_xgauge));
        writer.write_i16(self.xai.as_ref().map_or(0, |xai| xai.max_xcrystals));
        writer.finalize()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TamerXaiResourcesPacket {
    pub current_xgauge: i32,
    pub current_xcrystals: i16,
}

impl TamerXaiResourcesPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::TAMER_XAI_RESOURCES);
        writer.write_i32(self.current_xgauge);
        writer.write_i16(self.current_xcrystals);
        writer.finalize()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct UpdateStatusPacket {
    pub character: CharacterSummary,
}

impl UpdateStatusPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::UPDATE_STATUS);
        writer.write_i32(self.character.hp);
        writer.write_i32(self.character.ds);
        writer.write_i32(self.character.current_hp);
        writer.write_i32(self.character.current_ds);
        writer.write_u16(clamp_u16(self.character.at));
        writer.write_u16(clamp_u16(self.character.de));
        writer.write_u16(clamp_u16(self.character.ms));
        writer.write_i32(self.character.partner_hp);
        writer.write_i32(self.character.partner_ds);
        writer.write_i32(self.character.partner_current_hp);
        writer.write_i32(self.character.partner_current_ds);
        writer.write_u16(clamp_u16(self.character.partner_fs));
        writer.write_u16(clamp_u16(self.character.partner_at));
        writer.write_u16(clamp_u16(self.character.partner_de));
        writer.write_u16(clamp_u16(self.character.partner_cc));
        writer.write_u16(clamp_u16(self.character.partner_as));
        writer.write_u16(clamp_u16(self.character.partner_ev));
        writer.write_u16(clamp_u16(self.character.partner_ht));
        writer.write_u16(clamp_u16(self.character.partner_ar));
        writer.write_u16(clamp_u16(self.character.partner_bl));
        writer.write_u16(self.character.partner_clone_level);
        writer.write_u16(self.character.partner_clone_at_value);
        writer.write_u16(self.character.partner_clone_bl_value);
        writer.write_u16(self.character.partner_clone_ct_value);
        writer.write_u16(0);
        writer.write_u16(self.character.partner_clone_ev_value);
        writer.write_u16(0);
        writer.write_u16(self.character.partner_clone_hp_value);
        writer.write_u16(self.character.partner_clone_at_level);
        writer.write_u16(self.character.partner_clone_bl_level);
        writer.write_u16(self.character.partner_clone_ct_level);
        writer.write_u16(0);
        writer.write_u16(self.character.partner_clone_ev_level);
        writer.write_u16(0);
        writer.write_u16(self.character.partner_clone_hp_level);
        writer.finalize()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct UpdateMovementSpeedPacket {
    pub character: CharacterSummary,
}

impl UpdateMovementSpeedPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::UPDATE_MOVEMENT_SPEED);
        writer
            .write_u32(non_zero_handler(self.character.general_handler, self.character.id) as u32);
        writer.write_u32(non_zero_handler(
            self.character.partner_handler,
            self.character.id.saturating_add(10_000),
        ) as u32);

        let effective_speed = if self.character.current_condition == 1 {
            self.character.proper_ms.saturating_mul(2)
        } else {
            self.character.proper_ms
        };

        writer.write_i16(effective_speed);
        writer.write_i16(effective_speed);
        writer.write_i32(self.character.current_condition);
        writer.write_i32(self.character.partner_condition);
        writer.finalize()
    }
}

fn write_empty_item_records(writer: &mut PacketWriter, slots: usize) {
    for _ in 0..slots {
        writer.write_zeroes(60);
    }
}

fn write_zero_i16s(writer: &mut PacketWriter, count: usize) {
    for _ in 0..count {
        writer.write_i16(0);
    }
}

fn normalize_item_record(item: &ItemRecord) -> Vec<u8> {
    let mut record = item.record.clone();
    if record.len() > 69 {
        record.truncate(69);
    } else if record.len() < 69 {
        record.resize(69, 0);
    }
    record
}

fn write_map_region(writer: &mut PacketWriter, region: &[u8]) {
    let capped_len = region.len().min(255);
    writer.write_bytes(&region[..capped_len]);
    if capped_len < 255 {
        writer.write_zeroes(255 - capped_len);
    }
}

fn non_zero_handler(raw: u32, fallback_id: u64) -> i32 {
    if raw == 0 {
        fallback_id.min(i32::MAX as u64) as i32
    } else {
        raw.min(i32::MAX as u32) as i32
    }
}

fn clamp_i16_len(len: usize) -> i16 {
    len.min(i16::MAX as usize) as i16
}

fn clamp_u8_len(len: usize) -> u8 {
    len.min(u8::MAX as usize) as u8
}

fn clamp_u16(value: i32) -> u16 {
    value.clamp(u16::MIN as i32, u16::MAX as i32) as u16
}

fn write_active_buff(writer: &mut PacketWriter, buff: &ActiveBuffSnapshot) {
    writer.write_u16(buff.buff_id);
    writer.write_i16(1);
    writer.write_i32(buff.remaining_seconds.max(0));
    writer.write_i32(buff.skill_id);
}

fn hp_rate(current: i32, maximum: i32) -> u8 {
    if maximum <= 0 {
        return 0;
    }

    ((current.clamp(0, maximum) as i64 * 255) / maximum as i64)
        .clamp(u8::MIN as i64, u8::MAX as i64) as u8
}

fn normalized_visual_bytes(bytes: &[u8], expected_len: usize) -> Vec<u8> {
    let mut data = bytes.to_vec();
    if data.len() > expected_len {
        data.truncate(expected_len);
    } else if data.len() < expected_len {
        data.resize(expected_len, 0);
    }
    data
}

fn normalized_guild_authorities(guild: &GuildSnapshot) -> Vec<odmo_types::GuildAuthoritySnapshot> {
    let mut authorities = guild.authorities.clone();
    if authorities.len() < 5 {
        let defaults = odmo_types::GuildSnapshot::default().authorities;
        for authority in defaults.into_iter().skip(authorities.len()) {
            authorities.push(authority);
        }
    }
    authorities.truncate(5);
    authorities
}

fn guild_member_model(member: &odmo_types::GuildMemberSnapshot) -> u8 {
    member
        .character_model
        .saturating_sub(80_000)
        .clamp(u8::MIN as i32, u8::MAX as i32) as u8
}

fn write_party_member(writer: &mut PacketWriter, member: &PartyMemberListEntry) {
    writer.write_i32(member.party_slot);
    writer.write_i32(member.character.model);
    writer.write_i16(i16::from(member.character.level));
    writer.write_string(&member.character.name);
    writer.write_i32(member.character.partner_current_type);
    writer.write_i16(i16::from(member.character.partner_level));
    writer.write_string(&member.character.partner_name);
    writer.write_i32(i32::from(member.character.map_id));
    writer.write_i32(i32::from(member.character.channel));
    writer.write_u32(member.character.general_handler);
    writer.write_u32(member.character.partner_handler);
}

fn is_online_state(state: odmo_types::CharacterConnectionState) -> bool {
    matches!(
        state,
        odmo_types::CharacterConnectionState::Connected
            | odmo_types::CharacterConnectionState::Ready
    )
}

fn write_optional_legacy_string(writer: &mut PacketWriter, value: &str) {
    if value.is_empty() {
        writer.write_u8(0);
    } else {
        writer.write_string(value);
    }
}

fn non_empty_relation_name(entry: &RelationEntry) -> &str {
    if entry.name.is_empty() {
        "Friend"
    } else {
        &entry.name
    }
}

// --- Movement packets ---

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TamerWalkPacket {
    pub handler: u32,
    pub x: i32,
    pub y: i32,
}

impl TamerWalkPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::MAP_ENTITY);
        writer.write_u8(5);
        writer.write_i16(1);
        writer.write_u32(self.handler);
        writer.write_i32(self.x);
        writer.write_i32(self.y);
        writer.write_i32(0);
        writer.finalize()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DigimonWalkPacket {
    pub handler: u32,
    pub x: i32,
    pub y: i32,
}

impl DigimonWalkPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::MAP_ENTITY);
        writer.write_u8(6);
        writer.write_i16(1);
        writer.write_u32(self.handler);
        writer.write_i32(self.x);
        writer.write_i32(self.y);
        writer.write_i32(0);
        writer.finalize()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncConditionPacket {
    pub handler: i32,
    pub condition: i32,
}

impl SyncConditionPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::SYNC_CONDITION);
        writer.write_i32(self.handler);
        writer.write_i32(self.condition);
        writer.finalize()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapSwapPacket {
    pub address: String,
    pub port: i32,
    pub map_id: i16,
    pub x: i32,
    pub y: i32,
}

impl MapSwapPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::WARP_GATE);
        writer.write_string(&self.address);
        writer.write_i32(self.port);
        writer.write_i16(self.map_id);
        writer.write_i32(self.x);
        writer.write_i32(self.y);
        writer.finalize()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalMapSwapPacket {
    pub tamer_handler: i32,
    pub partner_handler: i32,
    pub x: i32,
    pub y: i32,
}

impl LocalMapSwapPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::LOCAL_MAP_SWAP);
        writer.write_i32(self.tamer_handler);
        writer.write_i32(self.partner_handler);
        writer.write_i32(self.x);
        writer.write_i32(self.y);
        writer.finalize()
    }
}

/// Packet sent to update skill cooldowns for a digimon (opcode 3246).
#[derive(Debug, Clone)]
pub struct SkillUpdateCooldownPacket {
    pub handler: i32,
    pub current_type: i32,
    pub cooldowns: Vec<(i32, i32)>, // (skill_id, end_timestamp)
}

impl SkillUpdateCooldownPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::SKILL_UPDATE_COOLDOWN);
        writer.write_i32(self.handler);
        writer.write_i32(self.current_type);
        writer.write_i32(self.cooldowns.len() as i32);
        for (skill_id, end_time) in &self.cooldowns {
            writer.write_i32(*skill_id);
            writer.write_i32(*end_time);
        }
        writer.finalize()
    }
}

// ---

impl TryFrom<RawPacket> for GameRequest {
    type Error = ProtocolError;

    fn try_from(packet: RawPacket) -> Result<Self, Self::Error> {
        let mut reader = PacketReader::new(packet.payload);
        match packet.packet_type {
            game::CONNECTION => Ok(Self::Connection {
                kind: reader.read_u8()?,
            }),
            game::KEEP_CONNECTION => Ok(Self::KeepConnection),
            game::INITIAL_INFORMATION => {
                // Client AccessCode packet payload: [XOR(4)][accountIdx(4)][accessCode(4)]
                // Skip XOR value; read accountIdx at offset 4 in payload
                reader.seek(4)?;
                Ok(Self::InitialInformation {
                    account_id: reader.read_u32()? as u64,
                    access_code: reader.read_u32()?,
                })
            }
            game::COMPLEMENTAR_INFORMATION => Ok(Self::ComplementarInformation),
            game::TAMER_MOVIMENTATION => {
                let ticks = reader.read_u32()?;
                let handler = reader.read_u32()?;
                let x = reader.read_i32()?;
                let y = reader.read_i32()?;
                let z = reader.read_f32()?;
                Ok(Self::TamerMovimentation {
                    ticks,
                    handler,
                    x,
                    y,
                    z,
                })
            }
            game::WARP_GATE => {
                let portal_id = reader.read_i32()?;
                Ok(Self::WarpGate { portal_id })
            }
            game::CONSUME_ITEM => {
                let target_handler = reader.read_i32()?;
                let slot = reader.read_u16()?;
                Ok(Self::ConsumeItem {
                    target_handler,
                    slot,
                })
            }
            game::MOVE_ITEM => {
                let origin_slot = reader.read_u16()?;
                let destination_slot = reader.read_u16()?;
                Ok(Self::MoveItem {
                    origin_slot,
                    destination_slot,
                })
            }
            game::SPLIT_ITEM => {
                let origin_slot = reader.read_u16()?;
                let destination_slot = reader.read_u16()?;
                let amount = reader.read_u16()?;
                Ok(Self::SplitItem {
                    origin_slot,
                    destination_slot,
                    amount,
                })
            }
            game::ITEM_REMOVE => {
                let slot = reader.read_u16()?;
                let x = reader.read_i32()?;
                let y = reader.read_i32()?;
                let amount = reader.read_u16()?;
                Ok(Self::RemoveItem { slot, x, y, amount })
            }
            game::NPC_PURCHASE => {
                let npc_id = reader.read_i32()?;
                let unk = reader.read_u8()?;
                let shop_slot = reader.read_i32()?;
                let purchase_count = reader.read_u16()?;
                Ok(Self::NpcPurchase {
                    npc_id,
                    unk,
                    shop_slot,
                    purchase_count,
                })
            }
            game::NPC_SELL => {
                let npc_id = reader.read_i32()?;
                let unk = reader.read_u8()?;
                let item_slot = reader.read_u8()?;
                let sell_amount = reader.read_u16()?;
                Ok(Self::NpcSell {
                    npc_id,
                    unk,
                    item_slot,
                    sell_amount,
                })
            }
            game::LOOT_ITEM => Ok(Self::LootItem {
                drop_handler: reader.read_u32()?,
            }),
            game::DIGI_SUMMON_SYNC_REQUEST => Ok(Self::DigiSummonSyncRequest),
            game::AVAILABLE_CHANNELS => Ok(Self::ChannelInfo),
            game::MEMBERSHIP => Ok(Self::Membership),
            game::EMOTICON => {
                let emoticon_type = reader.read_i32()?;
                let value = if reader.remaining_len() >= 4 {
                    reader.read_i32()?
                } else {
                    -1
                };
                Ok(Self::Emoticon {
                    emoticon_type,
                    value,
                })
            }
            game::FRIENDLY_INFO => {
                let target_handler = if reader.remaining_len() >= 4 {
                    reader.read_u32()?
                } else {
                    0
                };
                Ok(Self::FriendlyInfo { target_handler })
            }
            game::AVAILABLE_RELATIONS => Ok(Self::FriendlyMark),
            game::EXITEM_MOVE => {
                let category = reader.read_u16()?;
                let extra_slot = reader.read_u16()?;
                let inventory_slot = reader.read_u16()?;
                Ok(Self::ExtraInventoryMove {
                    category,
                    extra_slot,
                    inventory_slot,
                })
            }
            game::EXITEM_BATCH_MOVE => {
                let _unknown = reader.read_u8()?;
                let category = reader.read_u8()?;
                Ok(Self::ExtraInventoryBatchMove { category })
            }
            game::EXITEM_SORT => {
                let category = reader.read_u8()?;
                Ok(Self::ExtraInventorySort { category })
            }
            game::EXITEM_USE => {
                let category = reader.read_u8()?;
                let extra_slot = reader.read_u16()?;
                Ok(Self::ExtraInventoryUse {
                    category,
                    extra_slot,
                })
            }
            game::CHAT_MESSAGE => {
                let message = reader.read_string()?;
                Ok(Self::ChatMessage { message })
            }
            game::WHISPER_MESSAGE => {
                let target_name = reader.read_string()?;
                let message = reader.read_string()?;
                Ok(Self::WhisperMessage {
                    target_name,
                    message,
                })
            }
            game::SHOUT_MESSAGE => {
                let message = reader.read_string()?;
                Ok(Self::ShoutMessage { message })
            }
            game::MEGAPHONE_MESSAGE => {
                let message = reader.read_string()?;
                let item_slot = reader.read_i32()?;
                Ok(Self::MegaphoneMessage { message, item_slot })
            }
            game::TAMER_REACTION => {
                let reaction_type = reader.read_i32()?;
                Ok(Self::TamerReaction { reaction_type })
            }
            game::PARTNER_STOP => {
                let uid = reader.read_u32()?;
                Ok(Self::PartnerStop { uid })
            }
            game::EVOLUTION => {
                let digimon_handler = reader.read_u32()?;
                let evolution_slot = reader.read_u8()?;
                Ok(Self::PartnerEvolution {
                    digimon_handler,
                    evolution_slot,
                })
            }
            game::PARTNER_SWITCH => {
                let slot = reader.read_u8()?;
                Ok(Self::PartnerSwitch { slot })
            }
            game::PARTNER_DELETE => {
                let slot = reader.read_u8()?;
                let validation = reader.read_string()?;
                Ok(Self::PartnerDelete { slot, validation })
            }
            game::EVOLUTION_UNLOCK => {
                let evolution_type = reader.read_i32()?;
                let inven_idx = if reader.remaining_len() >= 2 {
                    Some(reader.read_i16()?)
                } else {
                    None
                };
                Ok(Self::EvolutionUnlock {
                    evolution_type,
                    inven_idx,
                })
            }
            game::RIDE_MODE_START => {
                let evolution_type = reader.read_i32()?;
                let item_type = reader.read_i32()?;
                Ok(Self::RideModeStart {
                    evolution_type,
                    item_type,
                })
            }
            game::RIDE_MODE_STOP => Ok(Self::RideModeStop),
            game::DIGIMON_CHANGE_NAME => {
                let inven_slot = reader.read_i32()?;
                let new_name = reader.read_string()?;
                Ok(Self::DigimonChangeName {
                    inven_slot,
                    new_name,
                })
            }
            game::HATCH_INSERT_EGG => {
                let vip = reader.read_u8()?;
                let inven_slot = reader.read_u16()?;
                let npc_idx = reader.read_i32()?;
                Ok(Self::HatchInsertEgg {
                    vip,
                    inven_slot,
                    npc_idx,
                })
            }
            game::HATCH_INCREASE => {
                let vip = reader.read_u8()?;
                let npc_idx = reader.read_i32()?;
                let data_level = reader.read_u8()? as i8;
                Ok(Self::HatchIncrease {
                    vip,
                    npc_idx,
                    data_level,
                })
            }
            game::HATCH_FINISH => {
                let vip = reader.read_u8()?;
                let portable_pos = reader.read_u32()?;
                let name = reader.read_string()?;
                let npc_idx = reader.read_i32()?;
                Ok(Self::HatchFinish {
                    vip,
                    portable_pos,
                    name,
                    npc_idx,
                })
            }
            game::HATCH_REMOVE_EGG => {
                let vip = reader.read_u8()?;
                let npc_idx = reader.read_i32()?;
                Ok(Self::HatchRemoveEgg { vip, npc_idx })
            }
            game::HATCH_BACKUP_INSERT => {
                let vip = reader.read_u8()?;
                let inven_slot = reader.read_u16()?;
                let npc_idx = reader.read_i32()?;
                Ok(Self::HatchBackupInsert {
                    vip,
                    inven_slot,
                    npc_idx,
                })
            }
            game::HATCH_BACKUP_CANCEL => {
                let vip = reader.read_u8()?;
                let npc_idx = reader.read_i32()?;
                Ok(Self::HatchBackupCancel { vip, npc_idx })
            }
            game::INCUBATOR_CLOSE => Ok(Self::IncubatorClose),
            game::DIGIMON_ARCHIVE_MOVE => {
                let vip = reader.read_u8()?;
                let slot1 = reader.read_i32()?;
                let slot2 = reader.read_i32()?;
                let npc_type = reader.read_u32()?;
                Ok(Self::DigimonArchiveMove {
                    vip,
                    slot1,
                    slot2,
                    npc_type,
                })
            }
            game::DIGIMON_ARCHIVE_LIST => {
                let vip = reader.read_u8()?;
                let inven_idx = reader.read_u32()?;
                let npc_type = reader.read_u32()?;
                Ok(Self::DigimonArchiveList {
                    vip,
                    inven_idx,
                    npc_type,
                })
            }
            game::DIGIMON_ARCHIVE_SWAP => {
                let npc_idx = reader.read_u32()?;
                let archive_type = reader.read_i32()?;
                let src_arr = reader.read_u8()?;
                let dst_arr = reader.read_u8()?;
                Ok(Self::DigimonArchiveSwap {
                    npc_idx,
                    archive_type,
                    src_arr,
                    dst_arr,
                })
            }
            game::INVENTORY_SORT => {
                let sort_type = reader.read_u8()?;
                Ok(Self::InventorySort { sort_type })
            }
            game::ITEM_IDENTIFY => {
                let item_slot = reader.read_i16()?;
                Ok(Self::ItemIdentify { item_slot })
            }
            game::ITEM_CRAFT => {
                let recipe_slot = reader.read_i16()?;
                Ok(Self::ItemCraft { recipe_slot })
            }
            game::ITEM_REROLL => {
                let item_slot = reader.read_i16()?;
                Ok(Self::ItemReroll { item_slot })
            }
            game::ITEM_SOCKET_IN => {
                let item_slot = reader.read_i16()?;
                let socket_slot = reader.read_u8()?;
                let chip_item_id = reader.read_i32()?;
                Ok(Self::ItemSocketIn {
                    item_slot,
                    socket_slot,
                    chip_item_id,
                })
            }
            game::ITEM_SOCKET_OUT => {
                let item_slot = reader.read_i16()?;
                let socket_slot = reader.read_u8()?;
                Ok(Self::ItemSocketOut {
                    item_slot,
                    socket_slot,
                })
            }
            game::ITEM_SOCKET_IDENTIFY => {
                let item_slot = reader.read_i16()?;
                Ok(Self::ItemSocketIdentify { item_slot })
            }
            game::ITEM_RETURN => {
                let item_slot = reader.read_i16()?;
                Ok(Self::ItemReturn { item_slot })
            }
            game::ITEM_SCAN => {
                let item_slot = reader.read_i16()?;
                Ok(Self::ItemScan { item_slot })
            }
            game::LOAD_GIFT_STORAGE => Ok(Self::LoadGiftStorage),
            game::GIFT_STORAGE_RETRIEVE => {
                let item_slot = reader.read_i16()?;
                Ok(Self::GiftStorageRetrieve { item_slot })
            }
            game::LOAD_REWARD_STORAGE => Ok(Self::LoadRewardStorage),
            game::RECOMPENSE_GAIN => {
                let reward_id = reader.read_i32()?;
                Ok(Self::RecompenseGain { reward_id })
            }
            game::TAMER_SHOP_OPEN => Ok(Self::TamerShopOpen),
            game::TAMER_SHOP_CLOSE => Ok(Self::TamerShopClose),
            game::TAMER_SHOP_BUY => {
                let item_id = reader.read_i32()?;
                let amount = reader.read_i16()?;
                Ok(Self::TamerShopBuy { item_id, amount })
            }
            game::CONSIGNSHOP_OPEN => Ok(Self::ConsignedShopOpen),
            game::CONSIGNSHOP_VIEW => {
                let shop_id = reader.read_i32()?;
                Ok(Self::ConsignedShopView { shop_id })
            }
            game::CONSIGNSHOP_PURCHASE => {
                let item_id = reader.read_i32()?;
                let amount = reader.read_i16()?;
                Ok(Self::ConsignedShopPurchase { item_id, amount })
            }
            game::CONSIGNSHOP_RETRIEVE => {
                let item_slot = reader.read_i16()?;
                Ok(Self::ConsignedShopRetrieve { item_slot })
            }
            game::CASHSHOP_OPEN => Ok(Self::CashShopOpen),
            game::CASHSHOP_BUY => {
                let amount = reader.read_u8()?;
                let total_price = reader.read_i32()?;
                let order_id = reader.read_u16()?;
                let mut product_ids = Vec::new();
                for _ in 0..amount {
                    product_ids.push(reader.read_i32()?);
                }
                Ok(Self::CashShopBuy {
                    amount,
                    total_price,
                    order_id,
                    product_ids,
                })
            }
            game::CASHSHOP_RELOAD => Ok(Self::CashShopReload),
            game::QUEST_AVAILABLE_LIST => Ok(Self::QuestAvailableList),
            game::QUEST_ACCEPT => {
                let quest_id = reader.read_i32()?;
                Ok(Self::QuestAccept { quest_id })
            }
            game::QUEST_DELIVER => {
                let quest_id = reader.read_i32()?;
                Ok(Self::QuestDeliver { quest_id })
            }
            game::QUEST_GIVE_UP => {
                let quest_id = reader.read_i32()?;
                Ok(Self::QuestGiveUp { quest_id })
            }
            game::QUEST_UPDATE => {
                let quest_id = reader.read_i32()?;
                let progress = reader.read_i32()?;
                Ok(Self::QuestUpdate { quest_id, progress })
            }
            game::DIE_CONFIRM => Ok(Self::DieConfirm),
            game::REMOVE_BUFF => {
                let buff_id = reader.read_i32()?;
                Ok(Self::RemoveBuff { buff_id })
            }
            game::DAMAGE_SKIN_CHANGE => {
                let skin_id = reader.read_i32()?;
                Ok(Self::DamageSkinChange { skin_id })
            }
            game::SEAL_OPEN => {
                let seal_idx = reader.read_i16()?;
                Ok(Self::SealOpen { seal_idx })
            }
            game::SEAL_CLOSE => {
                let seal_idx = reader.read_i16()?;
                Ok(Self::SealClose { seal_idx })
            }
            game::SEAL_SET_LEADER => {
                let card_code = reader.read_u16()?;
                Ok(Self::SealSetLeader { card_code })
            }
            game::SEAL_REMOVE_LEADER => Ok(Self::SealRemoveLeader),
            game::SEAL_SET_FAVORITE => {
                let card_code = reader.read_u16()?;
                let bookmark = reader.read_u8()?;
                Ok(Self::SealSetFavorite {
                    card_code,
                    bookmark,
                })
            }
            game::ENCYCLOPEDIA_LOAD => Ok(Self::EncyclopediaLoad),
            game::ENCYCLOPEDIA_GET_REWARD => {
                let digimon_id = reader.read_u32()?;
                Ok(Self::EncyclopediaGetReward { digimon_id })
            }
            game::ENCYCLOPEDIA_DECK_BUFF => {
                let deck_idx = reader.read_u32()?;
                Ok(Self::EncyclopediaDeckBuff { deck_idx })
            }
            game::ARENA_DAILY_POINTS => {
                let _skip = reader.read_u16()?; // skip 2 bytes
                let item_slot = reader.read_i16()?;
                let points = reader.read_i16()?;
                let item_id = reader.read_i16()?;
                Ok(Self::ArenaDailyPoints {
                    item_slot,
                    points,
                    item_id,
                })
            }
            game::ARENA_DAILY_RANKING => Ok(Self::ArenaDailyRanking),
            game::ARENA_RANKING_ALL => {
                let ranking_type = reader.read_u8()?;
                Ok(Self::ArenaRankingAll { ranking_type })
            }
            game::ARENA_REQUEST_RANK => {
                let ranking_type = reader.read_u8()?;
                Ok(Self::ArenaRequestRank { ranking_type })
            }
            game::ARENA_REQUEST_OLD_RANK => {
                let ranking_type = reader.read_u8()?;
                Ok(Self::ArenaRequestOldRank { ranking_type })
            }
            game::DUNGEON_NEXT_STAGE => Ok(Self::DungeonNextStage),
            game::DUNGEON_SURRENDER => Ok(Self::DungeonSurrender),
            game::BURNING_EVENT => Ok(Self::BurningEvent),
            game::DAILY_CHECK_EVENT => Ok(Self::DailyCheckEvent),
            game::DAILY_CHECK_EVENT_REQUEST => {
                let event_no = reader.read_i32()?;
                Ok(Self::DailyCheckEventRequest { event_no })
            }
            game::JOIN_EVENT_QUEUE => {
                let event_id = reader.read_i32()?;
                Ok(Self::JoinEventQueue { event_id })
            }
            game::REGION_UNLOCK => {
                let region_idx = reader.read_i16()?;
                Ok(Self::RegionUnlock { region_idx })
            }
            game::SET_TITLE => {
                let title_id = reader.read_i16()?;
                Ok(Self::SetTitle { title_id })
            }
            game::CHANGE_TAMER_MODEL => {
                let model_id = reader.read_i32()?;
                Ok(Self::ChangeTamerModel { model_id })
            }
            game::TAMER_NAME_CHANGE => {
                let new_name = reader.read_string()?;
                Ok(Self::TamerNameChange { new_name })
            }
            game::RARE_MACHINE_OPEN => {
                let npc_idx = reader.read_u32()?;
                Ok(Self::RareMachineOpen { npc_idx })
            }
            game::RARE_MACHINE_RUN => {
                let npc_idx = reader.read_u32()?;
                let inven_idx = reader.read_u32()?;
                let reset_count = reader.read_u32()?;
                Ok(Self::RareMachineRun {
                    npc_idx,
                    inven_idx,
                    reset_count,
                })
            }
            game::PARTY_INVITE => {
                let target_name = reader.read_string()?;
                Ok(Self::PartyInvite { target_name })
            }
            game::PARTY_INVITE_RESPONSE => {
                let result_type = reader.read_i32()?;
                let inviter_name = reader.read_string()?;
                Ok(Self::PartyInviteResponse {
                    result_type,
                    inviter_name,
                })
            }
            game::PARTY_CHAT => {
                let message = reader.read_string()?;
                Ok(Self::PartyChat { message })
            }
            game::PARTY_KICK => {
                let target_name = reader.read_string()?;
                Ok(Self::PartyKick { target_name })
            }
            game::PARTY_LEAVE => Ok(Self::PartyLeave),
            game::PARTY_CHANGE_MASTER => {
                let new_leader_slot = reader.read_i32()?;
                Ok(Self::PartyChangeMaster { new_leader_slot })
            }
            game::PARTY_CHANGE_LOOT => {
                let loot_type = reader.read_i32()?;
                let rare_type = reader.read_u8()?;
                let disp_rare_grade = reader.read_u8()?;
                Ok(Self::PartyChangeLoot {
                    loot_type,
                    rare_type,
                    disp_rare_grade,
                })
            }
            game::PARTY_DISMISS => Ok(Self::PartyDismiss),
            game::GUILD_CREATE => {
                let guild_name = reader.read_string()?;
                Ok(Self::GuildCreate { guild_name })
            }
            game::GUILD_DELETE => Ok(Self::GuildDelete),
            game::GUILD_INVITE => {
                let target_name = reader.read_string()?;
                Ok(Self::GuildInvite { target_name })
            }
            game::GUILD_INVITE_ACCEPT => {
                let guild_id = reader.read_i32()?;
                Ok(Self::GuildInviteAccept { guild_id })
            }
            game::GUILD_INVITE_DENY => {
                let guild_id = reader.read_i32()?;
                Ok(Self::GuildInviteDeny { guild_id })
            }
            game::GUILD_KICK => {
                let member_id = reader.read_i32()?;
                Ok(Self::GuildKick { member_id })
            }
            game::GUILD_LEAVE => Ok(Self::GuildLeave),
            game::GUILD_MESSAGE => {
                let message = reader.read_string()?;
                Ok(Self::GuildMessage { message })
            }
            game::GUILD_NOTICE => {
                let notice = reader.read_string()?;
                Ok(Self::GuildNotice { notice })
            }
            game::GUILD_HISTORY => Ok(Self::GuildHistory),
            game::GUILD_SET_TITLE => {
                let member_id = reader.read_i32()?;
                let title = reader.read_string()?;
                Ok(Self::GuildSetTitle { member_id, title })
            }
            game::TRADE_REQUEST => {
                let target_handler = reader.read_u32()?;
                Ok(Self::TradeRequest { target_handler })
            }
            game::TRADE_ACCEPT => {
                let accepter_handler = reader.read_u32()?;
                Ok(Self::TradeAccept { accepter_handler })
            }
            game::TRADE_CANCEL => Ok(Self::TradeCancel),
            game::TRADE_ADD_ITEM => {
                let item_slot = reader.read_i16()?;
                let trade_slot = reader.read_u8()?;
                Ok(Self::TradeAddItem {
                    item_slot,
                    trade_slot,
                })
            }
            game::TRADE_REMOVE_ITEM => {
                let trade_slot = reader.read_u8()?;
                Ok(Self::TradeRemoveItem { trade_slot })
            }
            game::TRADE_ADD_MONEY => {
                let amount = reader.read_i32()?;
                Ok(Self::TradeAddMoney { amount })
            }
            game::TRADE_CONFIRM => Ok(Self::TradeConfirm),
            game::TRADE_LOCK => Ok(Self::TradeLock),
            game::TRADE_UNLOCK => Ok(Self::TradeUnlock),
            game::SEASON_PASS_DETAILS => Ok(Self::SeasonPassDetails),
            game::SEASON_PASS_PURCHASE_EXP => {
                let purchase_count = reader.read_i32()?;
                Ok(Self::SeasonPassPurchaseExp { purchase_count })
            }
            game::SEASON_PASS_MISSION_REWARD => {
                let mission_id = reader.read_i32()?;
                Ok(Self::SeasonPassMissionReward { mission_id })
            }
            game::SEASON_PASS_SEASON_REWARD => {
                let level = reader.read_i32()?;
                Ok(Self::SeasonPassSeasonReward { level })
            }
            game::CHANGE_CHANNEL => {
                let channel = reader.read_u8()?;
                Ok(Self::ChangeChannel { channel })
            }
            game::CHANNEL_SWITCH_CONFIRM => Ok(Self::ChannelSwitchConfirm),
            game::TAMER_SHOP_LIST => Ok(Self::TamerShopList),
            game::CONSIGNSHOP_WAREHOUSE => Ok(Self::ConsignedWarehouse),
            game::CONSIGNSHOP_WAREHOUSE_RETRIEVE => {
                let item_slot = reader.read_i16()?;
                Ok(Self::ConsignedWarehouseRetrieve { item_slot })
            }
            game::CASHSHOP_BUY_HISTORY => Ok(Self::CashShopBuyHistory),
            game::ADD_FRIEND => {
                let friend_name = reader.read_string()?;
                Ok(Self::AddFriend { friend_name })
            }
            game::FRIEND_LIST => Ok(Self::FriendList),
            game::GUILD_AUTHORITY_MASTER => {
                let member_id = reader.read_i32()?;
                Ok(Self::GuildAuthorityMaster { member_id })
            }
            game::GUILD_AUTHORITY_SUBMASTER => {
                let member_id = reader.read_i32()?;
                Ok(Self::GuildAuthoritySubMaster { member_id })
            }
            game::GUILD_AUTHORITY_MEMBER => {
                let member_id = reader.read_i32()?;
                Ok(Self::GuildAuthorityMember { member_id })
            }
            game::GUILD_AUTHORITY_NEW_MEMBER => {
                let member_id = reader.read_i32()?;
                Ok(Self::GuildAuthorityNewMember { member_id })
            }
            game::GUILD_AUTHORITY_DATS => {
                let member_id = reader.read_i32()?;
                Ok(Self::GuildAuthorityDats { member_id })
            }
            game::HATCH_SPIRIT_EVOLUTION => {
                let model_id = reader.read_i32()?;
                let name = reader.read_string()?;
                let npc_id = reader.read_i32()?;
                Ok(Self::HatchSpiritEvolution {
                    model_id,
                    name,
                    npc_id,
                })
            }
            game::DIGI_SUMMON_PURCHASE => {
                let npc_idx = reader.read_u32()?;
                Ok(Self::DigiSummonPurchase { npc_idx })
            }
            game::LOAD_ACCOUNT_WAREHOUSE => Ok(Self::LoadAccountWarehouse),
            game::RETRIEVE_ACCOUNT_WAREHOUSE => {
                let item_slot = reader.read_i16()?;
                Ok(Self::RetrieveAccountWarehouse { item_slot })
            }
            game::EXTRA_INVENTORY_CATEGORY_REFRESH => {
                let category = reader.read_u8()?;
                Ok(Self::ExtraInventoryCategoryRefresh { category })
            }
            game::EXTRA_INVENTORY_MOVE => {
                let category = reader.read_u16()?;
                let extra_slot = reader.read_u16()?;
                let inventory_slot = reader.read_u16()?;
                Ok(Self::ExtraInventoryMove {
                    category,
                    extra_slot,
                    inventory_slot,
                })
            }
            game::EXTRA_INVENTORY_SORT => {
                let category = reader.read_u8()?;
                Ok(Self::ExtraInventorySort { category })
            }
            game::PARTY_CONFIG_CHANGE => {
                let loot_type = reader.read_u8()?;
                Ok(Self::PartyConfigChange { loot_type })
            }
            game::PARTY_MEMBER_DISCONNECT => Ok(Self::PartyMemberDisconnect),
            game::MONSTER_RESPAWN_TIMER => Ok(Self::MonsterRespawnTimer),
            game::JUMP_BOOSTER => Ok(Self::JumpBooster),
            game::SKILL_LEVEL_UP => {
                let uid = reader.read_u32()?;
                let evo_unit_idx = reader.read_u8()?;
                let skill_idx = reader.read_u8()?;
                Ok(Self::SkillLevelUp {
                    uid,
                    evo_unit_idx,
                    skill_idx,
                })
            }
            game::TAMER_CHARGE_XCRYSTAL => Ok(Self::TamerChargeXCrystal),
            game::TAMER_CONSUME_XCRYSTAL => {
                let amount = reader.read_i32()?;
                Ok(Self::TamerConsumeXCrystal { amount })
            }
            game::TAMER_SUMMON => {
                let target_name = reader.read_string()?;
                Ok(Self::TamerSummon { target_name })
            }
            game::TAMER_SKILL_REQUEST => {
                let skill_idx = reader.read_u32()?;
                let target_uid = reader.read_u32()?;
                Ok(Self::TamerSkillRequest {
                    skill_idx,
                    target_uid,
                })
            }
            game::TRANSCENDENCE_RECEIVE_EXP => Ok(Self::TranscendenceReceiveExp),
            game::TRANSCENDENCE_SUCCESS => Ok(Self::TranscendenceSuccess),
            game::TIME_CHARGE_RESULT => {
                let charge_type = reader.read_u8()?;
                Ok(Self::TimeChargeResult { charge_type })
            }
            game::WARP_GATE_DUNGEON => Ok(Self::WarpGateDungeon),
            game::SPIRIT_CRAFT => {
                let model_id = reader.read_i32()?;
                let name = reader.read_string()?;
                let npc_id = reader.read_i32()?;
                Ok(Self::SpiritCraft {
                    model_id,
                    name,
                    npc_id,
                })
            }
            other => Err(ProtocolError::InvalidGamePacketType(other)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reader::PacketReader;
    use odmo_types::{DEFAULT_PARTNER_MODEL_ID, DEFAULT_START_MAP_ID, DEFAULT_TAMER_MODEL_ID};

    #[test]
    fn parse_initial_information_reads_account_id_from_offset_4() {
        // Client sends: newp(1706); push(XOR); push(accountIdx); push(accessCode); endp();
        // Payload = [XOR(4)][accountIdx(4)][accessCode(4)] = 12 bytes
        // Skip XOR value; read accountIdx at offset 4 in payload
        let mut payload = Vec::new();
        payload.extend_from_slice(&(0xDEAD_u32).to_le_bytes()); // XOR value (skipped)
        payload.extend_from_slice(&(1_u32).to_le_bytes()); // account_idx
        payload.extend_from_slice(&(0xBEEF_u32).to_le_bytes()); // access_code

        let request = GameRequest::try_from(RawPacket {
            length: 0,
            packet_type: game::INITIAL_INFORMATION,
            payload,
        })
        .expect("request should parse");

        assert_eq!(
            request,
            GameRequest::InitialInformation {
                account_id: 1,
                access_code: 0xBEEF,
            }
        );
    }

    #[test]
    fn parse_loot_item_reads_drop_handler() {
        let request = GameRequest::try_from(RawPacket {
            length: 0,
            packet_type: game::LOOT_ITEM,
            payload: (49_200_u32).to_le_bytes().to_vec(),
        })
        .expect("loot request should parse");

        assert_eq!(
            request,
            GameRequest::LootItem {
                drop_handler: 49_200,
            }
        );
    }

    #[test]
    fn initial_info_packet_uses_expected_opcode() {
        let packet = GameInitialInfoPacket {
            character: CharacterSummary {
                id: 1,
                account_id: 1,
                slot: 0,
                name: "Admin".to_string(),
                partner_name: "Agumon".to_string(),
                model: DEFAULT_TAMER_MODEL_ID,
                partner_model: DEFAULT_PARTNER_MODEL_ID,
                ..CharacterSummary::default()
            },
        }
        .encode();

        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        assert_eq!(raw.packet_type, game::INITIAL_INFO_RESPONSE);
        assert!(
            packet.len() > 2_500,
            "packet should resemble the large legacy bootstrap"
        );
    }

    #[test]
    fn load_inventory_packet_uses_expected_opcode() {
        let packet = LoadInventoryPacket {
            inventory: InventorySnapshot {
                bits: 0,
                size: 2,
                items: vec![ItemRecord::default(), ItemRecord::default()],
            },
            inventory_type: InventoryType::Inventory,
        }
        .encode();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        assert_eq!(raw.packet_type, game::LOAD_INVENTORY);
    }

    #[test]
    fn load_tamer_packet_uses_expected_opcode() {
        let packet = LoadTamerPacket {
            character: CharacterSummary {
                id: 1,
                account_id: 1,
                name: "Admin".to_string(),
                partner_name: "Agumon".to_string(),
                partner_current_type: 31_001,
                ..CharacterSummary::default()
            },
        }
        .encode();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        assert_eq!(raw.packet_type, game::LOAD_UNLOAD_ENTITY);
    }

    #[test]
    fn unload_tamer_packet_uses_expected_opcode() {
        let packet = UnloadTamerPacket {
            character: CharacterSummary {
                id: 1,
                account_id: 1,
                name: "Admin".to_string(),
                partner_name: "Agumon".to_string(),
                ..CharacterSummary::default()
            },
        }
        .encode();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        assert_eq!(raw.packet_type, game::LOAD_UNLOAD_ENTITY);
    }

    #[test]
    fn load_buffs_packet_uses_expected_opcode() {
        let packet = LoadBuffsPacket {
            character: CharacterSummary {
                id: 1,
                account_id: 1,
                general_handler: 11_000,
                partner_handler: 21_000,
                active_buffs: vec![ActiveBuffSnapshot {
                    buff_id: 500,
                    buff_class: 1,
                    skill_id: 8001001,
                    remaining_seconds: 60,
                }],
                partner_active_buffs: vec![ActiveBuffSnapshot {
                    buff_id: 600,
                    buff_class: 1,
                    skill_id: 8002001,
                    remaining_seconds: 30,
                }],
                partner_active_debuffs: vec![ActiveBuffSnapshot {
                    buff_id: 700,
                    buff_class: 1,
                    skill_id: 8003001,
                    remaining_seconds: 15,
                }],
                ..CharacterSummary::default()
            },
        }
        .encode();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        assert_eq!(raw.packet_type, game::LOAD_BUFFS);
    }

    #[test]
    fn load_mobs_packet_uses_expected_opcode() {
        let packet = LoadMobsPacket {
            mob: MobSummary {
                id: 900,
                handler: 44_001,
                type_id: 51_001,
                x: 15_000,
                y: 10_000,
                previous_x: 14_980,
                previous_y: 9_980,
                level: 25,
                ..MobSummary::default()
            },
        }
        .encode();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        assert_eq!(raw.packet_type, game::LOAD_UNLOAD_ENTITY);
        assert_eq!(raw.payload[0], 3);
    }

    #[test]
    fn unload_mobs_packet_uses_expected_opcode() {
        let packet = UnloadMobsPacket {
            mob: MobSummary {
                id: 900,
                handler: 44_001,
                x: 15_000,
                y: 10_000,
                ..MobSummary::default()
            },
        }
        .encode();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        assert_eq!(raw.packet_type, game::LOAD_UNLOAD_ENTITY);
        assert_eq!(raw.payload[0], 4);
    }

    #[test]
    fn load_mob_buffs_packet_uses_expected_opcode() {
        let packet = LoadMobBuffsPacket {
            mob: MobSummary {
                id: 900,
                handler: 44_001,
                active_debuffs: vec![ActiveBuffSnapshot {
                    buff_id: 88,
                    buff_class: 1,
                    skill_id: 7001,
                    remaining_seconds: 30,
                }],
                ..MobSummary::default()
            },
        }
        .encode();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        assert_eq!(raw.packet_type, game::LOAD_BUFFS);
        assert_eq!(raw.payload[0], 16);
    }

    #[test]
    fn load_drops_packet_uses_expected_opcode() {
        let packet = LoadDropsPacket {
            drop: DropSummary {
                id: 990,
                handler: 49_200,
                item_id: 90600,
                owner_handler: 11_000,
                x: 15_010,
                y: 10_020,
                ..DropSummary::default()
            },
            viewer_handler: 11_000,
        }
        .encode();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        assert_eq!(raw.packet_type, game::LOAD_UNLOAD_ENTITY);
        assert_eq!(raw.payload[0], 3);
    }

    #[test]
    fn unload_drops_packet_uses_expected_opcode() {
        let packet = UnloadDropsPacket {
            drop: DropSummary {
                id: 990,
                handler: 49_200,
                x: 15_010,
                y: 10_020,
                ..DropSummary::default()
            },
        }
        .encode();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        assert_eq!(raw.packet_type, game::LOAD_UNLOAD_ENTITY);
        assert_eq!(raw.payload[0], 4);
    }

    #[test]
    fn pick_item_packet_uses_expected_opcode() {
        let packet = PickItemPacket {
            appearance_handler: 11_000,
            item_id: 5101,
            amount: 2,
        }
        .encode();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        assert_eq!(raw.packet_type, game::LOOT_ITEM);
    }

    #[test]
    fn pick_bits_packet_uses_expected_opcode() {
        let packet = PickBitsPacket {
            appearance_handler: 11_000,
            value: 123,
        }
        .encode();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        assert_eq!(raw.packet_type, game::PICK_BITS);
    }

    #[test]
    fn pick_item_fail_packet_uses_expected_opcode() {
        let packet = PickItemFailPacket {
            reason: PickItemFailReason::NotTheOwner,
        }
        .encode();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        assert_eq!(raw.packet_type, game::PICK_ITEM_FAIL);
    }

    #[test]
    fn seals_packet_uses_expected_opcode() {
        let packet = SealsPacket {
            seal_list: SealListSnapshot {
                seal_leader_id: 2,
                seals: vec![odmo_types::SealRecord {
                    seal_id: 5101,
                    amount: 12,
                    sequential_id: 7,
                    favorite: true,
                }],
            },
        }
        .encode();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        assert_eq!(raw.packet_type, game::SEALS);
    }

    #[test]
    fn time_reward_packet_uses_expected_opcode() {
        let packet = TimeRewardPacket {
            reward: DailyRewardStatus {
                event_no: 101,
                remaining_seconds: 30,
                total_seconds: 60,
                week: 4,
            },
        }
        .encode();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        assert_eq!(raw.packet_type, game::TIME_REWARD);
    }

    #[test]
    fn attendance_packet_uses_expected_opcode() {
        let packet = TamerAttendancePacket {
            attendance: AttendanceStatus {
                event_no: u8::MAX,
                attendance_count: 0,
                notify: false,
            },
        }
        .encode();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        assert_eq!(raw.packet_type, game::TAMER_ATTENDANCE);
    }

    #[test]
    fn relations_packet_uses_expected_opcode() {
        let packet = TamerRelationsPacket {
            friends: vec![RelationEntry {
                name: "Tai".to_string(),
                connected: true,
                annotation: String::new(),
            }],
            foes: vec![RelationEntry {
                name: "Etemon".to_string(),
                connected: false,
                annotation: "Nemesis".to_string(),
            }],
        }
        .encode();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        assert_eq!(raw.packet_type, game::AVAILABLE_RELATIONS);
    }

    #[test]
    fn friend_connect_packet_uses_expected_opcode() {
        let packet = FriendConnectPacket {
            name: "Tai".to_string(),
        }
        .encode();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        assert_eq!(raw.packet_type, game::FRIEND_CONNECT);
    }

    #[test]
    fn party_invite_packet_uses_expected_opcode() {
        let packet = PartyInvitePacket {
            inviter_name: "AdminTamer".to_string(),
        }
        .encode();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        assert_eq!(raw.packet_type, game::PARTY_INVITE);
    }

    #[test]
    fn party_created_packet_uses_expected_opcode() {
        let packet = PartyCreatedPacket {
            party_id: 77,
            loot_type: 0,
        }
        .encode();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        assert_eq!(raw.packet_type, game::PARTY_CREATED);
    }

    #[test]
    fn party_member_list_packet_uses_expected_opcode() {
        let packet = PartyMemberListPacket {
            party_id: 77,
            my_slot: 1,
            leader_slot: 0,
            loot_type: 0,
            rare_rate: 0,
            disp_rare_grade: 0,
            members: vec![PartyMemberListEntry {
                party_slot: 0,
                character: CharacterSummary {
                    name: "AdminTamer".to_string(),
                    model: DEFAULT_TAMER_MODEL_ID,
                    level: 70,
                    partner_current_type: 31_001,
                    partner_level: 65,
                    partner_name: "Agumon".to_string(),
                    map_id: DEFAULT_START_MAP_ID,
                    channel: 0,
                    general_handler: 11_000,
                    partner_handler: 21_000,
                    ..CharacterSummary::default()
                },
            }],
        }
        .encode();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        assert_eq!(raw.packet_type, game::PARTY_MEMBER_LIST);
    }

    #[test]
    fn party_leave_packet_uses_expected_opcode() {
        let packet = PartyLeavePacket { member_slot: 1 }.encode();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        assert_eq!(raw.packet_type, game::PARTY_LEAVE);
    }

    #[test]
    fn party_kick_packet_uses_expected_opcode() {
        let packet = PartyKickPacket { member_slot: 1 }.encode();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        assert_eq!(raw.packet_type, game::PARTY_KICK);
    }

    #[test]
    fn party_leader_changed_packet_uses_expected_opcode() {
        let packet = PartyLeaderChangedPacket { new_leader_slot: 2 }.encode();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        assert_eq!(raw.packet_type, game::PARTY_CHANGE_MASTER);
    }

    #[test]
    fn party_change_loot_packet_uses_expected_opcode() {
        let packet = PartyChangeLootTypePacket {
            loot_type: 2,
            rare_type: 3,
            disp_rare_grade: 4,
        }
        .encode();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        assert_eq!(raw.packet_type, game::PARTY_CHANGE_LOOT);
    }

    #[test]
    fn party_member_info_packet_uses_expected_opcode() {
        let packet = PartyMemberInfoPacket {
            member_slot: 1,
            digimon_type: 31_001,
            tamer_hp: 1000,
            tamer_max_hp: 1200,
            tamer_ds: 500,
            tamer_max_ds: 700,
            digimon_hp: 800,
            digimon_max_hp: 900,
            digimon_ds: 300,
            digimon_max_ds: 400,
            tamer_level: 70,
            digimon_level: 65,
        }
        .encode();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        assert_eq!(raw.packet_type, game::PARTY_MEMBER_INFO);
    }

    #[test]
    fn party_member_position_packet_uses_expected_opcode() {
        let packet = PartyMemberPositionPacket {
            member_slot: 1,
            tamer_x: 100,
            tamer_y: 200,
            digimon_x: 110,
            digimon_y: 210,
        }
        .encode();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        assert_eq!(raw.packet_type, game::PARTY_MEMBER_POSITION);
    }

    #[test]
    fn party_member_map_change_packet_uses_expected_opcode() {
        let packet = PartyMemberMapChangePacket {
            member_slot: 1,
            map_id: 1,
            channel: 2,
            tamer_handler: 11_000,
            digimon_handler: 21_000,
        }
        .encode();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        assert_eq!(raw.packet_type, game::PARTY_MEMBER_MAP_CHANGE);
    }

    #[test]
    fn party_member_digimon_change_packet_uses_expected_opcode() {
        let packet = PartyMemberDigimonChangePacket {
            member_slot: 1,
            digimon_type: 31_001,
            digimon_name: "Agumon".to_string(),
            digimon_hp: 800,
            digimon_max_hp: 900,
            digimon_ds: 300,
            digimon_max_ds: 400,
        }
        .encode();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        assert_eq!(raw.packet_type, game::PARTY_MEMBER_DIGIMON_CHANGE);
    }

    #[test]
    fn party_member_buff_change_packet_uses_expected_opcode() {
        let packet = PartyMemberBuffChangePacket {
            member_slot: 1,
            buffs: vec![
                PartyMemberBuffEntry {
                    status: 1,
                    buff_code: 700,
                },
                PartyMemberBuffEntry {
                    status: 0,
                    buff_code: 701,
                },
            ],
        }
        .encode();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        assert_eq!(raw.packet_type, game::PARTY_MEMBER_BUFF_CHANGE);
    }

    #[test]
    fn party_member_disconnected_packet_uses_expected_opcode() {
        let packet = PartyMemberDisconnectedPacket { member_slot: 1 }.encode();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        assert_eq!(raw.packet_type, game::PARTY_MEMBER_DISCONNECTED);
    }

    #[test]
    fn digimon_evolution_fail_packet_uses_expected_opcode() {
        let packet = DigimonEvolutionFailPacket.encode();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        assert_eq!(raw.packet_type, game::EVOLUTION_FAILURE);
        let mut payload = PacketReader::new(raw.payload);
        assert_eq!(payload.read_i32().expect("fail payload"), 0);
    }

    #[test]
    fn digimon_evolution_success_packet_uses_expected_opcode() {
        let packet = DigimonEvolutionSuccessPacket {
            digimon_handler: 21_000,
            tamer_handler: 11_000,
            new_type: 31_005,
            evolution_slot: 4,
            hp_rate: 255,
            parts_type: 0,
        }
        .encode();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        assert_eq!(raw.packet_type, game::EVOLUTION);
    }

    #[test]
    fn partner_switch_failure_packet_uses_expected_opcode_and_zero_uid() {
        let packet = PartnerSwitchFailurePacket.encode();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        assert_eq!(raw.packet_type, game::PARTNER_SWITCH_RESPONSE);
        let mut payload = PacketReader::new(raw.payload);
        assert_eq!(payload.read_u32().expect("uid"), 0);
    }

    #[test]
    fn partner_switch_packet_uses_expected_opcode() {
        let packet = PartnerSwitchPacket {
            handler: 21_000,
            old_partner_current_type: 31_001,
            slot: 2,
            partner: odmo_types::PartnerSlotSnapshot {
                slot: 2,
                digimon_type: 31_002,
                model: 31_002,
                level: 11,
                name: "Greymon".to_string(),
                active_buffs: vec![ActiveBuffSnapshot {
                    buff_id: 500,
                    buff_class: 1,
                    skill_id: 8_001_001,
                    remaining_seconds: 30,
                }],
                ..odmo_types::PartnerSlotSnapshot::default()
            },
        }
        .encode();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        assert_eq!(raw.packet_type, game::PARTNER_SWITCH_RESPONSE);
    }

    #[test]
    fn party_invite_request_decodes_modern_client_payload() {
        let mut writer = PacketWriter::new(game::PARTY_INVITE);
        writer.write_string("Matt");
        let packet = writer.finalize();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        let request = GameRequest::try_from(raw).expect("request should decode");
        assert_eq!(
            request,
            GameRequest::PartyInvite {
                target_name: "Matt".to_string(),
            }
        );
    }

    #[test]
    fn partner_evolution_request_decodes_modern_client_payload() {
        let mut writer = PacketWriter::new(game::EVOLUTION);
        writer.write_u32(21_000);
        writer.write_u8(4);
        let packet = writer.finalize();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        let request = GameRequest::try_from(raw).expect("request should decode");
        assert_eq!(
            request,
            GameRequest::PartnerEvolution {
                digimon_handler: 21_000,
                evolution_slot: 4,
            }
        );
    }

    #[test]
    fn party_invite_response_decodes_modern_client_payload() {
        let mut writer = PacketWriter::new(game::PARTY_INVITE_RESPONSE);
        writer.write_i32(1);
        writer.write_string("AdminTamer");
        let packet = writer.finalize();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        let request = GameRequest::try_from(raw).expect("request should decode");
        assert_eq!(
            request,
            GameRequest::PartyInviteResponse {
                result_type: 1,
                inviter_name: "AdminTamer".to_string(),
            }
        );
    }

    #[test]
    fn party_kick_request_decodes_modern_client_payload() {
        let mut writer = PacketWriter::new(game::PARTY_KICK);
        writer.write_string("Matt");
        let packet = writer.finalize();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        let request = GameRequest::try_from(raw).expect("request should decode");
        assert_eq!(
            request,
            GameRequest::PartyKick {
                target_name: "Matt".to_string(),
            }
        );
    }

    #[test]
    fn party_change_master_request_decodes_int_slot() {
        let mut writer = PacketWriter::new(game::PARTY_CHANGE_MASTER);
        writer.write_i32(2);
        let packet = writer.finalize();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        let request = GameRequest::try_from(raw).expect("request should decode");
        assert_eq!(
            request,
            GameRequest::PartyChangeMaster { new_leader_slot: 2 }
        );
    }

    #[test]
    fn party_change_loot_request_decodes_full_payload() {
        let mut writer = PacketWriter::new(game::PARTY_CHANGE_LOOT);
        writer.write_i32(2);
        writer.write_u8(3);
        writer.write_u8(4);
        let packet = writer.finalize();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        let request = GameRequest::try_from(raw).expect("request should decode");
        assert_eq!(
            request,
            GameRequest::PartyChangeLoot {
                loot_type: 2,
                rare_type: 3,
                disp_rare_grade: 4,
            }
        );
    }

    #[test]
    fn guild_information_packet_uses_expected_opcode() {
        let packet = GuildInformationPacket {
            guild: GuildSnapshot {
                id: 77,
                name: "Tamers".to_string(),
                members: vec![odmo_types::GuildMemberSnapshot {
                    authority: 1,
                    character_name: "Tai".to_string(),
                    character_level: 60,
                    character_model: 80_003,
                    map_id: DEFAULT_START_MAP_ID,
                    channel: 2,
                    state: odmo_types::CharacterConnectionState::Ready,
                    ..odmo_types::GuildMemberSnapshot::default()
                }],
                ..GuildSnapshot::default()
            },
        }
        .encode();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        assert_eq!(raw.packet_type, game::GUILD_INFORMATION);
    }

    #[test]
    fn guild_historic_packet_uses_expected_opcode() {
        let packet = GuildHistoricPacket {
            entries: vec![GuildHistoricEntry {
                historic_type: 101,
                date_utc_seconds: 1234,
                master_class: 1,
                master_name: "Tai".to_string(),
                member_class: 5,
                member_name: "TK".to_string(),
            }],
        }
        .encode();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        assert_eq!(raw.packet_type, game::GUILD_HISTORIC);
    }

    #[test]
    fn guild_rank_packet_uses_expected_opcode() {
        let packet = GuildRankPacket { position: 7 }.encode();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        assert_eq!(raw.packet_type, game::GUILD_RANK);
    }

    #[test]
    fn xai_info_packet_uses_expected_opcode() {
        let packet = XaiInfoPacket {
            xai: Some(XaiSnapshot {
                item_id: 131063,
                max_xgauge: 2000,
                max_xcrystals: 3,
            }),
        }
        .encode();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        assert_eq!(raw.packet_type, game::XAI_INFO);
    }

    #[test]
    fn tamer_xai_resources_packet_uses_expected_opcode() {
        let packet = TamerXaiResourcesPacket {
            current_xgauge: 500,
            current_xcrystals: 2,
        }
        .encode();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        assert_eq!(raw.packet_type, game::TAMER_XAI_RESOURCES);
    }
}
