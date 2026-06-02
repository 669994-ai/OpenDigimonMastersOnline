use odmo_types::{
    ActiveBuffSnapshot, AttendanceStatus, ChannelAvailability, CharacterSummary,
    CombineCeilingEntry, CombineItemRef, DailyRewardStatus, DigiCombineReward, DigiSummonProduct,
    DigiSummonReward, DropSummary, GuildHistoricEntry, GuildSnapshot, InventorySnapshot,
    ItemRecord, MobSummary, RelationEntry, SealListSnapshot, XaiSnapshot,
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
        vip: u8,
        npc_id: i32,
        marker: u8,
        shop_slot: i32,
        purchase_count: u16,
    },
    NpcSell {
        vip: u8,
        npc_id: i32,
        marker: u8,
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
    PartnerAttack {
        attacker_handler: u32,
        target_handler: u32,
    },
    PartnerSkill {
        skill_slot: u8,
        attacker_handler: u32,
        target_handler: u32,
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
    RideModeStart,
    RideModeStop,
    OpenRideMode {
        evo_unit_idx: u32,
        item_type: i32,
    },
    /// `pGame::SetTarget` C→S — opcode 1016. `[u4 attacker_handler][u4 target_handler]`.
    SetTarget {
        attacker_handler: u32,
        target_handler: u32,
    },
    /// `pGame::StatUp` C→S — opcode 1030. `[u4 uid][u1 stat]`.
    StatUp {
        uid: u32,
        stat: u8,
    },
    /// `pGame::RefreshScreen` C→S — opcode 1046. Empty payload.
    RefreshScreen,
    /// `pGame::AwayTime` C→S — opcode 1069. Empty payload.
    AwayTime,
    DigimonChangeName {
        inven_slot: i32,
        new_name: String,
    },
    HatchInsertEgg {
        vip: u8,
        inven_slot: u32,
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
        inven_slot: u32,
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
    /// `pItem::SocketIn` C→S — opcode 3926.
    /// Binary-verified wire (sender 0xF24F0):
    /// `[u1 vip][u4 inven_portable_pos][u4 npc_idx][u2 src][u2 dst][u1 socket_order]`.
    ItemSocketIn {
        vip: u8,
        inven_portable_pos: u32,
        npc_idx: i32,
        src_inven_pos: u16,
        dst_inven_pos: u16,
        socket_order: u8,
    },
    /// `pItem::SocketOut` C→S — opcode 3927. Same shape as SocketIn (binary-verified, sender 0xF2630).
    ItemSocketOut {
        vip: u8,
        inven_portable_pos: u32,
        npc_idx: i32,
        src_inven_pos: u16,
        dst_inven_pos: u16,
        socket_order: u8,
    },
    /// `pItem::SocketClear` C→S — opcode 3928. `[i32 npc_idx][u2 inven_pos][u1 socket_order]`.
    /// `pItem::Analysis` C→S — opcode 3929. Binary-verified wire (sender 0xF2770):
    /// `[u1 vip][u4 npc_idx][u4 inven_portable_pos][u2 inven_pos]`.
    ItemSocketIdentify {
        vip: u8,
        npc_idx: i32,
        inven_portable_pos: u32,
        inven_pos: u16,
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
    CashShopBuy {
        amount: u8,
        total_price: i32,
        order_id: u64,
        product_ids: Vec<i32>,
    },
    CashShopReload,
    QuestAvailableList {
        npc_id: i32,
    },
    QuestAccept {
        quest_id: i16,
    },
    QuestDeliver {
        quest_id: i16,
    },
    QuestGiveUp {
        quest_id: i16,
    },
    QuestUpdate {
        quest_id: i16,
        cond_index: u8,
        value: u8,
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
    OtherTamerDetailInfo {
        target_handler: u32,
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
        inven_slot: i32,
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
    /// `GuildCreate` C→S — opcode 2101. `[wstring guild_name][i32 inven_slot][i32 npc_id]`.
    GuildCreate {
        guild_name: String,
        inven_slot: i32,
        npc_id: i32,
    },
    GuildDelete,
    GuildInvite {
        target_name: String,
    },
    /// `GuildAllow` C→S — opcode 2103. `[u32 certified_code][wstring tamer_name]`.
    GuildInviteAccept {
        certified_code: u32,
        target_name: String,
    },
    /// `GuildReject` C→S — opcode 2105. `[u32 certified_code][wstring tamer_name]`.
    GuildInviteDeny {
        certified_code: u32,
        target_name: String,
    },
    /// `GuildDelete` C→S — opcode 2106 (KICK). `[wstring tamer_name]`.
    GuildKick {
        target_name: String,
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
    /// `TradeAddItem` C→S — opcode 1508. Modern client wire: `[u2 inven_pos][u2 amount]`.
    TradeAddItem {
        inven_pos: u16,
        amount: u16,
    },
    /// `TradeCancelItem` C→S — opcode 1531. Modern client wire: `[i1 trade_slot]`.
    TradeRemoveItem {
        trade_slot: i8,
    },
    /// `TradeAddMoney` C→S — opcode 1509. Modern client wire: `[u4 money]`.
    TradeAddMoney {
        amount: u32,
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
    /// `GuildToMaster` C→S — opcode 2119. `[wstring tamer_name]`.
    GuildAuthorityMaster {
        target_name: String,
    },
    /// `GuildToSubMaster` C→S — opcode 2118. `[wstring tamer_name]`.
    GuildAuthoritySubMaster {
        target_name: String,
    },
    /// `GuildToMember` C→S — opcode 2116. `[wstring tamer_name]`.
    GuildAuthorityMember {
        target_name: String,
    },
    /// `GuildToSubMember` C→S — opcode 2115. `[wstring tamer_name]`.
    GuildAuthorityNewMember {
        target_name: String,
    },
    /// `GuildToDatsMember` C→S — opcode 2117. `[wstring tamer_name]`.
    GuildAuthorityDats {
        target_name: String,
    },
    /// Item-to-digimon exchange: spend materials to obtain a new partner.
    /// `[i32 model_id][wstring name][i32 npc_id]`.
    HatchSpiritEvolution {
        model_id: i32,
        name: String,
        npc_id: i32,
    },
    DigiSummonPurchase {
        product_id: i32,
        ticket_slot: i32,
    },
    /// Digivice combine: open/sync the gacha window (bare body).
    DigiCombineSyncRequest,
    /// Digivice combine: submit the selected materials for a roll.
    DigiCombine {
        ceiling_type: u8,
        materials: Vec<CombineItemRef>,
    },
    /// Digivice combine: claim the reward for a resolved ceiling tier.
    DigiCombineRewardClaim {
        ceiling_type: u8,
    },
    /// Union combine: open/sync the gacha window (bare body).
    UnionCombineSyncRequest,
    /// Union combine: submit the selected materials for a roll.
    UnionCombine {
        ceiling_type: u8,
        materials: Vec<CombineItemRef>,
    },
    /// Union combine: claim the reward for a resolved ceiling tier.
    UnionCombineRewardClaim {
        ceiling_type: u8,
    },
    /// D-Unit: open the hacking grid window — returns the unlocked slot count
    /// and the equipped parts per slot (opcode 4311).
    UnionHackOpenRequest,
    /// D-Unit: replace the part installed in a given slot (opcode 4312).
    UnionHackModify {
        slot: u8,
        part_id: i32,
        grade: i16,
    },
    /// Random box: open/sync the box window (5-byte body).
    RandomBoxList {
        flag: u8,
        index: i32,
    },
    /// Random box: purchase a box entry (15-byte body).
    RandomBoxPurchase {
        flag: u8,
        product_id: i32,
        item_uid: i32,
        count: u16,
        state: i32,
    },
    LoadAccountWarehouse,
    RetrieveAccountWarehouse {
        item_slot: i16,
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
    /// Digimon-to-item exchange: delete a partner to obtain materials.
    /// `[u8 slot][string validation][i32 npc_id]`. The validation string carries
    /// the account secondary secret and is checked before any mutation.
    SpiritCraft {
        slot: u8,
        validation: String,
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
            let record = self
                .inventory
                .items
                .get(slot)
                .map_or_else(|| ItemRecord::default().record, normalize_item_record);
            writer.write_bytes(&record);
        }

        writer.finalize()
    }
}

// --- 1006 entity-load framing -------------------------------------------
//
// A 1006 entity block is `[u1 action][u2 count](entry)*count[u1 end]`. Each
// entry opens with a 16-byte header `[u4][u4][u4 kind_handle][u4]` whose third
// dword packs the entity kind in its high word and the 16-bit map handle in its
// low word; the client picks a per-kind body parser from that kind. Strings in
// entity bodies are `[u2 len LE][ASCII]` with no terminator.

/// Entity kinds packed into the high word of an entry header's third dword.
const ENTITY_KIND_DIGIMON: u16 = 1;
const ENTITY_KIND_TAMER: u16 = 2;
const ENTITY_KIND_ITEM: u16 = 3;
const ENTITY_KIND_MONSTER: u16 = 4;

/// Lifecycle action prefixing a 1006 entity block.
const ENTITY_ACTION_NEW: u8 = 1;
const ENTITY_ACTION_IN: u8 = 3;

/// Terminator byte that ends the 1006 dispatcher loop.
const ENTITY_BLOCK_END: u8 = 0;

/// Equipment slot records carried in a tamer body, plus a trailing visual record.
const TAMER_EQUIPMENT_SLOTS: usize = 16;
/// Byte length of one visual slot record.
const VISUAL_SLOT_LEN: usize = 69;
/// Fixed number of clone-stat words a digimon body carries.
const DIGIMON_CLONE_SLOTS: usize = 7;
/// Maximum string length the client accepts in an entity body.
const ENTITY_STRING_MAX: usize = 0x200;

/// Write a 1006 entity-body string as `[u2 len LE][ASCII]` (no terminator).
fn write_entity_string(writer: &mut PacketWriter, value: &str) {
    let bytes = value.as_bytes();
    let len = bytes.len().min(ENTITY_STRING_MAX);
    writer.write_u16(len as u16);
    writer.write_bytes(&bytes[..len]);
}

/// Write the 16-byte entry header: position pair, packed kind+handle, reserved.
fn write_entity_header(writer: &mut PacketWriter, kind: u16, handle: u16, x: i32, y: i32) {
    writer.write_i32(x);
    writer.write_i32(y);
    writer.write_u32((u32::from(kind) << 16) | u32::from(handle));
    writer.write_u32(0);
}

/// Resolve a 16-bit map handle, falling back to the entity id when unset.
fn entity_handle(raw: u32, fallback_id: u64) -> u16 {
    non_zero_handler(raw, fallback_id) as u16
}

#[derive(Debug, Clone, PartialEq)]
pub struct LoadTamerPacket {
    pub character: CharacterSummary,
}

impl LoadTamerPacket {
    /// Encode a peer tamer and its partner digimon as a 1006 `In` block. The
    /// block carries two typed entries (tamer + digimon) followed by the
    /// dispatcher terminator the client loop requires.
    pub fn encode(&self) -> Vec<u8> {
        let c = &self.character;
        let mut writer = PacketWriter::new(game::LOAD_UNLOAD_ENTITY);
        writer.write_u8(ENTITY_ACTION_IN);
        writer.write_u16(2);

        // --- Tamer entry ---
        let tamer_handle = entity_handle(c.general_handler, c.id);
        write_entity_header(&mut writer, ENTITY_KIND_TAMER, tamer_handle, c.x, c.y);
        writer.write_i32(c.x);
        writer.write_i32(c.y);
        write_entity_string(&mut writer, &c.name);
        writer.write_u8(0);
        writer.write_i32(c.model);
        writer.write_u16(c.size as u16);
        writer.write_u8(0);
        writer.write_zeroes(TAMER_EQUIPMENT_SLOTS * VISUAL_SLOT_LEN);
        writer.write_zeroes(VISUAL_SLOT_LEN);
        let condition = c.current_condition as u32;
        writer.write_u32(condition);
        writer.write_u32(0);
        writer.write_u32(0);
        writer.write_u16(c.ms.clamp(0, u16::MAX as i32) as u16);
        writer.write_u8(0); // no secondary name
        writer.write_u16(0);
        writer.write_u8(0);
        writer.write_u16(c.seal_list.seal_leader_id as u16);
        if condition & 0x4 != 0 {
            write_entity_string(&mut writer, &c.shop_name);
        }
        writer.write_u32(0);

        // --- Partner digimon entry ---
        let partner_handle = entity_handle(c.partner_handler, c.id.saturating_add(10_000));
        write_entity_header(
            &mut writer,
            ENTITY_KIND_DIGIMON,
            partner_handle,
            c.partner_x,
            c.partner_y,
        );
        writer.write_i32(c.partner_x);
        writer.write_i32(c.partner_y);
        write_entity_string(&mut writer, &c.partner_name);
        writer.write_u16(c.partner_size as u16);
        writer.write_u8(0);
        writer.write_i32(c.partner_current_type);
        writer.write_u16(0);
        writer.write_u16(0);
        writer.write_u8(0);
        writer.write_u32(0);
        writer.write_u8(0);
        writer.write_u32(0);
        writer.write_u16(DIGIMON_CLONE_SLOTS as u16);
        writer.write_u16(c.partner_clone_level);
        writer.write_u16(c.partner_clone_at_level);
        writer.write_u16(c.partner_clone_bl_level);
        writer.write_u16(c.partner_clone_ct_level);
        writer.write_u16(c.partner_clone_ev_level);
        writer.write_u16(c.partner_clone_hp_level);
        writer.write_u16(0);
        writer.write_u32(0);

        writer.write_u8(ENTITY_BLOCK_END);
        writer.finalize()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadMobsPacket {
    pub mob: MobSummary,
}

impl LoadMobsPacket {
    /// Encode a monster as a single-entry 1006 block. `New` marks a fresh spawn,
    /// `In` an entity already present when the viewer arrives.
    pub fn encode(&self) -> Vec<u8> {
        let m = &self.mob;
        let mut writer = PacketWriter::new(game::LOAD_UNLOAD_ENTITY);
        writer.write_u8(if m.respawn {
            ENTITY_ACTION_NEW
        } else {
            ENTITY_ACTION_IN
        });
        writer.write_u16(1);

        let handle = entity_handle(m.handler, m.id);
        write_entity_header(
            &mut writer,
            ENTITY_KIND_MONSTER,
            handle,
            m.previous_x,
            m.previous_y,
        );
        writer.write_i32(m.x);
        writer.write_i32(m.y);
        writer.write_u8(0);
        writer.write_u8(0);
        writer.write_i32(m.type_id);
        writer.write_i32(m.max_hp);
        writer.write_f32(0.0);
        writer.write_u32(0); // no skill/effect records
        writer.write_u8(ENTITY_BLOCK_END);
        writer.finalize()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadDropsPacket {
    pub drop: DropSummary,
    pub viewer_handler: u32,
}

impl LoadDropsPacket {
    /// Encode a ground item as a single-entry 1006 `In` block. The client
    /// resolves the drop's visuals from the item table, so the body carries only
    /// the item id and an owner/form flag after the entry header.
    pub fn encode(&self) -> Vec<u8> {
        let d = &self.drop;
        let mut writer = PacketWriter::new(game::LOAD_UNLOAD_ENTITY);
        writer.write_u8(ENTITY_ACTION_IN);
        writer.write_u16(1);

        let handle = entity_handle(d.handler, d.id);
        write_entity_header(&mut writer, ENTITY_KIND_ITEM, handle, d.x, d.y);
        writer.write_i32(d.item_id);
        writer.write_u8(0);
        writer.write_u8(ENTITY_BLOCK_END);
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
pub struct DigiSummonSyncResponsePacket {
    pub result: u8,
    pub products: Vec<DigiSummonProduct>,
}

impl DigiSummonSyncResponsePacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::DIGI_SUMMON_SYNC_RESPONSE);
        writer.write_u8(self.result);
        writer.write_u16(clamp_u16_len(self.products.len()));
        for product in &self.products {
            writer.write_i32(product.product_id);
            writer.write_i32(product.rank);
            writer.write_u16(product.draw_count.clamp(0, u16::MAX as i32) as u16);
            writer.write_i32(product.remaining_daily_limit);
        }
        writer.finalize()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DigiSummonPurchaseResponsePacket {
    pub result: u8,
    pub product_id: i32,
    pub rewards: Vec<DigiSummonReward>,
    pub products: Vec<DigiSummonProduct>,
}

impl DigiSummonPurchaseResponsePacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::DIGI_SUMMON_PURCHASE_RESPONSE);
        writer.write_u8(self.result);
        writer.write_i32(self.product_id);
        writer.write_u16(clamp_u16_len(self.rewards.len()));
        for reward in &self.rewards {
            writer.write_i32(reward.item_id);
            writer.write_u16(reward.amount.clamp(1, u16::MAX as i32) as u16);
            writer.write_u16(reward.grade.clamp(0, u16::MAX as i32) as u16);
        }

        writer.write_u16(clamp_u16_len(self.products.len()));
        for product in &self.products {
            writer.write_i32(product.product_id);
            writer.write_i32(product.rank);
            writer.write_u16(product.draw_count.clamp(0, u16::MAX as i32) as u16);
            writer.write_i32(product.remaining_daily_limit);
        }

        // The detail list carries per-reward item descriptors; empty for the
        // common purchase result, so emit a zero count followed by the trailer.
        writer.write_u16(0); // detail_count
        writer.write_i64(0); // trailer
        writer.finalize()
    }
}

/// Combine sync response: a leading result byte and the ceiling map only.
///
/// Digivice combine (3661) and union combine (4301) share this layout; the
/// target opcode selects which flow the response belongs to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CombineSyncResponsePacket {
    pub opcode: i16,
    pub result: u8,
    pub ceiling: Vec<CombineCeilingEntry>,
}

impl CombineSyncResponsePacket {
    /// Build a Digivice combine sync response (opcode 3661).
    pub fn digi(result: u8, ceiling: Vec<CombineCeilingEntry>) -> Self {
        Self {
            opcode: game::DIGI_COMBINE_SYNC,
            result,
            ceiling,
        }
    }

    /// Build a union combine sync response (opcode 4301).
    pub fn union(result: u8, ceiling: Vec<CombineCeilingEntry>) -> Self {
        Self {
            opcode: game::UNION_COMBINE_SYNC,
            result,
            ceiling,
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(self.opcode);
        writer.write_u8(self.result);
        write_combine_ceiling(&mut writer, &self.ceiling);
        writer.finalize()
    }
}

/// Combine result/reward response: result byte, ceiling map, the submitted
/// material echo list, and the reward detail list.
///
/// Digivice combine/reward (3662/3663) and union combine/reward (4302/4303)
/// share this layout; the target opcode selects which flow the response
/// belongs to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CombineResultResponsePacket {
    pub opcode: i16,
    pub result: u8,
    pub ceiling: Vec<CombineCeilingEntry>,
    pub materials: Vec<CombineItemRef>,
    pub rewards: Vec<DigiCombineReward>,
}

impl CombineResultResponsePacket {
    /// Build a Digivice combine result response (opcode 3662).
    pub fn digi_result(
        result: u8,
        ceiling: Vec<CombineCeilingEntry>,
        materials: Vec<CombineItemRef>,
        rewards: Vec<DigiCombineReward>,
    ) -> Self {
        Self {
            opcode: game::DIGI_COMBINE,
            result,
            ceiling,
            materials,
            rewards,
        }
    }

    /// Build a Digivice combine reward response (opcode 3663).
    pub fn digi_reward(
        result: u8,
        ceiling: Vec<CombineCeilingEntry>,
        materials: Vec<CombineItemRef>,
        rewards: Vec<DigiCombineReward>,
    ) -> Self {
        Self {
            opcode: game::DIGI_COMBINE_REWARD,
            result,
            ceiling,
            materials,
            rewards,
        }
    }

    /// Build a union combine result response (opcode 4302).
    pub fn union_result(
        result: u8,
        ceiling: Vec<CombineCeilingEntry>,
        materials: Vec<CombineItemRef>,
        rewards: Vec<DigiCombineReward>,
    ) -> Self {
        Self {
            opcode: game::UNION_COMBINE,
            result,
            ceiling,
            materials,
            rewards,
        }
    }

    /// Build a union combine reward response (opcode 4303).
    pub fn union_reward(
        result: u8,
        ceiling: Vec<CombineCeilingEntry>,
        materials: Vec<CombineItemRef>,
        rewards: Vec<DigiCombineReward>,
    ) -> Self {
        Self {
            opcode: game::UNION_COMBINE_REWARD,
            result,
            ceiling,
            materials,
            rewards,
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(self.opcode);
        writer.write_u8(self.result);
        write_combine_ceiling(&mut writer, &self.ceiling);
        write_combine_item_list(&mut writer, &self.materials);
        write_combine_reward_list(&mut writer, &self.rewards);
        writer.finalize()
    }
}

/// Random box list/sync response (opcode 16067).
///
/// Layout: a leading field, then a `u1`-counted list of fixed entries. Field
/// semantics are not yet decoded; the wire widths and order are fixed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RandomBoxListResponsePacket {
    pub field0: i32,
    pub entries: Vec<RandomBoxListEntry>,
}

/// One entry in the random box list response: three 32-bit fields and a
/// trailing 16-bit field. Semantics are undecoded; widths are fixed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RandomBoxListEntry {
    pub a: i32,
    pub b: i32,
    pub c: i32,
    pub d: u16,
}

impl RandomBoxListResponsePacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::RANDOM_BOX_LIST);
        writer.write_i32(self.field0);
        writer.write_u8(clamp_u8_len(self.entries.len()));
        for entry in &self.entries {
            writer.write_i32(entry.a);
            writer.write_i32(entry.b);
            writer.write_i32(entry.c);
            writer.write_u16(entry.d);
        }
        writer.finalize()
    }
}

/// Random box purchase result response (opcode 16068).
///
/// Layout: three leading fields, a `u1`-counted list of 32-bit pairs, a second
/// `u1`-counted list of `[u64, u16]` pairs, then a trailing `[u64, u16]` summary
/// block. Field semantics are not yet decoded; the wire widths and order are
/// fixed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RandomBoxPurchaseResponsePacket {
    pub field0: i32,
    pub field1: i32,
    pub field2: u16,
    pub list_a: Vec<(i32, i32)>,
    pub list_b: Vec<(u64, u16)>,
    pub summary: (u64, u16),
}

impl RandomBoxPurchaseResponsePacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::RANDOM_BOX_PURCHASE);
        writer.write_i32(self.field0);
        writer.write_i32(self.field1);
        writer.write_u16(self.field2);
        writer.write_u8(clamp_u8_len(self.list_a.len()));
        for (a, b) in &self.list_a {
            writer.write_i32(*a);
            writer.write_i32(*b);
        }
        writer.write_u8(clamp_u8_len(self.list_b.len()));
        for (a, b) in &self.list_b {
            writer.write_u64(*a);
            writer.write_u16(*b);
        }
        let (summary_a, summary_b) = self.summary;
        writer.write_u64(summary_a);
        writer.write_u16(summary_b);
        writer.finalize()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HatchSpiritEvolutionResultPacket {
    pub digimon_id: u32,
    pub remaining_bits: i64,
    pub consumed_items: Vec<(u8, u32)>,
}

impl HatchSpiritEvolutionResultPacket {
    /// Encode the item-to-digimon result: `[u32 digimon_id][i64 remaining_bits]`
    /// followed by consumed-item blocks (`[u8 count][u32 item_id]`) terminated by
    /// a zero count byte.
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::HATCH_SPIRIT_EVOLUTION);
        writer.write_u32(self.digimon_id);
        writer.write_i64(self.remaining_bits);
        for (amount, item_id) in &self.consumed_items {
            writer.write_u8(*amount);
            writer.write_u32(*item_id);
        }
        writer.write_u8(0);
        writer.finalize()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpiritCraftResultPacket {
    pub slot: u8,
    pub remaining_bits: i64,
    pub consumed_items: Vec<(u8, u32)>,
    pub gained_items: Vec<(u8, u32)>,
}

impl SpiritCraftResultPacket {
    /// Encode the digimon-to-item result: `[u8 deleted_slot][i64 remaining_bits]`
    /// followed by a zero-terminated consumed-item block list and then a
    /// zero-terminated gained-item block list (each block `[u8 count][u32 item_id]`).
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::SPIRIT_CRAFT);
        writer.write_u8(self.slot);
        writer.write_i64(self.remaining_bits);
        for (amount, item_id) in &self.consumed_items {
            writer.write_u8(*amount);
            writer.write_u32(*item_id);
        }
        writer.write_u8(0);
        for (amount, item_id) in &self.gained_items {
            writer.write_u8(*amount);
            writer.write_u32(*item_id);
        }
        writer.write_u8(0);
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

/// Hit type for combat damage packets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HitType {
    Normal = 0,
    Critical = 1,
    Block = 2,
}

fn write_modern_damage_block(writer: &mut PacketWriter, damage: i32) {
    // Modern client damage block: 10 i32 values; first carries the (negative) damage,
    // remaining nine are reserved/zero on the legacy server contract.
    writer.write_i32(damage);
    for _ in 1..10 {
        writer.write_i32(0);
    }
}

/// `PartnerAttack` success response — opcode 1013.
/// Mirrors `HitPacket(int attackerHandler, int targetHandler, int finalDamage,
/// long hpBeforeHit, long hpAfterHit, int hitType)` from the legacy server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HitPacket {
    pub attacker_handler: u32,
    pub target_handler: u32,
    pub final_damage: i32,
    pub hp_before_hit: i64,
    pub hp_after_hit: i64,
    pub hit_type: HitType,
}

impl HitPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::PARTNER_ATTACK_RESPONSE);
        writer.write_u32(self.attacker_handler);
        writer.write_u32(self.target_handler);
        write_modern_damage_block(&mut writer, -self.final_damage);
        writer.write_i32(self.hit_type as i32);
        writer.write_i64(self.hp_after_hit);
        writer.write_i64(self.hp_before_hit);
        writer.finalize()
    }
}

/// `PartnerAttack` miss response — opcode 1014.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissHitPacket {
    pub attacker_handler: u32,
    pub target_handler: u32,
}

impl MissHitPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::ATTACK_MISS);
        writer.write_u32(self.attacker_handler);
        writer.write_u32(self.target_handler);
        writer.finalize()
    }
}

/// Lethal partner attack response — opcode 1020.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KillOnHitPacket {
    pub attacker_handler: u32,
    pub target_handler: u32,
    pub final_damage: i32,
    pub hit_type: HitType,
}

impl KillOnHitPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::KILL_ON_HIT);
        writer.write_u32(self.attacker_handler);
        writer.write_u32(self.target_handler);
        write_modern_damage_block(&mut writer, -self.final_damage);
        writer.write_i32(self.hit_type as i32);
        writer.finalize()
    }
}

/// Partner skill cast announcement — opcode 1015.
/// Mirrors `CastSkillPacket(byte skillSlot, int attackerHandler, int targetHandler)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CastSkillPacket {
    pub skill_slot: u8,
    pub attacker_handler: u32,
    pub target_handler: u32,
}

impl CastSkillPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::PARTNER_SKILL_RESPONSE);
        writer.write_u8(self.skill_slot);
        writer.write_u32(self.attacker_handler);
        writer.write_u32(self.target_handler);
        writer.finalize()
    }
}

/// Lethal partner skill response — opcode 1021.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KillOnSkillPacket {
    pub attacker_handler: u32,
    pub target_handler: u32,
    pub skill_slot: u32,
    pub final_damage: i32,
}

impl KillOnSkillPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::KILL_ON_SKILL);
        writer.write_u32(self.attacker_handler);
        writer.write_u32(self.target_handler);
        writer.write_u32(self.skill_slot);
        write_modern_damage_block(&mut writer, -self.final_damage);
        writer.finalize()
    }
}

/// Skill request rejection — opcode 1105.
/// Mirrors `PartnerSkillErrorPacket(int attackerHandler, byte parameter, byte value, byte value2, int context)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartnerSkillErrorPacket {
    pub attacker_handler: u32,
    pub parameter: u8,
    pub value: u8,
    pub value2: u8,
    pub context: i32,
}

impl PartnerSkillErrorPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::PARTNER_SKILL_ERROR);
        writer.write_u32(self.attacker_handler);
        writer.write_u8(self.parameter);
        writer.write_u8(self.value);
        writer.write_u8(self.value2);
        writer.write_i32(self.context);
        writer.finalize()
    }
}

// ===========================================================================
// Quest packets
// ===========================================================================

/// `QuestAvailableList` response — opcode 11009.
/// Mirrors `QuestAvailableListPacket(int npcId, IEnumerable<int> questIds)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestAvailableListPacket {
    pub npc_id: i32,
    pub quest_ids: Vec<i16>,
}

impl QuestAvailableListPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::QUEST_AVAILABLE_LIST);
        writer.write_i32(self.npc_id);
        let len = self.quest_ids.len().min(u16::MAX as usize) as u16;
        writer.write_u16(len);
        for id in self.quest_ids.iter().take(u16::MAX as usize) {
            writer.write_i16(*id);
        }
        writer.finalize()
    }
}

/// `QuestGoalUpdate` packet — opcode 11001.
/// Mirrors `QuestGoalUpdatePacket(short questId, byte goalIndex, short currentGoalValue)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestGoalUpdatePacket {
    pub quest_id: i16,
    pub goal_index: u8,
    pub current_goal_value: i16,
}

impl QuestGoalUpdatePacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::QUEST_GOAL_UPDATE);
        writer.write_i16(self.quest_id);
        writer.write_u8(self.goal_index);
        writer.write_i16(self.current_goal_value);
        writer.finalize()
    }
}

/// `QuestDailyUpdate` packet — opcode 11006. Empty payload; signals daily reset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuestDailyUpdatePacket;

impl QuestDailyUpdatePacket {
    pub fn encode(&self) -> Vec<u8> {
        PacketWriter::new(game::QUEST_DAILY_UPDATE).finalize()
    }
}

// ===========================================================================
// Encyclopedia packets
// ===========================================================================

/// `EncyclopediaLoad` response — opcode 3234.
/// Each entry encodes the unlocked-evolution bitmask + enchant stats + size.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncyclopediaLoadPacket {
    pub entries: Vec<odmo_types::EncyclopediaEntry>,
}

impl EncyclopediaLoadPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::ENCYCLOPEDIA_LOAD_RESPONSE);
        writer.write_i32(self.entries.len() as i32);
        for entry in &self.entries {
            // Build unlocked-slot bitmask (bit n => slot_level (n+1) unlocked).
            let mut slot_opened: u64 = 0;
            for ev in &entry.evolutions {
                if ev.unlocked && ev.slot_level >= 1 && ev.slot_level <= 63 {
                    slot_opened |= 1u64 << (ev.slot_level - 1);
                }
            }
            // The stored "type" is the encyclopedia entry's first evolution type
            // (matches the legacy `EvolutionAsset.Type` field). For the Rust port
            // we use the digimon_evolution_id directly.
            writer.write_i32(entry.digimon_evolution_id as i32);
            writer.write_u16(u16::from(entry.level));
            writer.write_u64(slot_opened);
            writer.write_i16(entry.enchant_at);
            writer.write_i16(entry.enchant_bl);
            writer.write_i16(entry.enchant_ct);
            writer.write_i16(entry.enchant_ev);
            writer.write_i16(entry.enchant_hp);
            writer.write_i16(entry.size);
            // Reward "not allowed" flag = 1 when reward already received.
            writer.write_u8(if entry.reward_received { 1 } else { 0 });
        }
        writer.write_u8(0);
        writer.finalize()
    }
}

/// `EncyclopediaReceiveRewardItem` packet — opcode 3236.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncyclopediaReceiveRewardItemPacket {
    pub item_id: i32,
    pub amount: i16,
}

impl EncyclopediaReceiveRewardItemPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::ENCYCLOPEDIA_GET_REWARD);
        writer.write_u32(self.item_id as u32);
        writer.write_u16(self.amount as u16);
        writer.finalize()
    }
}

/// `EncyclopediaDeckBuffUse` packet — opcode 3237.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EncyclopediaDeckBuffUsePacket {
    pub deck_buff_hp: i32,
    pub deck_buff_as: i16,
}

impl EncyclopediaDeckBuffUsePacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::ENCYCLOPEDIA_DECK_BUFF);
        writer.write_i32(self.deck_buff_hp);
        writer.write_i16(self.deck_buff_as);
        writer.finalize()
    }
}

/// `OtherTamerDetailInfo` response — custom local bridge for the modern DetailInfo family.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OtherTamerDetailInfoPacket {
    pub valid: bool,
    pub target_handler: u32,
    pub tamer_name: String,
    pub guild_name: String,
    pub current_title: i32,
    pub tamer_model: i32,
    pub tamer_level: i32,
    pub tamer_size: i32,
    pub tamer_hp: i32,
    pub tamer_ds: i32,
    pub tamer_at: i32,
    pub tamer_de: i32,
    pub tamer_ms: i32,
    pub partner_name: String,
    pub partner_model: i32,
    pub partner_type: i32,
    pub partner_level: i32,
    pub partner_size: i32,
    pub partner_hp: i32,
    pub partner_ds: i32,
    pub partner_at: i32,
    pub partner_de: i32,
    pub partner_as: i32,
    pub partner_ht: i32,
    pub partner_ct: i32,
    pub partner_bl: i32,
    pub partner_ev: i32,
    pub partner_clone_level: i32,
    pub status: String,
}

impl OtherTamerDetailInfoPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::OTHER_TAMER_DETAIL_INFO_RESPONSE);
        writer.write_u8(self.valid as u8);
        writer.write_u32(self.target_handler);
        writer.write_string(&self.tamer_name);
        writer.write_string(&self.guild_name);
        writer.write_i32(self.current_title);
        writer.write_i32(self.tamer_model);
        writer.write_i32(self.tamer_level);
        writer.write_i32(self.tamer_size);
        writer.write_i32(self.tamer_hp);
        writer.write_i32(self.tamer_ds);
        writer.write_i32(self.tamer_at);
        writer.write_i32(self.tamer_de);
        writer.write_i32(self.tamer_ms);
        writer.write_string(&self.partner_name);
        writer.write_i32(self.partner_model);
        writer.write_i32(self.partner_type);
        writer.write_i32(self.partner_level);
        writer.write_i32(self.partner_size);
        writer.write_i32(self.partner_hp);
        writer.write_i32(self.partner_ds);
        writer.write_i32(self.partner_at);
        writer.write_i32(self.partner_de);
        writer.write_i32(self.partner_as);
        writer.write_i32(self.partner_ht);
        writer.write_i32(self.partner_ct);
        writer.write_i32(self.partner_bl);
        writer.write_i32(self.partner_ev);
        writer.write_i32(self.partner_clone_level);
        writer.write_string(&self.status);
        writer.finalize()
    }
}

// ===========================================================================
// D-Unit / Union hacking & init packets (modern flow, opcode 4311/4312/4313)
// ===========================================================================

/// One slot row in the D-Unit / Union hacking grid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnionHackSlot {
    /// Slot index inside the hacking grid (0..).
    pub slot: u8,
    /// Equipped part model id (0 when empty).
    pub part_id: i32,
    /// Grade / level of the part in that slot.
    pub grade: i16,
    /// Locked flag (cannot be replaced until unlocked with an item).
    pub locked: bool,
}

/// Response payload for `UNION_HACK_OPEN` (opcode 4311).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnionHackOpenResponsePacket {
    pub result: u8,
    pub unlocked_slots: u8,
    pub slots: Vec<UnionHackSlot>,
}

impl UnionHackOpenResponsePacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::UNION_HACK_OPEN_RESPONSE);
        writer.write_u8(self.result);
        writer.write_u8(self.unlocked_slots);
        writer.write_u8(self.slots.len() as u8);
        for slot in &self.slots {
            writer.write_u8(slot.slot);
            writer.write_i32(slot.part_id);
            writer.write_i16(slot.grade);
            writer.write_u8(if slot.locked { 1 } else { 0 });
        }
        writer.finalize()
    }
}

/// Response payload for `UNION_HACK_MODIFY` (opcode 4312).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnionHackModifyResponsePacket {
    pub result: u8,
    pub slot: u8,
    pub new_part_id: i32,
    pub new_grade: i16,
    pub total_rating: i32,
}

impl UnionHackModifyResponsePacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::UNION_HACK_MODIFY_RESPONSE);
        writer.write_u8(self.result);
        writer.write_u8(self.slot);
        writer.write_i32(self.new_part_id);
        writer.write_i16(self.new_grade);
        writer.write_i32(self.total_rating);
        writer.finalize()
    }
}

/// `UNION_INIT_DATA` push (opcode 4313) — sends the full D-Unit / Union state
/// to the client on login so the modern `cUnionContents` can hydrate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnionInitDataPacket {
    pub slots: Vec<UnionHackSlot>,
    pub total_rating: i32,
    pub synergy_bonus: i32,
}

impl UnionInitDataPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::UNION_INIT_DATA);
        writer.write_u8(self.slots.len() as u8);
        for slot in &self.slots {
            writer.write_u8(slot.slot);
            writer.write_i32(slot.part_id);
            writer.write_i16(slot.grade);
            writer.write_u8(if slot.locked { 1 } else { 0 });
        }
        writer.write_i32(self.total_rating);
        writer.write_i32(self.synergy_bonus);
        writer.finalize()
    }
}

// ===========================================================================
// Trade packets
// ===========================================================================

/// `TradeRequest` success — opcode 1501.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TradeRequestSuccessPacket {
    pub target_handler: u32,
}

impl TradeRequestSuccessPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::TRADE_REQUEST_SUCCESS);
        writer.write_u32(self.target_handler);
        writer.finalize()
    }
}

/// `TradeRequest` error — opcode 1507.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TradeRequestErrorPacket {
    pub result: i32,
}

impl TradeRequestErrorPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::TRADE_REQUEST_ERROR);
        writer.write_i32(self.result);
        writer.finalize()
    }
}

/// `TradeAccept` — opcode 1502.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TradeAcceptPacket {
    pub target_handler: u32,
}

impl TradeAcceptPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::TRADE_ACCEPT);
        writer.write_u32(self.target_handler);
        writer.finalize()
    }
}

/// `TradeCancel` — opcode 1506.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TradeCancelPacket {
    pub target_handler: u32,
}

impl TradeCancelPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::TRADE_CANCEL);
        writer.write_u32(self.target_handler);
        writer.finalize()
    }
}

/// `TradeConfirmation` — opcode 1503.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TradeConfirmationPacket {
    pub target_handler: u32,
}

impl TradeConfirmationPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::TRADE_CONFIRM);
        writer.write_u32(self.target_handler);
        writer.finalize()
    }
}

/// `TradeFinalConfirmation` — opcode 1504.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TradeFinalConfirmationPacket {
    pub target_handler: u32,
}

impl TradeFinalConfirmationPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::TRADE_FINAL_CONFIRMATION);
        writer.write_u32(self.target_handler);
        writer.finalize()
    }
}

/// `TradeAddItem` — opcode 1508.
/// `[u32 target][item bytes (~80)][u8 trade_slot][i32 inventory_slot]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TradeAddItemPacket {
    pub target_handler: u32,
    pub item_bytes: Vec<u8>,
    pub trade_slot: u8,
    pub inventory_slot: i32,
}

impl TradeAddItemPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::TRADE_ADD_ITEM);
        writer.write_u32(self.target_handler);
        writer.write_bytes(&self.item_bytes);
        writer.write_u8(self.trade_slot);
        writer.write_i32(self.inventory_slot);
        writer.finalize()
    }
}

/// `TradeRemoveItem` — opcode 1519.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TradeRemoveItemPacket {
    pub target_handler: u32,
    pub trade_slot: u8,
}

impl TradeRemoveItemPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::TRADE_REMOVE_ITEM_RESPONSE);
        writer.write_u32(self.target_handler);
        writer.write_u8(self.trade_slot);
        writer.finalize()
    }
}

/// `TradeAddMoney` — opcode 1509.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TradeAddMoneyPacket {
    pub target_handler: u32,
    pub money: i32,
}

impl TradeAddMoneyPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::TRADE_ADD_MONEY);
        writer.write_u32(self.target_handler);
        writer.write_i32(self.money);
        writer.finalize()
    }
}

/// `TradeInventoryLock` — opcode 1532.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TradeInventoryLockPacket {
    pub target_handler: u32,
}

impl TradeInventoryLockPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::TRADE_LOCK);
        writer.write_u32(self.target_handler);
        writer.write_u8(1);
        writer.finalize()
    }
}

/// `TradeInventoryUnlock` — opcode 1505.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TradeInventoryUnlockPacket {
    pub target_handler: u32,
}

impl TradeInventoryUnlockPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::TRADE_UNLOCK);
        writer.write_u32(self.target_handler);
        writer.write_u8(0);
        writer.finalize()
    }
}

// ===========================================================================
// Arena packets
// ===========================================================================

/// `ArenaRankingDailyLoad` — opcode 4130.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArenaRankingDailyLoadPacket {
    pub remaining_minutes: i64,
    pub points: i32,
}

impl ArenaRankingDailyLoadPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::ARENA_DAILY_RANKING);
        writer.write_u8(1);
        writer.write_i64(self.remaining_minutes);
        writer.write_i32(self.points);
        writer.finalize()
    }
}

/// `ArenaRankingDailyUpdatePoints` — opcode 4131.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArenaRankingDailyUpdatePointsPacket {
    pub points: i32,
}

impl ArenaRankingDailyUpdatePointsPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::ARENA_DAILY_POINTS);
        writer.write_u8(100);
        writer.write_i32(self.points);
        writer.finalize()
    }
}

/// `ArenaRankingInfo` — opcode 16023.
/// `[u8 status][u8 ranking_type][i32 entries] for entry: [name str][i32 points][i32 model][i64 tamer_id]`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArenaRankingInfoPacket {
    pub ranking_type: u8,
    pub entries: Vec<odmo_types::ArenaRankingEntry>,
}

impl ArenaRankingInfoPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::ARENA_RANKING_ALL);
        writer.write_u8(self.ranking_type);
        writer.write_i32(self.entries.len() as i32);
        for entry in &self.entries {
            writer.write_string(&entry.character_name);
            writer.write_i32(entry.points);
            writer.write_i32(entry.character_model);
            writer.write_u64(entry.character_id);
            writer.write_u8(entry.level);
            writer.write_i32(entry.kills);
            writer.write_i32(entry.deaths);
        }
        writer.finalize()
    }
}

/// `ModernArenaRankingInfo` — opcode 16025.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModernArenaRankingInfoPacket {
    pub ranking_type: u8,
    pub entries: Vec<odmo_types::ArenaRankingEntry>,
    pub tamer_position: i32,
}

impl ModernArenaRankingInfoPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::ARENA_REQUEST_RANK);
        writer.write_u8(self.ranking_type);
        writer.write_i32(self.tamer_position);
        writer.write_i32(self.entries.len() as i32);
        for entry in &self.entries {
            writer.write_string(&entry.character_name);
            writer.write_i32(entry.points);
            writer.write_i32(entry.character_model);
            writer.write_u64(entry.character_id);
            writer.write_u8(entry.level);
            writer.write_i32(entry.kills);
            writer.write_i32(entry.deaths);
        }
        writer.finalize()
    }
}

/// `ModernArenaOldRankingInfo` — opcode 16026.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModernArenaOldRankingInfoPacket {
    pub ranking_type: u8,
    pub entries: Vec<odmo_types::ArenaRankingEntry>,
}

impl ModernArenaOldRankingInfoPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::ARENA_REQUEST_OLD_RANK);
        writer.write_u8(self.ranking_type);
        writer.write_i32(self.entries.len() as i32);
        for entry in &self.entries {
            writer.write_string(&entry.character_name);
            writer.write_i32(entry.points);
            writer.write_i32(entry.character_model);
            writer.write_u64(entry.character_id);
            writer.write_u8(entry.level);
        }
        writer.finalize()
    }
}

/// `DungeonArenaNextStage` — opcode 4126.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DungeonArenaNextStagePacket {
    pub current_stage: u8,
    pub npc_id: i32,
    pub remain_time: i32,
}

impl DungeonArenaNextStagePacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::DUNGEON_NEXT_STAGE);
        writer.write_u8(self.current_stage);
        writer.write_i32(self.npc_id);
        writer.write_i32(self.remain_time);
        writer.finalize()
    }
}

// ===========================================================================
// Event packets
// ===========================================================================

/// `BurningEvent` — opcode 3132.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BurningEventPacket {
    pub exp_rate: u32,
    pub next_day_rate: u32,
    pub exp_target: u32,
}

impl BurningEventPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::BURNING_EVENT);
        writer.write_u32(self.exp_rate);
        writer.write_u32(self.next_day_rate);
        writer.write_u32(self.exp_target);
        writer.finalize()
    }
}

/// `DailyCheckEventInfo` — opcode 3136.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DailyCheckEventInfoRow {
    pub group_id: i32,
    pub current_day: i32,
    pub next_left_seconds: i32,
    pub claimed_days: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DailyCheckEventInfoPacket {
    pub rows: Vec<DailyCheckEventInfoRow>,
}

impl DailyCheckEventInfoPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::DAILY_CHECK_EVENT);
        let len = self.rows.len().min(u16::MAX as usize) as u16;
        writer.write_u16(len);
        for row in self.rows.iter().take(u16::MAX as usize) {
            writer.write_i32(row.group_id);
            writer.write_i32(row.current_day);
            writer.write_i32(row.next_left_seconds);
            writer.write_bytes(&row.claimed_days);
        }
        writer.finalize()
    }
}

/// `DailyCheckEventItemResult` — opcode 3137.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DailyCheckEventItemResultPacket {
    pub result: i32,
    pub group_id: i32,
    pub current_day: i32,
    pub next_left_seconds: i32,
    /// Each tuple: (slot, item bytes — modern network array).
    pub items: Vec<(u16, Vec<u8>)>,
}

impl DailyCheckEventItemResultPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::DAILY_CHECK_EVENT_REQUEST);
        writer.write_i32(self.result);
        writer.write_i32(self.group_id);
        writer.write_i32(self.current_day);
        writer.write_i32(self.next_left_seconds);
        writer.write_u8(self.items.len().min(u8::MAX as usize) as u8);
        for (slot, item_bytes) in self.items.iter().take(u8::MAX as usize) {
            writer.write_u16(*slot);
            writer.write_bytes(item_bytes);
        }
        writer.finalize()
    }
}

// ===========================================================================
// Misc packets
// ===========================================================================

/// `UpdateCurrentTitle` — opcode 15.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UpdateCurrentTitlePacket {
    pub handler: u32,
    pub title_id: i16,
}

impl UpdateCurrentTitlePacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::SET_TITLE);
        writer.write_u32(self.handler);
        writer.write_i16(self.title_id);
        writer.finalize()
    }
}

/// `ChangeTamerModel` — opcode 1314.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChangeTamerModelPacket {
    pub new_model: i32,
    pub item_slot: i16,
}

impl ChangeTamerModelPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::CHANGE_TAMER_MODEL);
        writer.write_i32(self.new_model);
        writer.write_i16(self.item_slot);
        writer.finalize()
    }
}

/// `TamerChangeName` — opcode 1311.
/// `[i32 result][string old_name][string new_name][i32 item_slot][u8 flag]`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TamerChangeNamePacket {
    pub result: i32,
    pub item_slot: i32,
    pub old_name: String,
    pub new_name: String,
}

impl TamerChangeNamePacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::TAMER_NAME_CHANGE);
        writer.write_i32(self.result);
        writer.write_string(&self.old_name);
        writer.write_string(&self.new_name);
        writer.write_i32(self.item_slot);
        writer.write_u8(1);
        writer.finalize()
    }
}

/// `RemoveBuff` — opcode 4002.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemoveBuffPacket {
    pub handler: u32,
    pub buff_id: u16,
    pub amount: i16,
}

impl RemoveBuffPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(4002);
        writer.write_u32(self.handler);
        writer.write_i16(self.amount);
        writer.write_u16(self.buff_id);
        writer.finalize()
    }
}

/// `MonsterRespawnTimer` — opcode 16064.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonsterRespawnTimerRow {
    pub state: u8,
    pub target_mob_type: i32,
    pub remaining_value: i32,
    pub source_mob_type: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonsterRespawnTimerPacket {
    pub rows: Vec<MonsterRespawnTimerRow>,
}

impl MonsterRespawnTimerPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::MONSTER_RESPAWN_TIMER);
        writer.write_u8(1);
        let len = self.rows.len().min(u8::MAX as usize) as u8;
        writer.write_u8(len);
        for row in self.rows.iter().take(u8::MAX as usize) {
            writer.write_u8(row.state);
            writer.write_i32(row.target_mob_type);
            writer.write_i32(row.remaining_value);
            writer.write_i32(row.source_mob_type);
        }
        writer.finalize()
    }
}

/// `InventorySort` — opcode 3980.
/// `[u8 inventory_type][i32 0][i16 size][bytes inventory_data]`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InventorySortPacket {
    pub inventory_type: u8,
    pub size: i16,
    pub inventory_data: Vec<u8>,
}

impl InventorySortPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::INVENTORY_SORT);
        writer.write_u8(self.inventory_type);
        writer.write_i32(0);
        writer.write_i16(self.size);
        writer.write_bytes(&self.inventory_data);
        writer.finalize()
    }
}

/// `ItemReturn` — opcode 3923.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemReturnPacket {
    pub received_bits: i32,
    pub previous_bits: i64,
}

impl ItemReturnPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::ITEM_RETURN);
        writer.write_i32(self.received_bits);
        writer.write_i64(self.previous_bits);
        writer.write_i32(0);
        writer.finalize()
    }
}

/// `ItemSocketIn` — opcode 3926.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemSocketInPacket {
    pub money: i32,
}

impl ItemSocketInPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::ITEM_SOCKET_IN);
        writer.write_i32(100);
        writer.write_i32(self.money);
        writer.write_i32(0);
        writer.finalize()
    }
}

/// `ItemSocketOut` — opcode 3927.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemSocketOutPacket {
    pub money: i32,
}

impl ItemSocketOutPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::ITEM_SOCKET_OUT);
        writer.write_i32(100);
        writer.write_i32(self.money);
        writer.write_i32(0);
        writer.finalize()
    }
}

/// `ItemSocketIdentify` — opcode 3929.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemSocketIdentifyPacket {
    pub power: u8,
    pub money: i32,
}

impl ItemSocketIdentifyPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::ITEM_SOCKET_IDENTIFY);
        writer.write_u8(self.power);
        writer.write_i32(self.money);
        writer.write_i32(0);
        writer.finalize()
    }
}

/// `ItemIdentify` — opcode 3968.
/// `[i16 slot][u8 power][u8 reroll_left][i16*4 types][i16*4 values]`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemIdentifyPacket {
    pub slot: i16,
    pub power: u8,
    pub reroll_left: u8,
    pub types: [i16; 4],
    pub values: [i16; 4],
}

impl ItemIdentifyPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::ITEM_IDENTIFY);
        writer.write_i16(self.slot);
        writer.write_u8(self.power);
        writer.write_u8(self.reroll_left);
        for ty in self.types {
            writer.write_i16(ty);
        }
        for val in self.values {
            writer.write_i16(val);
        }
        writer.finalize()
    }
}

/// `ItemReroll` — opcode 3969.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemRerollPacket {
    pub result: u8,
    pub accessory_slot: i16,
    pub power: u8,
    pub reroll_left: u8,
    pub types: [i16; 4],
    pub values: [i16; 4],
}

impl ItemRerollPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::ITEM_REROLL);
        writer.write_u8(self.result);
        writer.write_i16(self.accessory_slot);
        writer.write_u8(self.power);
        writer.write_u8(self.reroll_left);
        for ty in self.types {
            writer.write_i16(ty);
        }
        for val in self.values {
            writer.write_i16(val);
        }
        writer.finalize()
    }
}

/// `LoadGiftStorage` and `LoadRewardStorage` use the same envelope (opcodes 3935 / 16001):
/// `[i32 count][item_record bytes per entry]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemStoragePacket {
    pub opcode: i16,
    pub items: Vec<odmo_types::ItemRecord>,
}

impl ItemStoragePacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(self.opcode);
        writer.write_i32(self.items.len() as i32);
        for item in &self.items {
            writer.write_bytes(&item.record);
        }
        writer.finalize()
    }
}

/// `RecompenseGain` (`16002`) result payload — `[i32 result]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecompenseGainPacket {
    pub result: i32,
}

impl RecompenseGainPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::RECOMPENSE_GAIN);
        writer.write_i32(self.result);
        writer.finalize()
    }
}

/// `GiftStorageRetrieve` (`3936`) — same shape as recompense.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GiftStorageRetrievePacket {
    pub result: i32,
}

impl GiftStorageRetrievePacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::GIFT_STORAGE_RETRIEVE);
        writer.write_i32(self.result);
        writer.finalize()
    }
}

/// `LevelUp` — opcode 1019. `[u32 handler][u8 level]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LevelUpPacket {
    pub handler: u32,
    pub level: u8,
}

impl LevelUpPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(1019);
        writer.write_u32(self.handler);
        writer.write_u8(self.level);
        writer.finalize()
    }
}

/// `ReceiveExp` — opcode 1018.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReceiveExpPacket {
    pub tamer_exp: i64,
    pub tamer_bonus: i64,
    pub tamer_total: i64,
    pub partner_handler: u32,
    pub partner_exp: i64,
    pub partner_bonus: i64,
    pub partner_total: i64,
    pub skill_exp: i64,
}

impl ReceiveExpPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(1018);
        writer.write_i64(self.tamer_exp);
        writer.write_i64(self.tamer_bonus);
        writer.write_i64(self.tamer_total);
        writer.write_u32(self.partner_handler);
        writer.write_i64(self.partner_exp);
        writer.write_i64(self.partner_bonus);
        writer.write_i64(self.partner_total);
        writer.write_i64(self.skill_exp);
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

/// `GuildCreate` success response — opcode 2101.
/// Mirrors `GuildCreateSuccessPacket(string leaderName, int itemSlot, string guildName)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuildCreateSuccessPacket {
    pub leader_name: String,
    pub item_slot: i32,
    pub guild_name: String,
}

impl GuildCreateSuccessPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::GUILD_CREATE);
        writer.write_string(&self.leader_name);
        writer.write_i32(self.item_slot);
        writer.write_string(&self.guild_name);
        writer.finalize()
    }
}

/// `GuildCreate` failure response — opcode 2101 with `itemSlot = -1`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuildCreateFailPacket {
    pub leader_name: String,
    pub guild_name: String,
}

impl GuildCreateFailPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::GUILD_CREATE);
        writer.write_string(&self.leader_name);
        writer.write_i32(-1);
        writer.write_string(&self.guild_name);
        writer.finalize()
    }
}

/// `GuildDelete` notification — opcode 2102.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuildDeletePacket {
    pub guild_name: String,
}

impl GuildDeletePacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::GUILD_DELETE);
        writer.write_string(&self.guild_name);
        writer.finalize()
    }
}

/// `GuildInvite` success response — opcode 2109.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuildInviteSuccessPacket {
    pub target_name: String,
    pub guild_id: u32,
    pub guild_name: String,
}

impl GuildInviteSuccessPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::GUILD_INVITE);
        writer.write_string(&self.target_name);
        writer.write_u32(self.guild_id);
        writer.write_string(&self.guild_name);
        writer.finalize()
    }
}

/// `GuildInvite` failure — opcode 2110.
/// Reason codes mirror `GuildInviteFailEnum` from the legacy server (1=already in guild,
/// 2=offline, 3=on cooldown, 4=invalid target).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuildInviteFailPacket {
    pub reason: i32,
    pub target_name: String,
}

impl GuildInviteFailPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::GUILD_INVITE_FAIL);
        writer.write_i32(self.reason);
        writer.write_string(&self.target_name);
        writer.finalize()
    }
}

/// `GuildInviteAccept` projection of the new member — opcode 2108.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuildInviteAcceptPacket {
    pub authority: u8,
    pub member_model: u8,
    pub character_name: String,
    pub level: u8,
    pub map_id: i16,
    pub channel: u8,
    pub guild_name: String,
}

impl GuildInviteAcceptPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::GUILD_INVITE_RESPONSE_ACCEPT);
        writer.write_u8(self.authority);
        writer.write_u8(self.member_model);
        writer.write_string(&self.character_name);
        writer.write_u8(self.level);
        writer.write_i16(self.map_id);
        writer.write_u8(self.channel);
        writer.write_string(&self.guild_name);
        writer.finalize()
    }
}

/// `GuildInviteDeny` notification — opcode 2105.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuildInviteDenyPacket {
    pub target_name: String,
}

impl GuildInviteDenyPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::GUILD_INVITE_DENY);
        writer.write_string(&self.target_name);
        writer.finalize()
    }
}

/// `GuildMemberKick` notification — opcode 2106.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuildMemberKickPacket {
    pub target_name: String,
}

impl GuildMemberKickPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::GUILD_KICK);
        writer.write_string(&self.target_name);
        writer.finalize()
    }
}

/// `GuildMemberQuit` notification — opcode 2107.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuildMemberQuitPacket {
    pub target_name: String,
}

impl GuildMemberQuitPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::GUILD_LEAVE);
        writer.write_string(&self.target_name);
        writer.finalize()
    }
}

/// `GuildNoticeUpdate` — opcode 2126.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuildNoticeUpdatePacket {
    pub notice: String,
}

impl GuildNoticeUpdatePacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::GUILD_NOTICE);
        writer.write_string(&self.notice);
        writer.finalize()
    }
}

/// `GuildAuthorityUpdate` — opcode 2129.
/// Carries the new authority class and labels for a single rank.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuildAuthorityUpdatePacket {
    pub authority_class: u8,
    pub title: String,
    pub duty: String,
}

impl GuildAuthorityUpdatePacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::GUILD_SET_TITLE);
        writer.write_u8(self.authority_class);
        writer.write_string(&self.title);
        writer.write_string(&self.duty);
        writer.write_u8(0);
        writer.finalize()
    }
}

/// `GuildPromotionDemotion` — opcodes 2115/2116/2117/2118/2119.
/// Used by the per-rank authority change packets which all share the same payload shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuildPromotionDemotionPacket {
    pub opcode: i16,
    pub member_name: String,
    pub authority_description: String,
}

impl GuildPromotionDemotionPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(self.opcode);
        writer.write_string(&self.member_name);
        writer.write_string(&self.authority_description);
        writer.finalize()
    }
}

/// `GuildMessage` (chat) — opcode 2114.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuildMessagePacket {
    pub sender_handler: u32,
    pub sender_name: String,
    pub message: String,
}

impl GuildMessagePacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(game::GUILD_MESSAGE);
        writer.write_u32(self.sender_handler);
        writer.write_string(&self.sender_name);
        writer.write_string(&self.message);
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

fn clamp_u16_len(len: usize) -> u16 {
    len.min(u16::MAX as usize) as u16
}

/// Wire size of a reward detail node. The leading item fields are decoded; the
/// remaining bytes are a reserved fixed-size block (stat/option blobs) carried
/// verbatim and zero-filled when the server has nothing to populate.
const COMBINE_REWARD_NODE_LEN: usize = 71;

/// Read a `u2 count` followed by `count` material nodes `{u4 item_uid, u2 item_type, u2 count}`.
fn read_combine_materials(reader: &mut PacketReader) -> Result<Vec<CombineItemRef>, ProtocolError> {
    let count = reader.read_u16()? as usize;
    let mut materials = Vec::with_capacity(count);
    for _ in 0..count {
        let item_uid = reader.read_u32()?;
        let item_type = reader.read_u16()?;
        let node_count = reader.read_u16()?;
        materials.push(CombineItemRef {
            item_uid,
            item_type,
            count: node_count,
        });
    }
    Ok(materials)
}

/// Write a `u2 count` followed by the material/echo nodes `{u4 item_uid, u2 item_type, u2 count}`.
fn write_combine_item_list(writer: &mut PacketWriter, items: &[CombineItemRef]) {
    writer.write_u16(clamp_u16_len(items.len()));
    for item in items {
        writer.write_u32(item.item_uid);
        writer.write_u16(item.item_type);
        writer.write_u16(item.count);
    }
}

/// Write the ceiling map block: `u2 count` followed by `{u1 tier, u1 value_a, u2 value_b}` entries.
fn write_combine_ceiling(writer: &mut PacketWriter, entries: &[CombineCeilingEntry]) {
    writer.write_u16(clamp_u16_len(entries.len()));
    for entry in entries {
        writer.write_u8(entry.tier);
        writer.write_u8(entry.value_a);
        writer.write_u16(entry.value_b);
    }
}

/// Write the reward detail list: `u2 count` followed by fixed-size reward nodes.
///
/// Each reward detail node carries the decoded leading item fields
/// `{n4 item_id, u2 amount, u1 grade}`; the remaining bytes of the fixed-size
/// node are reserved and emitted as zeros so the wire length matches the node
/// width the client expects.
fn write_combine_reward_list(writer: &mut PacketWriter, rewards: &[DigiCombineReward]) {
    writer.write_u16(clamp_u16_len(rewards.len()));
    for reward in rewards {
        writer.write_i32(reward.item_id);
        writer.write_u16(reward.amount);
        writer.write_u8(reward.grade);
        writer.write_zeroes(COMBINE_REWARD_NODE_LEN - 7);
    }
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
                // On-wire layout:
                //   [u1 vip][u4 npc_id][u1 marker=0x38][n4 shop_slot][u2 count][u1 marker]
                // The leading vip byte is decoded first; the trailing marker is optional.
                let vip = reader.read_u8()?;
                let npc_id = reader.read_i32()?;
                let marker = reader.read_u8()?;
                let shop_slot = reader.read_i32()?;
                let purchase_count = reader.read_u16()?;
                if reader.remaining_len() >= 1 {
                    // Trailing marker byte.
                    let _trailing_marker = reader.read_u8()?;
                }
                Ok(Self::NpcPurchase {
                    vip,
                    npc_id,
                    marker,
                    shop_slot,
                    purchase_count,
                })
            }
            game::NPC_SELL => {
                // On-wire layout:
                //   [u1 vip][u4 npc_id][u1 marker=0x38][u1 inven_slot][u2 count][u1 marker]
                let vip = reader.read_u8()?;
                let npc_id = reader.read_i32()?;
                let marker = reader.read_u8()?;
                let item_slot = reader.read_u8()?;
                let sell_amount = reader.read_u16()?;
                if reader.remaining_len() >= 1 {
                    // Trailing marker byte.
                    let _trailing_marker = reader.read_u8()?;
                }
                Ok(Self::NpcSell {
                    vip,
                    npc_id,
                    marker,
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
            game::AVAILABLE_RELATIONS => Ok(Self::FriendList),
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
            game::PARTNER_ATTACK => {
                let attacker_handler = reader.read_u32()?;
                let target_handler = reader.read_u32()?;
                Ok(Self::PartnerAttack {
                    attacker_handler,
                    target_handler,
                })
            }
            game::PARTNER_SKILL => {
                let skill_slot = reader.read_u8()?;
                let attacker_handler = reader.read_u32()?;
                let target_handler = reader.read_u32()?;
                Ok(Self::PartnerSkill {
                    skill_slot,
                    attacker_handler,
                    target_handler,
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
            game::RIDE_MODE_START => Ok(Self::RideModeStart),
            game::RIDE_MODE_STOP => Ok(Self::RideModeStop),
            game::OPEN_RIDE_MODE => {
                let evo_unit_idx = reader.read_u32()?;
                let item_type = reader.read_i32()?;
                Ok(Self::OpenRideMode {
                    evo_unit_idx,
                    item_type,
                })
            }
            game::SET_TARGET => {
                let attacker_handler = reader.read_u32()?;
                let target_handler = reader.read_u32()?;
                Ok(Self::SetTarget {
                    attacker_handler,
                    target_handler,
                })
            }
            game::STAT_UP => {
                let uid = reader.read_u32()?;
                let stat = reader.read_u8()?;
                Ok(Self::StatUp { uid, stat })
            }
            game::REFRESH_SCREEN => Ok(Self::RefreshScreen),
            game::AWAY_TIME => Ok(Self::AwayTime),
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
                let inven_slot = reader.read_u32()?;
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
                let inven_slot = reader.read_u32()?;
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
                let vip = reader.read_u8()?;
                let inven_portable_pos = reader.read_u32()?;
                let npc_idx = reader.read_i32()?;
                let src_inven_pos = reader.read_u16()?;
                let dst_inven_pos = reader.read_u16()?;
                let socket_order = reader.read_u8()?;
                Ok(Self::ItemSocketIn {
                    vip,
                    inven_portable_pos,
                    npc_idx,
                    src_inven_pos,
                    dst_inven_pos,
                    socket_order,
                })
            }
            game::ITEM_SOCKET_OUT => {
                let vip = reader.read_u8()?;
                let inven_portable_pos = reader.read_u32()?;
                let npc_idx = reader.read_i32()?;
                let src_inven_pos = reader.read_u16()?;
                let dst_inven_pos = reader.read_u16()?;
                let socket_order = reader.read_u8()?;
                Ok(Self::ItemSocketOut {
                    vip,
                    inven_portable_pos,
                    npc_idx,
                    src_inven_pos,
                    dst_inven_pos,
                    socket_order,
                })
            }
            game::ITEM_SOCKET_IDENTIFY => {
                let vip = reader.read_u8()?;
                let npc_idx = reader.read_i32()?;
                let inven_portable_pos = reader.read_u32()?;
                let inven_pos = reader.read_u16()?;
                Ok(Self::ItemSocketIdentify {
                    vip,
                    npc_idx,
                    inven_portable_pos,
                    inven_pos,
                })
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
            game::CASHSHOP_BUY => {
                // On-wire layout:
                //   [u1 item_count][n4 total_price][u8 order_id][n4 product_ids[count]]
                // order_id is a true 8-byte value, so it is read as a u64.
                let amount = reader.read_u8()?;
                let total_price = reader.read_i32()?;
                let order_id = reader.read_u64()?;
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
            game::QUEST_AVAILABLE_LIST => {
                let npc_id = reader.read_i32()?;
                Ok(Self::QuestAvailableList { npc_id })
            }
            game::QUEST_ACCEPT => {
                let quest_id = reader.read_i16()?;
                Ok(Self::QuestAccept { quest_id })
            }
            game::QUEST_DELIVER => {
                let quest_id = reader.read_i16()?;
                Ok(Self::QuestDeliver { quest_id })
            }
            game::QUEST_GIVE_UP => {
                let quest_id = reader.read_i16()?;
                Ok(Self::QuestGiveUp { quest_id })
            }
            game::QUEST_UPDATE => {
                let quest_id = reader.read_i16()?;
                let cond_index = reader.read_u8()?;
                let value = reader.read_u8()?;
                Ok(Self::QuestUpdate {
                    quest_id,
                    cond_index,
                    value,
                })
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
            game::SEAL_REMOVE_LEADER => {
                if reader.remaining_len() == 0 {
                    // The modern client reuses opcode 3234 with an empty payload
                    // for EncyclopediaOpen, while SealMaster unset carries a u16.
                    Ok(Self::EncyclopediaLoad)
                } else {
                    let _card_code = reader.read_u16()?;
                    Ok(Self::SealRemoveLeader)
                }
            }
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
            game::OTHER_TAMER_DETAIL_INFO_REQUEST => {
                let target_handler = reader.read_u32()?;
                Ok(Self::OtherTamerDetailInfo { target_handler })
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
                // Modern client `SendOpenRegion` pushes a single signed byte.
                let region_idx = reader.read_u8()? as i16;
                Ok(Self::RegionUnlock { region_idx })
            }
            game::SET_TITLE => {
                let title_id = reader.read_i16()?;
                Ok(Self::SetTitle { title_id })
            }
            game::CHANGE_TAMER_MODEL => {
                let model_id = reader.read_i32()?;
                let inven_slot = reader.read_i32()?;
                Ok(Self::ChangeTamerModel {
                    model_id,
                    inven_slot,
                })
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
                let inven_slot = if reader.remaining_len() >= 4 {
                    reader.read_i32()?
                } else {
                    0
                };
                let npc_id = if reader.remaining_len() >= 4 {
                    reader.read_i32()?
                } else {
                    0
                };
                Ok(Self::GuildCreate {
                    guild_name,
                    inven_slot,
                    npc_id,
                })
            }
            game::GUILD_DELETE => Ok(Self::GuildDelete),
            game::GUILD_INVITE => {
                let target_name = reader.read_string()?;
                Ok(Self::GuildInvite { target_name })
            }
            game::GUILD_INVITE_ACCEPT => {
                let certified_code = reader.read_u32()?;
                let target_name = reader.read_string()?;
                Ok(Self::GuildInviteAccept {
                    certified_code,
                    target_name,
                })
            }
            game::GUILD_INVITE_DENY => {
                let certified_code = reader.read_u32()?;
                let target_name = reader.read_string()?;
                Ok(Self::GuildInviteDeny {
                    certified_code,
                    target_name,
                })
            }
            game::GUILD_KICK => {
                let target_name = reader.read_string()?;
                Ok(Self::GuildKick { target_name })
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
                let inven_pos = reader.read_u16()?;
                let amount = reader.read_u16()?;
                Ok(Self::TradeAddItem { inven_pos, amount })
            }
            game::TRADE_REMOVE_ITEM => {
                let trade_slot = reader.read_u8()? as i8;
                Ok(Self::TradeRemoveItem { trade_slot })
            }
            game::TRADE_ADD_MONEY => {
                let amount = reader.read_u32()?;
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
            game::GUILD_AUTHORITY_MASTER => {
                let target_name = reader.read_string()?;
                Ok(Self::GuildAuthorityMaster { target_name })
            }
            game::GUILD_AUTHORITY_SUBMASTER => {
                let target_name = reader.read_string()?;
                Ok(Self::GuildAuthoritySubMaster { target_name })
            }
            game::GUILD_AUTHORITY_MEMBER => {
                let target_name = reader.read_string()?;
                Ok(Self::GuildAuthorityMember { target_name })
            }
            game::GUILD_AUTHORITY_NEW_MEMBER => {
                let target_name = reader.read_string()?;
                Ok(Self::GuildAuthorityNewMember { target_name })
            }
            game::GUILD_AUTHORITY_DATS => {
                let target_name = reader.read_string()?;
                Ok(Self::GuildAuthorityDats { target_name })
            }
            game::HATCH_SPIRIT_EVOLUTION => {
                let model_id = reader.read_i32()?;
                let name = reader.read_wide_string()?;
                let npc_id = reader.read_i32()?;
                Ok(Self::HatchSpiritEvolution {
                    model_id,
                    name,
                    npc_id,
                })
            }
            game::DIGI_SUMMON_PURCHASE => {
                let product_id = reader.read_i32()?;
                let ticket_slot = reader.read_i32()?;
                Ok(Self::DigiSummonPurchase {
                    product_id,
                    ticket_slot,
                })
            }
            game::DIGI_COMBINE_SYNC => Ok(Self::DigiCombineSyncRequest),
            game::DIGI_COMBINE => {
                let ceiling_type = reader.read_u8()?;
                let materials = read_combine_materials(&mut reader)?;
                Ok(Self::DigiCombine {
                    ceiling_type,
                    materials,
                })
            }
            game::DIGI_COMBINE_REWARD => {
                let ceiling_type = reader.read_u8()?;
                Ok(Self::DigiCombineRewardClaim { ceiling_type })
            }
            game::UNION_COMBINE_SYNC => Ok(Self::UnionCombineSyncRequest),
            game::UNION_COMBINE => {
                let ceiling_type = reader.read_u8()?;
                let materials = read_combine_materials(&mut reader)?;
                Ok(Self::UnionCombine {
                    ceiling_type,
                    materials,
                })
            }
            game::UNION_COMBINE_REWARD => {
                let ceiling_type = reader.read_u8()?;
                Ok(Self::UnionCombineRewardClaim { ceiling_type })
            }
            game::UNION_HACK_OPEN_REQUEST => Ok(Self::UnionHackOpenRequest),
            game::UNION_HACK_MODIFY_REQUEST => {
                let slot = reader.read_u8()?;
                let part_id = reader.read_i32()?;
                let grade = reader.read_i16()?;
                Ok(Self::UnionHackModify {
                    slot,
                    part_id,
                    grade,
                })
            }
            game::RANDOM_BOX_LIST => {
                let flag = reader.read_u8()?;
                let index = reader.read_i32()?;
                Ok(Self::RandomBoxList { flag, index })
            }
            game::RANDOM_BOX_PURCHASE => {
                let flag = reader.read_u8()?;
                let product_id = reader.read_i32()?;
                let item_uid = reader.read_i32()?;
                let count = reader.read_u16()?;
                let state = reader.read_i32()?;
                Ok(Self::RandomBoxPurchase {
                    flag,
                    product_id,
                    item_uid,
                    count,
                    state,
                })
            }
            game::LOAD_ACCOUNT_WAREHOUSE => Ok(Self::LoadAccountWarehouse),
            game::RETRIEVE_ACCOUNT_WAREHOUSE => {
                let item_slot = reader.read_i16()?;
                Ok(Self::RetrieveAccountWarehouse { item_slot })
            }
            game::EXITEM_BATCH_MOVE => {
                let _unknown = reader.read_u8()?;
                let category = reader.read_u8()?;
                Ok(Self::ExtraInventoryBatchMove { category })
            }
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
            game::EXITEM_SORT => {
                let category = reader.read_u8()?;
                Ok(Self::ExtraInventorySort { category })
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
                let slot = reader.read_u8()?;
                let validation = reader.read_string()?;
                let npc_id = reader.read_i32()?;
                Ok(Self::SpiritCraft {
                    slot,
                    validation,
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
    use odmo_types::{
        CombineCeilingEntry, CombineItemRef, DEFAULT_PARTNER_MODEL_ID, DEFAULT_START_MAP_ID,
        DEFAULT_TAMER_MODEL_ID, DigiCombineReward, DigiSummonProduct, DigiSummonReward,
    };

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
    fn digi_summon_sync_response_uses_modern_wire_shape() {
        let packet = DigiSummonSyncResponsePacket {
            result: 0,
            products: vec![DigiSummonProduct {
                product_id: 9001,
                rank: 2,
                draw_count: 3,
                remaining_daily_limit: 4,
                ..DigiSummonProduct::default()
            }],
        }
        .encode();

        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        assert_eq!(raw.packet_type, game::DIGI_SUMMON_SYNC_RESPONSE);
        // [u8 result][u16 count] + one 14-byte product = 17 bytes.
        assert_eq!(raw.payload.len(), 17);
        let mut reader = crate::PacketReader::new(raw.payload);
        assert_eq!(reader.read_u8().expect("result"), 0);
        assert_eq!(reader.read_u16().expect("count"), 1);
        assert_eq!(reader.read_i32().expect("product"), 9001);
        assert_eq!(reader.read_i32().expect("rank"), 2);
        assert_eq!(reader.read_u16().expect("draw count"), 3);
        assert_eq!(reader.read_i32().expect("daily limit"), 4);
    }

    #[test]
    fn digi_summon_sync_response_empty_list_round_trips() {
        let packet = DigiSummonSyncResponsePacket {
            result: 0,
            products: Vec::new(),
        }
        .encode();

        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        assert_eq!(raw.packet_type, game::DIGI_SUMMON_SYNC_RESPONSE);
        // [u8 result][u16 count] with no products = 3 bytes.
        assert_eq!(raw.payload.len(), 3);
        let mut reader = crate::PacketReader::new(raw.payload);
        assert_eq!(reader.read_u8().expect("result"), 0);
        assert_eq!(reader.read_u16().expect("count"), 0);
    }

    #[test]
    fn digi_summon_purchase_response_uses_modern_wire_shape() {
        let packet = DigiSummonPurchaseResponsePacket {
            result: 0,
            product_id: 9001,
            rewards: vec![DigiSummonReward {
                item_id: 5101,
                amount: 2,
                grade: 5,
                ..DigiSummonReward::default()
            }],
            products: vec![DigiSummonProduct {
                product_id: 9001,
                rank: 1,
                draw_count: 1,
                remaining_daily_limit: 0,
                ..DigiSummonProduct::default()
            }],
        }
        .encode();

        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        assert_eq!(raw.packet_type, game::DIGI_SUMMON_PURCHASE_RESPONSE);
        // [u8 result][i32 product][u16 reward_count] + one 8-byte reward
        // + [u16 product_count] + one 14-byte product + [u16 detail_count]
        // + [i64 trailer] = 41 bytes.
        assert_eq!(raw.payload.len(), 41);
        let mut reader = crate::PacketReader::new(raw.payload);
        assert_eq!(reader.read_u8().expect("result"), 0);
        assert_eq!(reader.read_i32().expect("product"), 9001);
        assert_eq!(reader.read_u16().expect("reward count"), 1);
        assert_eq!(reader.read_i32().expect("reward item"), 5101);
        assert_eq!(reader.read_u16().expect("reward amount"), 2);
        assert_eq!(reader.read_u16().expect("reward grade"), 5);
        assert_eq!(reader.read_u16().expect("product count"), 1);
        assert_eq!(reader.read_i32().expect("synced product"), 9001);
        assert_eq!(reader.read_i32().expect("synced rank"), 1);
        assert_eq!(reader.read_u16().expect("synced draw count"), 1);
        assert_eq!(reader.read_i32().expect("synced daily limit"), 0);
        assert_eq!(reader.read_u16().expect("detail count"), 0);
        assert_eq!(reader.read_u64().expect("trailer"), 0);
    }

    #[test]
    fn digi_summon_purchase_response_empty_lists_round_trip() {
        let packet = DigiSummonPurchaseResponsePacket {
            result: 1,
            product_id: 9001,
            rewards: Vec::new(),
            products: Vec::new(),
        }
        .encode();

        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        assert_eq!(raw.packet_type, game::DIGI_SUMMON_PURCHASE_RESPONSE);
        // [u8 result][i32 product][u16 reward_count=0][u16 product_count=0]
        // [u16 detail_count=0][i64 trailer] = 19 bytes.
        assert_eq!(raw.payload.len(), 19);
        let mut reader = crate::PacketReader::new(raw.payload);
        assert_eq!(reader.read_u8().expect("result"), 1);
        assert_eq!(reader.read_i32().expect("product"), 9001);
        assert_eq!(reader.read_u16().expect("reward count"), 0);
        assert_eq!(reader.read_u16().expect("product count"), 0);
        assert_eq!(reader.read_u16().expect("detail count"), 0);
        assert_eq!(reader.read_u64().expect("trailer"), 0);
    }

    #[test]
    fn random_box_list_request_round_trips() {
        let mut payload = Vec::new();
        payload.push(1_u8); // flag
        payload.extend_from_slice(&(42_i32).to_le_bytes()); // index

        let request = GameRequest::try_from(RawPacket {
            length: 0,
            packet_type: game::RANDOM_BOX_LIST,
            payload,
        })
        .expect("request should parse");

        assert_eq!(request, GameRequest::RandomBoxList { flag: 1, index: 42 });
    }

    #[test]
    fn random_box_purchase_request_round_trips() {
        let mut payload = Vec::new();
        payload.push(1_u8); // flag
        payload.extend_from_slice(&(9001_i32).to_le_bytes()); // product_id
        payload.extend_from_slice(&(7777_i32).to_le_bytes()); // item_uid
        payload.extend_from_slice(&(3_u16).to_le_bytes()); // count
        payload.extend_from_slice(&(5_i32).to_le_bytes()); // state

        let request = GameRequest::try_from(RawPacket {
            length: 0,
            packet_type: game::RANDOM_BOX_PURCHASE,
            payload,
        })
        .expect("request should parse");

        assert_eq!(
            request,
            GameRequest::RandomBoxPurchase {
                flag: 1,
                product_id: 9001,
                item_uid: 7777,
                count: 3,
                state: 5,
            }
        );
    }

    #[test]
    fn random_box_list_response_uses_fixed_wire_shape() {
        let packet = RandomBoxListResponsePacket {
            field0: 11,
            entries: vec![RandomBoxListEntry {
                a: 1,
                b: 2,
                c: 3,
                d: 4,
            }],
        }
        .encode();

        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        assert_eq!(raw.packet_type, game::RANDOM_BOX_LIST);
        // [i32 field0][u8 count] + one 14-byte entry = 19 bytes.
        assert_eq!(raw.payload.len(), 19);
        let mut reader = crate::PacketReader::new(raw.payload);
        assert_eq!(reader.read_i32().expect("field0"), 11);
        assert_eq!(reader.read_u8().expect("count"), 1);
        assert_eq!(reader.read_i32().expect("entry a"), 1);
        assert_eq!(reader.read_i32().expect("entry b"), 2);
        assert_eq!(reader.read_i32().expect("entry c"), 3);
        assert_eq!(reader.read_u16().expect("entry d"), 4);
    }

    #[test]
    fn random_box_list_response_empty_list_round_trips() {
        let packet = RandomBoxListResponsePacket {
            field0: 11,
            entries: Vec::new(),
        }
        .encode();

        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        assert_eq!(raw.packet_type, game::RANDOM_BOX_LIST);
        // [i32 field0][u8 count] with no entries = 5 bytes.
        assert_eq!(raw.payload.len(), 5);
        let mut reader = crate::PacketReader::new(raw.payload);
        assert_eq!(reader.read_i32().expect("field0"), 11);
        assert_eq!(reader.read_u8().expect("count"), 0);
    }

    #[test]
    fn random_box_purchase_response_uses_fixed_wire_shape() {
        let packet = RandomBoxPurchaseResponsePacket {
            field0: 11,
            field1: 22,
            field2: 33,
            list_a: vec![(1, 2)],
            list_b: vec![(0x1122_3344_5566_7788, 9)],
            summary: (0x99AA_BBCC_DDEE_FF00, 7),
        }
        .encode();

        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        assert_eq!(raw.packet_type, game::RANDOM_BOX_PURCHASE);
        // [i32 field0][i32 field1][u16 field2][u8 count1] + one 8-byte pair
        // + [u8 count2] + one 10-byte pair + [u64][u16] summary = 40 bytes.
        assert_eq!(raw.payload.len(), 40);
        let mut reader = crate::PacketReader::new(raw.payload);
        assert_eq!(reader.read_i32().expect("field0"), 11);
        assert_eq!(reader.read_i32().expect("field1"), 22);
        assert_eq!(reader.read_u16().expect("field2"), 33);
        assert_eq!(reader.read_u8().expect("list_a count"), 1);
        assert_eq!(reader.read_i32().expect("list_a a"), 1);
        assert_eq!(reader.read_i32().expect("list_a b"), 2);
        assert_eq!(reader.read_u8().expect("list_b count"), 1);
        assert_eq!(reader.read_u64().expect("list_b a"), 0x1122_3344_5566_7788);
        assert_eq!(reader.read_u16().expect("list_b b"), 9);
        assert_eq!(reader.read_u64().expect("summary a"), 0x99AA_BBCC_DDEE_FF00);
        assert_eq!(reader.read_u16().expect("summary b"), 7);
    }

    #[test]
    fn random_box_purchase_response_empty_lists_round_trip() {
        let packet = RandomBoxPurchaseResponsePacket {
            field0: 11,
            field1: 22,
            field2: 33,
            list_a: Vec::new(),
            list_b: Vec::new(),
            summary: (0x99AA_BBCC_DDEE_FF00, 7),
        }
        .encode();

        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        assert_eq!(raw.packet_type, game::RANDOM_BOX_PURCHASE);
        // [i32 field0][i32 field1][u16 field2][u8 count1=0][u8 count2=0]
        // + [u64][u16] summary = 22 bytes.
        assert_eq!(raw.payload.len(), 22);
        let mut reader = crate::PacketReader::new(raw.payload);
        assert_eq!(reader.read_i32().expect("field0"), 11);
        assert_eq!(reader.read_i32().expect("field1"), 22);
        assert_eq!(reader.read_u16().expect("field2"), 33);
        assert_eq!(reader.read_u8().expect("list_a count"), 0);
        assert_eq!(reader.read_u8().expect("list_b count"), 0);
        assert_eq!(reader.read_u64().expect("summary a"), 0x99AA_BBCC_DDEE_FF00);
        assert_eq!(reader.read_u16().expect("summary b"), 7);
    }

    #[test]
    fn encyclopedia_open_request_reuses_3234_without_payload() {
        let packet = PacketWriter::new(game::SEAL_REMOVE_LEADER).finalize();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");

        let request = GameRequest::try_from(raw).expect("request should decode");
        assert!(matches!(request, GameRequest::EncyclopediaLoad));
    }

    #[test]
    fn seal_remove_leader_request_uses_3234_with_u16_payload() {
        let mut writer = PacketWriter::new(game::SEAL_REMOVE_LEADER);
        writer.write_u16(1);
        let packet = writer.finalize();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");

        let request = GameRequest::try_from(raw).expect("request should decode");
        assert!(matches!(request, GameRequest::SealRemoveLeader));
    }

    #[test]
    fn encyclopedia_open_request_still_accepts_3235_route() {
        let packet = PacketWriter::new(game::ENCYCLOPEDIA_LOAD).finalize();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");

        let request = GameRequest::try_from(raw).expect("request should decode");
        assert!(matches!(request, GameRequest::EncyclopediaLoad));
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
    fn other_tamer_detail_info_packet_uses_i32_levels_and_clone_level() {
        let packet = OtherTamerDetailInfoPacket {
            valid: true,
            target_handler: 33_480,
            tamer_name: "SmokeTamer".to_string(),
            guild_name: "Alpha".to_string(),
            current_title: 12,
            tamer_model: 31_001,
            tamer_level: 70,
            tamer_size: 130,
            tamer_hp: 1200,
            tamer_ds: 800,
            tamer_at: 155,
            tamer_de: 90,
            tamer_ms: 410,
            partner_name: "Agumon".to_string(),
            partner_model: 51_001,
            partner_type: 3,
            partner_level: 65,
            partner_size: 125,
            partner_hp: 2200,
            partner_ds: 1600,
            partner_at: 320,
            partner_de: 210,
            partner_as: 430,
            partner_ht: 17,
            partner_ct: 25,
            partner_bl: 9,
            partner_ev: 12,
            partner_clone_level: 8,
            status: "Detail info synchronized.".to_string(),
        }
        .encode();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        assert_eq!(raw.packet_type, game::OTHER_TAMER_DETAIL_INFO_RESPONSE);

        let mut reader = PacketReader::new(raw.payload);
        assert_eq!(reader.read_u8().expect("valid"), 1);
        assert_eq!(reader.read_u32().expect("handler"), 33_480);
        assert_eq!(reader.read_string().expect("tamer_name"), "SmokeTamer");
        assert_eq!(reader.read_string().expect("guild_name"), "Alpha");
        assert_eq!(reader.read_i32().expect("current_title"), 12);
        assert_eq!(reader.read_i32().expect("tamer_model"), 31_001);
        assert_eq!(reader.read_i32().expect("tamer_level"), 70);
        assert_eq!(reader.read_i32().expect("tamer_size"), 130);
        assert_eq!(reader.read_i32().expect("tamer_hp"), 1200);
        assert_eq!(reader.read_i32().expect("tamer_ds"), 800);
        assert_eq!(reader.read_i32().expect("tamer_at"), 155);
        assert_eq!(reader.read_i32().expect("tamer_de"), 90);
        assert_eq!(reader.read_i32().expect("tamer_ms"), 410);
        assert_eq!(reader.read_string().expect("partner_name"), "Agumon");
        assert_eq!(reader.read_i32().expect("partner_model"), 51_001);
        assert_eq!(reader.read_i32().expect("partner_type"), 3);
        assert_eq!(reader.read_i32().expect("partner_level"), 65);
        assert_eq!(reader.read_i32().expect("partner_size"), 125);
        assert_eq!(reader.read_i32().expect("partner_hp"), 2200);
        assert_eq!(reader.read_i32().expect("partner_ds"), 1600);
        assert_eq!(reader.read_i32().expect("partner_at"), 320);
        assert_eq!(reader.read_i32().expect("partner_de"), 210);
        assert_eq!(reader.read_i32().expect("partner_as"), 430);
        assert_eq!(reader.read_i32().expect("partner_ht"), 17);
        assert_eq!(reader.read_i32().expect("partner_ct"), 25);
        assert_eq!(reader.read_i32().expect("partner_bl"), 9);
        assert_eq!(reader.read_i32().expect("partner_ev"), 12);
        assert_eq!(reader.read_i32().expect("partner_clone_level"), 8);
        assert_eq!(
            reader.read_string().expect("status"),
            "Detail info synchronized."
        );
        assert_eq!(reader.remaining_len(), 0);
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
    fn hatch_spirit_evolution_result_packet_uses_expected_opcode() {
        let packet = HatchSpiritEvolutionResultPacket {
            digimon_id: 31_004,
            remaining_bits: 450,
            consumed_items: vec![(1, 81_001), (1, 81_002)],
        }
        .encode();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        assert_eq!(raw.packet_type, game::HATCH_SPIRIT_EVOLUTION);
        // [u32 id][i64 bits] + two 5-byte consumed blocks + [u8 0] = 23 bytes.
        assert_eq!(raw.payload.len(), 23);
        let mut payload = PacketReader::new(raw.payload);
        assert_eq!(payload.read_u32().expect("digimon id"), 31_004);
        assert_eq!(payload.read_u64().expect("bits"), 450);
        assert_eq!(payload.read_u8().expect("count1"), 1);
        assert_eq!(payload.read_u32().expect("item1"), 81_001);
        assert_eq!(payload.read_u8().expect("count2"), 1);
        assert_eq!(payload.read_u32().expect("item2"), 81_002);
        assert_eq!(payload.read_u8().expect("end"), 0);
    }

    #[test]
    fn hatch_spirit_evolution_result_empty_list_round_trips() {
        let packet = HatchSpiritEvolutionResultPacket {
            digimon_id: 31_004,
            remaining_bits: 450,
            consumed_items: Vec::new(),
        }
        .encode();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        assert_eq!(raw.packet_type, game::HATCH_SPIRIT_EVOLUTION);
        // [u32 id][i64 bits] + [u8 0] terminator = 13 bytes.
        assert_eq!(raw.payload.len(), 13);
        let mut payload = PacketReader::new(raw.payload);
        assert_eq!(payload.read_u32().expect("digimon id"), 31_004);
        assert_eq!(payload.read_u64().expect("bits"), 450);
        assert_eq!(payload.read_u8().expect("end"), 0);
    }

    #[test]
    fn hatch_spirit_evolution_result_single_entry_round_trips() {
        let packet = HatchSpiritEvolutionResultPacket {
            digimon_id: 31_004,
            remaining_bits: 450,
            consumed_items: vec![(2, 81_001)],
        }
        .encode();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        assert_eq!(raw.packet_type, game::HATCH_SPIRIT_EVOLUTION);
        // [u32 id][i64 bits] + one 5-byte consumed block + [u8 0] = 18 bytes.
        assert_eq!(raw.payload.len(), 18);
        let mut payload = PacketReader::new(raw.payload);
        assert_eq!(payload.read_u32().expect("digimon id"), 31_004);
        assert_eq!(payload.read_u64().expect("bits"), 450);
        assert_eq!(payload.read_u8().expect("count1"), 2);
        assert_eq!(payload.read_u32().expect("item1"), 81_001);
        assert_eq!(payload.read_u8().expect("end"), 0);
    }

    #[test]
    fn spirit_craft_result_packet_uses_expected_opcode() {
        let packet = SpiritCraftResultPacket {
            slot: 2,
            remaining_bits: 250,
            consumed_items: vec![(1, 81_001)],
            gained_items: vec![(1, 81_003)],
        }
        .encode();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        assert_eq!(raw.packet_type, game::SPIRIT_CRAFT);
        // [u8 slot][i64 bits] + one 5-byte consumed block + [u8 0]
        // + one 5-byte gained block + [u8 0] = 21 bytes.
        assert_eq!(raw.payload.len(), 21);
        let mut payload = PacketReader::new(raw.payload);
        assert_eq!(payload.read_u8().expect("slot"), 2);
        assert_eq!(payload.read_u64().expect("bits"), 250);
        assert_eq!(payload.read_u8().expect("consumed count"), 1);
        assert_eq!(payload.read_u32().expect("consumed item"), 81_001);
        assert_eq!(payload.read_u8().expect("consumed end"), 0);
        assert_eq!(payload.read_u8().expect("gained count"), 1);
        assert_eq!(payload.read_u32().expect("gained item"), 81_003);
        assert_eq!(payload.read_u8().expect("gained end"), 0);
    }

    #[test]
    fn spirit_craft_result_empty_lists_round_trip() {
        let packet = SpiritCraftResultPacket {
            slot: 2,
            remaining_bits: 250,
            consumed_items: Vec::new(),
            gained_items: Vec::new(),
        }
        .encode();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        assert_eq!(raw.packet_type, game::SPIRIT_CRAFT);
        // [u8 slot][i64 bits] + [u8 0] consumed terminator + [u8 0] gained
        // terminator = 11 bytes.
        assert_eq!(raw.payload.len(), 11);
        let mut payload = PacketReader::new(raw.payload);
        assert_eq!(payload.read_u8().expect("slot"), 2);
        assert_eq!(payload.read_u64().expect("bits"), 250);
        assert_eq!(payload.read_u8().expect("consumed end"), 0);
        assert_eq!(payload.read_u8().expect("gained end"), 0);
    }

    #[test]
    fn combine_sync_response_digi_empty_ceiling_round_trips() {
        let packet = CombineSyncResponsePacket::digi(0, Vec::new()).encode();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        assert_eq!(raw.packet_type, game::DIGI_COMBINE_SYNC);
        // [u8 result][u16 count=0] = 3 bytes.
        assert_eq!(raw.payload.len(), 3);
        let mut reader = PacketReader::new(raw.payload);
        assert_eq!(reader.read_u8().expect("result"), 0);
        assert_eq!(reader.read_u16().expect("ceiling count"), 0);
    }

    #[test]
    fn combine_sync_response_digi_single_ceiling_round_trips() {
        let packet = CombineSyncResponsePacket::digi(
            1,
            vec![CombineCeilingEntry {
                tier: 3,
                value_a: 7,
                value_b: 0x1234,
            }],
        )
        .encode();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        assert_eq!(raw.packet_type, game::DIGI_COMBINE_SYNC);
        // [u8 result][u16 count] + one 4-byte ceiling entry = 7 bytes.
        assert_eq!(raw.payload.len(), 7);
        let mut reader = PacketReader::new(raw.payload);
        assert_eq!(reader.read_u8().expect("result"), 1);
        assert_eq!(reader.read_u16().expect("ceiling count"), 1);
        assert_eq!(reader.read_u8().expect("tier"), 3);
        assert_eq!(reader.read_u8().expect("value_a"), 7);
        assert_eq!(reader.read_u16().expect("value_b"), 0x1234);
    }

    #[test]
    fn combine_sync_response_union_uses_union_opcode() {
        let packet = CombineSyncResponsePacket::union(
            0,
            vec![CombineCeilingEntry {
                tier: 5,
                value_a: 9,
                value_b: 0x00AB,
            }],
        )
        .encode();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        assert_eq!(raw.packet_type, game::UNION_COMBINE_SYNC);
        assert_eq!(raw.payload.len(), 7);
        let mut reader = PacketReader::new(raw.payload);
        assert_eq!(reader.read_u8().expect("result"), 0);
        assert_eq!(reader.read_u16().expect("ceiling count"), 1);
        assert_eq!(reader.read_u8().expect("tier"), 5);
        assert_eq!(reader.read_u8().expect("value_a"), 9);
        assert_eq!(reader.read_u16().expect("value_b"), 0x00AB);
    }

    #[test]
    fn combine_result_response_digi_result_empty_lists_round_trip() {
        let packet =
            CombineResultResponsePacket::digi_result(0, Vec::new(), Vec::new(), Vec::new())
                .encode();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        assert_eq!(raw.packet_type, game::DIGI_COMBINE);
        // [u8 result][u16 ceiling=0][u16 materials=0][u16 rewards=0] = 7 bytes.
        assert_eq!(raw.payload.len(), 7);
        let mut reader = PacketReader::new(raw.payload);
        assert_eq!(reader.read_u8().expect("result"), 0);
        assert_eq!(reader.read_u16().expect("ceiling count"), 0);
        assert_eq!(reader.read_u16().expect("material count"), 0);
        assert_eq!(reader.read_u16().expect("reward count"), 0);
    }

    #[test]
    fn combine_result_response_digi_result_single_entries_round_trip() {
        let packet = CombineResultResponsePacket::digi_result(
            1,
            vec![CombineCeilingEntry {
                tier: 2,
                value_a: 4,
                value_b: 0x0102,
            }],
            vec![CombineItemRef {
                item_uid: 0xAABB_CCDD,
                item_type: 0x0011,
                count: 0x0022,
            }],
            vec![DigiCombineReward {
                item_id: 5101,
                amount: 3,
                grade: 6,
            }],
        )
        .encode();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        assert_eq!(raw.packet_type, game::DIGI_COMBINE);
        // [u8 result] + [u16 count + 4B ceiling] + [u16 count + 8B material]
        // + [u16 count + 71B reward node] = 1 + 6 + 10 + 73 = 90 bytes.
        assert_eq!(raw.payload.len(), 90);
        let mut reader = PacketReader::new(raw.payload);
        assert_eq!(reader.read_u8().expect("result"), 1);
        assert_eq!(reader.read_u16().expect("ceiling count"), 1);
        assert_eq!(reader.read_u8().expect("tier"), 2);
        assert_eq!(reader.read_u8().expect("value_a"), 4);
        assert_eq!(reader.read_u16().expect("value_b"), 0x0102);
        assert_eq!(reader.read_u16().expect("material count"), 1);
        assert_eq!(reader.read_u32().expect("item_uid"), 0xAABB_CCDD);
        assert_eq!(reader.read_u16().expect("item_type"), 0x0011);
        assert_eq!(reader.read_u16().expect("material count field"), 0x0022);
        assert_eq!(reader.read_u16().expect("reward count"), 1);
        // Reward node: leading 7 decoded bytes, then 64 reserved zero bytes.
        assert_eq!(reader.read_i32().expect("reward item id"), 5101);
        assert_eq!(reader.read_u16().expect("reward amount"), 3);
        assert_eq!(reader.read_u8().expect("reward grade"), 6);
        let reserved = reader.read_bytes(64).expect("reserved reward block");
        assert!(
            reserved.iter().all(|&b| b == 0),
            "reserved reward bytes must be zero"
        );
    }

    #[test]
    fn combine_result_response_digi_reward_uses_reward_opcode() {
        let packet =
            CombineResultResponsePacket::digi_reward(0, Vec::new(), Vec::new(), Vec::new())
                .encode();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        assert_eq!(raw.packet_type, game::DIGI_COMBINE_REWARD);
        assert_eq!(raw.payload.len(), 7);
    }

    #[test]
    fn combine_result_response_union_result_uses_union_opcode() {
        let packet =
            CombineResultResponsePacket::union_result(0, Vec::new(), Vec::new(), Vec::new())
                .encode();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        assert_eq!(raw.packet_type, game::UNION_COMBINE);
        assert_eq!(raw.payload.len(), 7);
    }

    #[test]
    fn combine_result_response_union_reward_uses_union_opcode() {
        let packet =
            CombineResultResponsePacket::union_reward(0, Vec::new(), Vec::new(), Vec::new())
                .encode();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        assert_eq!(raw.packet_type, game::UNION_COMBINE_REWARD);
        assert_eq!(raw.payload.len(), 7);
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
    fn spirit_craft_request_decodes_modern_client_payload() {
        let mut writer = PacketWriter::new(game::SPIRIT_CRAFT);
        writer.write_u8(2);
        writer.write_string("4321");
        writer.write_i32(91001);
        let packet = writer.finalize();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        let request = GameRequest::try_from(raw).expect("request should decode");
        assert_eq!(
            request,
            GameRequest::SpiritCraft {
                slot: 2,
                validation: "4321".to_string(),
                npc_id: 91001,
            }
        );
    }

    #[test]
    fn hatch_spirit_evolution_request_decodes_wide_string_name() {
        let mut writer = PacketWriter::new(game::HATCH_SPIRIT_EVOLUTION);
        writer.write_i32(31_004);
        writer.write_wide_string("Agumon");
        writer.write_i32(91001);
        let packet = writer.finalize();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        let request = GameRequest::try_from(raw).expect("request should decode");
        assert_eq!(
            request,
            GameRequest::HatchSpiritEvolution {
                model_id: 31_004,
                name: "Agumon".to_string(),
                npc_id: 91001,
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

    #[test]
    fn hit_packet_uses_expected_opcode_and_layout() {
        let packet = HitPacket {
            attacker_handler: 11_000,
            target_handler: 22_000,
            final_damage: 250,
            hp_before_hit: 1_000,
            hp_after_hit: 750,
            hit_type: HitType::Normal,
        }
        .encode();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        assert_eq!(raw.packet_type, game::PARTNER_ATTACK_RESPONSE);
        // [u32 attacker][u32 target][10 * i32 damage block][i32 hit type][i64 hp_after][i64 hp_before]
        // = 4 + 4 + 40 + 4 + 8 + 8 = 68 bytes payload
        assert_eq!(raw.payload.len(), 68);
    }

    #[test]
    fn miss_hit_packet_uses_expected_opcode() {
        let packet = MissHitPacket {
            attacker_handler: 11_000,
            target_handler: 22_000,
        }
        .encode();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        assert_eq!(raw.packet_type, game::ATTACK_MISS);
        assert_eq!(raw.payload.len(), 8);
    }

    #[test]
    fn kill_on_hit_packet_uses_expected_opcode() {
        let packet = KillOnHitPacket {
            attacker_handler: 11_000,
            target_handler: 22_000,
            final_damage: 9999,
            hit_type: HitType::Critical,
        }
        .encode();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        assert_eq!(raw.packet_type, game::KILL_ON_HIT);
        // [u32 attacker][u32 target][10 * i32 damage block][i32 hit type] = 4 + 4 + 40 + 4 = 52
        assert_eq!(raw.payload.len(), 52);
    }

    #[test]
    fn cast_skill_packet_uses_expected_opcode() {
        let packet = CastSkillPacket {
            skill_slot: 2,
            attacker_handler: 11_000,
            target_handler: 22_000,
        }
        .encode();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        assert_eq!(raw.packet_type, game::PARTNER_SKILL_RESPONSE);
        // [u8 slot][u32 attacker][u32 target] = 1 + 4 + 4 = 9
        assert_eq!(raw.payload.len(), 9);
    }

    #[test]
    fn kill_on_skill_packet_uses_expected_opcode() {
        let packet = KillOnSkillPacket {
            attacker_handler: 11_000,
            target_handler: 22_000,
            skill_slot: 3,
            final_damage: 5000,
        }
        .encode();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        assert_eq!(raw.packet_type, game::KILL_ON_SKILL);
        // [u32 attacker][u32 target][u32 skill][10 * i32 damage block] = 4 + 4 + 4 + 40 = 52
        assert_eq!(raw.payload.len(), 52);
    }

    #[test]
    fn partner_skill_error_packet_uses_expected_opcode() {
        let packet = PartnerSkillErrorPacket {
            attacker_handler: 11_000,
            parameter: 1,
            value: 5,
            value2: 0,
            context: 0,
        }
        .encode();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        assert_eq!(raw.packet_type, game::PARTNER_SKILL_ERROR);
        // [u32 attacker][u8][u8][u8][i32 context] = 4 + 1 + 1 + 1 + 4 = 11
        assert_eq!(raw.payload.len(), 11);
    }

    #[test]
    fn hit_packet_damage_block_carries_negative_value() {
        let packet = HitPacket {
            attacker_handler: 11_000,
            target_handler: 22_000,
            final_damage: 250,
            hp_before_hit: 1_000,
            hp_after_hit: 750,
            hit_type: HitType::Normal,
        }
        .encode();
        // skip [length:u2][opcode:i2] = 4 bytes header, then [u32 attacker][u32 target] = 8 bytes payload
        // damage block starts at offset 4 + 4 + 4 = 12 (within frame, where length is at 0..2 and opcode is at 2..4)
        let damage_offset_in_frame = 4 + 4 + 4;
        let damage_bytes = [
            packet[damage_offset_in_frame],
            packet[damage_offset_in_frame + 1],
            packet[damage_offset_in_frame + 2],
            packet[damage_offset_in_frame + 3],
        ];
        let damage = i32::from_le_bytes(damage_bytes);
        assert_eq!(damage, -250);
    }

    #[test]
    fn move_item_request_decodes_modern_client_payload() {
        let mut writer = PacketWriter::new(game::MOVE_ITEM);
        writer.write_u16(12);
        writer.write_u16(34);
        let packet = writer.finalize();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        let request = GameRequest::try_from(raw).expect("request should decode");
        assert_eq!(
            request,
            GameRequest::MoveItem {
                origin_slot: 12,
                destination_slot: 34,
            }
        );
    }

    #[test]
    fn split_item_request_decodes_modern_client_payload() {
        let mut writer = PacketWriter::new(game::SPLIT_ITEM);
        writer.write_u16(5);
        writer.write_u16(9);
        writer.write_u16(20);
        let packet = writer.finalize();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        let request = GameRequest::try_from(raw).expect("request should decode");
        assert_eq!(
            request,
            GameRequest::SplitItem {
                origin_slot: 5,
                destination_slot: 9,
                amount: 20,
            }
        );
    }

    #[test]
    fn item_remove_request_decodes_modern_client_payload() {
        let mut writer = PacketWriter::new(game::ITEM_REMOVE);
        writer.write_u16(7);
        writer.write_i32(120);
        writer.write_i32(-30);
        writer.write_u16(3);
        let packet = writer.finalize();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        let request = GameRequest::try_from(raw).expect("request should decode");
        assert_eq!(
            request,
            GameRequest::RemoveItem {
                slot: 7,
                x: 120,
                y: -30,
                amount: 3,
            }
        );
    }

    #[test]
    fn npc_purchase_request_decodes_modern_client_payload() {
        let mut writer = PacketWriter::new(game::NPC_PURCHASE);
        writer.write_u8(1);
        writer.write_i32(9001);
        writer.write_u8(0x38);
        writer.write_i32(4);
        writer.write_u16(2);
        writer.write_u8(0x38);
        let packet = writer.finalize();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        let request = GameRequest::try_from(raw).expect("request should decode");
        assert_eq!(
            request,
            GameRequest::NpcPurchase {
                vip: 1,
                npc_id: 9001,
                marker: 0x38,
                shop_slot: 4,
                purchase_count: 2,
            }
        );
    }

    #[test]
    fn npc_sell_request_decodes_modern_client_payload() {
        let mut writer = PacketWriter::new(game::NPC_SELL);
        writer.write_u8(1);
        writer.write_i32(9001);
        writer.write_u8(0x38);
        writer.write_u8(6);
        writer.write_u16(2);
        writer.write_u8(0x38);
        let packet = writer.finalize();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        let request = GameRequest::try_from(raw).expect("request should decode");
        assert_eq!(
            request,
            GameRequest::NpcSell {
                vip: 1,
                npc_id: 9001,
                marker: 0x38,
                item_slot: 6,
                sell_amount: 2,
            }
        );
    }

    #[test]
    fn item_socket_in_request_decodes_modern_client_payload() {
        let mut writer = PacketWriter::new(game::ITEM_SOCKET_IN);
        writer.write_u8(1);
        writer.write_u32(40_010);
        writer.write_i32(9001);
        writer.write_u16(3);
        writer.write_u16(7);
        writer.write_u8(2);
        let packet = writer.finalize();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        let request = GameRequest::try_from(raw).expect("request should decode");
        assert_eq!(
            request,
            GameRequest::ItemSocketIn {
                vip: 1,
                inven_portable_pos: 40_010,
                npc_idx: 9001,
                src_inven_pos: 3,
                dst_inven_pos: 7,
                socket_order: 2,
            }
        );
    }

    #[test]
    fn item_socket_out_request_decodes_modern_client_payload() {
        let mut writer = PacketWriter::new(game::ITEM_SOCKET_OUT);
        writer.write_u8(1);
        writer.write_u32(40_010);
        writer.write_i32(9001);
        writer.write_u16(3);
        writer.write_u16(7);
        writer.write_u8(2);
        let packet = writer.finalize();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        let request = GameRequest::try_from(raw).expect("request should decode");
        assert_eq!(
            request,
            GameRequest::ItemSocketOut {
                vip: 1,
                inven_portable_pos: 40_010,
                npc_idx: 9001,
                src_inven_pos: 3,
                dst_inven_pos: 7,
                socket_order: 2,
            }
        );
    }

    #[test]
    fn item_socket_identify_request_decodes_modern_client_payload() {
        let mut writer = PacketWriter::new(game::ITEM_SOCKET_IDENTIFY);
        writer.write_u8(1);
        writer.write_i32(9001);
        writer.write_u32(40_010);
        writer.write_u16(5);
        let packet = writer.finalize();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        let request = GameRequest::try_from(raw).expect("request should decode");
        assert_eq!(
            request,
            GameRequest::ItemSocketIdentify {
                vip: 1,
                npc_idx: 9001,
                inven_portable_pos: 40_010,
                inven_pos: 5,
            }
        );
    }

    #[test]
    fn partner_attack_request_decodes_modern_client_payload() {
        let mut writer = PacketWriter::new(game::PARTNER_ATTACK);
        writer.write_u32(21_000);
        writer.write_u32(48_500);
        let packet = writer.finalize();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        let request = GameRequest::try_from(raw).expect("request should decode");
        assert_eq!(
            request,
            GameRequest::PartnerAttack {
                attacker_handler: 21_000,
                target_handler: 48_500,
            }
        );
    }

    #[test]
    fn partner_skill_request_decodes_modern_client_payload() {
        let mut writer = PacketWriter::new(game::PARTNER_SKILL);
        writer.write_u8(2);
        writer.write_u32(21_000);
        writer.write_u32(48_500);
        let packet = writer.finalize();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        let request = GameRequest::try_from(raw).expect("request should decode");
        assert_eq!(
            request,
            GameRequest::PartnerSkill {
                skill_slot: 2,
                attacker_handler: 21_000,
                target_handler: 48_500,
            }
        );
    }

    #[test]
    fn partner_switch_request_decodes_modern_client_payload() {
        let mut writer = PacketWriter::new(game::PARTNER_SWITCH);
        writer.write_u8(3);
        let packet = writer.finalize();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        let request = GameRequest::try_from(raw).expect("request should decode");
        assert_eq!(request, GameRequest::PartnerSwitch { slot: 3 });
    }

    #[test]
    fn partner_delete_request_decodes_modern_client_payload() {
        let mut writer = PacketWriter::new(game::PARTNER_DELETE);
        writer.write_u8(2);
        writer.write_string("secret42");
        let packet = writer.finalize();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        let request = GameRequest::try_from(raw).expect("request should decode");
        assert_eq!(
            request,
            GameRequest::PartnerDelete {
                slot: 2,
                validation: "secret42".to_string(),
            }
        );
    }

    #[test]
    fn open_ride_mode_request_decodes_modern_client_payload() {
        let mut writer = PacketWriter::new(game::OPEN_RIDE_MODE);
        writer.write_u32(7);
        writer.write_i32(101);
        let packet = writer.finalize();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        let request = GameRequest::try_from(raw).expect("request should decode");
        assert_eq!(
            request,
            GameRequest::OpenRideMode {
                evo_unit_idx: 7,
                item_type: 101,
            }
        );
    }

    #[test]
    fn ride_mode_start_request_decodes_empty_payload() {
        let writer = PacketWriter::new(game::RIDE_MODE_START);
        let packet = writer.finalize();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        let request = GameRequest::try_from(raw).expect("request should decode");
        assert_eq!(request, GameRequest::RideModeStart);
    }

    #[test]
    fn ride_mode_stop_request_decodes_empty_payload() {
        let writer = PacketWriter::new(game::RIDE_MODE_STOP);
        let packet = writer.finalize();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        let request = GameRequest::try_from(raw).expect("request should decode");
        assert_eq!(request, GameRequest::RideModeStop);
    }

    #[test]
    fn open_region_request_decodes_modern_client_payload() {
        let mut writer = PacketWriter::new(game::REGION_UNLOCK);
        writer.write_u8(5);
        let packet = writer.finalize();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        let request = GameRequest::try_from(raw).expect("request should decode");
        assert_eq!(request, GameRequest::RegionUnlock { region_idx: 5 });
    }

    #[test]
    fn set_target_request_decodes_modern_client_payload() {
        let mut writer = PacketWriter::new(game::SET_TARGET);
        writer.write_u32(21_000);
        writer.write_u32(48_500);
        let packet = writer.finalize();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        let request = GameRequest::try_from(raw).expect("request should decode");
        assert_eq!(
            request,
            GameRequest::SetTarget {
                attacker_handler: 21_000,
                target_handler: 48_500,
            }
        );
    }

    #[test]
    fn away_time_request_decodes_empty_payload() {
        let writer = PacketWriter::new(game::AWAY_TIME);
        let packet = writer.finalize();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        let request = GameRequest::try_from(raw).expect("request should decode");
        assert_eq!(request, GameRequest::AwayTime);
    }

    #[test]
    fn change_tamer_model_request_decodes_modern_client_payload() {
        let mut writer = PacketWriter::new(game::CHANGE_TAMER_MODEL);
        writer.write_i32(8001);
        writer.write_i32(4);
        let packet = writer.finalize();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        let request = GameRequest::try_from(raw).expect("request should decode");
        assert_eq!(
            request,
            GameRequest::ChangeTamerModel {
                model_id: 8001,
                inven_slot: 4,
            }
        );
    }

    #[test]
    fn cash_shop_reload_request_decodes_empty_payload() {
        let writer = PacketWriter::new(game::CASHSHOP_RELOAD);
        let packet = writer.finalize();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        let request = GameRequest::try_from(raw).expect("request should decode");
        assert_eq!(request, GameRequest::CashShopReload);
    }

    #[test]
    fn cash_shop_buy_request_decodes_modern_client_payload() {
        let mut writer = PacketWriter::new(game::CASHSHOP_BUY);
        writer.write_u8(2);
        writer.write_i32(1500);
        writer.write_u64(0x0102_0304_0506_0708);
        writer.write_i32(40_010);
        writer.write_i32(40_020);
        let packet = writer.finalize();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        let request = GameRequest::try_from(raw).expect("request should decode");
        assert_eq!(
            request,
            GameRequest::CashShopBuy {
                amount: 2,
                total_price: 1500,
                order_id: 0x0102_0304_0506_0708,
                product_ids: vec![40_010, 40_020],
            }
        );
    }

    #[test]
    fn hatch_backup_insert_request_decodes_modern_client_payload() {
        let mut writer = PacketWriter::new(game::HATCH_BACKUP_INSERT);
        writer.write_u8(1);
        writer.write_u32(12);
        writer.write_i32(9001);
        let packet = writer.finalize();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        let request = GameRequest::try_from(raw).expect("request should decode");
        assert_eq!(
            request,
            GameRequest::HatchBackupInsert {
                vip: 1,
                inven_slot: 12,
                npc_idx: 9001,
            }
        );
    }
}

#[cfg(test)]
mod sync_1006_exploration {
    //! Exploration test for the opcode-1006 (pGame::Sync / MAP_ENTITY) entity-load
    //! misalignment.
    //!
    //! This encodes a faithful model of the client's 1006 dispatcher and its
    //! subtype-3 handler, then asserts every 1006 entity-load payload is consumed
    //! to its exact end. It is the executable form of the wire contract: the
    //! dispatcher reads one `subtype` byte; the subtype-3 handler reads a
    //! `[u2 count]` and then walks `count` fixed 16-byte entries.
    //!
    //! Property 1 (Bug Condition): a 1006 sub-packet must be consumed exactly by
    //! the client parser - no underrun ("need N bytes, have M"), no leftover bytes.
    //!
    //! Validates: Requirements 1.1, 1.2, 1.3

    use super::*;
    use crate::reader::PacketReader;
    use odmo_types::{CharacterSummary, DropSummary, MobSummary};

    /// Result of running the client parser model over a 1006 payload.
    #[derive(Debug, PartialEq, Eq)]
    enum ParseOutcome {
        /// The handler consumed the payload to its exact end.
        Exact,
        /// The handler read past the buffer end (mirrors the client's
        /// "Insufficient data (need N bytes, have M)" exception).
        Underrun {
            need: usize,
            have: usize,
            offset: usize,
        },
        /// The handler returned before the payload end, leaving bytes unconsumed.
        Leftover {
            consumed: usize,
            total: usize,
            next_byte: u8,
        },
    }

    /// Bounds-checked cursor that mirrors the client's positional reads. A read
    /// past the end records the shortfall instead of consuming, matching the
    /// client's `cPacket::pop: Insufficient data` behavior.
    struct ModelCursor<'a> {
        buf: &'a [u8],
        pos: usize,
        underrun: Option<(usize, usize, usize)>,
    }

    impl<'a> ModelCursor<'a> {
        fn new(buf: &'a [u8]) -> Self {
            Self {
                buf,
                pos: 0,
                underrun: None,
            }
        }

        fn take(&mut self, n: usize) -> Option<&'a [u8]> {
            if self.pos + n > self.buf.len() {
                self.underrun = Some((n, self.buf.len() - self.pos, self.pos));
                return None;
            }
            let slice = &self.buf[self.pos..self.pos + n];
            self.pos += n;
            Some(slice)
        }

        fn read_u8(&mut self) -> Option<u8> {
            self.take(1).map(|s| s[0])
        }

        fn read_u16(&mut self) -> Option<u16> {
            self.take(2).map(|s| u16::from_le_bytes([s[0], s[1]]))
        }

        fn read_u32(&mut self) -> Option<u32> {
            self.take(4)
                .map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
        }

        /// `[u2 len LE][len ASCII bytes]`, no terminator. The client caps the
        /// length at 0x200 and treats an out-of-range length as empty; a
        /// misaligned length that points past the buffer underruns - the
        /// observed `need N bytes` symptom.
        fn read_entity_string(&mut self) -> Option<()> {
            let len = self.read_u16()? as usize;
            if len != 0 && len <= ENTITY_STRING_MAX {
                self.take(len)?;
            }
            Some(())
        }
    }

    /// Consume a digimon (kind 1) body, mirroring `sub_1040E0`'s read order:
    /// handle pair, name, size, flag, model, two shorts, a flag, then a fixed
    /// block of post-spawn fields and the `[u2 count] + 7 shorts + u4` clone tail.
    fn consume_digimon_body(cur: &mut ModelCursor) -> Option<()> {
        cur.read_u32()?; // handle pair
        cur.read_u32()?;
        cur.read_entity_string()?; // name
        cur.read_u16()?; // size
        cur.read_u8()?; // flag
        cur.read_u32()?; // model
        cur.read_u16()?;
        cur.read_u16()?;
        cur.read_u8()?;
        cur.read_u32()?;
        cur.read_u8()?;
        cur.read_u32()?;
        let clone_count = cur.read_u16()?; // clone-stat count
        for _ in 0..clone_count {
            cur.read_u16()?;
        }
        cur.read_u32()?; // item/aura id
        Some(())
    }

    /// Consume a tamer (kind 2) body, mirroring `sub_1032C0`: handle pair, name,
    /// flag, model, size, condition flag, the 16x69 equipment block plus a 69-byte
    /// visual record, three condition/sync dwords, speed, an optional second name,
    /// a couple of trailing fields, the seal id, an optional shop name when the
    /// condition bitfield requests it, and a trailing dword.
    fn consume_tamer_body(cur: &mut ModelCursor) -> Option<()> {
        cur.read_u32()?; // handle pair
        cur.read_u32()?;
        cur.read_entity_string()?; // name
        cur.read_u8()?; // flag
        cur.read_u32()?; // model
        cur.read_u16()?; // size
        cur.read_u8()?; // condition flag
        cur.take(TAMER_EQUIPMENT_SLOTS * VISUAL_SLOT_LEN)?; // equipment slots
        cur.take(VISUAL_SLOT_LEN)?; // trailing visual record
        let condition = cur.read_u32()?; // condition bitfield
        cur.read_u32()?;
        cur.read_u32()?;
        cur.read_u16()?; // speed
        let has_second_name = cur.read_u8()?; // secondary-name flag
        if has_second_name != 0 {
            cur.read_u32()?;
            cur.read_entity_string()?;
        }
        cur.read_u16()?;
        cur.read_u8()?;
        cur.read_u16()?; // seal id
        if condition & 0x4 != 0 {
            cur.read_entity_string()?; // shop name
        }
        cur.read_u32()?; // trailing
        Some(())
    }

    /// Consume an item/drop (kind 3) body, mirroring `sub_103210`:
    /// `[u4 item_id][u1 form]`.
    fn consume_item_body(cur: &mut ModelCursor) -> Option<()> {
        cur.read_u32()?; // item id
        cur.read_u8()?; // form
        Some(())
    }

    /// Consume a monster (kind 4) body, mirroring `sub_104F80`: handle/object
    /// pair, two flags, type id, HP/scale base, a float, then a skill/effect
    /// sub-count and that many `[u4][f4][u4][u4]` records.
    fn consume_monster_body(cur: &mut ModelCursor) -> Option<()> {
        cur.read_u32()?; // handle/object pair
        cur.read_u32()?;
        cur.read_u8()?; // state
        cur.read_u8()?; // flag
        cur.read_u32()?; // type id
        cur.read_u32()?; // HP/scale base
        cur.read_u32()?; // float
        let record_count = cur.read_u32()?; // skill/effect count
        for _ in 0..record_count {
            cur.read_u32()?;
            cur.read_u32()?;
            cur.read_u32()?;
            cur.read_u32()?;
        }
        Some(())
    }

    /// Consume one typed entry: a 16-byte header whose third dword carries the
    /// entity kind in its high word, then the kind-specific body. An unknown kind
    /// consumes no body (the client's `default` branch), which strands the cursor
    /// and surfaces as leftover/underrun downstream - exactly the drift the bug
    /// produces.
    fn consume_entry(cur: &mut ModelCursor) -> Option<()> {
        cur.read_u32()?; // header word 0
        cur.read_u32()?; // header word 1
        let kind_handle = cur.read_u32()?; // header word 2: HIWORD = kind
        cur.read_u32()?; // header word 3
        let kind = (kind_handle >> 16) as u16;
        match kind {
            1 => consume_digimon_body(cur),
            2 => consume_tamer_body(cur),
            3 => consume_item_body(cur),
            4 => consume_monster_body(cur),
            _ => Some(()),
        }
    }

    /// Model of a `New`/`In` count-framed handler (`sub_102420` / `sub_102630`):
    /// read `[u2 count]`, then consume `count` typed entries.
    fn consume_count_framed(cur: &mut ModelCursor) -> Option<()> {
        let count = cur.read_u16()?;
        for _ in 0..count {
            consume_entry(cur)?;
        }
        Some(())
    }

    /// Faithful model of the client 1006 dispatcher `sub_101F50`: a loop that
    /// reads `[u1 action]`, runs the matching handler, then reads the next action
    /// byte until a `0x00` terminator ends the block. Returns whether the payload
    /// was consumed exactly, underran, or left trailing bytes.
    fn client_parse_1006(payload: &[u8]) -> ParseOutcome {
        let mut cur = ModelCursor::new(payload);
        loop {
            let action = match cur.read_u8() {
                Some(b) => b,
                None => {
                    let (need, have, offset) = cur.underrun.expect("underrun recorded on None");
                    return ParseOutcome::Underrun { need, have, offset };
                }
            };

            if action == ENTITY_BLOCK_END {
                break;
            }

            // New (1) and In (3) carry count-framed typed entries; other
            // lifecycle actions are not produced by these encoders.
            let handled = match action {
                ENTITY_ACTION_NEW | ENTITY_ACTION_IN => consume_count_framed(&mut cur),
                _ => Some(()),
            };

            if handled.is_none() {
                let (need, have, offset) = cur.underrun.expect("underrun recorded on None");
                return ParseOutcome::Underrun { need, have, offset };
            }
        }

        if cur.pos == payload.len() {
            ParseOutcome::Exact
        } else {
            ParseOutcome::Leftover {
                consumed: cur.pos,
                total: payload.len(),
                next_byte: payload[cur.pos],
            }
        }
    }

    /// Decode a finalized frame and run the client 1006 parser model over its
    /// payload (payload starts at frame offset 4, after length + opcode).
    fn parse_encoded_1006(frame: &[u8]) -> ParseOutcome {
        let raw = PacketReader::from_frame(frame).expect("frame should decode");
        assert_eq!(
            raw.packet_type,
            game::LOAD_UNLOAD_ENTITY,
            "opcode must be 1006"
        );
        client_parse_1006(&raw.payload)
    }

    /// Assert exact consumption, formatting the counterexample like the client's
    /// own diagnostic when the payload is misaligned.
    fn assert_consumed_exactly(label: &str, frame: &[u8]) {
        let outcome = parse_encoded_1006(frame);
        assert!(
            outcome == ParseOutcome::Exact,
            "{label}: client 1006 parser did not consume the payload exactly: {outcome:?}"
        );
    }

    fn tamer_fixture(seed: u32) -> CharacterSummary {
        let name = match seed % 3 {
            0 => String::new(),
            1 => "Admin".to_string(),
            _ => "LongTamerName".to_string(),
        };
        CharacterSummary {
            id: 1 + seed as u64,
            account_id: 1,
            name,
            partner_name: "Agumon".to_string(),
            x: 15_000 + seed as i32,
            y: 10_000 + seed as i32,
            level: (1 + seed % 99) as u8,
            general_handler: if seed.is_multiple_of(2) {
                0
            } else {
                11_000 + seed
            },
            partner_handler: if seed.is_multiple_of(2) {
                0
            } else {
                21_000 + seed
            },
            partner_current_type: 31_001,
            ..CharacterSummary::default()
        }
    }

    fn mob_fixture(seed: u32) -> MobSummary {
        MobSummary {
            id: 900 + seed as u64,
            handler: if seed.is_multiple_of(2) {
                0
            } else {
                44_000 + seed
            },
            type_id: 51_001 + seed as i32,
            x: 15_000 + seed as i32,
            y: 10_000 + seed as i32,
            previous_x: 14_980 + seed as i32,
            previous_y: 9_980 + seed as i32,
            level: (1 + seed % 99) as u8,
            grow_stack: (seed % 7) as u8,
            disposed_objects: (seed % 5) as u8,
            respawn: false,
            ..MobSummary::default()
        }
    }

    fn drop_fixture(seed: u32) -> DropSummary {
        DropSummary {
            id: 990 + seed as u64,
            handler: if seed.is_multiple_of(2) {
                0
            } else {
                49_000 + seed
            },
            item_id: 20_000 + seed as i32,
            x: 15_010 + seed as i32,
            y: 10_020 + seed as i32,
            owner_id: seed as u64,
            owner_handler: if seed.is_multiple_of(3) {
                0
            } else {
                60_000 + seed
            },
            no_owner: seed.is_multiple_of(2),
            ..DropSummary::default()
        }
    }

    #[test]
    fn tamer_subtype3_payload_is_consumed_exactly() {
        let frame = LoadTamerPacket {
            character: tamer_fixture(1),
        }
        .encode();
        assert_consumed_exactly("LoadTamerPacket (subtype 3)", &frame);
    }

    #[test]
    fn mob_subtype3_payload_is_consumed_exactly() {
        let frame = LoadMobsPacket {
            mob: mob_fixture(1),
        }
        .encode();
        assert_consumed_exactly("LoadMobsPacket (subtype 3)", &frame);
    }

    #[test]
    fn drop_subtype3_payload_is_consumed_exactly() {
        let frame = LoadDropsPacket {
            drop: drop_fixture(1),
            viewer_handler: 11_000,
        }
        .encode();
        assert_consumed_exactly("LoadDropsPacket (subtype 3)", &frame);
    }

    #[test]
    fn empty_name_tamer_is_consumed_exactly() {
        let frame = LoadTamerPacket {
            character: tamer_fixture(0),
        }
        .encode();
        assert_consumed_exactly("LoadTamerPacket (empty name)", &frame);
    }

    /// Scoped property: across many generated tamer/mob/drop inputs, every 1006
    /// entity-load payload must be consumed exactly by the client parser model.
    /// The first counterexample is reported in the client's own diagnostic shape.
    ///
    /// Validates: Requirements 1.1, 1.2, 1.3
    #[test]
    fn prop_all_1006_subpackets_consumed_exactly() {
        for seed in 0..64u32 {
            let tamer = LoadTamerPacket {
                character: tamer_fixture(seed),
            }
            .encode();
            assert_consumed_exactly(&format!("LoadTamerPacket seed={seed}"), &tamer);

            let mob = LoadMobsPacket {
                mob: mob_fixture(seed),
            }
            .encode();
            assert_consumed_exactly(&format!("LoadMobsPacket seed={seed}"), &mob);

            let drop = LoadDropsPacket {
                drop: drop_fixture(seed),
                viewer_handler: 11_000 + seed,
            }
            .encode();
            assert_consumed_exactly(&format!("LoadDropsPacket seed={seed}"), &drop);
        }
    }
}

#[cfg(test)]
mod sync_1006_preservation {
    //! Preservation tests for the opcode-1006 entity-load misalignment fix.
    //!
    //! Property 2 (Preservation): every encoder that is NOT a bug target must
    //! produce byte-identical output before and after the fix. These tests
    //! capture the current byte output as golden baselines so the upcoming fix
    //! is proven not to touch them.
    //!
    //! Two groups are covered:
    //!   1. Non-1006 encoders (distinct opcodes), which the fix never touches.
    //!   2. The 1006 unload/buff sub-variants, which the client already parses
    //!      and which the fix must leave alone (only LoadTamer/LoadMobs/LoadDrops
    //!      are the bug targets).
    //!
    //! Validates: Requirements 3.1, 3.2

    use super::*;
    use odmo_types::{
        ActiveBuffSnapshot, CharacterSummary, DropSummary, InventorySnapshot, ItemRecord,
        MobSummary,
    };

    /// Render bytes as lowercase hex for golden capture and counterexamples.
    fn to_hex(bytes: &[u8]) -> String {
        let mut s = String::with_capacity(bytes.len() * 2);
        for b in bytes {
            s.push_str(&format!("{b:02x}"));
        }
        s
    }

    // ---- deterministic fixtures -------------------------------------------

    fn inventory_packet() -> LoadInventoryPacket {
        LoadInventoryPacket {
            inventory: InventorySnapshot {
                bits: 0x0102_0304_0506_0708,
                size: 2,
                items: vec![ItemRecord::new(5101, 3)],
            },
            inventory_type: InventoryType::Inventory,
        }
    }

    fn server_experience_packet() -> ServerExperiencePacket {
        ServerExperiencePacket {
            experience: 123_456,
        }
    }

    fn pick_item_packet() -> PickItemPacket {
        PickItemPacket {
            appearance_handler: 11_000,
            item_id: 5101,
            amount: 2,
        }
    }

    fn pick_bits_packet() -> PickBitsPacket {
        PickBitsPacket {
            appearance_handler: 11_000,
            value: 123,
        }
    }

    fn receive_exp_packet() -> ReceiveExpPacket {
        ReceiveExpPacket {
            tamer_exp: 1_000,
            tamer_bonus: 100,
            tamer_total: 1_100,
            partner_handler: 21_000,
            partner_exp: 2_000,
            partner_bonus: 200,
            partner_total: 2_200,
            skill_exp: 50,
        }
    }

    fn level_up_packet() -> LevelUpPacket {
        LevelUpPacket {
            handler: 11_000,
            level: 42,
        }
    }

    fn tamer_fixture() -> CharacterSummary {
        CharacterSummary {
            id: 1,
            name: "Admin".to_string(),
            x: 15_000,
            y: 10_000,
            general_handler: 11_000,
            partner_handler: 21_000,
            partner_x: 15_010,
            partner_y: 10_020,
            active_buffs: vec![ActiveBuffSnapshot {
                buff_id: 500,
                buff_class: 1,
                skill_id: 8_001_001,
                remaining_seconds: 30,
            }],
            partner_active_buffs: vec![ActiveBuffSnapshot {
                buff_id: 600,
                buff_class: 1,
                skill_id: 8_002_002,
                remaining_seconds: 45,
            }],
            ..CharacterSummary::default()
        }
    }

    fn mob_fixture() -> MobSummary {
        MobSummary {
            id: 900,
            handler: 44_001,
            type_id: 51_001,
            x: 15_000,
            y: 10_000,
            previous_x: 14_980,
            previous_y: 9_980,
            level: 25,
            active_debuffs: vec![ActiveBuffSnapshot {
                buff_id: 88,
                buff_class: 1,
                skill_id: 7001,
                remaining_seconds: 30,
            }],
            ..MobSummary::default()
        }
    }

    fn drop_fixture() -> DropSummary {
        DropSummary {
            id: 990,
            handler: 49_200,
            item_id: 90_600,
            owner_handler: 11_000,
            x: 15_010,
            y: 10_020,
            ..DropSummary::default()
        }
    }

    fn unload_tamer_packet() -> UnloadTamerPacket {
        UnloadTamerPacket {
            character: tamer_fixture(),
        }
    }

    fn unload_mobs_packet() -> UnloadMobsPacket {
        UnloadMobsPacket { mob: mob_fixture() }
    }

    fn unload_drops_packet() -> UnloadDropsPacket {
        UnloadDropsPacket {
            drop: drop_fixture(),
        }
    }

    fn load_buffs_packet() -> LoadBuffsPacket {
        LoadBuffsPacket {
            character: tamer_fixture(),
        }
    }

    fn load_mob_buffs_packet() -> LoadMobBuffsPacket {
        LoadMobBuffsPacket { mob: mob_fixture() }
    }

    /// Parse a lowercase-hex string into its byte vector.
    fn from_hex(hex: &str) -> Vec<u8> {
        assert!(hex.len().is_multiple_of(2), "hex length must be even");
        (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).expect("valid hex byte"))
            .collect()
    }

    // ---- golden baselines (captured from the current encoders) ------------
    //
    // These byte vectors are the baseline F(X). The fix must reproduce them
    // exactly (F(X) == F'(X)) for every encoder below.

    const GOLDEN_INVENTORY: &str = "9f00893e000000000807060504030201000200ed1306000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000a31a";
    const GOLDEN_SERVER_EXPERIENCE: &str = "1a001e040100000040e20100010000000000000040e20100261a";
    const GOLDEN_PICK_ITEM: &str = "1500460ff82a0000ed13000002000000000000291a";
    const GOLDEN_PICK_BITS: &str = "1a00470ff82a00007b000000000000000000000000000000261a";
    const GOLDEN_RECEIVE_EXP: &str = "4200fa03e80300000000000064000000000000004c0400000000000008520000d007000000000000c800000000000000980800000000000032000000000000007e1a";
    const GOLDEN_LEVEL_UP: &str = "0b00fb03f82a00002a371a";
    const GOLDEN_UNLOAD_TAMER: &str =
        "2500ee03040200f82a0000983a00001027000008520000a23a00002427000000000000191a";
    const GOLDEN_UNLOAD_MOBS: &str = "1900ee03040100e1ab0000983a00001027000000000000251a";
    const GOLDEN_UNLOAD_DROPS: &str = "1900ee0304010030c00000a23a00002427000000000000251a";
    const GOLDEN_LOAD_BUFFS: &str = "3000ee03100100f82a000001f40101001e000000e9157a0001000852000001580201002d000000d2197a000000000c1a";
    const GOLDEN_LOAD_MOB_BUFFS: &str =
        "1f00ee0310000000000100e1ab000001580001001e000000591b000000231a";

    /// Assert an encoder reproduces its captured golden bytes, reporting the
    /// divergence as hex when it does not.
    fn assert_golden(label: &str, golden_hex: &str, actual: &[u8]) {
        let expected = from_hex(&golden_hex.replace(' ', ""));
        assert!(
            actual == expected.as_slice(),
            "{label}: encoder output changed.\n expected={}\n actual  ={}",
            to_hex(&expected),
            to_hex(actual)
        );
    }

    // ---- non-1006 byte-identity (distinct opcodes, never touched) ---------

    #[test]
    fn load_inventory_matches_golden() {
        assert_golden(
            "LoadInventoryPacket",
            GOLDEN_INVENTORY,
            &inventory_packet().encode(),
        );
    }

    #[test]
    fn server_experience_matches_golden() {
        assert_golden(
            "ServerExperiencePacket",
            GOLDEN_SERVER_EXPERIENCE,
            &server_experience_packet().encode(),
        );
    }

    #[test]
    fn pick_item_matches_golden() {
        assert_golden(
            "PickItemPacket",
            GOLDEN_PICK_ITEM,
            &pick_item_packet().encode(),
        );
    }

    #[test]
    fn pick_bits_matches_golden() {
        assert_golden(
            "PickBitsPacket",
            GOLDEN_PICK_BITS,
            &pick_bits_packet().encode(),
        );
    }

    #[test]
    fn receive_exp_matches_golden() {
        assert_golden(
            "ReceiveExpPacket",
            GOLDEN_RECEIVE_EXP,
            &receive_exp_packet().encode(),
        );
    }

    #[test]
    fn level_up_matches_golden() {
        assert_golden(
            "LevelUpPacket",
            GOLDEN_LEVEL_UP,
            &level_up_packet().encode(),
        );
    }

    // ---- already-correct 1006 sub-variants (unload/buff, not bug targets) -

    #[test]
    fn unload_tamer_matches_golden() {
        assert_golden(
            "UnloadTamerPacket",
            GOLDEN_UNLOAD_TAMER,
            &unload_tamer_packet().encode(),
        );
    }

    #[test]
    fn unload_mobs_matches_golden() {
        assert_golden(
            "UnloadMobsPacket",
            GOLDEN_UNLOAD_MOBS,
            &unload_mobs_packet().encode(),
        );
    }

    #[test]
    fn unload_drops_matches_golden() {
        assert_golden(
            "UnloadDropsPacket",
            GOLDEN_UNLOAD_DROPS,
            &unload_drops_packet().encode(),
        );
    }

    #[test]
    fn load_buffs_matches_golden() {
        assert_golden(
            "LoadBuffsPacket",
            GOLDEN_LOAD_BUFFS,
            &load_buffs_packet().encode(),
        );
    }

    #[test]
    fn load_mob_buffs_matches_golden() {
        assert_golden(
            "LoadMobBuffsPacket",
            GOLDEN_LOAD_MOB_BUFFS,
            &load_mob_buffs_packet().encode(),
        );
    }

    // ---- frame-wire invariants --------------------------------------------

    /// Verify the frame envelope: `[Length u16 LE][Opcode i16 LE][..][Checksum u16 LE]`,
    /// with `checksum = length XOR 6716`.
    fn assert_frame_invariants(label: &str, frame: &[u8], expected_opcode: i16) {
        assert!(
            frame.len() >= 6,
            "{label}: frame too short: {}",
            frame.len()
        );
        let length = u16::from_le_bytes([frame[0], frame[1]]) as usize;
        assert_eq!(
            length,
            frame.len(),
            "{label}: length field must equal frame size"
        );
        let opcode = i16::from_le_bytes([frame[2], frame[3]]);
        assert_eq!(opcode, expected_opcode, "{label}: opcode mismatch");
        let checksum = i16::from_le_bytes([frame[length - 2], frame[length - 1]]);
        let expected = (length as i16) ^ crate::opcode::CHECKSUM_VALIDATION;
        assert_eq!(checksum, expected, "{label}: checksum mismatch");
    }

    // ---- property-based determinism over generated inputs -----------------
    //
    // There is no fixed F' yet, so the oracle for random inputs is a
    // deterministic re-encode equality: a pure encoder must produce identical
    // bytes for identical input, and a well-formed frame envelope every time.
    // After the fix lands, re-running these proves the non-bug encoders are
    // still byte-stable for the same inputs.

    fn seeded_string(seed: u32) -> String {
        match seed % 4 {
            0 => String::new(),
            1 => "A".to_string(),
            2 => "Admin".to_string(),
            _ => "LongTamerNameXYZ".to_string(),
        }
    }

    fn seeded_handler(seed: u32) -> u32 {
        match seed % 3 {
            0 => 0,
            1 => 11_000 + seed,
            _ => u32::MAX,
        }
    }

    fn seeded_i32(seed: u32) -> i32 {
        match seed % 4 {
            0 => 0,
            1 => seed as i32,
            2 => i32::MAX,
            _ => i32::MIN,
        }
    }

    fn seeded_buffs(seed: u32) -> Vec<ActiveBuffSnapshot> {
        match seed % 3 {
            0 => Vec::new(),
            1 => vec![ActiveBuffSnapshot {
                buff_id: (seed % u16::MAX as u32) as u16,
                buff_class: 1,
                skill_id: seed as i32,
                remaining_seconds: (seed % 600) as i32,
            }],
            _ => vec![
                ActiveBuffSnapshot {
                    buff_id: 500,
                    buff_class: 1,
                    skill_id: 8_001_001,
                    remaining_seconds: 30,
                },
                ActiveBuffSnapshot {
                    buff_id: 600,
                    buff_class: 1,
                    skill_id: 8_002_002,
                    remaining_seconds: -5,
                },
            ],
        }
    }

    /// Property: non-1006 encoders are byte-stable and emit well-formed frames
    /// across generated inputs (empty strings, boundary handlers, extreme ints).
    ///
    /// Validates: Requirements 3.1
    #[test]
    fn prop_non_1006_encoders_are_byte_stable() {
        for seed in 0..96u32 {
            let server_exp = ServerExperiencePacket {
                experience: seeded_i32(seed),
            };
            let a = server_exp.encode();
            assert_eq!(
                a,
                server_exp.encode(),
                "ServerExperiencePacket seed={seed} not stable"
            );
            assert_frame_invariants("ServerExperiencePacket", &a, game::SERVER_EXPERIENCE);

            let pick = PickItemPacket {
                appearance_handler: seeded_handler(seed),
                item_id: seeded_i32(seed),
                amount: (seed % i16::MAX as u32) as i16,
            };
            let b = pick.encode();
            assert_eq!(b, pick.encode(), "PickItemPacket seed={seed} not stable");
            assert_frame_invariants("PickItemPacket", &b, game::LOOT_ITEM);

            let bits = PickBitsPacket {
                appearance_handler: seeded_handler(seed),
                value: seeded_i32(seed),
            };
            let c = bits.encode();
            assert_eq!(c, bits.encode(), "PickBitsPacket seed={seed} not stable");
            assert_frame_invariants("PickBitsPacket", &c, game::PICK_BITS);

            let inventory = LoadInventoryPacket {
                inventory: InventorySnapshot {
                    bits: seed as i64,
                    size: (seed % 5) as u16,
                    items: if seed % 2 == 0 {
                        Vec::new()
                    } else {
                        vec![ItemRecord::new(5101 + seed as i32, (seed % 100) as i32)]
                    },
                },
                inventory_type: InventoryType::Inventory,
            };
            let d = inventory.encode();
            assert_eq!(
                d,
                inventory.encode(),
                "LoadInventoryPacket seed={seed} not stable"
            );
            assert_frame_invariants("LoadInventoryPacket", &d, game::LOAD_INVENTORY);
        }
    }

    /// Property: the 1006 unload/buff sub-variants (already-correct, not bug
    /// targets) are byte-stable and emit well-formed 1006 frames across
    /// generated inputs (empty names, empty buff lists, boundary handlers).
    ///
    /// Validates: Requirements 3.2
    #[test]
    fn prop_unload_and_buff_encoders_are_byte_stable() {
        for seed in 0..96u32 {
            let character = CharacterSummary {
                id: 1 + seed as u64,
                name: seeded_string(seed),
                general_handler: seeded_handler(seed),
                partner_handler: seeded_handler(seed + 1),
                x: seeded_i32(seed),
                y: seeded_i32(seed + 1),
                partner_x: seeded_i32(seed + 2),
                partner_y: seeded_i32(seed + 3),
                active_buffs: seeded_buffs(seed),
                partner_active_buffs: seeded_buffs(seed + 1),
                partner_active_debuffs: seeded_buffs(seed + 2),
                ..CharacterSummary::default()
            };
            let mob = MobSummary {
                id: 900 + seed as u64,
                handler: seeded_handler(seed),
                x: seeded_i32(seed),
                y: seeded_i32(seed + 1),
                active_debuffs: seeded_buffs(seed),
                ..MobSummary::default()
            };
            let drop = DropSummary {
                id: 990 + seed as u64,
                handler: seeded_handler(seed),
                x: seeded_i32(seed),
                y: seeded_i32(seed + 1),
                ..DropSummary::default()
            };

            let ut = UnloadTamerPacket {
                character: character.clone(),
            }
            .encode();
            assert_eq!(
                ut,
                UnloadTamerPacket {
                    character: character.clone()
                }
                .encode(),
                "UnloadTamerPacket seed={seed} not stable"
            );
            assert_frame_invariants("UnloadTamerPacket", &ut, game::LOAD_UNLOAD_ENTITY);

            let um = UnloadMobsPacket { mob: mob.clone() }.encode();
            assert_eq!(
                um,
                UnloadMobsPacket { mob: mob.clone() }.encode(),
                "UnloadMobsPacket seed={seed} not stable"
            );
            assert_frame_invariants("UnloadMobsPacket", &um, game::LOAD_UNLOAD_ENTITY);

            let ud = UnloadDropsPacket { drop: drop.clone() }.encode();
            assert_eq!(
                ud,
                UnloadDropsPacket { drop: drop.clone() }.encode(),
                "UnloadDropsPacket seed={seed} not stable"
            );
            assert_frame_invariants("UnloadDropsPacket", &ud, game::LOAD_UNLOAD_ENTITY);

            let lb = LoadBuffsPacket {
                character: character.clone(),
            }
            .encode();
            assert_eq!(
                lb,
                LoadBuffsPacket {
                    character: character.clone()
                }
                .encode(),
                "LoadBuffsPacket seed={seed} not stable"
            );
            assert_frame_invariants("LoadBuffsPacket", &lb, game::LOAD_BUFFS);

            let lmb = LoadMobBuffsPacket { mob: mob.clone() }.encode();
            assert_eq!(
                lmb,
                LoadMobBuffsPacket { mob: mob.clone() }.encode(),
                "LoadMobBuffsPacket seed={seed} not stable"
            );
            assert_frame_invariants("LoadMobBuffsPacket", &lmb, game::LOAD_BUFFS);
        }
    }
}
