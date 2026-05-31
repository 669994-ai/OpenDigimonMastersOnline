use crate::opcode::CHECKSUM_VALIDATION;

#[derive(Debug, Clone, Default)]
pub struct PacketWriter {
    buffer: Vec<u8>,
}

impl PacketWriter {
    pub fn new(packet_type: i16) -> Self {
        let mut buffer = Vec::with_capacity(32);
        buffer.extend_from_slice(&0_u16.to_le_bytes());
        buffer.extend_from_slice(&packet_type.to_le_bytes());
        Self { buffer }
    }

    pub fn write_u8(&mut self, value: u8) {
        self.buffer.push(value);
    }

    pub fn write_i8(&mut self, value: i8) {
        self.buffer.push(value as u8);
    }

    pub fn write_i16(&mut self, value: i16) {
        self.buffer.extend_from_slice(&value.to_le_bytes());
    }

    pub fn write_u16(&mut self, value: u16) {
        self.buffer.extend_from_slice(&value.to_le_bytes());
    }

    pub fn write_i32(&mut self, value: i32) {
        self.buffer.extend_from_slice(&value.to_le_bytes());
    }

    pub fn write_i64(&mut self, value: i64) {
        self.buffer.extend_from_slice(&value.to_le_bytes());
    }

    pub fn write_u64(&mut self, value: u64) {
        self.buffer.extend_from_slice(&value.to_le_bytes());
    }

    pub fn write_u32(&mut self, value: u32) {
        self.buffer.extend_from_slice(&value.to_le_bytes());
    }

    pub fn write_f32(&mut self, value: f32) {
        self.buffer.extend_from_slice(&value.to_le_bytes());
    }

    pub fn write_bytes(&mut self, bytes: &[u8]) {
        self.buffer.extend_from_slice(bytes);
    }

    pub fn write_u32_at(&mut self, value: u32, pos: usize) {
        self.buffer[pos..pos + 4].copy_from_slice(&value.to_le_bytes());
    }

    pub fn write_i32_at(&mut self, value: i32, pos: usize) {
        self.buffer[pos..pos + 4].copy_from_slice(&value.to_le_bytes());
    }

    pub fn write_string(&mut self, value: &str) {
        let bytes = value.as_bytes();
        self.write_u8(bytes.len() as u8);
        self.buffer.extend_from_slice(bytes);
        self.write_u8(0);
    }

    pub fn write_fixed_wide_string(&mut self, value: &str, char_count: usize) {
        let mut wide: Vec<u16> = value.encode_utf16().take(char_count).collect();
        wide.resize(char_count, 0);
        for code_unit in wide {
            self.write_u16(code_unit);
        }
    }

    /// Write a length-prefixed UTF-16LE string: `[u8 code-unit count][units...][u16 0]`.
    /// This is the wide analogue of `write_string`, used for variable-length names.
    pub fn write_wide_string(&mut self, value: &str) {
        let units: Vec<u16> = value.encode_utf16().collect();
        self.write_u8(units.len() as u8);
        for unit in units {
            self.write_u16(unit);
        }
        self.write_u16(0);
    }

    pub fn write_string_at(&mut self, value: &str, pos: usize) {
        let bytes = value.as_bytes();
        self.buffer[pos] = bytes.len() as u8;
        self.buffer[pos + 1..pos + 1 + bytes.len()].copy_from_slice(bytes);
        self.buffer[pos + 1 + bytes.len()] = 0;
    }

    pub fn write_zeroes(&mut self, count: usize) {
        self.buffer.resize(self.buffer.len() + count, 0);
    }

    pub fn finalize(mut self) -> Vec<u8> {
        self.write_i16(0);
        let length = self.buffer.len() as u16;
        self.buffer[0..2].copy_from_slice(&length.to_le_bytes());
        let checksum = (length as i16) ^ CHECKSUM_VALIDATION;
        let checksum_idx = self.buffer.len() - 2;
        self.buffer[checksum_idx..].copy_from_slice(&checksum.to_le_bytes());
        self.buffer
    }
}
