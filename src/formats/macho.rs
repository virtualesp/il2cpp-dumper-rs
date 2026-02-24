use crate::io::BinaryStream;
use crate::search::{SectionHelper, SearchSection};
use crate::error::{Error, Result};

pub const MH_MAGIC: u32 = 0xFEEDFACE;
pub const MH_MAGIC_64: u32 = 0xFEEDFACF;
pub const FAT_MAGIC: u32 = 0xCAFEBABE;
pub const FAT_CIGAM: u32 = 0xBEBAFECA;

pub const LC_SEGMENT: u32 = 0x01;
pub const LC_SYMTAB: u32 = 0x02;
pub const LC_SEGMENT_64: u32 = 0x19;
pub const LC_ENCRYPTION_INFO: u32 = 0x21;
pub const LC_ENCRYPTION_INFO_64: u32 = 0x2C;

pub const S_ATTR_PURE_INSTRUCTIONS: u32 = 0x80000000;
pub const S_ATTR_SOME_INSTRUCTIONS: u32 = 0x00000400;

#[derive(Debug, Clone)]
pub struct FatArch {
    pub cputype: u32,
    pub cpusubtype: u32,
    pub offset: u32,
    pub size: u32,
    pub align: u32,
    pub magic: u32,
}

#[derive(Debug, Clone, Default)]
pub struct Segment {
    pub segname: String,
    pub vmaddr: u64,
    pub vmsize: u64,
    pub fileoff: u64,
    pub filesize: u64,
    pub nsects: u32,
}

#[derive(Debug, Clone, Default)]
struct Section {
    pub sectname: String,
    pub addr: u64,
    pub size: u64,
    pub offset: u32,
    pub flags: u32,
}

#[derive(Debug, Clone, Default)]
struct SymtabCmd {
    pub symoff: u32,
    pub nsyms: u32,
    pub stroff: u32,
    pub strsize: u32,
}

#[derive(Debug, Clone, Default)]
struct NlistEntry {
    pub n_strx: u32,
    pub n_type: u8,
    pub n_sect: u8,
    pub n_desc: i16,
    pub n_value: u64,
}

pub fn parse_fat(data: &[u8]) -> Result<Vec<FatArch>> {
    if data.len() < 8 {
        return Err(Error::InvalidFormat("FAT header too small".into()));
    }
    let magic = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
    if magic != FAT_MAGIC && magic != FAT_CIGAM {
        return Err(Error::InvalidFormat("Not a FAT Mach-O".into()));
    }

    let nfat = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
    let mut arches = Vec::new();
    let mut offset = 8usize;

    for _ in 0..nfat {
        if offset + 20 > data.len() {
            break;
        }
        let cputype = u32::from_be_bytes([data[offset], data[offset + 1], data[offset + 2], data[offset + 3]]);
        let cpusubtype = u32::from_be_bytes([data[offset + 4], data[offset + 5], data[offset + 6], data[offset + 7]]);
        let arch_offset = u32::from_be_bytes([data[offset + 8], data[offset + 9], data[offset + 10], data[offset + 11]]);
        let size = u32::from_be_bytes([data[offset + 12], data[offset + 13], data[offset + 14], data[offset + 15]]);
        let align = u32::from_be_bytes([data[offset + 16], data[offset + 17], data[offset + 18], data[offset + 19]]);

        let slice_magic = if (arch_offset as usize) + 4 <= data.len() {
            u32::from_le_bytes([
                data[arch_offset as usize],
                data[arch_offset as usize + 1],
                data[arch_offset as usize + 2],
                data[arch_offset as usize + 3],
            ])
        } else {
            0
        };

        arches.push(FatArch {
            cputype,
            cpusubtype,
            offset: arch_offset,
            size,
            align,
            magic: slice_magic,
        });
        offset += 20;
    }

    Ok(arches)
}

pub fn extract_fat_slice(data: &[u8], arch: &FatArch) -> Result<Vec<u8>> {
    let start = arch.offset as usize;
    let end = start + arch.size as usize;
    if end > data.len() {
        return Err(Error::InvalidFormat("Fat slice extends beyond file".into()));
    }
    Ok(data[start..end].to_vec())
}

pub struct MachO {
    pub stream: BinaryStream,
    pub is_32bit: bool,
    pub segments: Vec<Segment>,
    sections: Vec<Section>,
    symbols: Vec<NlistEntry>,
    string_table: Vec<u8>,
    vmaddr: u64,
}

impl MachO {
    pub fn new(data: Vec<u8>, is_32bit: bool) -> Result<Self> {
        let mut macho = Self {
            stream: BinaryStream::new(data),
            is_32bit,
            segments: Vec::new(),
            sections: Vec::new(),
            symbols: Vec::new(),
            string_table: Vec::new(),
            vmaddr: 0,
        };
        macho.stream.is_32bit = is_32bit;
        macho.load()?;
        Ok(macho)
    }

    fn load(&mut self) -> Result<()> {
        self.stream.set_position(0);
        let magic = self.stream.read_u32()?;

        let expected = if self.is_32bit { MH_MAGIC } else { MH_MAGIC_64 };
        if magic != expected {
            return Err(Error::InvalidFormat("Invalid Mach-O magic".into()));
        }

        let _cputype = self.stream.read_i32()?;
        let _cpusubtype = self.stream.read_i32()?;
        let _filetype = self.stream.read_u32()?;
        let ncmds = self.stream.read_u32()?;
        let _sizeofcmds = self.stream.read_u32()?;
        let _flags = self.stream.read_u32()?;

        if !self.is_32bit {
            let _reserved = self.stream.read_u32()?;
        }

        let mut symtab: Option<SymtabCmd> = None;
        let mut cryptid = 0u32;

        let lc_segment_cmd = if self.is_32bit { LC_SEGMENT } else { LC_SEGMENT_64 };
        let lc_enc_cmd = if self.is_32bit { LC_ENCRYPTION_INFO } else { LC_ENCRYPTION_INFO_64 };

        for _ in 0..ncmds {
            let cmd_pos = self.stream.position();
            let cmd = self.stream.read_u32()?;
            let cmdsize = self.stream.read_u32()?;

            if cmd == lc_segment_cmd {
                self.stream.set_position(cmd_pos);
                let seg = self.read_segment()?;
                let nsects = seg.nsects;

                if seg.segname == "__TEXT" {
                    self.vmaddr = seg.vmaddr;
                }

                self.segments.push(seg);

                for _ in 0..nsects {
                    let section = self.read_section()?;
                    self.sections.push(section);
                }
            } else if cmd == LC_SYMTAB {
                self.stream.set_position(cmd_pos + 8);
                symtab = Some(SymtabCmd {
                    symoff: self.stream.read_u32()?,
                    nsyms: self.stream.read_u32()?,
                    stroff: self.stream.read_u32()?,
                    strsize: self.stream.read_u32()?,
                });
            } else if cmd == lc_enc_cmd {
                self.stream.set_position(cmd_pos + 8);
                let _cryptoff = self.stream.read_u32()?;
                let _cryptsize = self.stream.read_u32()?;
                cryptid = self.stream.read_u32()?;
            }

            self.stream.set_position(cmd_pos + cmdsize as u64);
        }

        if let Some(st) = symtab {
            self.load_symbols(&st)?;
        }

        if cryptid != 0 {
            eprintln!("ERROR: This Mach-O executable is encrypted and cannot be processed.");
        }

        Ok(())
    }

    fn read_segment(&mut self) -> Result<Segment> {
        let _cmd = self.stream.read_u32()?;
        let _cmdsize = self.stream.read_u32()?;
        let segname_bytes = self.stream.read_bytes(16)?;
        let segname = String::from_utf8_lossy(&segname_bytes)
            .trim_end_matches('\0')
            .to_string();

        let mut seg = Segment::default();
        seg.segname = segname;
        if self.is_32bit {
            seg.vmaddr = self.stream.read_u32()? as u64;
            seg.vmsize = self.stream.read_u32()? as u64;
            seg.fileoff = self.stream.read_u32()? as u64;
            seg.filesize = self.stream.read_u32()? as u64;
        } else {
            seg.vmaddr = self.stream.read_u64()?;
            seg.vmsize = self.stream.read_u64()?;
            seg.fileoff = self.stream.read_u64()?;
            seg.filesize = self.stream.read_u64()?;
        }
        let _maxprot = self.stream.read_i32()?;
        let _initprot = self.stream.read_i32()?;
        seg.nsects = self.stream.read_u32()?;
        let _flags = self.stream.read_u32()?;

        Ok(seg)
    }

    fn read_section(&mut self) -> Result<Section> {
        let sectname_bytes = self.stream.read_bytes(16)?;
        let sectname = String::from_utf8_lossy(&sectname_bytes)
            .trim_end_matches('\0')
            .to_string();
        let _segname = self.stream.read_bytes(16)?;

        let mut sect = Section::default();
        sect.sectname = sectname;
        if self.is_32bit {
            sect.addr = self.stream.read_u32()? as u64;
            sect.size = self.stream.read_u32()? as u64;
        } else {
            sect.addr = self.stream.read_u64()?;
            sect.size = self.stream.read_u64()?;
        }
        sect.offset = self.stream.read_u32()?;
        let _align = self.stream.read_u32()?;
        let _reloff = self.stream.read_u32()?;
        let _nreloc = self.stream.read_u32()?;
        sect.flags = self.stream.read_u32()?;
        let _reserved1 = self.stream.read_u32()?;
        let _reserved2 = self.stream.read_u32()?;
        if !self.is_32bit {
            let _reserved3 = self.stream.read_u32()?;
        }

        Ok(sect)
    }

    fn load_symbols(&mut self, symtab: &SymtabCmd) -> Result<()> {
        self.stream.set_position(symtab.stroff as u64);
        self.string_table = self.stream.read_bytes(symtab.strsize as usize)?;

        self.stream.set_position(symtab.symoff as u64);
        self.symbols.clear();

        for _ in 0..symtab.nsyms {
            let mut entry = NlistEntry::default();
            entry.n_strx = self.stream.read_u32()?;
            entry.n_type = self.stream.read_u8()?;
            entry.n_sect = self.stream.read_u8()?;
            if self.is_32bit {
                entry.n_desc = self.stream.read_i16()?;
                entry.n_value = self.stream.read_u32()? as u64;
            } else {
                let n_desc_u16 = self.stream.read_u16()?;
                entry.n_desc = n_desc_u16 as i16;
                entry.n_value = self.stream.read_u64()?;
            }
            self.symbols.push(entry);
        }

        Ok(())
    }

    fn get_symbol_name(&self, sym: &NlistEntry) -> String {
        let start = sym.n_strx as usize;
        if start >= self.string_table.len() {
            return String::new();
        }
        let end = self.string_table[start..].iter().position(|&b| b == 0)
            .map(|p| start + p)
            .unwrap_or(self.string_table.len());
        String::from_utf8_lossy(&self.string_table[start..end]).to_string()
    }

    pub fn map_vatr(&self, addr: u64) -> Result<u64> {
        for sect in &self.sections {
            if addr >= sect.addr && addr <= sect.addr + sect.size {
                if sect.sectname == "__bss" {
                    continue;
                }
                return Ok(addr - sect.addr + sect.offset as u64);
            }
        }
        for seg in &self.segments {
            if addr >= seg.vmaddr && addr < seg.vmaddr + seg.vmsize {
                return Ok(addr - seg.vmaddr + seg.fileoff);
            }
        }
        Err(Error::AddressNotMapped(addr))
    }

    pub fn map_rtva(&self, offset: u64) -> u64 {
        for sect in &self.sections {
            if offset >= sect.offset as u64 && offset <= sect.offset as u64 + sect.size {
                if sect.sectname == "__bss" {
                    return 0;
                }
                return offset - sect.offset as u64 + sect.addr;
            }
        }
        for seg in &self.segments {
            if offset >= seg.fileoff && offset < seg.fileoff + seg.filesize {
                return offset - seg.fileoff + seg.vmaddr;
            }
        }
        0
    }

    pub fn read_uint_ptr(&mut self) -> Result<u64> {
        if self.is_32bit {
            return Ok(self.stream.read_u32()? as u64);
        }
        let pointer = self.stream.read_u64()?;
        if pointer > self.vmaddr + 0xFFFFFFFF {
            let addr = self.stream.position();
            for sect in &self.sections {
                if addr >= sect.offset as u64 && addr <= sect.offset as u64 + sect.size {
                    if sect.sectname == "__const" || sect.sectname == "__data" {
                        let rva = pointer - self.vmaddr;
                        let masked = rva & 0xFFFFFFFF;
                        return Ok(masked + self.vmaddr);
                    }
                    break;
                }
            }
        }
        Ok(pointer)
    }

    pub fn symbol_search(&self) -> Option<(u64, u64)> {
        let mut code_reg = 0u64;
        let mut metadata_reg = 0u64;

        for sym in &self.symbols {
            let name = self.get_symbol_name(sym);
            if name == "_g_CodeRegistration" {
                code_reg = sym.n_value;
            } else if name == "_g_MetadataRegistration" {
                metadata_reg = sym.n_value;
            }
        }

        if code_reg > 0 && metadata_reg > 0 {
            Some((code_reg, metadata_reg))
        } else {
            None
        }
    }

    pub fn search_mod_init_func(&mut self, version: f64) -> Option<(u64, u64)> {
        let mod_init = self.sections.iter()
            .find(|s| s.sectname == "__mod_init_func")?
            .clone();

        if self.is_32bit {
            self.search_32bit(&mod_init, version)
        } else {
            self.search_64bit(&mod_init, version)
        }
    }

    fn search_32bit(&mut self, mod_init: &Section, version: f64) -> Option<(u64, u64)> {
        let feature_bytes_1: [u8; 2] = [0x0, 0x22]; // MOVS R2, #0
        let feature_bytes_2: [u8; 4] = [0x78, 0x44, 0x79, 0x44]; // ADD R0, PC; ADD R1, PC

        let count = (mod_init.size / 4) as usize;
        self.stream.set_position(mod_init.offset as u64);
        let mut addrs = Vec::with_capacity(count);
        for _ in 0..count {
            addrs.push(self.stream.read_u32().unwrap_or(0) as u64);
        }

        for a in &addrs {
            if *a == 0 { continue; }
            let i = *a - 1; // ARM Thumb bit
            if let Ok(mapped) = self.map_vatr(i) {
                self.stream.set_position(mapped + 4);
                let buff = self.stream.read_bytes(2).unwrap_or_default();
                if buff == feature_bytes_1 {
                    self.stream.set_position(mapped + 18);
                    let buff2 = self.stream.read_bytes(4).unwrap_or_default();
                    if buff2 == feature_bytes_2 {
                        self.stream.set_position(mapped + 10);
                        let mov_bytes = self.stream.read_bytes(8).unwrap_or_default();
                        let subaddr = decode_mov_arm32(&mov_bytes).wrapping_add(i + 24 - 1);
                        if let Ok(rsubaddr) = self.map_vatr(subaddr) {
                            self.stream.set_position(rsubaddr);
                            let mov_bytes2 = self.stream.read_bytes(8).unwrap_or_default();
                            let ptr = decode_mov_arm32(&mov_bytes2).wrapping_add(subaddr + 16);
                            if let Ok(ptr_offset) = self.map_vatr(ptr) {
                                self.stream.set_position(ptr_offset);
                                let metadata_registration = self.stream.read_u32().unwrap_or(0) as u64;

                                self.stream.set_position(rsubaddr + 8);
                                let buff3 = self.stream.read_bytes(4).unwrap_or_default();
                                self.stream.set_position(rsubaddr + 14);
                                let buff4 = self.stream.read_bytes(4).unwrap_or_default();
                                let combined: Vec<u8> = buff3.iter().chain(buff4.iter()).cloned().collect();

                                let code_extra = if version < 21.0 { 22u64 } else { 26u64 };
                                let code_registration = decode_mov_arm32(&combined).wrapping_add(subaddr + code_extra);

                                return Some((code_registration, metadata_registration));
                            }
                        }
                    }
                }
            }
        }
        None
    }

    fn search_64bit(&mut self, mod_init: &Section, version: f64) -> Option<(u64, u64)> {
        let feature_bytes_1: [u8; 4] = [0x2, 0x0, 0x80, 0xD2]; // MOV X2, #0
        let feature_bytes_2: [u8; 4] = [0x3, 0x0, 0x80, 0x52]; // MOV W3, #0

        let count = (mod_init.size / 8) as usize;
        self.stream.set_position(mod_init.offset as u64);
        let mut addrs = Vec::with_capacity(count);
        for _ in 0..count {
            addrs.push(self.stream.read_u64().unwrap_or(0));
        }

        let code_registration = 0u64;
        let metadata_registration = 0u64;

        for i in &addrs {
            if *i == 0 { continue; }
            let mapped = match self.map_vatr(*i) {
                Ok(m) => m,
                Err(_) => continue,
            };

            if version < 23.0 {
                // v<23: FeatureBytes1 then FeatureBytes2, OR vice versa
                self.stream.set_position(mapped);
                let buff = self.stream.read_bytes(4).unwrap_or_default();
                if buff == feature_bytes_1 {
                    let buff2 = self.stream.read_bytes(4).unwrap_or_default();
                    if buff2 == feature_bytes_2 {
                        self.stream.set_position(mapped + 16);
                        let inst = self.stream.read_bytes(4).unwrap_or_default();
                        if is_adr(&inst) {
                            let subaddr = decode_adr(*i + 16, &inst);
                            if let Some(result) = self.resolve_adrp_pair(*i, subaddr) {
                                return Some(result);
                            }
                        }
                    }
                } else {
                    self.stream.set_position(mapped + 0x10);
                    let buff2 = self.stream.read_bytes(4).unwrap_or_default();
                    if buff2 == feature_bytes_2 {
                        let buff3 = self.stream.read_bytes(4).unwrap_or_default();
                        if buff3 == feature_bytes_1 {
                            self.stream.set_position(mapped + 8);
                            let inst = self.stream.read_bytes(4).unwrap_or_default();
                            if is_adr(&inst) {
                                let subaddr = decode_adr(*i + 8, &inst);
                                if let Some(result) = self.resolve_adrp_pair(*i, subaddr) {
                                    return Some(result);
                                }
                            }
                        }
                    }
                }
            }

            if version >= 23.0 && version < 24.0 {
                // v==23: offset+16 has FeatureBytes1 then FeatureBytes2
                self.stream.set_position(mapped + 16);
                let buff = self.stream.read_bytes(4).unwrap_or_default();
                if buff == feature_bytes_1 {
                    let buff2 = self.stream.read_bytes(4).unwrap_or_default();
                    if buff2 == feature_bytes_2 {
                        self.stream.set_position(mapped + 8);
                        let inst = self.stream.read_bytes(4).unwrap_or_default();
                        let subaddr = decode_adr(*i + 8, &inst);
                        if let Some(result) = self.resolve_adrp_pair(*i, subaddr) {
                            return Some(result);
                        }
                    }
                }
            }

            if version >= 24.0 {
                // v>=24: offset+16 has FeatureBytes2 then FeatureBytes1 (swapped)
                self.stream.set_position(mapped + 16);
                let buff = self.stream.read_bytes(4).unwrap_or_default();
                if buff == feature_bytes_2 {
                    let buff2 = self.stream.read_bytes(4).unwrap_or_default();
                    if buff2 == feature_bytes_1 {
                        self.stream.set_position(mapped + 8);
                        let inst = self.stream.read_bytes(4).unwrap_or_default();
                        let subaddr = decode_adr(*i + 8, &inst);
                        if let Some(result) = self.resolve_adrp_pair(*i, subaddr) {
                            return Some(result);
                        }
                    }
                }
            }
        }

        if code_registration != 0 && metadata_registration != 0 {
            Some((code_registration, metadata_registration))
        } else {
            None
        }
    }

    fn resolve_adrp_pair(&mut self, _base_addr: u64, subaddr: u64) -> Option<(u64, u64)> {
        let rsubaddr = self.map_vatr(subaddr).ok()?;
        self.stream.set_position(rsubaddr);
        let adrp_bytes = self.stream.read_bytes(4).unwrap_or_default();
        let add_bytes = self.stream.read_bytes(4).unwrap_or_default();
        let code_registration = decode_adrp(subaddr, &adrp_bytes) + decode_add(&add_bytes);

        self.stream.set_position(rsubaddr + 8);
        let adrp_bytes2 = self.stream.read_bytes(4).unwrap_or_default();
        let add_bytes2 = self.stream.read_bytes(4).unwrap_or_default();
        let metadata_registration = decode_adrp(subaddr + 8, &adrp_bytes2) + decode_add(&add_bytes2);

        if code_registration != 0 && metadata_registration != 0 {
            Some((code_registration, metadata_registration))
        } else {
            None
        }
    }

    pub fn get_section_helper(&self, method_count: usize, type_definitions_count: usize, metadata_usages_count: usize, image_count: usize, version: f64) -> SectionHelper<'_> {
        let mut data_list = Vec::new();
        let mut exec_list = Vec::new();
        let mut bss_list = Vec::new();
        let mut all_sections = Vec::new();

        for sect in &self.sections {
            let search_section = SearchSection::new(
                sect.offset as u64,
                sect.offset as u64 + sect.size,
                sect.addr,
                sect.addr + sect.size,
            );

            all_sections.push(search_section.clone());

            if sect.flags == (S_ATTR_PURE_INSTRUCTIONS | S_ATTR_SOME_INSTRUCTIONS) {
                exec_list.push(search_section);
            } else if sect.flags == 1 {
                bss_list.push(search_section);
            } else if self.is_32bit {
                if sect.sectname == "__const" {
                    data_list.push(search_section);
                }
            } else {
                if sect.sectname == "__const" || sect.sectname == "__cstring" || sect.sectname == "__data" {
                    data_list.push(search_section);
                }
            }
        }

        SectionHelper::new(
            self.stream.data(),
            self.is_32bit,
            version,
            false,
            all_sections,
            data_list,
            exec_list,
            bss_list,
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
        pointer - self.vmaddr
    }
}

fn is_adr(inst: &[u8]) -> bool {
    if inst.len() < 4 { return false; }
    let value = u32::from_le_bytes([inst[0], inst[1], inst[2], inst[3]]);
    (value & 0x9F000000) == 0x10000000
}

fn decode_adr(pc: u64, inst: &[u8]) -> u64 {
    if inst.len() < 4 { return 0; }
    let value = u32::from_le_bytes([inst[0], inst[1], inst[2], inst[3]]);
    let immhi = ((value >> 5) & 0x7FFFF) as i64;
    let immlo = ((value >> 29) & 0x3) as i64;
    let imm = (immhi << 2) | immlo;
    let sign_extended = if imm & (1 << 20) != 0 {
        imm | !0xFFFFF // sign extend 21-bit
    } else {
        imm
    };
    (pc as i64 + sign_extended) as u64
}

fn decode_adrp(pc: u64, inst: &[u8]) -> u64 {
    if inst.len() < 4 { return 0; }
    let value = u32::from_le_bytes([inst[0], inst[1], inst[2], inst[3]]);
    let immhi = ((value >> 5) & 0x7FFFF) as i64;
    let immlo = ((value >> 29) & 0x3) as i64;
    let imm = ((immhi << 2) | immlo) << 12;
    let sign_extended = if imm & (1i64 << 32) != 0 {
        imm | !0xFFFFFFFF
    } else {
        imm
    };
    ((pc as i64 & !0xFFF) + sign_extended) as u64
}

fn decode_add(inst: &[u8]) -> u64 {
    if inst.len() < 4 { return 0; }
    let value = u32::from_le_bytes([inst[0], inst[1], inst[2], inst[3]]);
    let imm12 = (value >> 10) & 0xFFF;
    let shift = (value >> 22) & 0x3;
    if shift == 1 {
        (imm12 << 12) as u64
    } else {
        imm12 as u64
    }
}

fn decode_mov_arm32(data: &[u8]) -> u64 {
    if data.len() < 4 { return 0; }
    let low = u16::from_le_bytes([data[0], data[1]]);
    let high = if data.len() >= 4 {
        u16::from_le_bytes([data[2], data[3]])
    } else {
        0
    };
    let imm8 = (low & 0xFF) as u32;
    let imm3 = ((low >> 12) & 0x7) as u32;
    let i = ((high >> 10) & 0x1) as u32;
    let imm4 = (high & 0xF) as u32;
    let result = (imm4 << 12) | (i << 11) | (imm3 << 8) | imm8;
    result as u64
}
