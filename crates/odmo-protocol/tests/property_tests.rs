//! Property-based test harness for the wire codecs.
//!
//! Round-trip and frame-invariant properties for the covered packets are added
//! here. Every property runs at least `CASES` generated inputs.

use odmo_protocol::game::{
    CombineResultResponsePacket, CombineSyncResponsePacket, DigimonToSpiritResultPacket,
    RandomBoxListEntry, RandomBoxListResponsePacket, RandomBoxPurchaseResponsePacket,
    SpiritToDigimonResultPacket, UnionHackModifyResponsePacket, UnionHackOpenResponsePacket,
    UnionHackSlot, UnionInitDataPacket,
};
use odmo_protocol::{
    DigiSummonPurchaseResponsePacket, DigiSummonSyncResponsePacket, GameRequest, PacketReader,
    PacketWriter, RawPacket,
    opcode::{CHECKSUM_VALIDATION, game},
};
use odmo_types::{
    CombineCeilingEntry, CombineItemRef, DigiCombineReward, DigiSummonProduct, DigiSummonReward,
};
use proptest::prelude::*;

/// Minimum number of generated cases per property.
const CASES: u32 = 100;

fn config() -> ProptestConfig {
    ProptestConfig {
        cases: CASES,
        ..ProptestConfig::default()
    }
}

/// Strategy for a sync-wire product. Only the four fields that travel on the
/// 3652 wire are randomized; `draw_count` stays within `u16` range so the
/// encoder's clamp is a no-op and the round-trip is exact.
fn sync_product_strategy() -> impl Strategy<Value = DigiSummonProduct> {
    (
        any::<i32>(),
        any::<i32>(),
        0..=i32::from(u16::MAX),
        any::<i32>(),
    )
        .prop_map(
            |(product_id, rank, draw_count, remaining_daily_limit)| DigiSummonProduct {
                product_id,
                rank,
                draw_count,
                remaining_daily_limit,
                ..DigiSummonProduct::default()
            },
        )
}

/// Strategy for a purchase-wire reward. Only the three fields that travel on
/// the 3651 wire are randomized. `amount` stays within `1..=u16::MAX` and
/// `grade` within `0..=u16::MAX` so the encoder's clamps are no-ops and the
/// round-trip is exact.
fn reward_strategy() -> impl Strategy<Value = DigiSummonReward> {
    (
        any::<i32>(),
        1..=i32::from(u16::MAX),
        0..=i32::from(u16::MAX),
    )
        .prop_map(|(item_id, amount, grade)| DigiSummonReward {
            item_id,
            amount,
            grade,
            ..DigiSummonReward::default()
        })
}

/// Strategy for a `stCeiling` entry `{u1 tier, u1 value_a, u2 value_b}`. All
/// three fields cover their full wire width so the round-trip is exact.
fn ceiling_entry_strategy() -> impl Strategy<Value = CombineCeilingEntry> {
    (any::<u8>(), any::<u8>(), any::<u16>()).prop_map(|(tier, value_a, value_b)| {
        CombineCeilingEntry {
            tier,
            value_a,
            value_b,
        }
    })
}

/// Strategy for a combine material node `{u4 item_uid, u2 item_type, u2 count}`.
/// Each field covers its full wire width so the round-trip is exact.
fn combine_item_strategy() -> impl Strategy<Value = CombineItemRef> {
    (any::<u32>(), any::<u16>(), any::<u16>()).prop_map(|(item_uid, item_type, count)| {
        CombineItemRef {
            item_uid,
            item_type,
            count,
        }
    })
}

/// Strategy for a zero-terminated item block `[u8 count][u32 item_id]`. The
/// count byte doubles as the list terminator, so a non-zero count is required:
/// a zero anywhere mid-list would be read as the terminator and truncate it.
fn item_block_strategy() -> impl Strategy<Value = (u8, u32)> {
    (1..=u8::MAX, any::<u32>())
}

/// Strategy for one row in the D-Unit (Union hacking tool) response list.
/// The locked flag is a single bit so any boolean is fair game.
fn union_hack_slot_strategy() -> impl Strategy<Value = UnionHackSlot> {
    (any::<u8>(), any::<i32>(), any::<i16>(), any::<bool>()).prop_map(
        |(slot, part_id, grade, locked)| UnionHackSlot {
            slot,
            part_id,
            grade,
            locked,
        },
    )
}

/// Strategy for a spirit-evolution partner name. The wide-string reader trims ASCII
/// whitespace, so names exclude leading/trailing spaces; length stays within
/// the `u8` code-unit count and uses printable ASCII so the UTF-16 round-trip
/// is exact.
fn extra_evolution_name_strategy() -> impl Strategy<Value = String> {
    prop::collection::vec(0x21_u8..=0x7e, 0..=32)
        .prop_map(|bytes| bytes.into_iter().map(char::from).collect::<String>())
}

/// Write a material list `[u2 count][{u4 item_uid, u2 item_type, u2 count}...]`
/// in the exact order the combine decoder reads it. Lengths stay within the
/// `0..=44` generator bound, so the `u16` count never truncates.
fn write_materials(writer: &mut PacketWriter, materials: &[CombineItemRef]) {
    writer.write_u16(materials.len() as u16);
    for node in materials {
        writer.write_u32(node.item_uid);
        writer.write_u16(node.item_type);
        writer.write_u16(node.count);
    }
}

/// Encode a covered request back to a frame, writing each variant's body with
/// the exact field order the decoder reads. This mirrors the decode arms so the
/// re-encode is a faithful inverse and the round-trip stays byte-exact.
fn encode_request(req: &GameRequest) -> Vec<u8> {
    match req {
        // Empty body: opcode plus the framing only.
        GameRequest::DigiSummonSyncRequest => {
            PacketWriter::new(game::DIGI_SUMMON_SYNC_REQUEST).finalize()
        }
        // [i32 product_id][i32 ticket_slot].
        GameRequest::DigiSummonPurchase {
            product_id,
            ticket_slot,
        } => {
            let mut writer = PacketWriter::new(game::DIGI_SUMMON_PURCHASE);
            writer.write_i32(*product_id);
            writer.write_i32(*ticket_slot);
            writer.finalize()
        }
        // Bare body: opcode plus the framing only.
        GameRequest::DigiCombineSyncRequest => {
            PacketWriter::new(game::DIGI_COMBINE_SYNC).finalize()
        }
        // [u8 ceiling_type][material list].
        GameRequest::DigiCombine {
            ceiling_type,
            materials,
        } => {
            let mut writer = PacketWriter::new(game::DIGI_COMBINE);
            writer.write_u8(*ceiling_type);
            write_materials(&mut writer, materials);
            writer.finalize()
        }
        // [u8 ceiling_type].
        GameRequest::DigiCombineRewardClaim { ceiling_type } => {
            let mut writer = PacketWriter::new(game::DIGI_COMBINE_REWARD);
            writer.write_u8(*ceiling_type);
            writer.finalize()
        }
        // Bare body: opcode plus the framing only.
        GameRequest::UnionCombineSyncRequest => {
            PacketWriter::new(game::UNION_COMBINE_SYNC).finalize()
        }
        // [u8 ceiling_type][material list].
        GameRequest::UnionCombine {
            ceiling_type,
            materials,
        } => {
            let mut writer = PacketWriter::new(game::UNION_COMBINE);
            writer.write_u8(*ceiling_type);
            write_materials(&mut writer, materials);
            writer.finalize()
        }
        // [u8 ceiling_type].
        GameRequest::UnionCombineRewardClaim { ceiling_type } => {
            let mut writer = PacketWriter::new(game::UNION_COMBINE_REWARD);
            writer.write_u8(*ceiling_type);
            writer.finalize()
        }
        // Bare body: opcode plus the framing only.
        GameRequest::UnionHackOpenRequest => {
            PacketWriter::new(game::UNION_HACK_OPEN_REQUEST).finalize()
        }
        // [u8 slot][i32 part_id][i16 grade].
        GameRequest::UnionHackModify {
            slot,
            part_id,
            grade,
        } => {
            let mut writer = PacketWriter::new(game::UNION_HACK_MODIFY_REQUEST);
            writer.write_u8(*slot);
            writer.write_i32(*part_id);
            writer.write_i16(*grade);
            writer.finalize()
        }
        // [i32 model_id][wstring name][i32 npc_id].
        GameRequest::SpiritToDigimon {
            model_id,
            name,
            npc_id,
        } => {
            let mut writer = PacketWriter::new(game::SPIRIT_TO_DIGIMON);
            writer.write_i32(*model_id);
            writer.write_wide_string(name);
            writer.write_i32(*npc_id);
            writer.finalize()
        }
        // [u8 slot][string validation][i32 npc_id].
        GameRequest::DigimonToSpirit {
            slot,
            validation,
            npc_id,
        } => {
            let mut writer = PacketWriter::new(game::DIGIMON_TO_SPIRIT);
            writer.write_u8(*slot);
            writer.write_string(validation);
            writer.write_i32(*npc_id);
            writer.finalize()
        }
        // [u8 flag][i32 index].
        GameRequest::RandomBoxList { flag, index } => {
            let mut writer = PacketWriter::new(game::RANDOM_BOX_LIST);
            writer.write_u8(*flag);
            writer.write_i32(*index);
            writer.finalize()
        }
        // [u8 flag][i32 product_id][i32 item_uid][u16 count][i32 state].
        GameRequest::RandomBoxPurchase {
            flag,
            product_id,
            item_uid,
            count,
            state,
        } => {
            let mut writer = PacketWriter::new(game::RANDOM_BOX_PURCHASE);
            writer.write_u8(*flag);
            writer.write_i32(*product_id);
            writer.write_i32(*item_uid);
            writer.write_u16(*count);
            writer.write_i32(*state);
            writer.finalize()
        }
        other => panic!("encode_request: variant outside Property 16 coverage: {other:?}"),
    }
}

/// Strategy that draws one covered request per case. Each arm constrains its
/// fields to the wire's exact representable space so the round-trip is byte-exact:
/// wide-string names are printable ASCII without edge whitespace, validation
/// strings are `[0-9A-Za-z]{0,16}`, and material lists stay within `0..=44`.
fn covered_request_strategy() -> impl Strategy<Value = GameRequest> {
    prop_oneof![
        Just(GameRequest::DigiSummonSyncRequest),
        (any::<i32>(), any::<i32>()).prop_map(|(product_id, ticket_slot)| {
            GameRequest::DigiSummonPurchase {
                product_id,
                ticket_slot,
            }
        }),
        Just(GameRequest::DigiCombineSyncRequest),
        (
            any::<u8>(),
            prop::collection::vec(combine_item_strategy(), 0..=44),
        )
            .prop_map(|(ceiling_type, materials)| GameRequest::DigiCombine {
                ceiling_type,
                materials,
            }),
        any::<u8>().prop_map(|ceiling_type| GameRequest::DigiCombineRewardClaim { ceiling_type }),
        Just(GameRequest::UnionCombineSyncRequest),
        (
            any::<u8>(),
            prop::collection::vec(combine_item_strategy(), 0..=44),
        )
            .prop_map(|(ceiling_type, materials)| GameRequest::UnionCombine {
                ceiling_type,
                materials,
            }),
        any::<u8>().prop_map(|ceiling_type| GameRequest::UnionCombineRewardClaim { ceiling_type }),
        Just(GameRequest::UnionHackOpenRequest),
        (any::<u8>(), any::<i32>(), any::<i16>()).prop_map(|(slot, part_id, grade)| {
            GameRequest::UnionHackModify {
                slot,
                part_id,
                grade,
            }
        }),
        (any::<i32>(), extra_evolution_name_strategy(), any::<i32>()).prop_map(
            |(model_id, name, npc_id)| GameRequest::SpiritToDigimon {
                model_id,
                name,
                npc_id,
            }
        ),
        (any::<u8>(), "[0-9A-Za-z]{0,16}", any::<i32>()).prop_map(|(slot, validation, npc_id)| {
            GameRequest::DigimonToSpirit {
                slot,
                validation,
                npc_id,
            }
        }),
        (any::<u8>(), any::<i32>())
            .prop_map(|(flag, index)| GameRequest::RandomBoxList { flag, index }),
        (
            any::<u8>(),
            any::<i32>(),
            any::<i32>(),
            any::<u16>(),
            any::<i32>(),
        )
            .prop_map(|(flag, product_id, item_uid, count, state)| {
                GameRequest::RandomBoxPurchase {
                    flag,
                    product_id,
                    item_uid,
                    count,
                    state,
                }
            }),
    ]
}

/// Strategy for a combine reward node `{n4 item_id, u2 amount, u1 grade}`. Each
/// field covers its full wire width; the encoder zero-fills the reserved tail
/// of the fixed node, so the envelope invariants hold for any draw.
fn combine_reward_strategy() -> impl Strategy<Value = DigiCombineReward> {
    (any::<i32>(), any::<u16>(), any::<u8>()).prop_map(|(item_id, amount, grade)| {
        DigiCombineReward {
            item_id,
            amount,
            grade,
        }
    })
}

/// Strategy for a random box list entry `{n4 a, n4 b, n4 c, u2 d}`. Each field
/// covers its full wire width.
fn random_box_list_entry_strategy() -> impl Strategy<Value = RandomBoxListEntry> {
    (any::<i32>(), any::<i32>(), any::<i32>(), any::<u16>())
        .prop_map(|(a, b, c, d)| RandomBoxListEntry { a, b, c, d })
}

/// The opcode a covered request frames itself under. Pairing each variant with
/// its expected opcode gives the frame-invariant property an independent value
/// to check the opcode field against, rather than reading it back from the frame.
fn request_opcode(req: &GameRequest) -> i16 {
    match req {
        GameRequest::DigiSummonSyncRequest => game::DIGI_SUMMON_SYNC_REQUEST,
        GameRequest::DigiSummonPurchase { .. } => game::DIGI_SUMMON_PURCHASE,
        GameRequest::DigiCombineSyncRequest => game::DIGI_COMBINE_SYNC,
        GameRequest::DigiCombine { .. } => game::DIGI_COMBINE,
        GameRequest::DigiCombineRewardClaim { .. } => game::DIGI_COMBINE_REWARD,
        GameRequest::UnionCombineSyncRequest => game::UNION_COMBINE_SYNC,
        GameRequest::UnionCombine { .. } => game::UNION_COMBINE,
        GameRequest::UnionCombineRewardClaim { .. } => game::UNION_COMBINE_REWARD,
        GameRequest::UnionHackOpenRequest => game::UNION_HACK_OPEN_REQUEST,
        GameRequest::UnionHackModify { .. } => game::UNION_HACK_MODIFY_REQUEST,
        GameRequest::SpiritToDigimon { .. } => game::SPIRIT_TO_DIGIMON,
        GameRequest::DigimonToSpirit { .. } => game::DIGIMON_TO_SPIRIT,
        GameRequest::RandomBoxList { .. } => game::RANDOM_BOX_LIST,
        GameRequest::RandomBoxPurchase { .. } => game::RANDOM_BOX_PURCHASE,
        other => panic!("request_opcode: variant outside Property 18 coverage: {other:?}"),
    }
}

/// Strategy that draws one covered response per case, paired with the opcode it
/// frames itself under. Field values cover the wire widths; list lengths stay
/// small and bounded so generation is cheap. Each arm is boxed so the differing
/// builder types unify into a single `(frame, opcode)` strategy.
fn covered_response_strategy() -> impl Strategy<Value = (Vec<u8>, i16)> {
    let sync = (
        any::<u8>(),
        prop::collection::vec(sync_product_strategy(), 0..=8),
    )
        .prop_map(|(result, products)| {
            (
                DigiSummonSyncResponsePacket { result, products }.encode(),
                game::DIGI_SUMMON_SYNC_RESPONSE,
            )
        })
        .boxed();

    let purchase = (
        any::<u8>(),
        any::<i32>(),
        prop::collection::vec(reward_strategy(), 0..=8),
        prop::collection::vec(sync_product_strategy(), 0..=8),
    )
        .prop_map(|(result, product_id, rewards, products)| {
            (
                DigiSummonPurchaseResponsePacket {
                    result,
                    product_id,
                    rewards,
                    products,
                }
                .encode(),
                game::DIGI_SUMMON_PURCHASE_RESPONSE,
            )
        })
        .boxed();

    let combine_sync = (
        any::<bool>(),
        any::<u8>(),
        prop::collection::vec(ceiling_entry_strategy(), 0..=8),
    )
        .prop_map(|(union, result, ceiling)| {
            if union {
                (
                    CombineSyncResponsePacket::union(result, ceiling).encode(),
                    game::UNION_COMBINE_SYNC,
                )
            } else {
                (
                    CombineSyncResponsePacket::digi(result, ceiling).encode(),
                    game::DIGI_COMBINE_SYNC,
                )
            }
        })
        .boxed();

    let combine_result = (
        0_u8..=3,
        any::<u8>(),
        prop::collection::vec(ceiling_entry_strategy(), 0..=8),
        prop::collection::vec(combine_item_strategy(), 0..=44),
        prop::collection::vec(combine_reward_strategy(), 0..=8),
    )
        .prop_map(|(flow, result, ceiling, materials, rewards)| {
            let packet = match flow {
                0 => CombineResultResponsePacket::digi_result(result, ceiling, materials, rewards),
                1 => CombineResultResponsePacket::digi_reward(result, ceiling, materials, rewards),
                2 => CombineResultResponsePacket::union_result(result, ceiling, materials, rewards),
                _ => CombineResultResponsePacket::union_reward(result, ceiling, materials, rewards),
            };
            let opcode = packet.opcode;
            (packet.encode(), opcode)
        })
        .boxed();

    let hatch_result = (
        any::<u32>(),
        any::<i64>(),
        prop::collection::vec(item_block_strategy(), 0..=8),
    )
        .prop_map(|(digimon_id, remaining_bits, consumed_items)| {
            (
                SpiritToDigimonResultPacket {
                    digimon_id,
                    remaining_bits,
                    consumed_items,
                }
                .encode(),
                game::SPIRIT_TO_DIGIMON,
            )
        })
        .boxed();

    let craft_result = (
        any::<u8>(),
        any::<i64>(),
        prop::collection::vec(item_block_strategy(), 0..=8),
        prop::collection::vec(item_block_strategy(), 0..=8),
    )
        .prop_map(|(slot, remaining_bits, consumed_items, gained_items)| {
            (
                DigimonToSpiritResultPacket {
                    slot,
                    remaining_bits,
                    consumed_items,
                    gained_items,
                }
                .encode(),
                game::DIGIMON_TO_SPIRIT,
            )
        })
        .boxed();

    let random_box_list = (
        any::<i32>(),
        prop::collection::vec(random_box_list_entry_strategy(), 0..=8),
    )
        .prop_map(|(field0, entries)| {
            (
                RandomBoxListResponsePacket { field0, entries }.encode(),
                game::RANDOM_BOX_LIST,
            )
        })
        .boxed();

    let random_box_purchase = (
        any::<i32>(),
        any::<i32>(),
        any::<u16>(),
        prop::collection::vec((any::<i32>(), any::<i32>()), 0..=8),
        prop::collection::vec((any::<u64>(), any::<u16>()), 0..=8),
        (any::<u64>(), any::<u16>()),
    )
        .prop_map(|(field0, field1, field2, list_a, list_b, summary)| {
            (
                RandomBoxPurchaseResponsePacket {
                    field0,
                    field1,
                    field2,
                    list_a,
                    list_b,
                    summary,
                }
                .encode(),
                game::RANDOM_BOX_PURCHASE,
            )
        })
        .boxed();

    let union_hack_open = (
        any::<u8>(),
        any::<u8>(),
        prop::collection::vec(union_hack_slot_strategy(), 0..=6),
    )
        .prop_map(|(result, unlocked_slots, slots)| {
            (
                UnionHackOpenResponsePacket {
                    result,
                    unlocked_slots,
                    slots,
                }
                .encode(),
                game::UNION_HACK_OPEN_RESPONSE,
            )
        })
        .boxed();

    let union_hack_modify = (
        any::<u8>(),
        any::<u8>(),
        any::<i32>(),
        any::<i16>(),
        any::<i32>(),
    )
        .prop_map(|(result, slot, new_part_id, new_grade, total_rating)| {
            (
                UnionHackModifyResponsePacket {
                    result,
                    slot,
                    new_part_id,
                    new_grade,
                    total_rating,
                }
                .encode(),
                game::UNION_HACK_MODIFY_RESPONSE,
            )
        })
        .boxed();

    let union_init_data = (
        prop::collection::vec(union_hack_slot_strategy(), 0..=6),
        any::<i32>(),
        any::<i32>(),
    )
        .prop_map(|(slots, total_rating, synergy_bonus)| {
            (
                UnionInitDataPacket {
                    slots,
                    total_rating,
                    synergy_bonus,
                }
                .encode(),
                game::UNION_INIT_DATA,
            )
        })
        .boxed();

    prop_oneof![
        sync,
        purchase,
        combine_sync,
        combine_result,
        hatch_result,
        craft_result,
        random_box_list,
        random_box_purchase,
        union_hack_open,
        union_hack_modify,
        union_init_data,
    ]
}

/// Assert the frame envelope invariants for a covered packet: the length field
/// equals the byte length, the opcode field equals the expected opcode, the
/// trailing checksum equals `length XOR CHECKSUM_VALIDATION`, and the reader
/// (which revalidates the checksum) recovers the same opcode.
fn assert_frame_invariants(frame: &[u8], expected_opcode: i16) -> Result<(), TestCaseError> {
    prop_assert!(
        frame.len() >= 6,
        "frame must hold length, opcode, and checksum: {} bytes",
        frame.len()
    );

    let length = u16::from_le_bytes([frame[0], frame[1]]);
    prop_assert_eq!(
        length as usize,
        frame.len(),
        "length field equals frame length"
    );

    let opcode = i16::from_le_bytes([frame[2], frame[3]]);
    prop_assert_eq!(
        opcode,
        expected_opcode,
        "opcode field equals expected opcode"
    );

    let checksum = i16::from_le_bytes([frame[frame.len() - 2], frame[frame.len() - 1]]);
    prop_assert_eq!(
        checksum,
        (length as i16) ^ CHECKSUM_VALIDATION,
        "checksum equals length XOR validation"
    );

    let raw = PacketReader::from_frame(frame)
        .map_err(|e| TestCaseError::fail(format!("frame should decode: {e:?}")))?;
    prop_assert_eq!(
        raw.packet_type,
        expected_opcode,
        "decoded opcode equals expected opcode"
    );

    Ok(())
}

/// Dummy opcode for the standalone string-codec round-trips. The opcode is
/// irrelevant to the string body; any valid value frames the same way.
const STRING_CODEC_OPCODE: i16 = 0x0001;

/// Strategy for a printable-ASCII length-prefixed string. Bytes are drawn from
/// `0x21..=0x7e`, which excludes space, so no value carries edge whitespace for
/// the reader's `trim` to alter. Length spans `0..=255` (including empty) so the
/// `u8` byte-length prefix never truncates.
fn printable_ascii_string_strategy() -> impl Strategy<Value = String> {
    prop::collection::vec(0x21_u8..=0x7e, 0..=255)
        .prop_map(|bytes| bytes.into_iter().map(char::from).collect())
}

/// Characters mixed into the Unicode string generator: printable ASCII plus a
/// handful of multi-byte code points (Latin-1, Greek, CJK, and a
/// supplementary-plane emoji that encodes as a UTF-16 surrogate pair). None is
/// whitespace, so the reader's `trim` never alters a generated value.
fn unicode_char_strategy() -> impl Strategy<Value = char> {
    prop_oneof![
        (0x21_u8..=0x7e).prop_map(char::from),
        Just('é'),
        Just('ñ'),
        Just('Ω'),
        Just('あ'),
        Just('好'),
        Just('😀'),
    ]
}

/// Strategy for a Unicode length-prefixed string with no edge whitespace. The
/// char count stays within `0..=40`; with at most 4 UTF-8 bytes and 2 UTF-16
/// code units per char, byte length stays under 160 and code-unit count under
/// 80, so neither codec's `u8` prefix truncates. Includes the empty string.
fn unicode_string_strategy() -> impl Strategy<Value = String> {
    prop::collection::vec(unicode_char_strategy(), 0..=40)
        .prop_map(|chars| chars.into_iter().collect())
}

/// Round-trip a value through the ASCII length-prefixed codec
/// (`write_string`/`read_string`). Asserts the `[u8 byte-length][bytes][0x00]`
/// wire form, that the NUL terminator is present, and that the decoded value
/// equals the original. The reader trims edge whitespace, so callers must pass
/// values without leading/trailing whitespace for an exact round-trip.
fn assert_ascii_string_round_trips(value: &str) -> Result<(), TestCaseError> {
    let mut writer = PacketWriter::new(STRING_CODEC_OPCODE);
    writer.write_string(value);
    let frame = writer.finalize();

    let raw = PacketReader::from_frame(&frame)
        .map_err(|e| TestCaseError::fail(format!("ascii string frame should decode: {e:?}")))?;

    let byte_len = value.len();
    prop_assert_eq!(
        raw.payload[0] as usize,
        byte_len,
        "ascii length prefix counts bytes"
    );
    prop_assert_eq!(
        raw.payload[1 + byte_len],
        0,
        "ascii string carries its NUL terminator"
    );

    let mut reader = PacketReader::new(raw.payload);
    let decoded = reader
        .read_string()
        .map_err(|e| TestCaseError::fail(format!("ascii string should read: {e:?}")))?;
    prop_assert_eq!(
        decoded.as_str(),
        value,
        "ascii length-prefixed string round-trips"
    );
    Ok(())
}

/// Round-trip a value through the wide length-prefixed codec
/// (`write_wide_string`/`read_wide_string`). Asserts the
/// `[u8 unit-count][u16 units...][u16 0]` wire form, that the `u16` terminator
/// is present, and that the decoded value equals the original. As with the
/// ASCII codec, callers must pass values without edge whitespace.
fn assert_wide_string_round_trips(value: &str) -> Result<(), TestCaseError> {
    let mut writer = PacketWriter::new(STRING_CODEC_OPCODE);
    writer.write_wide_string(value);
    let frame = writer.finalize();

    let raw = PacketReader::from_frame(&frame)
        .map_err(|e| TestCaseError::fail(format!("wide string frame should decode: {e:?}")))?;

    let unit_count = value.encode_utf16().count();
    prop_assert_eq!(
        raw.payload[0] as usize,
        unit_count,
        "wide length prefix counts code units"
    );
    prop_assert_eq!(
        (
            raw.payload[1 + 2 * unit_count],
            raw.payload[2 + 2 * unit_count]
        ),
        (0, 0),
        "wide string carries its u16 terminator"
    );

    let mut reader = PacketReader::new(raw.payload);
    let decoded = reader
        .read_wide_string()
        .map_err(|e| TestCaseError::fail(format!("wide string should read: {e:?}")))?;
    prop_assert_eq!(
        decoded.as_str(),
        value,
        "wide length-prefixed string round-trips"
    );
    Ok(())
}

proptest! {
    #![proptest_config(config())]

    /// A `u32` written into a frame survives a finalize/parse round-trip.
    #[test]
    fn u32_payload_round_trips(value in any::<u32>()) {
        let mut writer = PacketWriter::new(0x0001);
        writer.write_u32(value);
        let frame = writer.finalize();

        let raw = PacketReader::from_frame(&frame).expect("frame parses");
        let mut reader = PacketReader::new(raw.payload);
        prop_assert_eq!(reader.read_u32().expect("payload has the u32"), value);
    }

    /// Feature: babel-npc-summon-fusion, Property 1: DATA Summon sync response round-trips.
    ///
    /// Encoding the 3652 response then parsing the frame preserves the result
    /// byte and each product's `product_id`, `rank`, `draw_count`, and
    /// `remaining_daily_limit` in order.
    #[test]
    fn digi_summon_sync_response_round_trips(
        result in any::<u8>(),
        products in prop::collection::vec(sync_product_strategy(), 0..=8),
    ) {
        let frame = DigiSummonSyncResponsePacket {
            result,
            products: products.clone(),
        }
        .encode();

        let raw = PacketReader::from_frame(&frame).expect("frame should decode");
        prop_assert_eq!(raw.packet_type, game::DIGI_SUMMON_SYNC_RESPONSE);

        let mut reader = PacketReader::new(raw.payload);
        prop_assert_eq!(reader.read_u8().expect("result"), result);
        prop_assert_eq!(reader.read_u16().expect("count") as usize, products.len());
        for product in &products {
            prop_assert_eq!(reader.read_i32().expect("product_id"), product.product_id);
            prop_assert_eq!(reader.read_i32().expect("rank"), product.rank);
            prop_assert_eq!(reader.read_u16().expect("draw_count") as i32, product.draw_count);
            prop_assert_eq!(
                reader.read_i32().expect("remaining_daily_limit"),
                product.remaining_daily_limit
            );
        }
    }

    /// Feature: babel-npc-summon-fusion, Property 3: DATA Summon purchase response round-trips.
    ///
    /// Encoding the 3651 response then parsing the frame preserves the result
    /// byte, `product_id`, each reward's `item_id`/`amount`/`grade`, and the
    /// resynced product-state list (`product_id`/`rank`/`draw_count`/
    /// `remaining_daily_limit`) in order, followed by the empty detail count
    /// and the zero trailer that close the frame.
    #[test]
    fn digi_summon_purchase_response_round_trips(
        result in any::<u8>(),
        product_id in any::<i32>(),
        rewards in prop::collection::vec(reward_strategy(), 0..=8),
        products in prop::collection::vec(sync_product_strategy(), 0..=8),
    ) {
        let frame = DigiSummonPurchaseResponsePacket {
            result,
            product_id,
            rewards: rewards.clone(),
            products: products.clone(),
        }
        .encode();

        let raw = PacketReader::from_frame(&frame).expect("frame should decode");
        prop_assert_eq!(raw.packet_type, game::DIGI_SUMMON_PURCHASE_RESPONSE);

        let mut reader = PacketReader::new(raw.payload);
        prop_assert_eq!(reader.read_u8().expect("result"), result);
        prop_assert_eq!(reader.read_i32().expect("product_id"), product_id);

        prop_assert_eq!(reader.read_u16().expect("reward count") as usize, rewards.len());
        for reward in &rewards {
            prop_assert_eq!(reader.read_i32().expect("reward item_id"), reward.item_id);
            prop_assert_eq!(reader.read_u16().expect("reward amount") as i32, reward.amount);
            prop_assert_eq!(reader.read_u16().expect("reward grade") as i32, reward.grade);
        }

        prop_assert_eq!(reader.read_u16().expect("product count") as usize, products.len());
        for product in &products {
            prop_assert_eq!(reader.read_i32().expect("product_id"), product.product_id);
            prop_assert_eq!(reader.read_i32().expect("rank"), product.rank);
            prop_assert_eq!(reader.read_u16().expect("draw_count") as i32, product.draw_count);
            prop_assert_eq!(
                reader.read_i32().expect("remaining_daily_limit"),
                product.remaining_daily_limit
            );
        }

        prop_assert_eq!(reader.read_u16().expect("detail_count"), 0);
        prop_assert_eq!(reader.read_u64().expect("trailer"), 0);
    }

    /// Feature: babel-npc-summon-fusion, Property 8: DigiSummon responses never use retired opcodes.
    ///
    /// Every DATA Summon response frames its opcode as 3652 (sync) or 3651
    /// (purchase) and never as the retired 3701/3702 values. This locks the
    /// opcode reconciliation so a regression to the old constants fails here.
    #[test]
    fn digi_summon_responses_never_use_retired_opcodes(
        sync_result in any::<u8>(),
        sync_products in prop::collection::vec(sync_product_strategy(), 0..=8),
        purchase_result in any::<u8>(),
        purchase_product_id in any::<i32>(),
        purchase_rewards in prop::collection::vec(reward_strategy(), 0..=8),
        purchase_products in prop::collection::vec(sync_product_strategy(), 0..=8),
    ) {
        const RETIRED_SYNC: i16 = 3702;
        const RETIRED_PURCHASE: i16 = 3701;

        let sync_frame = DigiSummonSyncResponsePacket {
            result: sync_result,
            products: sync_products,
        }
        .encode();
        let sync_opcode = PacketReader::from_frame(&sync_frame)
            .expect("sync frame should decode")
            .packet_type;
        prop_assert_eq!(sync_opcode, game::DIGI_SUMMON_SYNC_RESPONSE);
        prop_assert_eq!(sync_opcode, 3652);
        prop_assert_ne!(sync_opcode, RETIRED_SYNC);
        prop_assert_ne!(sync_opcode, RETIRED_PURCHASE);

        let purchase_frame = DigiSummonPurchaseResponsePacket {
            result: purchase_result,
            product_id: purchase_product_id,
            rewards: purchase_rewards,
            products: purchase_products,
        }
        .encode();
        let purchase_opcode = PacketReader::from_frame(&purchase_frame)
            .expect("purchase frame should decode")
            .packet_type;
        prop_assert_eq!(purchase_opcode, game::DIGI_SUMMON_PURCHASE_RESPONSE);
        prop_assert_eq!(purchase_opcode, 3651);
        prop_assert_ne!(purchase_opcode, RETIRED_PURCHASE);
        prop_assert_ne!(purchase_opcode, RETIRED_SYNC);
    }

    /// Feature: babel-npc-summon-fusion, Property 10: Combine catalog and request packets round-trip.
    ///
    /// For the Digi (3661 sync / 3662 combine) and Union (4301 sync / 4302
    /// combine) flows, encoding then decoding preserves the leading
    /// `ceiling_type` byte of the combine request, the `stCeiling` block
    /// (`{tier, value_a, value_b}` entries in order), and every
    /// `{item_uid, item_type, count}` material node in order.
    #[test]
    fn combine_catalog_and_request_packets_round_trip(
        union in any::<bool>(),
        result in any::<u8>(),
        ceiling in prop::collection::vec(ceiling_entry_strategy(), 0..=8),
        materials in prop::collection::vec(combine_item_strategy(), 0..=44),
        ceiling_type in any::<u8>(),
    ) {
        let (sync_opcode, combine_opcode) = if union {
            (game::UNION_COMBINE_SYNC, game::UNION_COMBINE)
        } else {
            (game::DIGI_COMBINE_SYNC, game::DIGI_COMBINE)
        };

        // S->C sync (3661 / 4301): result byte then the stCeiling block only.
        let sync_packet = if union {
            CombineSyncResponsePacket::union(result, ceiling.clone())
        } else {
            CombineSyncResponsePacket::digi(result, ceiling.clone())
        };
        let sync_frame = sync_packet.encode();
        let sync_raw = PacketReader::from_frame(&sync_frame).expect("sync frame should decode");
        prop_assert_eq!(sync_raw.packet_type, sync_opcode);

        let mut sync_reader = PacketReader::new(sync_raw.payload);
        prop_assert_eq!(sync_reader.read_u8().expect("sync result"), result);
        prop_assert_eq!(
            sync_reader.read_u16().expect("ceiling count") as usize,
            ceiling.len()
        );
        for entry in &ceiling {
            prop_assert_eq!(sync_reader.read_u8().expect("tier"), entry.tier);
            prop_assert_eq!(sync_reader.read_u8().expect("value_a"), entry.value_a);
            prop_assert_eq!(sync_reader.read_u16().expect("value_b"), entry.value_b);
        }

        // S->C combine (3662 / 4302): result byte, the stCeiling block, then the
        // submitted material echo list (reward list trails but is not asserted here).
        let combine_packet = if union {
            CombineResultResponsePacket::union_result(result, ceiling.clone(), materials.clone(), Vec::new())
        } else {
            CombineResultResponsePacket::digi_result(result, ceiling.clone(), materials.clone(), Vec::new())
        };
        let combine_frame = combine_packet.encode();
        let combine_raw =
            PacketReader::from_frame(&combine_frame).expect("combine frame should decode");
        prop_assert_eq!(combine_raw.packet_type, combine_opcode);

        let mut combine_reader = PacketReader::new(combine_raw.payload);
        prop_assert_eq!(combine_reader.read_u8().expect("combine result"), result);
        prop_assert_eq!(
            combine_reader.read_u16().expect("ceiling count") as usize,
            ceiling.len()
        );
        for entry in &ceiling {
            prop_assert_eq!(combine_reader.read_u8().expect("tier"), entry.tier);
            prop_assert_eq!(combine_reader.read_u8().expect("value_a"), entry.value_a);
            prop_assert_eq!(combine_reader.read_u16().expect("value_b"), entry.value_b);
        }
        prop_assert_eq!(
            combine_reader.read_u16().expect("material count") as usize,
            materials.len()
        );
        for node in &materials {
            prop_assert_eq!(combine_reader.read_u32().expect("item_uid"), node.item_uid);
            prop_assert_eq!(combine_reader.read_u16().expect("item_type"), node.item_type);
            prop_assert_eq!(combine_reader.read_u16().expect("count"), node.count);
        }

        // C->S combine request (3662 / 4302): leading ceiling_type byte then the
        // material list. Decoding the frame restores both in order.
        let mut request_writer = PacketWriter::new(combine_opcode);
        request_writer.write_u8(ceiling_type);
        request_writer.write_u16(materials.len() as u16);
        for node in &materials {
            request_writer.write_u32(node.item_uid);
            request_writer.write_u16(node.item_type);
            request_writer.write_u16(node.count);
        }
        let request_frame = request_writer.finalize();
        let request_raw =
            PacketReader::from_frame(&request_frame).expect("request frame should decode");
        let decoded = GameRequest::try_from(RawPacket {
            length: request_raw.length,
            packet_type: request_raw.packet_type,
            payload: request_raw.payload,
        })
        .expect("combine request should parse");

        let (decoded_ceiling_type, decoded_materials) = match decoded {
            GameRequest::DigiCombine {
                ceiling_type,
                materials,
            }
            | GameRequest::UnionCombine {
                ceiling_type,
                materials,
            } => (ceiling_type, materials),
            other => panic!("expected a combine request, got {other:?}"),
        };
        prop_assert_eq!(decoded_ceiling_type, ceiling_type);
        prop_assert_eq!(decoded_materials, materials);
    }

    /// Feature: babel-npc-summon-fusion, Property 13: Spirit conversion packets round-trip.
    ///
    /// The modern spirit requests (3239 spirit-to-digimon, 3240
    /// digimon-to-spirit) and their responses round-trip. Requests preserve every
    /// field, including the wide-string name. Responses preserve the leading
    /// fields and every zero-terminated item block in order, followed by the
    /// terminating zero count byte(s).
    #[test]
    fn extra_evolution_packets_round_trip(
        model_id in any::<i32>(),
        name in extra_evolution_name_strategy(),
        hatch_npc_id in any::<i32>(),
        slot in any::<u8>(),
        validation in "[0-9A-Za-z]{0,16}",
        craft_npc_id in any::<i32>(),
        digimon_id in any::<u32>(),
        hatch_bits in any::<i64>(),
        hatch_consumed in prop::collection::vec(item_block_strategy(), 0..=8),
        craft_slot in any::<u8>(),
        craft_bits in any::<i64>(),
        craft_consumed in prop::collection::vec(item_block_strategy(), 0..=8),
        craft_gained in prop::collection::vec(item_block_strategy(), 0..=8),
    ) {
        // Spirit-to-digimon request (3239): [i32 model_id][wstring name][i32 npc_id].
        let mut hatch_request = PacketWriter::new(game::SPIRIT_TO_DIGIMON);
        hatch_request.write_i32(model_id);
        hatch_request.write_wide_string(&name);
        hatch_request.write_i32(hatch_npc_id);
        let hatch_request_frame = hatch_request.finalize();
        let hatch_request_raw =
            PacketReader::from_frame(&hatch_request_frame).expect("hatch request decodes");
        let decoded_hatch = GameRequest::try_from(RawPacket {
            length: hatch_request_raw.length,
            packet_type: hatch_request_raw.packet_type,
            payload: hatch_request_raw.payload,
        })
        .expect("hatch request should parse");
        prop_assert_eq!(
            decoded_hatch,
            GameRequest::SpiritToDigimon {
                model_id,
                name: name.clone(),
                npc_id: hatch_npc_id,
            }
        );

        // Digimon-to-spirit request (3240): [u8 slot][string validation][i32 npc_id].
        let mut craft_request = PacketWriter::new(game::DIGIMON_TO_SPIRIT);
        craft_request.write_u8(slot);
        craft_request.write_string(&validation);
        craft_request.write_i32(craft_npc_id);
        let craft_request_frame = craft_request.finalize();
        let craft_request_raw =
            PacketReader::from_frame(&craft_request_frame).expect("craft request decodes");
        let decoded_craft = GameRequest::try_from(RawPacket {
            length: craft_request_raw.length,
            packet_type: craft_request_raw.packet_type,
            payload: craft_request_raw.payload,
        })
        .expect("craft request should parse");
        prop_assert_eq!(
            decoded_craft,
            GameRequest::DigimonToSpirit {
                slot,
                validation: validation.clone(),
                npc_id: craft_npc_id,
            }
        );

        // Spirit-to-digimon result (3239): [u32 digimon_id][i64 remaining_bits]
        // then a zero-terminated consumed-item block list.
        let hatch_frame = SpiritToDigimonResultPacket {
            digimon_id,
            remaining_bits: hatch_bits,
            consumed_items: hatch_consumed.clone(),
        }
        .encode();
        let hatch_raw = PacketReader::from_frame(&hatch_frame).expect("hatch result decodes");
        prop_assert_eq!(hatch_raw.packet_type, game::SPIRIT_TO_DIGIMON);
        let mut hatch_reader = PacketReader::new(hatch_raw.payload);
        prop_assert_eq!(hatch_reader.read_u32().expect("digimon_id"), digimon_id);
        prop_assert_eq!(hatch_reader.read_u64().expect("remaining_bits") as i64, hatch_bits);
        for (count, item_id) in &hatch_consumed {
            prop_assert_eq!(hatch_reader.read_u8().expect("consumed count"), *count);
            prop_assert_eq!(hatch_reader.read_u32().expect("consumed item_id"), *item_id);
        }
        prop_assert_eq!(hatch_reader.read_u8().expect("consumed terminator"), 0);

        // Digimon-to-spirit result (3240): [u8 slot][i64 remaining_bits] then a
        // zero-terminated consumed list and a zero-terminated gained list.
        let craft_frame = DigimonToSpiritResultPacket {
            slot: craft_slot,
            remaining_bits: craft_bits,
            consumed_items: craft_consumed.clone(),
            gained_items: craft_gained.clone(),
        }
        .encode();
        let craft_raw = PacketReader::from_frame(&craft_frame).expect("craft result decodes");
        prop_assert_eq!(craft_raw.packet_type, game::DIGIMON_TO_SPIRIT);
        let mut craft_reader = PacketReader::new(craft_raw.payload);
        prop_assert_eq!(craft_reader.read_u8().expect("slot"), craft_slot);
        prop_assert_eq!(craft_reader.read_u64().expect("remaining_bits") as i64, craft_bits);
        for (count, item_id) in &craft_consumed {
            prop_assert_eq!(craft_reader.read_u8().expect("consumed count"), *count);
            prop_assert_eq!(craft_reader.read_u32().expect("consumed item_id"), *item_id);
        }
        prop_assert_eq!(craft_reader.read_u8().expect("consumed terminator"), 0);
        for (count, item_id) in &craft_gained {
            prop_assert_eq!(craft_reader.read_u8().expect("gained count"), *count);
            prop_assert_eq!(craft_reader.read_u32().expect("gained item_id"), *item_id);
        }
        prop_assert_eq!(craft_reader.read_u8().expect("gained terminator"), 0);
    }

    /// D-Unit (Union hacking tool) opcodes 4311/4312/4313 round-trip exactly.
    ///
    /// The C2S open request is a bare body — encoding the variant yields a
    /// frame that decodes back to the same `UnionHackOpenRequest`. The modify
    /// request carries `[u8 slot][i32 part_id][i16 grade]` and round-trips all
    /// three fields. The S2C open / modify / init data packets each preserve
    /// the opcode, the unlocked slot count, every row in order, and the
    /// trailing rating/synergy integers.
    #[test]
    fn union_hack_packets_round_trip(
        slot in any::<u8>(),
        part_id in any::<i32>(),
        grade in any::<i16>(),
        result in any::<u8>(),
        unlocked_slots in any::<u8>(),
        slots in prop::collection::vec(union_hack_slot_strategy(), 0..=6),
        total_rating in any::<i32>(),
        synergy_bonus in any::<i32>(),
    ) {
        // C2S open (4311): bare body.
        let open_frame = encode_request(&GameRequest::UnionHackOpenRequest);
        let open_raw = PacketReader::from_frame(&open_frame).expect("open frame decodes");
        prop_assert_eq!(open_raw.packet_type, game::UNION_HACK_OPEN_REQUEST);
        let decoded_open = GameRequest::try_from(RawPacket {
            length: open_raw.length,
            packet_type: open_raw.packet_type,
            payload: open_raw.payload,
        })
        .expect("open request parses");
        prop_assert_eq!(&decoded_open, &GameRequest::UnionHackOpenRequest);
        let re_encoded_open = encode_request(&decoded_open);
        prop_assert_eq!(re_encoded_open, open_frame);

        // C2S modify (4312): [u8 slot][i32 part_id][i16 grade].
        let modify_req = GameRequest::UnionHackModify {
            slot,
            part_id,
            grade,
        };
        let modify_frame = encode_request(&modify_req);
        let modify_raw = PacketReader::from_frame(&modify_frame).expect("modify frame decodes");
        let decoded_modify = GameRequest::try_from(RawPacket {
            length: modify_raw.length,
            packet_type: modify_raw.packet_type,
            payload: modify_raw.payload,
        })
        .expect("modify request parses");
        prop_assert_eq!(&decoded_modify, &modify_req);
        let re_encoded_modify = encode_request(&decoded_modify);
        prop_assert_eq!(re_encoded_modify, modify_frame);

        // S2C open (4311): [u8 result][u8 unlocked][u8 count][rows...].
        let open_resp_frame = UnionHackOpenResponsePacket {
            result,
            unlocked_slots,
            slots: slots.clone(),
        }
        .encode();
        let open_resp_raw =
            PacketReader::from_frame(&open_resp_frame).expect("open response decodes");
        prop_assert_eq!(open_resp_raw.packet_type, game::UNION_HACK_OPEN_RESPONSE);
        let mut reader = PacketReader::new(open_resp_raw.payload);
        prop_assert_eq!(reader.read_u8().expect("open result"), result);
        prop_assert_eq!(reader.read_u8().expect("open unlocked"), unlocked_slots);
        prop_assert_eq!(reader.read_u8().expect("open count") as usize, slots.len());
        for row in &slots {
            prop_assert_eq!(reader.read_u8().expect("row slot"), row.slot);
            prop_assert_eq!(reader.read_i32().expect("row part"), row.part_id);
            prop_assert_eq!(reader.read_i16().expect("row grade"), row.grade);
            prop_assert_eq!(reader.read_u8().expect("row locked"), u8::from(row.locked));
        }

        // S2C modify (4312): [u8 result][u8 slot][i32 part][i16 grade][i32 total].
        let modify_resp_frame = UnionHackModifyResponsePacket {
            result,
            slot,
            new_part_id: part_id,
            new_grade: grade,
            total_rating,
        }
        .encode();
        let modify_resp_raw =
            PacketReader::from_frame(&modify_resp_frame).expect("modify response decodes");
        prop_assert_eq!(modify_resp_raw.packet_type, game::UNION_HACK_MODIFY_RESPONSE);
        let mut reader = PacketReader::new(modify_resp_raw.payload);
        prop_assert_eq!(reader.read_u8().expect("mod result"), result);
        prop_assert_eq!(reader.read_u8().expect("mod slot"), slot);
        prop_assert_eq!(reader.read_i32().expect("mod part"), part_id);
        prop_assert_eq!(reader.read_i16().expect("mod grade"), grade);
        prop_assert_eq!(reader.read_i32().expect("mod total"), total_rating);

        // S2C init (4313): [u8 count][rows...][i32 total][i32 synergy].
        let init_frame = UnionInitDataPacket {
            slots: slots.clone(),
            total_rating,
            synergy_bonus,
        }
        .encode();
        let init_raw = PacketReader::from_frame(&init_frame).expect("init frame decodes");
        prop_assert_eq!(init_raw.packet_type, game::UNION_INIT_DATA);
        let mut reader = PacketReader::new(init_raw.payload);
        prop_assert_eq!(reader.read_u8().expect("init count") as usize, slots.len());
        for row in &slots {
            prop_assert_eq!(reader.read_u8().expect("init row slot"), row.slot);
            prop_assert_eq!(reader.read_i32().expect("init row part"), row.part_id);
            prop_assert_eq!(reader.read_i16().expect("init row grade"), row.grade);
            prop_assert_eq!(reader.read_u8().expect("init row locked"), u8::from(row.locked));
        }
        prop_assert_eq!(reader.read_i32().expect("init total"), total_rating);
        prop_assert_eq!(reader.read_i32().expect("init synergy"), synergy_bonus);
    }

    /// Feature: babel-npc-summon-fusion, Property 16: Every covered C2S request round-trips.
    ///
    /// For any covered request, encoding to a frame, decoding the frame, and
    /// re-encoding the decoded value yields a byte-identical frame and an equal
    /// request value. This is the standard round-trip identity stated both ways:
    /// `decode ∘ encode` preserves the value and `encode ∘ decode` preserves the
    /// bytes. Coverage spans the summon (3651/3652), Digi combine (3661-3663),
    /// Union combine (4301-4303), Spirit conversion (3239/3240), and random box
    /// (16067/16068) request opcodes.
    #[test]
    fn covered_c2s_requests_round_trip(req in covered_request_strategy()) {
        let frame1 = encode_request(&req);

        let raw = PacketReader::from_frame(&frame1).expect("request frame should decode");
        let decoded = GameRequest::try_from(RawPacket {
            length: raw.length,
            packet_type: raw.packet_type,
            payload: raw.payload,
        })
        .expect("covered request should parse");

        prop_assert_eq!(&decoded, &req);

        let frame2 = encode_request(&decoded);
        prop_assert_eq!(frame1, frame2);
    }

    /// Feature: babel-npc-summon-fusion, Property 18: Covered packets satisfy the frame and checksum invariants.
    ///
    /// Every covered frame, in both directions, is
    /// `[Length u16 LE][Opcode i16 LE][Payload][Checksum u16 LE]`: the length
    /// field equals the byte length, the opcode field equals the packet's
    /// opcode, and the trailing checksum equals `length XOR CHECKSUM_VALIDATION`.
    /// `PacketReader::from_frame` revalidates the checksum and recovers the same
    /// opcode. Coverage spans every covered C2S request and every S2C response
    /// added by this feature.
    #[test]
    fn covered_packets_satisfy_frame_and_checksum_invariants(
        req in covered_request_strategy(),
        (response_frame, response_opcode) in covered_response_strategy(),
    ) {
        // C2S request direction: opcode taken from the variant→opcode map.
        let request_frame = encode_request(&req);
        assert_frame_invariants(&request_frame, request_opcode(&req))?;

        // S2C response direction: opcode paired with the generated frame.
        assert_frame_invariants(&response_frame, response_opcode)?;
    }

    /// Feature: babel-npc-summon-fusion, Property 19: String fields round-trip including empty strings.
    ///
    /// Both length-prefixed string codecs round-trip exactly. The ASCII codec
    /// (`write_string`/`read_string`, wire `[u8 byte-length][bytes][0x00]`)
    /// round-trips printable-ASCII and Unicode strings up to the `u8` byte cap.
    /// The wide codec (`write_wide_string`/`read_wide_string`, wire
    /// `[u8 unit-count][u16 units...][u16 0]`) round-trips the same Unicode
    /// strings, with the code-unit count governing the prefix. Generated values
    /// carry no edge whitespace, so the readers' `trim` never alters them, and
    /// the empty string is covered both by length-0 draws and the explicit
    /// assertions below. Each round-trip checks the terminator is present.
    #[test]
    fn string_fields_round_trip_including_empty(
        ascii in printable_ascii_string_strategy(),
        unicode in unicode_string_strategy(),
    ) {
        // The empty string is the key boundary: prefix 0 and a bare terminator.
        assert_ascii_string_round_trips("")?;
        assert_wide_string_round_trips("")?;

        // ASCII codec carries any valid UTF-8 (lossless via from_utf8_lossy);
        // the byte-length prefix governs reads, so multi-byte content is fine.
        assert_ascii_string_round_trips(&ascii)?;
        assert_ascii_string_round_trips(&unicode)?;

        // Wide codec carries any String (encode_utf16 never emits lone
        // surrogates, so from_utf16_lossy is lossless here).
        assert_wide_string_round_trips(&ascii)?;
        assert_wide_string_round_trips(&unicode)?;
    }
}
