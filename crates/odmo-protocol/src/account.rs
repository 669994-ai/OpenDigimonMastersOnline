use odmo_types::{AccountId, CharacterServerTarget, ServerDescriptor};

use crate::{
    error::ProtocolError,
    opcode::account,
    reader::{PacketReader, RawPacket},
    writer::PacketWriter,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountRequest {
    Connection {
        kind: u8,
    },
    KeepConnection,
    Login(LoginPayload),
    SecondaryPasswordRegister {
        password: String,
    },
    SecondaryPasswordCheck {
        check_mode: SecondaryPasswordCheck,
        password: Option<String>,
    },
    SecondaryPasswordChange {
        current_password: String,
        new_password: String,
    },
    LoadServerList,
    ConnectCharacterServer {
        server_id: i32,
    },
    ResourcesHash {
        client_hash: Vec<u8>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginPayload {
    pub username: String,
    pub password: String,
    pub cpu: String,
    pub gpu: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoginResponse {
    UserNotFound = 1,
    IncorrectPassword = 2,
    AccountBlocked = 4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecondaryPasswordScreen {
    Hide = 1,
    RequestInput = 2,
    RequestSetup = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecondaryPasswordCheck {
    Check = 2,
    DontCheck = 3,
    CorrectOrSkipped = 0,
    Incorrect = 20052,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecondaryPasswordChange {
    Changed = 0,
    IncorrectCurrentPassword = 20052,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionPacket {
    pub handshake: i16,
    pub handshake_timestamp: u32,
}

impl ConnectionPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(account::CONNECTION_RESPONSE);
        writer.write_i16(self.handshake);
        writer.write_u32(self.handshake_timestamp);
        writer.finalize()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoginRequestAnswerPacket {
    Failed(LoginResponse),
    Success(SecondaryPasswordScreen),
}

impl LoginRequestAnswerPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(account::LOGIN_REQUEST);
        match self {
            Self::Failed(reason) => {
                writer.write_u8(*reason as u8);
                writer.write_u8(39);
                writer.write_u8(0);
                writer.write_u8(0);
                writer.write_u8(0);
            }
            Self::Success(screen) => {
                writer.write_i32(0);
                writer.write_u8(*screen as u8);
            }
        }
        writer.finalize()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginRequestBannedAnswerPacket {
    pub remaining_time_in_seconds: u32,
    pub reason: String,
}

impl LoginRequestBannedAnswerPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(3308);
        writer.write_u32(self.remaining_time_in_seconds);
        writer.write_string(&self.reason);
        writer.finalize()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerListPacket {
    pub servers: Vec<ServerDescriptor>,
}

impl ServerListPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(account::LOAD_SERVER_LIST);
        writer.write_u8(self.servers.len() as u8);
        for server in &self.servers {
            writer.write_i32(server.id as i32);
            writer.write_string(&server.name);
            writer.write_u8(server.maintenance as u8);
            writer.write_u8(server.overloaded as u8);
            writer.write_u8(server.character_count);
            writer.write_u8(server.is_new as u8);
        }
        writer.write_i32(0);
        writer.finalize()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecondaryPasswordCheckResultPacket {
    pub result: SecondaryPasswordCheck,
}

impl SecondaryPasswordCheckResultPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(9804);
        writer.write_i32(self.result as i32);
        writer.finalize()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecondaryPasswordChangeResultPacket {
    pub result: SecondaryPasswordChange,
}

impl SecondaryPasswordChangeResultPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(9806);
        writer.write_i32(self.result as i32);
        writer.finalize()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourcesHashPacket {
    pub hash_hex: String,
}

impl ResourcesHashPacket {
    pub fn encode(&self) -> Vec<u8> {
        let hash_bytes = decode_hex(&self.hash_hex);
        let mut writer = PacketWriter::new(10003);
        writer.write_i16((hash_bytes.len()) as i16);
        writer.write_bytes(&hash_bytes);
        writer.finalize()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectCharacterServerPacket {
    pub account_id: AccountId,
    pub target: CharacterServerTarget,
}

impl ConnectCharacterServerPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = PacketWriter::new(901);
        // Client reads sequentially: pop(account_idx:u4), pop(access_code:u4), pop(ip:string), pop(port:u4)
        writer.write_u32(self.account_id as u32); // account_idx
        writer.write_u32(self.account_id as u32); // access_code
        writer.write_string(&self.target.address); // ip address
        writer.write_u32(self.target.port as u32); // port
        writer.write_u8(0); // VERSION_KOR: IsPcBang (u1)
        writer.finalize()
    }
}

impl TryFrom<RawPacket> for AccountRequest {
    type Error = ProtocolError;

    fn try_from(packet: RawPacket) -> Result<Self, Self::Error> {
        let mut reader = PacketReader::new(packet.payload);
        match packet.packet_type {
            account::CONNECTION => Ok(Self::Connection {
                kind: reader.read_u8()?,
            }),
            account::KEEP_CONNECTION => Ok(Self::KeepConnection),
            account::LOGIN_REQUEST => Ok(Self::Login(parse_login_payload(&mut reader)?)),
            9801 => Ok(Self::SecondaryPasswordRegister {
                password: reader.read_zstring()?,
            }),
            9804 => Ok(parse_secondary_password_check(&mut reader)?),
            9806 => Ok(Self::SecondaryPasswordChange {
                current_password: reader.read_zstring()?,
                new_password: reader.read_zstring()?,
            }),
            account::LOAD_SERVER_LIST => Ok(Self::LoadServerList),
            account::CONNECT_CHARACTER_SERVER => Ok(Self::ConnectCharacterServer {
                server_id: reader.read_i32()?,
            }),
            10003 => Ok(Self::ResourcesHash {
                client_hash: reader.read_bytes(reader.remaining_len())?,
            }),
            other => Err(ProtocolError::InvalidAccountPacketType(other)),
        }
    }
}

fn parse_login_payload(reader: &mut PacketReader) -> Result<LoginPayload, ProtocolError> {
    // Client sends: [u4 net_version][string user_type][string username][u1 dummy][string password]
    let _net_version = reader.read_u32()?;
    let _user_type = read_sized_ascii(reader)?;
    let username = read_sized_ascii(reader)?;
    let _dummy = reader.read_u8()?;
    let password = read_sized_ascii(reader)?;

    Ok(LoginPayload {
        username,
        password,
        cpu: String::new(),
        gpu: String::new(),
    })
}

fn read_sized_ascii(reader: &mut PacketReader) -> Result<String, ProtocolError> {
    let size = reader.read_u8()? as usize;
    let bytes = reader.read_bytes(size)?;
    Ok(String::from_utf8_lossy(&bytes).trim().to_string())
}

fn parse_secondary_password_check(
    reader: &mut PacketReader,
) -> Result<AccountRequest, ProtocolError> {
    let raw_mode = reader.read_i16()?;
    let check_mode = match raw_mode {
        2 => SecondaryPasswordCheck::Check,
        3 => SecondaryPasswordCheck::DontCheck,
        _ => return Err(ProtocolError::InvalidAccountPacketType(raw_mode)),
    };

    let password = if check_mode == SecondaryPasswordCheck::Check {
        Some(reader.read_zstring()?)
    } else {
        None
    };

    Ok(AccountRequest::SecondaryPasswordCheck {
        check_mode,
        password,
    })
}

fn decode_hex(hex: &str) -> Vec<u8> {
    hex.as_bytes()
        .chunks(2)
        .filter_map(|pair| {
            let hi = *pair.first()?;
            let lo = *pair.get(1)?;
            let hex_pair = [hi, lo];
            let text = std::str::from_utf8(&hex_pair).ok()?;
            u8::from_str_radix(text, 16).ok()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reader::PacketReader;

    #[test]
    fn connection_packet_uses_legacy_framing() {
        let packet = ConnectionPacket {
            handshake: 123,
            handshake_timestamp: 456,
        }
        .encode();

        let raw = PacketReader::from_frame(&packet).expect("frame should decode");
        assert_eq!(raw.packet_type, account::CONNECTION_RESPONSE);
    }

    #[test]
    fn login_success_matches_legacy_shape() {
        let packet = LoginRequestAnswerPacket::Success(SecondaryPasswordScreen::Hide).encode();
        let raw = PacketReader::from_frame(&packet).expect("frame should decode");

        assert_eq!(raw.packet_type, account::LOGIN_REQUEST);
        assert_eq!(raw.payload, vec![0, 0, 0, 0, 1]);
    }

    #[test]
    fn parse_login_payload_uses_legacy_offsets() {
        // Client sends: [u4 net_version][string user_type][string username][u1 dummy][string password]
        let mut payload = Vec::new();
        payload.extend_from_slice(&1234_u32.to_le_bytes()); // net_version
        payload.push(4);
        payload.extend_from_slice(b"test"); // user_type
        payload.push(4);
        payload.extend_from_slice(b"demo"); // username
        payload.push(0); // dummy
        payload.push(4);
        payload.extend_from_slice(b"pass"); // password

        let raw = RawPacket {
            length: 0,
            packet_type: account::LOGIN_REQUEST,
            payload,
        };

        let request = AccountRequest::try_from(raw).expect("request should parse");
        match request {
            AccountRequest::Login(login) => {
                assert_eq!(login.username, "demo");
                assert_eq!(login.password, "pass");
            }
            other => panic!("unexpected request: {other:?}"),
        }
    }

    #[test]
    fn parse_secondary_password_check_packet() {
        let raw = RawPacket {
            length: 0,
            packet_type: 9804,
            payload: {
                let mut payload = Vec::new();
                payload.extend_from_slice(&(2_i16).to_le_bytes());
                payload.extend_from_slice(b"4321");
                payload.push(0);
                payload
            },
        };

        let request = AccountRequest::try_from(raw).expect("request should parse");
        assert_eq!(
            request,
            AccountRequest::SecondaryPasswordCheck {
                check_mode: SecondaryPasswordCheck::Check,
                password: Some("4321".to_string()),
            }
        );
    }
}
