use std::io::{Cursor, Read, Seek, SeekFrom};

use crate::{error::ProtocolError, opcode::CHECKSUM_VALIDATION};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawPacket {
    pub length: u16,
    pub packet_type: i16,
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct PacketReader {
    cursor: Cursor<Vec<u8>>,
}

impl PacketReader {
    pub fn from_frame(frame: &[u8]) -> Result<RawPacket, ProtocolError> {
        if frame.len() < 6 {
            return Err(ProtocolError::PacketTooShort);
        }

        let mut reader = Self::new(frame.to_vec());
        let length = reader.read_u16()?;

        if usize::from(length) != frame.len() {
            return Err(ProtocolError::PacketTooShort);
        }

        let packet_type = reader.read_i16()?;

        reader.seek(u64::from(length.saturating_sub(2)))?;
        let checksum = reader.read_i16()?;
        let expected = (length as i16) ^ CHECKSUM_VALIDATION;
        if checksum != expected {
            return Err(ProtocolError::InvalidChecksum {
                length: usize::from(length),
                expected,
                actual: checksum,
            });
        }

        reader.seek(4)?;
        let payload_len = usize::from(length).saturating_sub(6);
        let payload = reader.read_bytes(payload_len)?;

        Ok(RawPacket {
            length,
            packet_type,
            payload,
        })
    }

    pub fn new(buffer: Vec<u8>) -> Self {
        Self {
            cursor: Cursor::new(buffer),
        }
    }

    pub fn seek(&mut self, position: u64) -> Result<(), ProtocolError> {
        self.cursor
            .seek(SeekFrom::Start(position))
            .map(|_| ())
            .map_err(|_| ProtocolError::UnexpectedEof)
    }

    pub fn read_u8(&mut self) -> Result<u8, ProtocolError> {
        let mut buf = [0_u8; 1];
        self.cursor
            .read_exact(&mut buf)
            .map_err(|_| ProtocolError::UnexpectedEof)?;
        Ok(buf[0])
    }

    pub fn read_i16(&mut self) -> Result<i16, ProtocolError> {
        let mut buf = [0_u8; 2];
        self.cursor
            .read_exact(&mut buf)
            .map_err(|_| ProtocolError::UnexpectedEof)?;
        Ok(i16::from_le_bytes(buf))
    }

    pub fn read_u16(&mut self) -> Result<u16, ProtocolError> {
        let mut buf = [0_u8; 2];
        self.cursor
            .read_exact(&mut buf)
            .map_err(|_| ProtocolError::UnexpectedEof)?;
        Ok(u16::from_le_bytes(buf))
    }

    pub fn read_u32(&mut self) -> Result<u32, ProtocolError> {
        let mut buf = [0_u8; 4];
        self.cursor
            .read_exact(&mut buf)
            .map_err(|_| ProtocolError::UnexpectedEof)?;
        Ok(u32::from_le_bytes(buf))
    }

    pub fn read_i32(&mut self) -> Result<i32, ProtocolError> {
        let mut buf = [0_u8; 4];
        self.cursor
            .read_exact(&mut buf)
            .map_err(|_| ProtocolError::UnexpectedEof)?;
        Ok(i32::from_le_bytes(buf))
    }

    pub fn read_f32(&mut self) -> Result<f32, ProtocolError> {
        let mut buf = [0_u8; 4];
        self.cursor
            .read_exact(&mut buf)
            .map_err(|_| ProtocolError::UnexpectedEof)?;
        Ok(f32::from_le_bytes(buf))
    }

    pub fn read_bytes(&mut self, len: usize) -> Result<Vec<u8>, ProtocolError> {
        let mut buf = vec![0_u8; len];
        self.cursor
            .read_exact(&mut buf)
            .map_err(|_| ProtocolError::UnexpectedEof)?;
        Ok(buf)
    }

    pub fn remaining_len(&self) -> usize {
        self.cursor
            .get_ref()
            .len()
            .saturating_sub(self.cursor.position() as usize)
    }

    pub fn read_string(&mut self) -> Result<String, ProtocolError> {
        let len = self.read_u8()? as usize;
        let bytes = self.read_bytes(len)?;
        let _terminator = self.read_u8()?;
        Ok(String::from_utf8_lossy(&bytes).trim().to_string())
    }

    pub fn read_zstring(&mut self) -> Result<String, ProtocolError> {
        let mut out = Vec::new();
        loop {
            let byte = self.read_u8()?;
            if byte == 0 {
                break;
            }
            out.push(byte);
        }
        Ok(String::from_utf8_lossy(&out).to_string())
    }
}
