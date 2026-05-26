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
        writer.write_u8(1); // success
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
        let mut writer = PacketWriter::new(game::MOVE_ITEM);
        writer.write_u8(0); // fail
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
                    skill_id: 8001001,
                    remaining_seconds: 60,
                }],
                partner_active_buffs: vec![ActiveBuffSnapshot {
                    buff_id: 600,
                    skill_id: 8002001,
                    remaining_seconds: 30,
                }],
                partner_active_debuffs: vec![ActiveBuffSnapshot {
                    buff_id: 700,
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
