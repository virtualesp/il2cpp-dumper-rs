use crate::io::BinaryStream;
use crate::error::Result;
use crate::read_versioned;

#[derive(Debug, Clone, Default)]
pub struct Il2CppGlobalMetadataHeader {
    pub sanity: u32,
    pub version: i32,
    pub string_literal_offset: i32,
    pub string_literal_size: i32,
    pub string_literal_data_offset: i32,
    pub string_literal_data_size: i32,
    pub string_offset: i32,
    pub string_size: i32,
    pub events_offset: i32,
    pub events_size: i32,
    pub properties_offset: i32,
    pub properties_size: i32,
    pub methods_offset: i32,
    pub methods_size: i32,
    pub parameter_default_values_offset: i32,
    pub parameter_default_values_size: i32,
    pub field_default_values_offset: i32,
    pub field_default_values_size: i32,
    pub field_and_parameter_default_value_data_offset: i32,
    pub field_and_parameter_default_value_data_size: i32,
    pub field_marshaled_sizes_offset: i32,
    pub field_marshaled_sizes_size: i32,
    pub parameters_offset: i32,
    pub parameters_size: i32,
    pub fields_offset: i32,
    pub fields_size: i32,
    pub generic_parameters_offset: i32,
    pub generic_parameters_size: i32,
    pub generic_parameter_constraints_offset: i32,
    pub generic_parameter_constraints_size: i32,
    pub generic_containers_offset: i32,
    pub generic_containers_size: i32,
    pub nested_types_offset: i32,
    pub nested_types_size: i32,
    pub interfaces_offset: i32,
    pub interfaces_size: i32,
    pub vtable_methods_offset: i32,
    pub vtable_methods_size: i32,
    pub interface_offsets_offset: i32,
    pub interface_offsets_size: i32,
    pub type_definitions_offset: i32,
    pub type_definitions_size: i32,
    pub rgctx_entries_offset: i32,
    pub rgctx_entries_count: i32,
    pub images_offset: i32,
    pub images_size: i32,
    pub assemblies_offset: i32,
    pub assemblies_size: i32,
    pub metadata_usage_lists_offset: i32,
    pub metadata_usage_lists_count: i32,
    pub metadata_usage_pairs_offset: i32,
    pub metadata_usage_pairs_count: i32,
    pub field_refs_offset: i32,
    pub field_refs_size: i32,
    pub referenced_assemblies_offset: i32,
    pub referenced_assemblies_size: i32,
    pub attributes_info_offset: i32,
    pub attributes_info_count: i32,
    pub attribute_types_offset: i32,
    pub attribute_types_count: i32,
    pub attribute_data_offset: i32,
    pub attribute_data_size: i32,
    pub attribute_data_range_offset: i32,
    pub attribute_data_range_size: i32,
    pub unresolved_virtual_call_parameter_types_offset: i32,
    pub unresolved_virtual_call_parameter_types_size: i32,
    pub unresolved_virtual_call_parameter_ranges_offset: i32,
    pub unresolved_virtual_call_parameter_ranges_size: i32,
    pub windows_runtime_type_names_offset: i32,
    pub windows_runtime_type_names_size: i32,
    pub windows_runtime_strings_offset: i32,
    pub windows_runtime_strings_size: i32,
    pub exported_type_definitions_offset: i32,
    pub exported_type_definitions_size: i32,
}

impl Il2CppGlobalMetadataHeader {
    pub fn read(stream: &mut BinaryStream, version: f64) -> Result<Self> {
        Ok(Self {
            sanity: stream.read_u32()?,
            version: stream.read_i32()?,
            string_literal_offset: stream.read_i32()?,
            string_literal_size: stream.read_i32()?,
            string_literal_data_offset: stream.read_i32()?,
            string_literal_data_size: stream.read_i32()?,
            string_offset: stream.read_i32()?,
            string_size: stream.read_i32()?,
            events_offset: stream.read_i32()?,
            events_size: stream.read_i32()?,
            properties_offset: stream.read_i32()?,
            properties_size: stream.read_i32()?,
            methods_offset: stream.read_i32()?,
            methods_size: stream.read_i32()?,
            parameter_default_values_offset: stream.read_i32()?,
            parameter_default_values_size: stream.read_i32()?,
            field_default_values_offset: stream.read_i32()?,
            field_default_values_size: stream.read_i32()?,
            field_and_parameter_default_value_data_offset: stream.read_i32()?,
            field_and_parameter_default_value_data_size: stream.read_i32()?,
            field_marshaled_sizes_offset: stream.read_i32()?,
            field_marshaled_sizes_size: stream.read_i32()?,
            parameters_offset: stream.read_i32()?,
            parameters_size: stream.read_i32()?,
            fields_offset: stream.read_i32()?,
            fields_size: stream.read_i32()?,
            generic_parameters_offset: stream.read_i32()?,
            generic_parameters_size: stream.read_i32()?,
            generic_parameter_constraints_offset: stream.read_i32()?,
            generic_parameter_constraints_size: stream.read_i32()?,
            generic_containers_offset: stream.read_i32()?,
            generic_containers_size: stream.read_i32()?,
            nested_types_offset: stream.read_i32()?,
            nested_types_size: stream.read_i32()?,
            interfaces_offset: stream.read_i32()?,
            interfaces_size: stream.read_i32()?,
            vtable_methods_offset: stream.read_i32()?,
            vtable_methods_size: stream.read_i32()?,
            interface_offsets_offset: stream.read_i32()?,
            interface_offsets_size: stream.read_i32()?,
            type_definitions_offset: stream.read_i32()?,
            type_definitions_size: stream.read_i32()?,
            rgctx_entries_offset: read_versioned!(stream, version, 0.0, 24.1, read_i32, 0),
            rgctx_entries_count: read_versioned!(stream, version, 0.0, 24.1, read_i32, 0),
            images_offset: stream.read_i32()?,
            images_size: stream.read_i32()?,
            assemblies_offset: stream.read_i32()?,
            assemblies_size: stream.read_i32()?,
            metadata_usage_lists_offset: read_versioned!(stream, version, 19.0, 24.5, read_i32, 0),
            metadata_usage_lists_count: read_versioned!(stream, version, 19.0, 24.5, read_i32, 0),
            metadata_usage_pairs_offset: read_versioned!(stream, version, 19.0, 24.5, read_i32, 0),
            metadata_usage_pairs_count: read_versioned!(stream, version, 19.0, 24.5, read_i32, 0),
            field_refs_offset: read_versioned!(stream, version, 19.0, 99.0, read_i32, 0),
            field_refs_size: read_versioned!(stream, version, 19.0, 99.0, read_i32, 0),
            referenced_assemblies_offset: read_versioned!(stream, version, 20.0, 99.0, read_i32, 0),
            referenced_assemblies_size: read_versioned!(stream, version, 20.0, 99.0, read_i32, 0),
            attributes_info_offset: read_versioned!(stream, version, 21.0, 27.2, read_i32, 0),
            attributes_info_count: read_versioned!(stream, version, 21.0, 27.2, read_i32, 0),
            attribute_types_offset: read_versioned!(stream, version, 21.0, 27.2, read_i32, 0),
            attribute_types_count: read_versioned!(stream, version, 21.0, 27.2, read_i32, 0),
            attribute_data_offset: read_versioned!(stream, version, 29.0, 99.0, read_i32, 0),
            attribute_data_size: read_versioned!(stream, version, 29.0, 99.0, read_i32, 0),
            attribute_data_range_offset: read_versioned!(stream, version, 29.0, 99.0, read_i32, 0),
            attribute_data_range_size: read_versioned!(stream, version, 29.0, 99.0, read_i32, 0),
            unresolved_virtual_call_parameter_types_offset: read_versioned!(stream, version, 22.0, 99.0, read_i32, 0),
            unresolved_virtual_call_parameter_types_size: read_versioned!(stream, version, 22.0, 99.0, read_i32, 0),
            unresolved_virtual_call_parameter_ranges_offset: read_versioned!(stream, version, 22.0, 99.0, read_i32, 0),
            unresolved_virtual_call_parameter_ranges_size: read_versioned!(stream, version, 22.0, 99.0, read_i32, 0),
            windows_runtime_type_names_offset: read_versioned!(stream, version, 23.0, 99.0, read_i32, 0),
            windows_runtime_type_names_size: read_versioned!(stream, version, 23.0, 99.0, read_i32, 0),
            windows_runtime_strings_offset: read_versioned!(stream, version, 27.0, 99.0, read_i32, 0),
            windows_runtime_strings_size: read_versioned!(stream, version, 27.0, 99.0, read_i32, 0),
            exported_type_definitions_offset: read_versioned!(stream, version, 24.0, 99.0, read_i32, 0),
            exported_type_definitions_size: read_versioned!(stream, version, 24.0, 99.0, read_i32, 0),
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct Il2CppImageDefinition {
    pub name_index: i32,
    pub assembly_index: i32,
    pub type_start: i32,
    pub type_count: i32,
    pub exported_type_start: i32,
    pub exported_type_count: i32,
    pub entry_point_index: i32,
    pub token: u32,
    pub custom_attribute_start: i32,
    pub custom_attribute_count: i32,
}

impl Il2CppImageDefinition {
    pub fn read(stream: &mut BinaryStream, version: f64) -> Result<Self> {
        Ok(Self {
            name_index: stream.read_i32()?,
            assembly_index: stream.read_i32()?,
            type_start: stream.read_i32()?,
            type_count: stream.read_i32()?,
            exported_type_start: read_versioned!(stream, version, 24.0, 99.0, read_i32, 0),
            exported_type_count: read_versioned!(stream, version, 24.0, 99.0, read_i32, 0),
            entry_point_index: stream.read_i32()?,
            token: read_versioned!(stream, version, 19.0, 99.0, read_u32, 0),
            custom_attribute_start: read_versioned!(stream, version, 24.1, 99.0, read_i32, 0),
            custom_attribute_count: read_versioned!(stream, version, 24.1, 99.0, read_i32, 0),
        })
    }

    pub fn byte_size(version: f64) -> usize {
        let mut size = 4 * 5; // name_index, assembly_index, type_start, type_count, entry_point_index
        if version >= 24.0 { size += 8; } // exported_type_start, exported_type_count
        if version >= 19.0 { size += 4; } // token
        if version >= 24.1 { size += 8; } // custom_attribute_start, custom_attribute_count
        size
    }
}

#[derive(Debug, Clone, Default)]
pub struct Il2CppAssemblyNameDefinition {
    pub name_index: i32,
    pub culture_index: i32,
    pub hash_value_index: i32,
    pub public_key_index: i32,
    pub hash_alg: i32,
    pub hash_len: i32,
    pub flags: u32,
    pub major: i32,
    pub minor: i32,
    pub build: i32,
    pub revision: i32,
    pub public_key_token: [u8; 8],
}

impl Il2CppAssemblyNameDefinition {
    pub fn read(stream: &mut BinaryStream, version: f64) -> Result<Self> {
        let name_index = stream.read_i32()?;
        let culture_index = stream.read_i32()?;
        let hash_value_index = read_versioned!(stream, version, 0.0, 24.3, read_i32, 0);
        let public_key_index = stream.read_i32()?;
        let hash_alg = stream.read_i32()?;
        let hash_len = stream.read_i32()?;
        let flags = stream.read_u32()?;
        let major = stream.read_i32()?;
        let minor = stream.read_i32()?;
        let build = stream.read_i32()?;
        let revision = stream.read_i32()?;
        let token_bytes = stream.read_bytes(8)?;
        let mut public_key_token = [0u8; 8];
        public_key_token.copy_from_slice(&token_bytes);
        Ok(Self {
            name_index,
            culture_index,
            hash_value_index,
            public_key_index,
            hash_alg,
            hash_len,
            flags,
            major,
            minor,
            build,
            revision,
            public_key_token,
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct Il2CppAssemblyDefinition {
    pub image_index: i32,
    pub token: u32,
    pub custom_attribute_index: i32,
    pub referenced_assembly_start: i32,
    pub referenced_assembly_count: i32,
    pub aname: Il2CppAssemblyNameDefinition,
}

impl Il2CppAssemblyDefinition {
    pub fn read(stream: &mut BinaryStream, version: f64) -> Result<Self> {
        let image_index = stream.read_i32()?;
        let token = read_versioned!(stream, version, 24.1, 99.0, read_u32, 0);
        let custom_attribute_index = read_versioned!(stream, version, 0.0, 24.0, read_i32, 0);
        let referenced_assembly_start = read_versioned!(stream, version, 20.0, 99.0, read_i32, 0);
        let referenced_assembly_count = read_versioned!(stream, version, 20.0, 99.0, read_i32, 0);
        let aname = Il2CppAssemblyNameDefinition::read(stream, version)?;
        Ok(Self {
            image_index,
            token,
            custom_attribute_index,
            referenced_assembly_start,
            referenced_assembly_count,
            aname,
        })
    }

    pub fn byte_size(version: f64) -> usize {
        let mut size = 4; // image_index
        if version >= 24.1 { size += 4; } // token
        if version <= 24.0 { size += 4; } // custom_attribute_index
        if version >= 20.0 { size += 4 + 4; } // referenced_assembly_start + count
        // aname: Il2CppAssemblyNameDefinition
        size += 4 * 10 + 8; // 10 i32/u32 fields + 8 bytes public_key_token
        if version <= 24.3 { size += 4; } // hash_value_index
        size
    }
}

#[derive(Debug, Clone, Default)]
pub struct Il2CppTypeDefinition {
    pub name_index: i32,
    pub namespace_index: i32,
    pub custom_attribute_index: i32,
    pub byval_type_index: i32,
    pub byref_type_index: i32,
    pub declaring_type_index: i32,
    pub parent_index: i32,
    pub element_type_index: i32,
    pub rgctx_start_index: i32,
    pub rgctx_count: i32,
    pub generic_container_index: i32,
    pub delegate_wrapper_from_managed_to_native_index: i32,
    pub marshaling_functions_index: i32,
    pub ccw_function_index: i32,
    pub guid_index: i32,
    pub flags: u32,
    pub field_start: i32,
    pub method_start: i32,
    pub event_start: i32,
    pub property_start: i32,
    pub nested_types_start: i32,
    pub interfaces_start: i32,
    pub vtable_start: i32,
    pub interface_offsets_start: i32,
    pub method_count: u16,
    pub property_count: u16,
    pub field_count: u16,
    pub event_count: u16,
    pub nested_type_count: u16,
    pub vtable_count: u16,
    pub interfaces_count: u16,
    pub interface_offsets_count: u16,
    pub bitfield: u32,
    pub token: u32,
}

impl Il2CppTypeDefinition {
    pub fn read(stream: &mut BinaryStream, version: f64) -> Result<Self> {
        Ok(Self {
            name_index: stream.read_i32()?,
            namespace_index: stream.read_i32()?,
            custom_attribute_index: read_versioned!(stream, version, 0.0, 24.0, read_i32, 0),
            byval_type_index: stream.read_i32()?,
            byref_type_index: read_versioned!(stream, version, 0.0, 24.5, read_i32, 0),
            declaring_type_index: stream.read_i32()?,
            parent_index: stream.read_i32()?,
            element_type_index: stream.read_i32()?,
            rgctx_start_index: read_versioned!(stream, version, 0.0, 24.1, read_i32, 0),
            rgctx_count: read_versioned!(stream, version, 0.0, 24.1, read_i32, 0),
            generic_container_index: stream.read_i32()?,
            delegate_wrapper_from_managed_to_native_index: read_versioned!(stream, version, 0.0, 22.0, read_i32, 0),
            marshaling_functions_index: read_versioned!(stream, version, 0.0, 22.0, read_i32, 0),
            ccw_function_index: read_versioned!(stream, version, 21.0, 22.0, read_i32, 0),
            guid_index: read_versioned!(stream, version, 21.0, 22.0, read_i32, 0),
            flags: stream.read_u32()?,
            field_start: stream.read_i32()?,
            method_start: stream.read_i32()?,
            event_start: stream.read_i32()?,
            property_start: stream.read_i32()?,
            nested_types_start: stream.read_i32()?,
            interfaces_start: stream.read_i32()?,
            vtable_start: stream.read_i32()?,
            interface_offsets_start: stream.read_i32()?,
            method_count: stream.read_u16()?,
            property_count: stream.read_u16()?,
            field_count: stream.read_u16()?,
            event_count: stream.read_u16()?,
            nested_type_count: stream.read_u16()?,
            vtable_count: stream.read_u16()?,
            interfaces_count: stream.read_u16()?,
            interface_offsets_count: stream.read_u16()?,
            bitfield: stream.read_u32()?,
            token: read_versioned!(stream, version, 19.0, 99.0, read_u32, 0),
        })
    }

    pub fn is_value_type(&self) -> bool {
        (self.bitfield & 0x1) == 1
    }

    pub fn is_enum(&self) -> bool {
        ((self.bitfield >> 1) & 0x1) == 1
    }

    pub fn byte_size(version: f64) -> usize {
        let mut size = 0usize;
        size += 4; // name_index
        size += 4; // namespace_index
        if version <= 24.0 { size += 4; } // custom_attribute_index
        size += 4; // byval_type_index
        if version <= 24.5 { size += 4; } // byref_type_index
        size += 4; // declaring_type_index
        size += 4; // parent_index
        size += 4; // element_type_index
        if version <= 24.1 { size += 8; } // rgctx_start_index, rgctx_count
        size += 4; // generic_container_index
        if version <= 22.0 { size += 8; } // delegate_wrapper, marshaling
        if version >= 21.0 && version <= 22.0 { size += 8; } // ccw, guid
        size += 4; // flags
        size += 4 * 8; // field_start..interface_offsets_start
        size += 2 * 8; // method_count..interface_offsets_count
        size += 4; // bitfield
        if version >= 19.0 { size += 4; } // token
        size
    }
}

#[derive(Debug, Clone, Default)]
pub struct Il2CppMethodDefinition {
    pub name_index: u32,
    pub declaring_type: i32,
    pub return_type: i32,
    pub return_parameter_token: i32,
    pub parameter_start: i32,
    pub custom_attribute_index: i32,
    pub generic_container_index: i32,
    pub method_index: i32,
    pub invoker_index: i32,
    pub delegate_wrapper_index: i32,
    pub rgctx_start_index: i32,
    pub rgctx_count: i32,
    pub token: u32,
    pub flags: u16,
    pub iflags: u16,
    pub slot: u16,
    pub parameter_count: u16,
}

impl Il2CppMethodDefinition {
    pub fn read(stream: &mut BinaryStream, version: f64) -> Result<Self> {
        Ok(Self {
            name_index: stream.read_u32()?,
            declaring_type: stream.read_i32()?,
            return_type: stream.read_i32()?,
            return_parameter_token: read_versioned!(stream, version, 31.0, 99.0, read_i32, 0),
            parameter_start: stream.read_i32()?,
            custom_attribute_index: read_versioned!(stream, version, 0.0, 24.0, read_i32, 0),
            generic_container_index: stream.read_i32()?,
            method_index: read_versioned!(stream, version, 0.0, 24.1, read_i32, 0),
            invoker_index: read_versioned!(stream, version, 0.0, 24.1, read_i32, 0),
            delegate_wrapper_index: read_versioned!(stream, version, 0.0, 24.1, read_i32, 0),
            rgctx_start_index: read_versioned!(stream, version, 0.0, 24.1, read_i32, 0),
            rgctx_count: read_versioned!(stream, version, 0.0, 24.1, read_i32, 0),
            token: stream.read_u32()?,
            flags: stream.read_u16()?,
            iflags: stream.read_u16()?,
            slot: stream.read_u16()?,
            parameter_count: stream.read_u16()?,
        })
    }

    pub fn byte_size(version: f64) -> usize {
        let mut size = 4 + 4 + 4; // name_index, declaring_type, return_type
        if version >= 31.0 { size += 4; } // return_parameter_token
        size += 4; // parameter_start
        if version <= 24.0 { size += 4; } // custom_attribute_index
        size += 4; // generic_container_index
        if version <= 24.1 { size += 4 * 5; } // method_index..rgctx_count
        size += 4; // token
        size += 2 * 4; // flags, iflags, slot, parameter_count
        size
    }
}

#[derive(Debug, Clone, Default)]
pub struct Il2CppParameterDefinition {
    pub name_index: i32,
    pub token: u32,
    pub custom_attribute_index: i32,
    pub type_index: i32,
}

impl Il2CppParameterDefinition {
    pub fn read(stream: &mut BinaryStream, version: f64) -> Result<Self> {
        Ok(Self {
            name_index: stream.read_i32()?,
            token: stream.read_u32()?,
            custom_attribute_index: read_versioned!(stream, version, 0.0, 24.0, read_i32, 0),
            type_index: stream.read_i32()?,
        })
    }

    pub fn byte_size(version: f64) -> usize {
        let mut size = 4 + 4 + 4; // name_index, token, type_index
        if version <= 24.0 { size += 4; } // custom_attribute_index
        size
    }
}

#[derive(Debug, Clone, Default)]
pub struct Il2CppFieldDefinition {
    pub name_index: i32,
    pub type_index: i32,
    pub custom_attribute_index: i32,
    pub token: u32,
}

impl Il2CppFieldDefinition {
    pub fn read(stream: &mut BinaryStream, version: f64) -> Result<Self> {
        Ok(Self {
            name_index: stream.read_i32()?,
            type_index: stream.read_i32()?,
            custom_attribute_index: read_versioned!(stream, version, 0.0, 24.0, read_i32, 0),
            token: read_versioned!(stream, version, 19.0, 99.0, read_u32, 0),
        })
    }

    pub fn byte_size(version: f64) -> usize {
        let mut size = 4 + 4; // name_index, type_index
        if version <= 24.0 { size += 4; }
        if version >= 19.0 { size += 4; }
        size
    }
}

#[derive(Debug, Clone, Default)]
pub struct Il2CppFieldDefaultValue {
    pub field_index: i32,
    pub type_index: i32,
    pub data_index: i32,
}

impl Il2CppFieldDefaultValue {
    pub fn read(stream: &mut BinaryStream) -> Result<Self> {
        Ok(Self {
            field_index: stream.read_i32()?,
            type_index: stream.read_i32()?,
            data_index: stream.read_i32()?,
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct Il2CppParameterDefaultValue {
    pub parameter_index: i32,
    pub type_index: i32,
    pub data_index: i32,
}

impl Il2CppParameterDefaultValue {
    pub fn read(stream: &mut BinaryStream) -> Result<Self> {
        Ok(Self {
            parameter_index: stream.read_i32()?,
            type_index: stream.read_i32()?,
            data_index: stream.read_i32()?,
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct Il2CppPropertyDefinition {
    pub name_index: i32,
    pub get: i32,
    pub set: i32,
    pub attrs: u32,
    pub custom_attribute_index: i32,
    pub token: u32,
}

impl Il2CppPropertyDefinition {
    pub fn read(stream: &mut BinaryStream, version: f64) -> Result<Self> {
        Ok(Self {
            name_index: stream.read_i32()?,
            get: stream.read_i32()?,
            set: stream.read_i32()?,
            attrs: stream.read_u32()?,
            custom_attribute_index: read_versioned!(stream, version, 0.0, 24.0, read_i32, 0),
            token: read_versioned!(stream, version, 19.0, 99.0, read_u32, 0),
        })
    }

    pub fn byte_size(version: f64) -> usize {
        let mut size = 4 * 4; // name_index, get, set, attrs
        if version <= 24.0 { size += 4; }
        if version >= 19.0 { size += 4; }
        size
    }
}

#[derive(Debug, Clone, Default)]
pub struct Il2CppEventDefinition {
    pub name_index: i32,
    pub type_index: i32,
    pub add: i32,
    pub remove: i32,
    pub raise: i32,
    pub custom_attribute_index: i32,
    pub token: u32,
}

impl Il2CppEventDefinition {
    pub fn read(stream: &mut BinaryStream, version: f64) -> Result<Self> {
        Ok(Self {
            name_index: stream.read_i32()?,
            type_index: stream.read_i32()?,
            add: stream.read_i32()?,
            remove: stream.read_i32()?,
            raise: stream.read_i32()?,
            custom_attribute_index: read_versioned!(stream, version, 0.0, 24.0, read_i32, 0),
            token: read_versioned!(stream, version, 19.0, 99.0, read_u32, 0),
        })
    }

    pub fn byte_size(version: f64) -> usize {
        let mut size = 4 * 5;
        if version <= 24.0 { size += 4; }
        if version >= 19.0 { size += 4; }
        size
    }
}

#[derive(Debug, Clone, Default)]
pub struct Il2CppGenericContainer {
    pub owner_index: i32,
    pub type_argc: i32,
    pub is_method: i32,
    pub generic_parameter_start: i32,
}

impl Il2CppGenericContainer {
    pub fn read(stream: &mut BinaryStream) -> Result<Self> {
        Ok(Self {
            owner_index: stream.read_i32()?,
            type_argc: stream.read_i32()?,
            is_method: stream.read_i32()?,
            generic_parameter_start: stream.read_i32()?,
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct Il2CppGenericParameter {
    pub owner_index: i32,
    pub name_index: u32,
    pub constraints_start: i16,
    pub constraints_count: i16,
    pub num: u16,
    pub flags: u16,
}

impl Il2CppGenericParameter {
    pub fn read(stream: &mut BinaryStream) -> Result<Self> {
        Ok(Self {
            owner_index: stream.read_i32()?,
            name_index: stream.read_u32()?,
            constraints_start: stream.read_i16()?,
            constraints_count: stream.read_i16()?,
            num: stream.read_u16()?,
            flags: stream.read_u16()?,
        })
    }

    pub fn byte_size() -> usize {
        4 + 4 + 2 + 2 + 2 + 2
    }
}

#[derive(Debug, Clone, Default)]
pub struct Il2CppCustomAttributeTypeRange {
    pub token: u32,
    pub start: i32,
    pub count: i32,
}

impl Il2CppCustomAttributeTypeRange {
    pub fn read(stream: &mut BinaryStream, version: f64) -> Result<Self> {
        Ok(Self {
            token: read_versioned!(stream, version, 24.1, 99.0, read_u32, 0),
            start: stream.read_i32()?,
            count: stream.read_i32()?,
        })
    }

    pub fn byte_size(version: f64) -> usize {
        let mut size = 8; // start, count
        if version >= 24.1 { size += 4; } // token
        size
    }
}

#[derive(Debug, Clone, Default)]
pub struct Il2CppCustomAttributeDataRange {
    pub token: u32,
    pub start_offset: u32,
}

impl Il2CppCustomAttributeDataRange {
    pub fn read(stream: &mut BinaryStream) -> Result<Self> {
        Ok(Self {
            token: stream.read_u32()?,
            start_offset: stream.read_u32()?,
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct Il2CppMetadataUsageList {
    pub start: u32,
    pub count: u32,
}

impl Il2CppMetadataUsageList {
    pub fn read(stream: &mut BinaryStream) -> Result<Self> {
        Ok(Self {
            start: stream.read_u32()?,
            count: stream.read_u32()?,
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct Il2CppMetadataUsagePair {
    pub destination_index: u32,
    pub encoded_source_index: u32,
}

impl Il2CppMetadataUsagePair {
    pub fn read(stream: &mut BinaryStream) -> Result<Self> {
        Ok(Self {
            destination_index: stream.read_u32()?,
            encoded_source_index: stream.read_u32()?,
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct Il2CppStringLiteral {
    pub length: u32,
    pub data_index: u32,
}

impl Il2CppStringLiteral {
    pub fn read(stream: &mut BinaryStream) -> Result<Self> {
        Ok(Self {
            length: stream.read_u32()?,
            data_index: stream.read_u32()?,
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct Il2CppFieldRef {
    pub type_index: i32,
    pub field_index: i32,
}

impl Il2CppFieldRef {
    pub fn read(stream: &mut BinaryStream) -> Result<Self> {
        Ok(Self {
            type_index: stream.read_i32()?,
            field_index: stream.read_i32()?,
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct Il2CppRGCTXDefinitionData {
    pub rgctx_data_dummy: i32,
}

impl Il2CppRGCTXDefinitionData {
    pub fn read(stream: &mut BinaryStream) -> Result<Self> {
        Ok(Self {
            rgctx_data_dummy: stream.read_i32()?,
        })
    }

    pub fn method_index(&self) -> i32 {
        self.rgctx_data_dummy
    }

    pub fn type_index(&self) -> i32 {
        self.rgctx_data_dummy
    }
}

#[derive(Debug, Clone, Default)]
pub struct Il2CppRGCTXDefinition {
    pub type_pre29: i32,
    pub type_post29: i32,
    pub data: Option<Il2CppRGCTXDefinitionData>,
    pub data_ptr: i32,
}

impl Il2CppRGCTXDefinition {
    pub fn read(stream: &mut BinaryStream, version: f64) -> Result<Self> {
        let type_pre29 = read_versioned!(stream, version, 0.0, 27.1, read_i32, 0);
        let type_post29 = read_versioned!(stream, version, 29.0, 99.0, read_i32, 0);
        let data = if version <= 27.1 {
            Some(Il2CppRGCTXDefinitionData::read(stream)?)
        } else {
            None
        };
        let data_ptr = read_versioned!(stream, version, 27.2, 99.0, read_i32, 0);
        Ok(Self {
            type_pre29,
            type_post29,
            data,
            data_ptr,
        })
    }

    pub fn rgctx_type(&self) -> i32 {
        if self.type_post29 != 0 {
            self.type_post29
        } else {
            self.type_pre29
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Il2CppCodeRegistration {
    pub method_pointers_count: u64,
    pub method_pointers: u64,
    pub delegate_wrappers_from_native_to_managed_count: u64,
    pub delegate_wrappers_from_native_to_managed: u64,
    pub reverse_pinvoke_wrapper_count: u64,
    pub reverse_pinvoke_wrappers: u64,
    pub delegate_wrappers_from_managed_to_native_count: u64,
    pub delegate_wrappers_from_managed_to_native: u64,
    pub marshaling_functions_count: u64,
    pub marshaling_functions: u64,
    pub ccw_marshaling_functions_count: u64,
    pub ccw_marshaling_functions: u64,
    pub generic_method_pointers_count: u64,
    pub generic_method_pointers: u64,
    pub generic_adjustor_thunks: u64,
    pub invoker_pointers_count: u64,
    pub invoker_pointers: u64,
    pub custom_attribute_count: u64,
    pub custom_attribute_generators: u64,
    pub guid_count: u64,
    pub guids: u64,
    pub unresolved_virtual_call_count: u64,
    pub unresolved_virtual_call_pointers: u64,
    pub unresolved_instance_call_pointers: u64,
    pub unresolved_static_call_pointers: u64,
    pub interop_data_count: u64,
    pub interop_data: u64,
    pub windows_runtime_factory_count: u64,
    pub windows_runtime_factory_table: u64,
    pub code_gen_modules_count: u64,
    pub code_gen_modules: u64,
}

impl Il2CppCodeRegistration {
    pub fn read(stream: &mut BinaryStream, version: f64) -> Result<Self> {
        Ok(Self {
            method_pointers_count: if version <= 24.1 { stream.read_ptr()? } else { 0 },
            method_pointers: if version <= 24.1 { stream.read_ptr()? } else { 0 },
            delegate_wrappers_from_native_to_managed_count: if version <= 21.0 { stream.read_ptr()? } else { 0 },
            delegate_wrappers_from_native_to_managed: if version <= 21.0 { stream.read_ptr()? } else { 0 },
            reverse_pinvoke_wrapper_count: if version >= 22.0 { stream.read_ptr()? } else { 0 },
            reverse_pinvoke_wrappers: if version >= 22.0 { stream.read_ptr()? } else { 0 },
            delegate_wrappers_from_managed_to_native_count: if version <= 22.0 { stream.read_ptr()? } else { 0 },
            delegate_wrappers_from_managed_to_native: if version <= 22.0 { stream.read_ptr()? } else { 0 },
            marshaling_functions_count: if version <= 22.0 { stream.read_ptr()? } else { 0 },
            marshaling_functions: if version <= 22.0 { stream.read_ptr()? } else { 0 },
            ccw_marshaling_functions_count: if version >= 21.0 && version <= 22.0 { stream.read_ptr()? } else { 0 },
            ccw_marshaling_functions: if version >= 21.0 && version <= 22.0 { stream.read_ptr()? } else { 0 },
            generic_method_pointers_count: stream.read_ptr()?,
            generic_method_pointers: stream.read_ptr()?,
            generic_adjustor_thunks: if version >= 24.5 { stream.read_ptr()? } else { 0 },
            invoker_pointers_count: stream.read_ptr()?,
            invoker_pointers: stream.read_ptr()?,
            custom_attribute_count: if version <= 24.5 { stream.read_ptr()? } else { 0 },
            custom_attribute_generators: if version <= 24.5 { stream.read_ptr()? } else { 0 },
            guid_count: if version >= 21.0 && version <= 22.0 { stream.read_ptr()? } else { 0 },
            guids: if version >= 21.0 && version <= 22.0 { stream.read_ptr()? } else { 0 },
            unresolved_virtual_call_count: if version >= 22.0 { stream.read_ptr()? } else { 0 },
            unresolved_virtual_call_pointers: if version >= 22.0 { stream.read_ptr()? } else { 0 },
            unresolved_instance_call_pointers: if version >= 29.1 { stream.read_ptr()? } else { 0 },
            unresolved_static_call_pointers: if version >= 29.1 { stream.read_ptr()? } else { 0 },
            interop_data_count: if version >= 23.0 { stream.read_ptr()? } else { 0 },
            interop_data: if version >= 23.0 { stream.read_ptr()? } else { 0 },
            windows_runtime_factory_count: if version >= 24.3 { stream.read_ptr()? } else { 0 },
            windows_runtime_factory_table: if version >= 24.3 { stream.read_ptr()? } else { 0 },
            code_gen_modules_count: if version >= 24.2 { stream.read_ptr()? } else { 0 },
            code_gen_modules: if version >= 24.2 { stream.read_ptr()? } else { 0 },
        })
    }

    pub fn field_count(version: f64) -> usize {
        let mut count = 0usize;
        if version <= 24.1 { count += 2; }
        if version <= 21.0 { count += 2; }
        if version >= 22.0 { count += 2; }
        if version <= 22.0 { count += 4; }
        if version >= 21.0 && version <= 22.0 { count += 2; }
        count += 4; // generic_method_pointers_count, generic_method_pointers, invoker_pointers_count, invoker_pointers
        if version >= 24.5 { count += 1; }
        if version <= 24.5 { count += 2; }
        if version >= 21.0 && version <= 22.0 { count += 2; }
        if version >= 22.0 { count += 2; }
        if version >= 29.1 { count += 2; }
        if version >= 23.0 { count += 2; }
        if version >= 24.3 { count += 2; }
        if version >= 24.2 { count += 2; }
        count
    }
}

#[derive(Debug, Clone, Default)]
pub struct Il2CppMetadataRegistration {
    pub generic_classes_count: u64,
    pub generic_classes: u64,
    pub generic_insts_count: u64,
    pub generic_insts: u64,
    pub generic_method_table_count: u64,
    pub generic_method_table: u64,
    pub types_count: u64,
    pub types: u64,
    pub method_specs_count: u64,
    pub method_specs: u64,
    pub method_references_count: u64,
    pub method_references: u64,
    pub field_offsets_count: u64,
    pub field_offsets: u64,
    pub type_definitions_sizes_count: u64,
    pub type_definitions_sizes: u64,
    pub metadata_usages_count: u64,
    pub metadata_usages: u64,
}

impl Il2CppMetadataRegistration {
    pub fn read(stream: &mut BinaryStream, version: f64) -> Result<Self> {
        Ok(Self {
            generic_classes_count: stream.read_ptr()?,
            generic_classes: stream.read_ptr()?,
            generic_insts_count: stream.read_ptr()?,
            generic_insts: stream.read_ptr()?,
            generic_method_table_count: stream.read_ptr()?,
            generic_method_table: stream.read_ptr()?,
            types_count: stream.read_ptr()?,
            types: stream.read_ptr()?,
            method_specs_count: stream.read_ptr()?,
            method_specs: stream.read_ptr()?,
            method_references_count: if version <= 16.0 { stream.read_ptr()? } else { 0 },
            method_references: if version <= 16.0 { stream.read_ptr()? } else { 0 },
            field_offsets_count: stream.read_ptr()?,
            field_offsets: stream.read_ptr()?,
            type_definitions_sizes_count: stream.read_ptr()?,
            type_definitions_sizes: stream.read_ptr()?,
            metadata_usages_count: if version >= 19.0 { stream.read_ptr()? } else { 0 },
            metadata_usages: if version >= 19.0 { stream.read_ptr()? } else { 0 },
        })
    }

    pub fn field_count(version: f64) -> usize {
        let mut count = 16usize; // 8 pairs of count+pointer
        if version <= 16.0 { count += 2; }
        if version >= 19.0 { count += 2; }
        count
    }
}

#[derive(Debug, Clone, Default)]
pub struct Il2CppType {
    pub datapoint: u64,
    pub bits: u32,
    pub attrs: u32,
    pub type_enum: u8,
    pub num_mods: u8,
    pub byref: u8,
    pub pinned: u8,
    pub valuetype: u8,
}

impl Il2CppType {
    pub fn read(stream: &mut BinaryStream) -> Result<Self> {
        let datapoint = stream.read_ptr()?;
        let bits = stream.read_u32()?;
        let mut t = Self {
            datapoint,
            bits,
            ..Default::default()
        };
        t.attrs = bits & 0xFFFF;
        t.type_enum = ((bits >> 16) & 0xFF) as u8;
        Ok(t)
    }

    pub fn init(&mut self, version: f64) {
        self.attrs = self.bits & 0xFFFF;
        self.type_enum = ((self.bits >> 16) & 0xFF) as u8;
        if version >= 27.2 {
            self.num_mods = ((self.bits >> 24) & 0x1F) as u8;
            self.byref = ((self.bits >> 29) & 1) as u8;
            self.pinned = ((self.bits >> 30) & 1) as u8;
            self.valuetype = (self.bits >> 31) as u8;
        } else {
            self.num_mods = ((self.bits >> 24) & 0x3F) as u8;
            self.byref = ((self.bits >> 30) & 1) as u8;
            self.pinned = (self.bits >> 31) as u8;
        }
    }

    pub fn klass_index(&self) -> u64 {
        self.datapoint
    }

    pub fn type_handle(&self) -> u64 {
        self.datapoint
    }

    pub fn type_ptr(&self) -> u64 {
        self.datapoint
    }

    pub fn array(&self) -> u64 {
        self.datapoint
    }

    pub fn generic_parameter_index(&self) -> u64 {
        self.datapoint
    }

    pub fn generic_parameter_handle(&self) -> u64 {
        self.datapoint
    }

    pub fn generic_class(&self) -> u64 {
        self.datapoint
    }
}

#[derive(Debug, Clone, Default)]
pub struct Il2CppGenericClass {
    pub type_definition_index: u64,
    pub type_ptr: u64,
    pub context: Il2CppGenericContext,
    pub cached_class: u64,
}

impl Il2CppGenericClass {
    pub fn read(stream: &mut BinaryStream, version: f64) -> Result<Self> {
        let type_definition_index = if version <= 24.5 { stream.read_ptr()? } else { 0 };
        let type_ptr = if version >= 27.0 { stream.read_ptr()? } else { 0 };
        let context = Il2CppGenericContext::read(stream)?;
        let cached_class = stream.read_ptr()?;
        Ok(Self {
            type_definition_index,
            type_ptr,
            context,
            cached_class,
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct Il2CppGenericContext {
    pub class_inst: u64,
    pub method_inst: u64,
}

impl Il2CppGenericContext {
    pub fn read(stream: &mut BinaryStream) -> Result<Self> {
        Ok(Self {
            class_inst: stream.read_ptr()?,
            method_inst: stream.read_ptr()?,
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct Il2CppGenericInst {
    pub type_argc: u64,
    pub type_argv: u64,
}

impl Il2CppGenericInst {
    pub fn read(stream: &mut BinaryStream) -> Result<Self> {
        Ok(Self {
            type_argc: stream.read_ptr()?,
            type_argv: stream.read_ptr()?,
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct Il2CppArrayType {
    pub etype: u64,
    pub rank: u8,
    pub numsizes: u8,
    pub numlobounds: u8,
    pub sizes: u64,
    pub lobounds: u64,
}

impl Il2CppArrayType {
    pub fn read(stream: &mut BinaryStream) -> Result<Self> {
        Ok(Self {
            etype: stream.read_ptr()?,
            rank: stream.read_u8()?,
            numsizes: stream.read_u8()?,
            numlobounds: stream.read_u8()?,
            sizes: stream.read_ptr()?,
            lobounds: stream.read_ptr()?,
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct Il2CppGenericMethodFunctionsDefinitions {
    pub generic_method_index: i32,
    pub indices: Il2CppGenericMethodIndices,
}

impl Il2CppGenericMethodFunctionsDefinitions {
    pub fn read(stream: &mut BinaryStream, version: f64) -> Result<Self> {
        Ok(Self {
            generic_method_index: stream.read_i32()?,
            indices: Il2CppGenericMethodIndices::read(stream, version)?,
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct Il2CppGenericMethodIndices {
    pub method_index: i32,
    pub invoker_index: i32,
    pub adjustor_thunk: i32,
}

impl Il2CppGenericMethodIndices {
    pub fn read(stream: &mut BinaryStream, version: f64) -> Result<Self> {
        Ok(Self {
            method_index: stream.read_i32()?,
            invoker_index: stream.read_i32()?,
            adjustor_thunk: read_versioned!(stream, version, 24.5, 99.0, read_i32, 0),
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct Il2CppMethodSpec {
    pub method_definition_index: i32,
    pub class_index_index: i32,
    pub method_index_index: i32,
}

impl Il2CppMethodSpec {
    pub fn read(stream: &mut BinaryStream) -> Result<Self> {
        Ok(Self {
            method_definition_index: stream.read_i32()?,
            class_index_index: stream.read_i32()?,
            method_index_index: stream.read_i32()?,
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct Il2CppCodeGenModule {
    pub module_name: u64,
    pub method_pointer_count: i64,
    pub method_pointers: u64,
    pub adjustor_thunk_count: u64,
    pub adjustor_thunks: u64,
    pub invoker_indices: u64,
    pub reverse_pinvoke_wrapper_count: u64,
    pub reverse_pinvoke_wrapper_indices: u64,
    pub rgctx_ranges_count: i64,
    pub rgctx_ranges: u64,
    pub rgctxs_count: i64,
    pub rgctxs: u64,
    pub debugger_metadata: u64,
    pub custom_attribute_cache_generator: u64,
    pub module_initializer: u64,
    pub static_constructor_type_indices: u64,
    pub metadata_registration: u64,
    pub code_registration: u64,
}

impl Il2CppCodeGenModule {
    pub fn read(stream: &mut BinaryStream, version: f64) -> Result<Self> {
        Ok(Self {
            module_name: stream.read_ptr()?,
            method_pointer_count: stream.read_ptr_signed()?,
            method_pointers: stream.read_ptr()?,
            adjustor_thunk_count: if version >= 24.5 { stream.read_ptr()? } else { 0 },
            adjustor_thunks: if version >= 24.5 { stream.read_ptr()? } else { 0 },
            invoker_indices: stream.read_ptr()?,
            reverse_pinvoke_wrapper_count: stream.read_ptr()?,
            reverse_pinvoke_wrapper_indices: stream.read_ptr()?,
            rgctx_ranges_count: stream.read_ptr_signed()?,
            rgctx_ranges: stream.read_ptr()?,
            rgctxs_count: stream.read_ptr_signed()?,
            rgctxs: stream.read_ptr()?,
            debugger_metadata: stream.read_ptr()?,
            custom_attribute_cache_generator: if version >= 27.0 && version <= 27.2 { stream.read_ptr()? } else { 0 },
            module_initializer: if version >= 27.0 { stream.read_ptr()? } else { 0 },
            static_constructor_type_indices: if version >= 27.0 { stream.read_ptr()? } else { 0 },
            metadata_registration: if version >= 27.0 { stream.read_ptr()? } else { 0 },
            code_registration: if version >= 27.0 { stream.read_ptr()? } else { 0 },
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct Il2CppRange {
    pub start: i32,
    pub length: i32,
}

impl Il2CppRange {
    pub fn read(stream: &mut BinaryStream) -> Result<Self> {
        Ok(Self {
            start: stream.read_i32()?,
            length: stream.read_i32()?,
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct Il2CppTokenRangePair {
    pub token: u32,
    pub range: Il2CppRange,
}

impl Il2CppTokenRangePair {
    pub fn read(stream: &mut BinaryStream) -> Result<Self> {
        Ok(Self {
            token: stream.read_u32()?,
            range: Il2CppRange::read(stream)?,
        })
    }
}
