use crate::io::BinaryStream;
use crate::search::{SectionHelper, SearchSection};
use crate::error::{Error, Result};

pub const NSO_MAGIC: u32 = 0x304F534E;

pub struct Nso {
    pub stream: BinaryStream,
    pub is_32bit: bool,
    text_start: u64,
    text_end: u64,
    rodata_start: u64,
    rodata_end: u64,
    data_start: u64,
    data_end: u64,
    bss_size: u64,
    bss_end: u64,
}

impl Nso {
    pub fn new(data: Vec<u8>) -> Result<Self> {
        let header = Self::parse_header(&data)?;
        let decompressed = Self::decompress_if_needed(&data, &header)?;

        let text_start = header.text_memory_offset as u64;
        let text_end = text_start + header.text_decompressed_size as u64;
        let rodata_start = header.rodata_memory_offset as u64;
        let rodata_end = rodata_start + header.rodata_decompressed_size as u64;
        let data_start = header.data_memory_offset as u64;
        let data_end = data_start + header.data_decompressed_size as u64;
        let bss_size = header.bss_size as u64;
        let bss_end = data_end + bss_size;

        Ok(Self {
            stream: BinaryStream::new(decompressed),
            is_32bit: false,
            text_start,
            text_end,
            rodata_start,
            rodata_end,
            data_start,
            data_end,
            bss_size,
            bss_end,
        })
    }

    fn parse_header(data: &[u8]) -> Result<NsoHeader> {
        if data.len() < 0x70 {
            return Err(Error::InvalidFormat("NSO header too small".into()));
        }

        let magic = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        if magic != NSO_MAGIC {
            return Err(Error::InvalidFormat("Invalid NSO magic".into()));
        }

        Ok(NsoHeader {
            flags: u32::from_le_bytes([data[12], data[13], data[14], data[15]]),
            text_file_offset: u32::from_le_bytes([data[16], data[17], data[18], data[19]]),
            text_memory_offset: u32::from_le_bytes([data[20], data[21], data[22], data[23]]),
            text_decompressed_size: u32::from_le_bytes([data[24], data[25], data[26], data[27]]),
            rodata_file_offset: u32::from_le_bytes([data[32], data[33], data[34], data[35]]),
            rodata_memory_offset: u32::from_le_bytes([data[36], data[37], data[38], data[39]]),
            rodata_decompressed_size: u32::from_le_bytes([data[40], data[41], data[42], data[43]]),
            data_file_offset: u32::from_le_bytes([data[48], data[49], data[50], data[51]]),
            data_memory_offset: u32::from_le_bytes([data[52], data[53], data[54], data[55]]),
            data_decompressed_size: u32::from_le_bytes([data[56], data[57], data[58], data[59]]),
            bss_size: u32::from_le_bytes([data[60], data[61], data[62], data[63]]),
            text_compressed_size: u32::from_le_bytes([data[0x60], data[0x61], data[0x62], data[0x63]]),
            rodata_compressed_size: u32::from_le_bytes([data[0x64], data[0x65], data[0x66], data[0x67]]),
            data_compressed_size: u32::from_le_bytes([data[0x68], data[0x69], data[0x6A], data[0x6B]]),
        })
    }

    fn decompress_if_needed(data: &[u8], header: &NsoHeader) -> Result<Vec<u8>> {
        let text_compressed = (header.flags & 1) != 0;
        let rodata_compressed = (header.flags & 2) != 0;
        let data_compressed = (header.flags & 4) != 0;

        if !text_compressed && !rodata_compressed && !data_compressed {
            return Ok(data.to_vec());
        }

        let total_size = header.data_memory_offset as usize +
            header.data_decompressed_size as usize +
            header.bss_size as usize;

        let mut result = vec![0u8; total_size];

        Self::decompress_segment(
            data, &mut result,
            header.text_file_offset as usize,
            header.text_memory_offset as usize,
            header.text_decompressed_size as usize,
            header.text_compressed_size as usize,
            text_compressed,
        )?;

        Self::decompress_segment(
            data, &mut result,
            header.rodata_file_offset as usize,
            header.rodata_memory_offset as usize,
            header.rodata_decompressed_size as usize,
            header.rodata_compressed_size as usize,
            rodata_compressed,
        )?;

        Self::decompress_segment(
            data, &mut result,
            header.data_file_offset as usize,
            header.data_memory_offset as usize,
            header.data_decompressed_size as usize,
            header.data_compressed_size as usize,
            data_compressed,
        )?;

        Ok(result)
    }

    fn decompress_segment(
        src: &[u8],
        dst: &mut [u8],
        file_offset: usize,
        memory_offset: usize,
        decompressed_size: usize,
        compressed_size: usize,
        is_compressed: bool,
    ) -> Result<()> {
        if is_compressed {
            let end = file_offset + compressed_size;
            if end > src.len() {
                return Err(Error::InvalidFormat("Compressed segment out of bounds".into()));
            }
            let compressed = &src[file_offset..end];
            let decompressed = lz4_flex::decompress(compressed, decompressed_size)
                .map_err(|e| Error::InvalidFormat(format!("LZ4 decompression failed: {}", e)))?;
            let copy_len = std::cmp::min(decompressed.len(), dst.len().saturating_sub(memory_offset));
            dst[memory_offset..memory_offset + copy_len].copy_from_slice(&decompressed[..copy_len]);
        } else {
            let end = file_offset + decompressed_size;
            if end > src.len() {
                return Err(Error::InvalidFormat("Uncompressed segment out of bounds".into()));
            }
            let copy_len = std::cmp::min(decompressed_size, dst.len().saturating_sub(memory_offset));
            dst[memory_offset..memory_offset + copy_len].copy_from_slice(&src[file_offset..file_offset + copy_len]);
        }
        Ok(())
    }

    pub fn map_vatr(&self, addr: u64) -> Result<u64> {
        Ok(addr)
    }

    pub fn map_rtva(&self, addr: u64) -> u64 {
        addr
    }

    pub fn get_section_helper(&self, method_count: usize, type_definitions_count: usize, metadata_usages_count: usize, image_count: usize, version: f64) -> SectionHelper<'_> {
        let mut exec_list = Vec::new();
        let mut data_list = Vec::new();
        let mut bss_list = Vec::new();
        let mut all = Vec::new();

        let text = SearchSection::new(self.text_start, self.text_end, self.text_start, self.text_end);
        all.push(text.clone());
        exec_list.push(text);

        let rodata = SearchSection::new(self.rodata_start, self.rodata_end, self.rodata_start, self.rodata_end);
        all.push(rodata.clone());
        data_list.push(rodata);

        let data_sec = SearchSection::new(self.data_start, self.data_end, self.data_start, self.data_end);
        all.push(data_sec.clone());
        data_list.push(data_sec);

        if self.bss_size > 0 {
            let bss = SearchSection::new(self.data_end, self.bss_end, self.data_end, self.bss_end);
            all.push(bss.clone());
            bss_list.push(bss);
        }

        let bss = if bss_list.is_empty() { data_list.clone() } else { bss_list };

        SectionHelper::new(
            self.stream.data(),
            self.is_32bit,
            version,
            all,
            data_list,
            exec_list,
            bss,
            method_count,
            type_definitions_count,
            metadata_usages_count,
            image_count,
        )
    }

    pub fn check_dump(&self) -> bool {
        false
    }

    pub fn get_rva(&self, pointer: u64) -> u64 {
        pointer
    }
}

struct NsoHeader {
    flags: u32,
    text_file_offset: u32,
    text_memory_offset: u32,
    text_decompressed_size: u32,
    rodata_file_offset: u32,
    rodata_memory_offset: u32,
    rodata_decompressed_size: u32,
    data_file_offset: u32,
    data_memory_offset: u32,
    data_decompressed_size: u32,
    bss_size: u32,
    text_compressed_size: u32,
    rodata_compressed_size: u32,
    data_compressed_size: u32,
}
