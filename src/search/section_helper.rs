use crate::utils::pattern_search::find_bytes;

#[derive(Debug, Clone)]
pub struct SearchSection {
    pub offset: u64,
    pub offset_end: u64,
    pub address: u64,
    pub address_end: u64,
}

impl SearchSection {
    pub fn new(offset: u64, offset_end: u64, address: u64, address_end: u64) -> Self {
        Self { offset, offset_end, address, address_end }
    }
}

pub struct SectionHelper<'a> {
    data: &'a [u8],
    is_32bit: bool,
    version: f64,
    _sections: Vec<SearchSection>,
    data_sections: Vec<SearchSection>,
    code_sections: Vec<SearchSection>,
    bss_sections: Vec<SearchSection>,
    method_count: usize,
    type_definitions_count: usize,
    _metadata_usages_count: usize,
    image_count: usize,
    pointer_in_exec: bool,
}

impl<'a> SectionHelper<'a> {
    pub fn new(
        data: &'a [u8],
        is_32bit: bool,
        version: f64,
        sections: Vec<SearchSection>,
        data_sections: Vec<SearchSection>,
        code_sections: Vec<SearchSection>,
        bss_sections: Vec<SearchSection>,
        method_count: usize,
        type_definitions_count: usize,
        metadata_usages_count: usize,
        image_count: usize,
    ) -> Self {
        Self {
            data,
            is_32bit,
            version,
            _sections: sections,
            data_sections,
            code_sections,
            bss_sections,
            method_count,
            type_definitions_count,
            _metadata_usages_count: metadata_usages_count,
            image_count,
            pointer_in_exec: false,
        }
    }

    fn ptr_size(&self) -> usize {
        if self.is_32bit { 4 } else { 8 }
    }

    pub fn find_code_registration(&mut self) -> Option<u64> {
        if self.version >= 24.2 {
            let result = self.find_code_registration_2019(&true);
            if result.is_some() {
                self.pointer_in_exec = true;
                return result;
            }
            let result = self.find_code_registration_2019(&false);
            if result.is_some() {
                return result;
            }
            return None;
        }
        self.find_code_registration_old()
    }

    pub fn find_metadata_registration(&self) -> Option<u64> {
        if self.version < 19.0 {
            return None;
        }
        if self.version >= 27.0 {
            return self.find_metadata_registration_v21();
        }
        self.find_metadata_registration_old()
    }

    fn offset_to_va(&self, offset: usize) -> Option<u64> {
        for section in &self.data_sections {
            if offset as u64 >= section.offset && (offset as u64) < section.offset_end {
                return Some(offset as u64 - section.offset + section.address);
            }
        }
        None
    }

    fn find_refs_fast(&self, addr: u64) -> Vec<(usize, u64)> {
        let ptr_size = self.ptr_size();
        let addr_bytes = if self.is_32bit {
            (addr as u32).to_le_bytes().to_vec()
        } else {
            addr.to_le_bytes().to_vec()
        };

        let mut refs = Vec::new();
        let mut start = 0;
        while let Some(idx) = find_bytes(&self.data[start..], &addr_bytes) {
            let abs_idx = start + idx;
            if abs_idx % ptr_size == 0 {
                if let Some(va) = self.offset_to_va(abs_idx) {
                    refs.push((abs_idx, va));
                }
            }
            start = abs_idx + 1;
        }
        refs
    }

    fn search_bytes_iter(data: &[u8], pattern: &[u8]) -> Vec<usize> {
        let mut results = Vec::new();
        let mut start = 0;
        while let Some(idx) = find_bytes(&data[start..], pattern) {
            results.push(start + idx);
            start = start + idx + 1;
        }
        results
    }

    fn find_code_registration_2019(&self, use_exec: &bool) -> Option<u64> {
        let ptr_size = self.ptr_size();
        let feature_bytes = b"mscorlib.dll\x00";

        let sections = if *use_exec { &self.code_sections } else { &self.data_sections };

        for section in sections {
            let start = section.offset as usize;
            let end = section.offset_end as usize;
            if start >= self.data.len() || end > self.data.len() {
                continue;
            }
            let section_data = &self.data[start..end];

            for index in Self::search_bytes_iter(section_data, feature_bytes) {
                let dll_va = index as u64 + section.address;

                let refs1 = self.find_refs_fast(dll_va);

                for (_, ref_va) in &refs1 {
                    let refs2 = self.find_refs_fast(*ref_va);

                    for (_ref_offset2, ref_va2) in &refs2 {
                        if self.version >= 27.0 {
                            let min_target = ref_va2 - (self.image_count as u64 - 1) * ptr_size as u64;
                            let max_target = *ref_va2;

                            let count_bytes = if self.is_32bit {
                                (self.image_count as u32).to_le_bytes().to_vec()
                            } else {
                                (self.image_count as u64).to_le_bytes().to_vec()
                            };

                            let mut start_search = 0usize;
                            while let Some(idx) = find_bytes(&self.data[start_search..], &count_bytes) {
                                let abs_idx = start_search + idx;
                                if abs_idx % ptr_size == 0 {
                                    let next_offset = abs_idx + ptr_size;
                                    if next_offset + ptr_size <= self.data.len() {
                                        let ptr_val = self.read_ptr_at(next_offset);
                                        if let Some(pv) = ptr_val {
                                            if pv >= min_target && pv <= max_target {
                                                let i = (ref_va2 - pv) / ptr_size as u64;
                                                if i < self.image_count as u64 && pv == ref_va2 - i * ptr_size as u64 {
                                                    if let Some(ref_va3) = self.offset_to_va(next_offset) {
                                                        if self.version >= 29.1 {
                                                            return Some(ref_va3 - ptr_size as u64 * 16);
                                                        } else if self.version >= 29.0 {
                                                            return Some(ref_va3 - ptr_size as u64 * 14);
                                                        }
                                                        return Some(ref_va3 - ptr_size as u64 * 13);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                start_search = abs_idx + 1;
                            }
                        } else {
                            for i in 0..self.image_count {
                                let target = ref_va2 - (i as u64) * ptr_size as u64;
                                let refs3 = self.find_refs_fast(target);
                                for (_, ref_va3) in &refs3 {
                                    return Some(ref_va3 - ptr_size as u64 * 13);
                                }
                            }
                        }
                    }
                }
            }
        }
        None
    }

    fn find_code_registration_old(&self) -> Option<u64> {
        let count_bytes = if self.is_32bit {
            (self.method_count as u32).to_le_bytes().to_vec()
        } else {
            (self.method_count as u64).to_le_bytes().to_vec()
        };

        for section in &self.data_sections {
            let start = section.offset as usize;
            let end = section.offset_end as usize;
            if start >= self.data.len() || end > self.data.len() {
                continue;
            }
            let slice = &self.data[start..end];
            let mut search_start = 0;

            while let Some(pos) = find_bytes(&slice[search_start..], &count_bytes) {
                let abs_pos = start + search_start + pos;
                let ptr_size = self.ptr_size();
                let addr = section.address + (search_start + pos) as u64;

                if self.is_valid_code_registration(addr, abs_pos, ptr_size) {
                    return Some(addr);
                }
                search_start += pos + 1;
            }
        }
        None
    }

    fn find_metadata_registration_old(&self) -> Option<u64> {
        let count_bytes = if self.is_32bit {
            (self.type_definitions_count as u32).to_le_bytes().to_vec()
        } else {
            (self.type_definitions_count as u64).to_le_bytes().to_vec()
        };

        for section in &self.data_sections {
            let start = section.offset as usize;
            let end = section.offset_end as usize;
            if start >= self.data.len() || end > self.data.len() {
                continue;
            }
            let slice = &self.data[start..end];
            let mut search_start = 0;

            while let Some(pos) = find_bytes(&slice[search_start..], &count_bytes) {
                let abs_pos = start + search_start + pos;
                let ptr_size = self.ptr_size();
                let addr = section.address + (search_start + pos) as u64;

                if self.is_valid_metadata_registration(addr, abs_pos, ptr_size) {
                    return Some(addr);
                }
                search_start += pos + 1;
            }
        }
        None
    }

    fn find_metadata_registration_v21(&self) -> Option<u64> {
        let ptr_size = self.ptr_size();
        let type_count = self.type_definitions_count;

        let count_bytes = if self.is_32bit {
            (type_count as u32).to_le_bytes().to_vec()
        } else {
            (type_count as u64).to_le_bytes().to_vec()
        };

        for section in &self.data_sections {
            let start = section.offset as usize;
            let end = section.offset_end as usize;
            if start >= self.data.len() || end > self.data.len() {
                continue;
            }
            let section_data = &self.data[start..end];
            let mut search_start = 0usize;

            while let Some(idx) = find_bytes(&section_data[search_start..], &count_bytes) {
                let abs_idx = search_start + idx;

                if abs_idx % ptr_size == 0 {
                    let second_idx = abs_idx + 2 * ptr_size;
                    if second_idx + ptr_size <= section_data.len() {
                        let second_val = if self.is_32bit {
                            u32::from_le_bytes([
                                section_data[second_idx],
                                section_data[second_idx + 1],
                                section_data[second_idx + 2],
                                section_data[second_idx + 3],
                            ]) as u64
                        } else {
                            u64::from_le_bytes([
                                section_data[second_idx],
                                section_data[second_idx + 1],
                                section_data[second_idx + 2],
                                section_data[second_idx + 3],
                                section_data[second_idx + 4],
                                section_data[second_idx + 5],
                                section_data[second_idx + 6],
                                section_data[second_idx + 7],
                            ])
                        };

                        if second_val == type_count as u64 {
                            let ptr_offset = start + abs_idx + 3 * ptr_size;
                            if ptr_offset + ptr_size <= self.data.len() {
                                if let Some(pointer_va) = self.read_ptr_at(ptr_offset) {
                                    let pointer_offset = self.va_to_offset_data(pointer_va);
                                    if let Some(po) = pointer_offset {
                                        let sample_size = std::cmp::min(10, type_count);
                                        let mut valid = true;
                                        for i in 0..sample_size {
                                            let sample_offset = po + i * ptr_size;
                                            if sample_offset + ptr_size > self.data.len() {
                                                valid = false;
                                                break;
                                            }
                                            if let Some(ptr_val) = self.read_ptr_at(sample_offset) {
                                                let in_range = if self.pointer_in_exec {
                                                    self.is_in_code_sections(ptr_val)
                                                } else {
                                                    self.is_in_data_sections(ptr_val)
                                                };
                                                if !in_range {
                                                    valid = false;
                                                    break;
                                                }
                                            } else {
                                                valid = false;
                                                break;
                                            }
                                        }

                                        if valid {
                                            let addr = start + abs_idx;
                                            let result = addr as u64 - ptr_size as u64 * 10 - section.offset + section.address;
                                            return Some(result);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                search_start = abs_idx + 1;
            }
        }
        None
    }

    fn va_to_offset_data(&self, va: u64) -> Option<usize> {
        for sec in &self.data_sections {
            if va >= sec.address && va < sec.address_end {
                return Some((va - sec.address + sec.offset) as usize);
            }
        }
        None
    }

    fn is_valid_code_registration(&self, _addr: u64, offset: usize, ptr_size: usize) -> bool {
        if offset + ptr_size > self.data.len() {
            return false;
        }
        let next_ptr = self.read_ptr_at(offset + ptr_size);
        if let Some(ptr) = next_ptr {
            self.is_in_data_sections(ptr) || self.is_in_code_sections(ptr)
        } else {
            false
        }
    }

    fn is_valid_metadata_registration(&self, _addr: u64, offset: usize, ptr_size: usize) -> bool {
        if offset + ptr_size > self.data.len() {
            return false;
        }
        let next_ptr = self.read_ptr_at(offset + ptr_size);
        if let Some(ptr) = next_ptr {
            self.is_in_data_sections(ptr) || self.is_in_bss_sections(ptr)
        } else {
            false
        }
    }

    fn is_in_data_sections(&self, addr: u64) -> bool {
        self.data_sections.iter().any(|s| addr >= s.address && addr <= s.address_end)
    }

    fn is_in_code_sections(&self, addr: u64) -> bool {
        self.code_sections.iter().any(|s| addr >= s.address && addr <= s.address_end)
    }

    fn is_in_bss_sections(&self, addr: u64) -> bool {
        self.bss_sections.iter().any(|s| addr >= s.address && addr <= s.address_end)
    }

    fn _addr_to_offset(&self, addr: u64) -> Option<u64> {
        for section in &self._sections {
            if addr >= section.address && addr < section.address_end {
                return Some(section.offset + (addr - section.address));
            }
        }
        None
    }

    fn read_ptr_at(&self, offset: usize) -> Option<u64> {
        if self.is_32bit {
            if offset + 4 > self.data.len() {
                return None;
            }
            Some(u32::from_le_bytes([
                self.data[offset],
                self.data[offset + 1],
                self.data[offset + 2],
                self.data[offset + 3],
            ]) as u64)
        } else {
            if offset + 8 > self.data.len() {
                return None;
            }
            Some(u64::from_le_bytes([
                self.data[offset],
                self.data[offset + 1],
                self.data[offset + 2],
                self.data[offset + 3],
                self.data[offset + 4],
                self.data[offset + 5],
                self.data[offset + 6],
                self.data[offset + 7],
            ]))
        }
    }
}
