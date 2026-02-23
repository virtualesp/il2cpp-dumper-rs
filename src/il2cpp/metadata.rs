use std::collections::HashMap;
use crate::io::BinaryStream;
use crate::error::{Error, Result};
use super::structures::*;

pub const METADATA_MAGIC: u32 = 0xFAB11BAF;

pub struct Metadata {
    pub stream: BinaryStream,
    pub version: f64,
    pub header: Il2CppGlobalMetadataHeader,

    pub image_defs: Vec<Il2CppImageDefinition>,
    pub assembly_defs: Vec<Il2CppAssemblyDefinition>,
    pub type_defs: Vec<Il2CppTypeDefinition>,
    pub method_defs: Vec<Il2CppMethodDefinition>,
    pub parameter_defs: Vec<Il2CppParameterDefinition>,
    pub field_defs: Vec<Il2CppFieldDefinition>,
    pub property_defs: Vec<Il2CppPropertyDefinition>,
    pub event_defs: Vec<Il2CppEventDefinition>,
    pub generic_containers: Vec<Il2CppGenericContainer>,
    pub generic_parameters: Vec<Il2CppGenericParameter>,
    pub string_literals: Vec<Il2CppStringLiteral>,
    pub field_refs: Vec<Il2CppFieldRef>,

    pub interface_indices: Vec<i32>,
    pub nested_type_indices: Vec<i32>,
    pub constraint_indices: Vec<i32>,
    pub vtable_methods: Vec<u32>,

    pub attribute_type_ranges: Vec<Il2CppCustomAttributeTypeRange>,
    pub attribute_types: Vec<i32>,
    pub attribute_data_ranges: Vec<Il2CppCustomAttributeDataRange>,

    pub metadata_usage_dic: HashMap<u32, HashMap<u32, u32>>,
    pub metadata_usages_count: usize,

    field_default_values: HashMap<i32, Il2CppFieldDefaultValue>,
    param_default_values: HashMap<i32, Il2CppParameterDefaultValue>,
    attribute_type_ranges_dic: HashMap<usize, HashMap<u32, usize>>,
    string_cache: HashMap<i32, String>,
}

impl Metadata {
    pub fn new(data: Vec<u8>) -> Result<Self> {
        let mut stream = BinaryStream::new(data);
        stream.set_position(0);

        let sanity = stream.read_u32()?;
        if sanity != METADATA_MAGIC {
            return Err(Error::InvalidMetadata("Wrong magic number".into()));
        }

        let version_raw = stream.read_i32()?;
        if version_raw < 0 || version_raw > 1000 {
            return Err(Error::InvalidMetadata("Invalid version".into()));
        }
        if version_raw < 16 || version_raw > 31 {
            return Err(Error::UnsupportedVersion(version_raw));
        }

        let version = version_raw as f64;

        stream.set_position(0);
        let header = Il2CppGlobalMetadataHeader::read(&mut stream, version)?;

        let mut meta = Self {
            stream,
            version,
            header,
            image_defs: Vec::new(),
            assembly_defs: Vec::new(),
            type_defs: Vec::new(),
            method_defs: Vec::new(),
            parameter_defs: Vec::new(),
            field_defs: Vec::new(),
            property_defs: Vec::new(),
            event_defs: Vec::new(),
            generic_containers: Vec::new(),
            generic_parameters: Vec::new(),
            string_literals: Vec::new(),
            field_refs: Vec::new(),
            interface_indices: Vec::new(),
            nested_type_indices: Vec::new(),
            constraint_indices: Vec::new(),
            vtable_methods: Vec::new(),
            attribute_type_ranges: Vec::new(),
            attribute_types: Vec::new(),
            attribute_data_ranges: Vec::new(),
            metadata_usage_dic: HashMap::new(),
            metadata_usages_count: 0,
            field_default_values: HashMap::new(),
            param_default_values: HashMap::new(),
            attribute_type_ranges_dic: HashMap::new(),
            string_cache: HashMap::new(),
        };

        meta.detect_subversion()?;
        meta.load_metadata()?;
        Ok(meta)
    }

    fn detect_subversion(&mut self) -> Result<()> {
        if self.version == 24.0 {
            if self.header.string_literal_offset == 264 {
                self.version = 24.2;
                self.stream.set_position(0);
                self.header = Il2CppGlobalMetadataHeader::read(&mut self.stream, self.version)?;
            } else {
                let images = self.read_metadata_array::<Il2CppImageDefinition>(
                    self.header.images_offset as u64,
                    self.header.images_size as u64,
                )?;
                if images.iter().any(|img| img.token != 1) {
                    self.version = 24.1;
                }
            }

            if self.version != 24.0 {
                self.stream.set_position(0);
                self.header = Il2CppGlobalMetadataHeader::read(&mut self.stream, self.version)?;
            }
        }
        Ok(())
    }

    fn load_metadata(&mut self) -> Result<()> {
        let h = self.header.clone();

        self.image_defs = self.read_metadata_array::<Il2CppImageDefinition>(
            h.images_offset as u64,
            h.images_size as u64,
        )?;

        if self.version == 24.2 {
            let assembly_element_size = 68u64;
            if h.assemblies_size as u64 / assembly_element_size < self.image_defs.len() as u64 {
                self.version = 24.4;
            }
        }

        self.assembly_defs = self.read_metadata_array::<Il2CppAssemblyDefinition>(
            h.assemblies_offset as u64,
            h.assemblies_size as u64,
        )?;

        self.type_defs = self.read_metadata_array::<Il2CppTypeDefinition>(
            h.type_definitions_offset as u64,
            h.type_definitions_size as u64,
        )?;

        self.method_defs = self.read_metadata_array::<Il2CppMethodDefinition>(
            h.methods_offset as u64,
            h.methods_size as u64,
        )?;

        self.parameter_defs = self.read_metadata_array::<Il2CppParameterDefinition>(
            h.parameters_offset as u64,
            h.parameters_size as u64,
        )?;

        self.field_defs = self.read_metadata_array::<Il2CppFieldDefinition>(
            h.fields_offset as u64,
            h.fields_size as u64,
        )?;

        let field_defaults = self.read_metadata_array::<Il2CppFieldDefaultValue>(
            h.field_default_values_offset as u64,
            h.field_default_values_size as u64,
        )?;
        self.field_default_values = field_defaults.into_iter()
            .map(|v| (v.field_index, v))
            .collect();

        let param_defaults = self.read_metadata_array::<Il2CppParameterDefaultValue>(
            h.parameter_default_values_offset as u64,
            h.parameter_default_values_size as u64,
        )?;
        self.param_default_values = param_defaults.into_iter()
            .map(|v| (v.parameter_index, v))
            .collect();

        self.property_defs = self.read_metadata_array::<Il2CppPropertyDefinition>(
            h.properties_offset as u64,
            h.properties_size as u64,
        )?;

        self.interface_indices = self.stream.read_i32_array(
            h.interfaces_offset as u64,
            h.interfaces_size as usize / 4,
        )?;

        self.nested_type_indices = self.stream.read_i32_array(
            h.nested_types_offset as u64,
            h.nested_types_size as usize / 4,
        )?;

        self.event_defs = self.read_metadata_array::<Il2CppEventDefinition>(
            h.events_offset as u64,
            h.events_size as u64,
        )?;

        self.generic_containers = self.read_metadata_array::<Il2CppGenericContainer>(
            h.generic_containers_offset as u64,
            h.generic_containers_size as u64,
        )?;

        self.generic_parameters = self.read_metadata_array::<Il2CppGenericParameter>(
            h.generic_parameters_offset as u64,
            h.generic_parameters_size as u64,
        )?;

        self.constraint_indices = self.stream.read_i32_array(
            h.generic_parameter_constraints_offset as u64,
            h.generic_parameter_constraints_size as usize / 4,
        )?;

        self.vtable_methods = self.stream.read_u32_array(
            h.vtable_methods_offset as u64,
            h.vtable_methods_size as usize / 4,
        )?;

        self.string_literals = self.read_metadata_array::<Il2CppStringLiteral>(
            h.string_literal_offset as u64,
            h.string_literal_size as u64,
        )?;

        if self.version > 16.0 {
            self.field_refs = self.read_metadata_array::<Il2CppFieldRef>(
                h.field_refs_offset as u64,
                h.field_refs_size as u64,
            )?;

            if self.version < 27.0 {
                let usage_lists = self.read_metadata_array::<Il2CppMetadataUsageList>(
                    h.metadata_usage_lists_offset as u64,
                    h.metadata_usage_lists_count as u64,
                )?;
                let usage_pairs = self.read_metadata_array::<Il2CppMetadataUsagePair>(
                    h.metadata_usage_pairs_offset as u64,
                    h.metadata_usage_pairs_count as u64,
                )?;
                self.process_metadata_usage(&usage_lists, &usage_pairs);
            }
        }

        if self.version > 20.0 && self.version < 29.0 {
            self.attribute_type_ranges = self.read_metadata_array::<Il2CppCustomAttributeTypeRange>(
                h.attributes_info_offset as u64,
                h.attributes_info_count as u64,
            )?;
            self.attribute_types = self.stream.read_i32_array(
                h.attribute_types_offset as u64,
                h.attribute_types_count as usize / 4,
            )?;
        }

        if self.version >= 29.0 {
            self.attribute_data_ranges = self.read_metadata_array::<Il2CppCustomAttributeDataRange>(
                h.attribute_data_range_offset as u64,
                h.attribute_data_range_size as u64,
            )?;
        }

        if self.version > 24.0 {
            self.build_attribute_lookup();
        }

        self.metadata_usages_count = self.calculate_metadata_usages_count();

        Ok(())
    }

    fn read_metadata_array<T: MetadataReadable>(&mut self, offset: u64, size: u64) -> Result<Vec<T>> {
        if offset == 0 || size == 0 {
            return Ok(Vec::new());
        }
        let element_size = T::byte_size(self.version) as u64;
        if element_size == 0 {
            return Ok(Vec::new());
        }
        let count = (size / element_size) as usize;
        self.stream.set_position(offset);
        let mut items = Vec::with_capacity(count);
        for _ in 0..count {
            items.push(T::read(&mut self.stream, self.version)?);
        }
        Ok(items)
    }

    fn process_metadata_usage(
        &mut self,
        lists: &[Il2CppMetadataUsageList],
        pairs: &[Il2CppMetadataUsagePair],
    ) {
        for i in 1..=6u32 {
            self.metadata_usage_dic.insert(i, HashMap::new());
        }

        for list in lists {
            for i in 0..list.count as usize {
                let offset = list.start as usize + i;
                if offset >= pairs.len() {
                    continue;
                }
                let pair = &pairs[offset];
                let usage = (pair.encoded_source_index & 0xE0000000) >> 29;
                let decoded = if self.version >= 27.0 {
                    (pair.encoded_source_index & 0x1FFFFFFE) >> 1
                } else {
                    pair.encoded_source_index & 0x1FFFFFFF
                };

                if usage >= 1 && usage <= 6 {
                    if let Some(dic) = self.metadata_usage_dic.get_mut(&usage) {
                        dic.insert(pair.destination_index, decoded);
                    }
                }
            }
        }
    }

    fn calculate_metadata_usages_count(&self) -> usize {
        let mut max_index = 0u32;
        for dic in self.metadata_usage_dic.values() {
            if let Some(&m) = dic.keys().max() {
                if m > max_index {
                    max_index = m;
                }
            }
        }
        if max_index == 0 && self.metadata_usage_dic.values().all(|d| d.is_empty()) {
            0
        } else {
            max_index as usize + 1
        }
    }

    fn build_attribute_lookup(&mut self) {
        self.attribute_type_ranges_dic.clear();

        for (img_idx, image_def) in self.image_defs.iter().enumerate() {
            let mut dic = HashMap::new();
            let end = image_def.custom_attribute_start as usize + image_def.custom_attribute_count as usize;

            for i in image_def.custom_attribute_start as usize..end {
                if self.version >= 29.0 {
                    if let Some(range) = self.attribute_data_ranges.get(i) {
                        dic.insert(range.token, i);
                    }
                } else if let Some(range) = self.attribute_type_ranges.get(i) {
                    dic.insert(range.token, i);
                }
            }

            self.attribute_type_ranges_dic.insert(img_idx, dic);
        }
    }

    pub fn get_string_from_index(&mut self, index: i32) -> Result<String> {
        if let Some(cached) = self.string_cache.get(&index) {
            return Ok(cached.clone());
        }
        let offset = self.header.string_offset as u64 + index as u64;
        let result = self.stream.read_string_to_null_at(offset)?;
        self.string_cache.insert(index, result.clone());
        Ok(result)
    }

    pub fn get_string_literal_from_index(&mut self, index: usize) -> Result<String> {
        let sl = &self.string_literals[index];
        self.stream.set_position(self.header.string_literal_data_offset as u64 + sl.data_index as u64);
        let bytes = self.stream.read_bytes(sl.length as usize)?;
        Ok(String::from_utf8_lossy(&bytes).to_string())
    }

    pub fn get_field_default_value(&self, field_index: i32) -> Option<&Il2CppFieldDefaultValue> {
        self.field_default_values.get(&field_index)
    }

    pub fn get_parameter_default_value(&self, param_index: i32) -> Option<&Il2CppParameterDefaultValue> {
        self.param_default_values.get(&param_index)
    }

    pub fn get_default_value_offset(&self, index: i32) -> u64 {
        self.header.field_and_parameter_default_value_data_offset as u64 + index as u64
    }

    pub fn get_custom_attribute_index(
        &self,
        image_index: usize,
        _custom_attribute_index: i32,
        token: u32,
    ) -> Option<usize> {
        if self.version > 24.0 {
            self.attribute_type_ranges_dic
                .get(&image_index)
                .and_then(|dic| dic.get(&token))
                .copied()
        } else if _custom_attribute_index >= 0 {
            Some(_custom_attribute_index as usize)
        } else {
            None
        }
    }
}

pub trait MetadataReadable: Sized {
    fn read(stream: &mut BinaryStream, version: f64) -> Result<Self>;
    fn byte_size(version: f64) -> usize;
}

macro_rules! impl_metadata_readable {
    ($t:ty) => {
        impl MetadataReadable for $t {
            fn read(stream: &mut BinaryStream, version: f64) -> Result<Self> {
                <$t>::read(stream, version)
            }
            fn byte_size(version: f64) -> usize {
                <$t>::byte_size(version)
            }
        }
    };
}

macro_rules! impl_metadata_readable_simple {
    ($t:ty, $size:expr) => {
        impl MetadataReadable for $t {
            fn read(stream: &mut BinaryStream, _version: f64) -> Result<Self> {
                <$t>::read(stream)
            }
            fn byte_size(_version: f64) -> usize {
                $size
            }
        }
    };
}

impl_metadata_readable!(Il2CppImageDefinition);
impl_metadata_readable!(Il2CppAssemblyDefinition);
impl_metadata_readable!(Il2CppTypeDefinition);
impl_metadata_readable!(Il2CppMethodDefinition);
impl_metadata_readable!(Il2CppParameterDefinition);
impl_metadata_readable!(Il2CppFieldDefinition);
impl_metadata_readable_simple!(Il2CppFieldDefaultValue, 12);
impl_metadata_readable_simple!(Il2CppParameterDefaultValue, 12);
impl_metadata_readable!(Il2CppPropertyDefinition);
impl_metadata_readable!(Il2CppEventDefinition);
impl_metadata_readable_simple!(Il2CppGenericContainer, 16);
impl_metadata_readable_simple!(Il2CppGenericParameter, Il2CppGenericParameter::byte_size());
impl_metadata_readable_simple!(Il2CppStringLiteral, 8);
impl_metadata_readable_simple!(Il2CppFieldRef, 8);
impl_metadata_readable_simple!(Il2CppMetadataUsageList, 8);
impl_metadata_readable_simple!(Il2CppMetadataUsagePair, 8);
impl_metadata_readable!(Il2CppCustomAttributeTypeRange);
impl_metadata_readable_simple!(Il2CppCustomAttributeDataRange, 8);

