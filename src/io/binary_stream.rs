use std::io::{Cursor, Read, Seek, SeekFrom, Write};
use byteorder::{LittleEndian, BigEndian, ReadBytesExt, WriteBytesExt};
use crate::error::{Error, Result};

#[derive(Clone)]
pub struct BinaryStream {
    cursor: Cursor<Vec<u8>>,
    pub version: f64,
    pub is_32bit: bool,
    pub image_base: u64,
    pub is_big_endian: bool,
}

impl BinaryStream {
    pub fn new(data: Vec<u8>) -> Self {
        Self {
            cursor: Cursor::new(data),
            version: 24.0,
            is_32bit: true,
            image_base: 0,
            is_big_endian: false,
        }
    }

    pub fn position(&self) -> u64 {
        self.cursor.position()
    }

    pub fn set_position(&mut self, pos: u64) {
        self.cursor.set_position(pos);
    }

    pub fn len(&self) -> u64 {
        self.cursor.get_ref().len() as u64
    }

    pub fn is_empty(&self) -> bool {
        self.cursor.get_ref().is_empty()
    }

    pub fn pointer_size(&self) -> usize {
        if self.is_32bit { 4 } else { 8 }
    }

    pub fn data(&self) -> &[u8] {
        self.cursor.get_ref()
    }

    pub fn data_mut(&mut self) -> &mut Vec<u8> {
        self.cursor.get_mut()
    }

    pub fn read_bytes(&mut self, count: usize) -> Result<Vec<u8>> {
        let mut buf = vec![0u8; count];
        self.cursor
            .read_exact(&mut buf)
            .map_err(|_| Error::OutOfBounds {
                offset: self.cursor.position(),
                size: count,
            })?;
        Ok(buf)
    }

    pub fn read_u8(&mut self) -> Result<u8> {
        self.cursor.read_u8().map_err(Error::Io)
    }

    pub fn read_i8(&mut self) -> Result<i8> {
        self.cursor.read_i8().map_err(Error::Io)
    }

    pub fn read_bool(&mut self) -> Result<bool> {
        Ok(self.read_u8()? != 0)
    }

    pub fn read_u16(&mut self) -> Result<u16> {
        if self.is_big_endian {
            self.cursor.read_u16::<BigEndian>().map_err(Error::Io)
        } else {
            self.cursor.read_u16::<LittleEndian>().map_err(Error::Io)
        }
    }

    pub fn read_i16(&mut self) -> Result<i16> {
        if self.is_big_endian {
            self.cursor.read_i16::<BigEndian>().map_err(Error::Io)
        } else {
            self.cursor.read_i16::<LittleEndian>().map_err(Error::Io)
        }
    }

    pub fn read_u32(&mut self) -> Result<u32> {
        if self.is_big_endian {
            self.cursor.read_u32::<BigEndian>().map_err(Error::Io)
        } else {
            self.cursor.read_u32::<LittleEndian>().map_err(Error::Io)
        }
    }

    pub fn read_i32(&mut self) -> Result<i32> {
        if self.is_big_endian {
            self.cursor.read_i32::<BigEndian>().map_err(Error::Io)
        } else {
            self.cursor.read_i32::<LittleEndian>().map_err(Error::Io)
        }
    }

    pub fn read_u64(&mut self) -> Result<u64> {
        if self.is_big_endian {
            self.cursor.read_u64::<BigEndian>().map_err(Error::Io)
        } else {
            self.cursor.read_u64::<LittleEndian>().map_err(Error::Io)
        }
    }

    pub fn read_i64(&mut self) -> Result<i64> {
        if self.is_big_endian {
            self.cursor.read_i64::<BigEndian>().map_err(Error::Io)
        } else {
            self.cursor.read_i64::<LittleEndian>().map_err(Error::Io)
        }
    }

    pub fn read_f32(&mut self) -> Result<f32> {
        if self.is_big_endian {
            self.cursor.read_f32::<BigEndian>().map_err(Error::Io)
        } else {
            self.cursor.read_f32::<LittleEndian>().map_err(Error::Io)
        }
    }

    pub fn read_f64(&mut self) -> Result<f64> {
        if self.is_big_endian {
            self.cursor.read_f64::<BigEndian>().map_err(Error::Io)
        } else {
            self.cursor.read_f64::<LittleEndian>().map_err(Error::Io)
        }
    }

    pub fn read_ptr(&mut self) -> Result<u64> {
        if self.is_32bit {
            self.read_u32().map(u64::from)
        } else {
            self.read_u64()
        }
    }

    pub fn read_ptr_signed(&mut self) -> Result<i64> {
        if self.is_32bit {
            self.read_i32().map(i64::from)
        } else {
            self.read_i64()
        }
    }

    pub fn read_string_to_null(&mut self) -> Result<String> {
        let mut bytes = Vec::with_capacity(64);
        loop {
            let b = self.read_u8()?;
            if b == 0 {
                break;
            }
            bytes.push(b);
        }
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }

    pub fn read_string_to_null_at(&mut self, offset: u64) -> Result<String> {
        let saved = self.position();
        self.set_position(offset);
        let result = self.read_string_to_null();
        self.set_position(saved);
        result
    }

    pub fn read_string(&mut self, length: usize) -> Result<String> {
        let bytes = self.read_bytes(length)?;
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }

    pub fn read_compressed_u32(&mut self) -> Result<u32> {
        let b = self.read_u8()? as u32;
        if (b & 0x80) == 0 {
            Ok(b)
        } else if (b & 0x40) == 0 {
            let b2 = self.read_u8()? as u32;
            Ok(((b & 0x3F) << 8) | b2)
        } else {
            let b2 = self.read_u8()? as u32;
            let b3 = self.read_u8()? as u32;
            let b4 = self.read_u8()? as u32;
            Ok(((b & 0x1F) << 24) | (b2 << 16) | (b3 << 8) | b4)
        }
    }

    pub fn read_compressed_i32(&mut self) -> Result<i32> {
        let encoded = self.read_compressed_u32()?;
        if encoded & 1 != 0 {
            Ok(-((encoded >> 1) as i32) - 1)
        } else {
            Ok((encoded >> 1) as i32)
        }
    }

    pub fn read_uleb128(&mut self) -> Result<u64> {
        let mut result: u64 = 0;
        let mut shift = 0u32;
        loop {
            let b = self.read_u8()?;
            result |= ((b & 0x7F) as u64) << shift;
            if (b & 0x80) == 0 {
                break;
            }
            shift += 7;
        }
        Ok(result)
    }

    pub fn read_u32_array(&mut self, offset: u64, count: usize) -> Result<Vec<u32>> {
        if count == 0 {
            return Ok(Vec::new());
        }
        self.set_position(offset);
        let mut result = Vec::with_capacity(count);
        for _ in 0..count {
            result.push(self.read_u32()?);
        }
        Ok(result)
    }

    pub fn read_i32_array(&mut self, offset: u64, count: usize) -> Result<Vec<i32>> {
        if count == 0 {
            return Ok(Vec::new());
        }
        self.set_position(offset);
        let mut result = Vec::with_capacity(count);
        for _ in 0..count {
            result.push(self.read_i32()?);
        }
        Ok(result)
    }

    pub fn read_u64_array(&mut self, offset: u64, count: usize) -> Result<Vec<u64>> {
        if count == 0 {
            return Ok(Vec::new());
        }
        self.set_position(offset);
        let mut result = Vec::with_capacity(count);
        for _ in 0..count {
            result.push(self.read_u64()?);
        }
        Ok(result)
    }

    pub fn read_ptr_array(&mut self, offset: u64, count: usize) -> Result<Vec<u64>> {
        if count == 0 {
            return Ok(Vec::new());
        }
        self.set_position(offset);
        let mut result = Vec::with_capacity(count);
        for _ in 0..count {
            result.push(self.read_ptr()?);
        }
        Ok(result)
    }

    pub fn read_ptr_array_inline(&mut self, count: usize) -> Result<Vec<u64>> {
        let mut result = Vec::with_capacity(count);
        for _ in 0..count {
            result.push(self.read_ptr()?);
        }
        Ok(result)
    }

    pub fn read_u32_array_inline(&mut self, count: usize) -> Result<Vec<u32>> {
        let mut result = Vec::with_capacity(count);
        for _ in 0..count {
            result.push(self.read_u32()?);
        }
        Ok(result)
    }

    pub fn write_bytes(&mut self, data: &[u8]) -> Result<()> {
        self.cursor.write_all(data).map_err(Error::Io)
    }

    pub fn write_i32(&mut self, value: i32) -> Result<()> {
        self.cursor.write_i32::<LittleEndian>(value).map_err(Error::Io)
    }

    pub fn write_u32(&mut self, value: u32) -> Result<()> {
        self.cursor.write_u32::<LittleEndian>(value).map_err(Error::Io)
    }

    pub fn write_i64(&mut self, value: i64) -> Result<()> {
        self.cursor.write_i64::<LittleEndian>(value).map_err(Error::Io)
    }

    pub fn write_u64(&mut self, value: u64) -> Result<()> {
        self.cursor.write_u64::<LittleEndian>(value).map_err(Error::Io)
    }

    pub fn seek_to(&mut self, pos: u64) -> Result<()> {
        self.cursor.seek(SeekFrom::Start(pos)).map_err(Error::Io)?;
        Ok(())
    }

    pub fn seek_relative(&mut self, offset: i64) -> Result<()> {
        self.cursor.seek(SeekFrom::Current(offset)).map_err(Error::Io)?;
        Ok(())
    }

    pub fn peek_u32_at(&self, offset: u64) -> Result<u32> {
        let data = self.cursor.get_ref();
        let off = offset as usize;
        if off + 4 > data.len() {
            return Err(Error::OutOfBounds { offset, size: 4 });
        }
        Ok(u32::from_le_bytes([data[off], data[off + 1], data[off + 2], data[off + 3]]))
    }

    pub fn peek_u64_at(&self, offset: u64) -> Result<u64> {
        let data = self.cursor.get_ref();
        let off = offset as usize;
        if off + 8 > data.len() {
            return Err(Error::OutOfBounds { offset, size: 8 });
        }
        Ok(u64::from_le_bytes([
            data[off], data[off + 1], data[off + 2], data[off + 3],
            data[off + 4], data[off + 5], data[off + 6], data[off + 7],
        ]))
    }

    pub fn peek_ptr_at(&self, offset: u64) -> Result<u64> {
        if self.is_32bit {
            self.peek_u32_at(offset).map(u64::from)
        } else {
            self.peek_u64_at(offset)
        }
    }

    pub fn slice(&self, start: u64, len: usize) -> Result<&[u8]> {
        let s = start as usize;
        let data = self.cursor.get_ref();
        if s + len > data.len() {
            return Err(Error::OutOfBounds { offset: start, size: len });
        }
        Ok(&data[s..s + len])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_primitives() {
        let data = vec![
            0x42,
            0x00, 0x01,
            0x12, 0x34, 0x56, 0x78,
            0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        let mut stream = BinaryStream::new(data);
        assert_eq!(stream.read_u8().unwrap(), 0x42);
        assert_eq!(stream.read_u16().unwrap(), 256);
        assert_eq!(stream.read_u32().unwrap(), 0x78563412);
        assert_eq!(stream.read_u64().unwrap(), 1);
    }

    #[test]
    fn test_read_string_to_null() {
        let data = b"hello\x00world\x00".to_vec();
        let mut stream = BinaryStream::new(data);
        assert_eq!(stream.read_string_to_null().unwrap(), "hello");
        assert_eq!(stream.read_string_to_null().unwrap(), "world");
    }

    #[test]
    fn test_read_ptr_32bit() {
        let data = vec![0x78, 0x56, 0x34, 0x12];
        let mut stream = BinaryStream::new(data);
        stream.is_32bit = true;
        assert_eq!(stream.read_ptr().unwrap(), 0x12345678);
    }

    #[test]
    fn test_read_ptr_64bit() {
        let data = vec![0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80];
        let mut stream = BinaryStream::new(data);
        stream.is_32bit = false;
        assert_eq!(stream.read_ptr().unwrap(), 0x8000000000000001);
    }

    #[test]
    fn test_compressed_u32() {
        let mut stream = BinaryStream::new(vec![0x05]);
        assert_eq!(stream.read_compressed_u32().unwrap(), 5);

        let mut stream = BinaryStream::new(vec![0x81, 0x00]);
        assert_eq!(stream.read_compressed_u32().unwrap(), 256);
    }

    #[test]
    fn test_read_array() {
        let data: Vec<u8> = vec![
            0x01, 0x00, 0x00, 0x00,
            0x02, 0x00, 0x00, 0x00,
            0x03, 0x00, 0x00, 0x00,
        ];
        let mut stream = BinaryStream::new(data);
        let arr = stream.read_u32_array(0, 3).unwrap();
        assert_eq!(arr, vec![1, 2, 3]);
    }

    #[test]
    fn test_peek() {
        let data = vec![0x12, 0x34, 0x56, 0x78, 0xAB, 0xCD, 0xEF, 0x01];
        let stream = BinaryStream::new(data);
        assert_eq!(stream.peek_u32_at(0).unwrap(), 0x78563412);
        assert_eq!(stream.peek_u32_at(4).unwrap(), 0x01EFCDAB);
    }
}
