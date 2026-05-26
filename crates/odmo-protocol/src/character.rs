use odmo_types::{AccountId, CharacterSummary};

use crate::{
    error::ProtocolError,
    opcode::character,
    reader::{PacketReader, RawPacket},
    writer::PacketWriter,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CharacterRequest {
    Connection {
        kind: u8,
    },
    KeepConnection,
    RequestCharacters {
        account_id: AccountId,
    },
    CreateCharacter {
        slot: u8,
        tamer_model: i32,
        tamer_name: String,
        partner_model: i32,
        partner_name: String,
    },
    DeleteCharacter {
        slot: u8,
        validation: String,
    },
    GetCharacterPosition {
        slot: u8,
    },
    ConnectGameServer,
    CheckNameDuplicity {
        name: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeleteCharacterResult {
    Error = 0,
    Deleted = 1,
    ValidationFail = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharacterCreationFailure {
    Generic = 1,
    ConflictingTamerName = 9,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterConnectionPacket {
    pub handshake: i16,
}

impl CharacterConnectionPacket {
    /// The proactive handshake sent on TCP connect.
    /// Uses opcode -1 (65535 as u16).
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
pub struct CharacterListPacket {
    pub characters: Vec<CharacterSummary>,
}

impl CharacterListPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(character::CHARACTER_LIST);
        writer.write_u8(self.characters.len() as u8);

        for character in &self.characters {
            writer.write_u8(character.slot);
            writer.write_i16(character.map_id);
            writer.write_i32(character.model);
            writer.write_u8(character.level);
            writer.write_string(&character.name);

            for _ in 0..18 {
                writer.write_bytes(&[0; 60]);
            }

            writer.write_i32(character.partner_model);
            writer.write_u8(character.partner_level);
            writer.write_string(&character.partner_name);
            writer.write_i16(character.partner_size);
            writer.write_i16(0);
            writer.write_i16(0);
            writer.write_i16(0);
        }

        writer.finalize()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectGameServerInfoPacket {
    pub address: String,
    pub port: u16,
    pub map_id: i16,
}

impl ConnectGameServerInfoPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(character::CONNECT_GAME_SERVER_INFO);
        writer.write_string(&self.address);
        writer.write_i32(self.port as i32);
        writer.write_i32(self.map_id as i32);
        writer.finalize()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectGameServerPacket;

impl ConnectGameServerPacket {
    pub fn encode(&self) -> Vec<u8> {
        PacketWriter::new(character::CONNECT_GAME_SERVER).finalize()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AvailableNamePacket {
    pub available: bool,
}

impl AvailableNamePacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(character::CHECK_NAME_DUPLICITY);
        writer.write_i32(self.available as i32);
        writer.finalize()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterDeletedPacket {
    pub result: DeleteCharacterResult,
}

impl CharacterDeletedPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(character::CHARACTER_DELETED);
        writer.write_i32(self.result as i32);
        writer.finalize()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterCreationFailedPacket {
    pub result: CharacterCreationFailure,
}

impl CharacterCreationFailedPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(character::CHARACTER_CREATION_FAILED);
        writer.write_u32(self.result as u32);
        writer.finalize()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CharacterCreatedPacket {
    pub character: CharacterSummary,
    pub handshake: i16,
}

impl CharacterCreatedPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(character::CHARACTER_CREATED);
        writer.write_i16(self.handshake);
        writer.write_i16(0);
        writer.write_i16(0);
        writer.write_i16(0);
        writer.write_i16(self.handshake);
        writer.write_i16(0);
        writer.write_i16(0);
        writer.write_i16(0);
        writer.write_u8(self.character.slot);
        writer.write_i16(self.character.map_id);
        writer.write_i32(self.character.model);
        writer.write_u8(1);
        writer.write_string(&self.character.name);

        for _ in 0..18 {
            writer.write_bytes(&[0; 60]);
        }

        writer.write_i32(self.character.partner_model);
        writer.write_u8(1);
        writer.write_string(&self.character.partner_name);
        writer.write_bytes(&[0; 8]);
        writer.finalize()
    }
}

impl TryFrom<RawPacket> for CharacterRequest {
    type Error = ProtocolError;

    fn try_from(packet: RawPacket) -> Result<Self, Self::Error> {
        let mut reader = PacketReader::new(packet.payload);
        match packet.packet_type {
            character::CONNECTION => Ok(Self::Connection {
                kind: reader.read_u8()?,
            }),
            character::KEEP_CONNECTION => Ok(Self::KeepConnection),
            character::REQUEST_CHARACTERS => {
                // Client AccessCode packet payload: [XOR(4)][accountIdx(4)][accessCode(4)]
                // AccessCode payload: [XOR(4)][accountIdx(4)][accessCode(4)]
                reader.seek(4)?;
                Ok(Self::RequestCharacters {
                    account_id: reader.read_u32()? as AccountId,
                })
            }
            character::CREATE_CHARACTER => Ok(parse_create_character(&mut reader)?),
            character::DELETE_CHARACTER => {
                let slot = reader.read_u8()?;
                reader.seek(4)?;
                Ok(Self::DeleteCharacter {
                    slot,
                    validation: reader.read_string()?,
                })
            }
            character::GET_CHARACTER_POSITION => {
                let slot = if reader.remaining_len() >= 4 {
                    reader.read_i32()? as u8
                } else {
                    reader.read_u8()?
                };
                Ok(Self::GetCharacterPosition { slot })
            }
            character::CONNECT_GAME_SERVER => Ok(Self::ConnectGameServer),
            character::CHECK_NAME_DUPLICITY => Ok(Self::CheckNameDuplicity {
                name: reader.read_string()?,
            }),
            other => Err(ProtocolError::InvalidCharacterPacketType(other)),
        }
    }
}

fn parse_create_character(reader: &mut PacketReader) -> Result<CharacterRequest, ProtocolError> {
    let slot = reader.read_u8()?;
    let tamer_model = reader.read_i32()?;
    let tamer_name = read_fixed_ascii(reader, 33)?;
    let partner_model = reader.read_i32()?;
    let partner_name = read_fixed_ascii(reader, 33)?;

    Ok(CharacterRequest::CreateCharacter {
        slot,
        tamer_model,
        tamer_name,
        partner_model,
        partner_name,
    })
}

fn read_fixed_ascii(reader: &mut PacketReader, len: usize) -> Result<String, ProtocolError> {
    let bytes = reader.read_bytes(len)?;
    let end = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len());
    Ok(String::from_utf8_lossy(&bytes[..end]).trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reader::PacketReader;
    use odmo_types::{DEFAULT_PARTNER_MODEL_ID, DEFAULT_TAMER_MODEL_ID};

    #[test]
    fn parse_request_characters_reads_account_id_from_offset_4() {
        // Client sends: newp(1706); push(XOR); push(accountIdx); push(accessCode); endp();
        // Payload = [XOR(4)][accountIdx(4)][accessCode(4)] = 12 bytes
        // Skip XOR value; read accountIdx at offset 4 in payload
        let mut payload = Vec::new();
        payload.extend_from_slice(&(0xDEAD_u32).to_le_bytes()); // XOR value (skipped)
        payload.extend_from_slice(&(1_u32).to_le_bytes()); // account_idx
        payload.extend_from_slice(&(0xBEEF_u32).to_le_bytes()); // access_code

        let request = CharacterRequest::try_from(RawPacket {
            length: 0,
            packet_type: character::REQUEST_CHARACTERS,
            payload,
        })
        .expect("request should parse");

        assert_eq!(
            request,
            CharacterRequest::RequestCharacters { account_id: 1 }
        );
    }

    #[test]
    fn character_list_packet_uses_expected_opcode() {
        let packet = CharacterListPacket {
            characters: vec![CharacterSummary {
                id: 1,
                account_id: 1,
                slot: 0,
                name: "Admin".to_string(),
                partner_name: "Agumon".to_string(),
                model: DEFAULT_TAMER_MODEL_ID,
                partner_model: DEFAULT_PARTNER_MODEL_ID,
                ..CharacterSummary::default()
            }],
        }
        .encode();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        assert_eq!(raw.packet_type, character::CHARACTER_LIST);
    }

    #[test]
    fn parse_create_character_uses_fixed_name_fields() {
        let mut payload = Vec::new();
        payload.push(1);
        payload.extend_from_slice(&DEFAULT_TAMER_MODEL_ID.to_le_bytes());
        let mut tamer = [0_u8; 33];
        tamer[..5].copy_from_slice(b"Admin");
        payload.extend_from_slice(&tamer);
        payload.extend_from_slice(&31001_i32.to_le_bytes());
        let mut partner = [0_u8; 33];
        partner[..6].copy_from_slice(b"Agumon");
        payload.extend_from_slice(&partner);

        let request = CharacterRequest::try_from(RawPacket {
            length: 0,
            packet_type: character::CREATE_CHARACTER,
            payload,
        })
        .expect("request should parse");

        assert_eq!(
            request,
            CharacterRequest::CreateCharacter {
                slot: 1,
                tamer_model: DEFAULT_TAMER_MODEL_ID,
                tamer_name: "Admin".to_string(),
                partner_model: 31001,
                partner_name: "Agumon".to_string(),
            }
        );
    }
}
