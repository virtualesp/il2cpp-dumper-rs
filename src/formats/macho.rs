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
    pub vmaddr: u64,
    pub vmsize: u64,
    pub fileoff: u64,
    pub filesize: u64,
    pub nsects: u32,
}

#[derive(Debug, Clone, Default)]
struct Section {
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

pub struct MachO {
    pub stream: BinaryStream,
    pub is_32bit: bool,
    pub segments: Vec<Segment>,
    sections: Vec<Section>,
    symbols: Vec<NlistEntry>,
    string_table: Vec<u8>,
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
            eprintln!("Warning: Binary is encrypted");
        }

        Ok(())
    }

    fn read_segment(&mut self) -> Result<Segment> {
        let _cmd = self.stream.read_u32()?;
        let _cmdsize = self.stream.read_u32()?;
        let _segname = self.stream.read_bytes(16)?;

        let mut seg = Segment::default();
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
        let _sectname = self.stream.read_bytes(16)?;
        let _segname = self.stream.read_bytes(16)?;

        let mut sect = Section::default();
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
        for seg in &self.segments {
            if addr >= seg.vmaddr && addr < seg.vmaddr + seg.vmsize {
                return Ok(addr - seg.vmaddr + seg.fileoff);
            }
        }
        Err(Error::AddressNotMapped(addr))
    }

    pub fn map_rtva(&self, offset: u64) -> u64 {
        for seg in &self.segments {
            if offset >= seg.fileoff && offset < seg.fileoff + seg.filesize {
                return offset - seg.fileoff + seg.vmaddr;
            }
        }
        0
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

    pub fn get_section_helper(&self, method_count: usize, type_definitions_count: usize, metadata_usages_count: usize, image_count: usize, version: f64) -> SectionHelper<'_> {
        let mut data_list = Vec::new();
        let mut exec_list = Vec::new();
        let mut all_sections = Vec::new();

        for sect in &self.sections {
            let search_section = SearchSection::new(
                sect.offset as u64,
                sect.offset as u64 + sect.size,
                sect.addr,
                sect.addr + sect.size,
            );

            all_sections.push(search_section.clone());

            if (sect.flags & S_ATTR_PURE_INSTRUCTIONS) != 0 ||
               (sect.flags & S_ATTR_SOME_INSTRUCTIONS) != 0 {
                exec_list.push(search_section);
            } else {
                data_list.push(search_section);
            }
        }

        let bss = data_list.clone();

        SectionHelper::new(
            self.stream.data(),
            self.is_32bit,
            version,
            all_sections,
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
