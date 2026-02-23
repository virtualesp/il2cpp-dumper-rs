use std::collections::HashSet;
use std::fmt::Write as FmtWrite;
use std::fs;
use std::path::Path;
use crate::error::Result;
use crate::il2cpp::base::*;
use crate::il2cpp::metadata::Metadata;
use crate::il2cpp::enums::*;
use crate::il2cpp::structures::*;
use crate::executor::Il2CppExecutor;
use super::script_json::*;

static C_TYPE_MAP: &[(Il2CppTypeEnum, &str)] = &[
    (Il2CppTypeEnum::Void, "void"),
    (Il2CppTypeEnum::Boolean, "bool"),
    (Il2CppTypeEnum::Char, "uint16_t"),
    (Il2CppTypeEnum::I1, "int8_t"),
    (Il2CppTypeEnum::U1, "uint8_t"),
    (Il2CppTypeEnum::I2, "int16_t"),
    (Il2CppTypeEnum::U2, "uint16_t"),
    (Il2CppTypeEnum::I4, "int32_t"),
    (Il2CppTypeEnum::U4, "uint32_t"),
    (Il2CppTypeEnum::I8, "int64_t"),
    (Il2CppTypeEnum::U8, "uint64_t"),
    (Il2CppTypeEnum::R4, "float"),
    (Il2CppTypeEnum::R8, "double"),
    (Il2CppTypeEnum::String, "System_String_o*"),
    (Il2CppTypeEnum::Object, "Il2CppObject*"),
    (Il2CppTypeEnum::I, "intptr_t"),
    (Il2CppTypeEnum::U, "uintptr_t"),
];

fn get_c_type(type_enum: Il2CppTypeEnum) -> Option<&'static str> {
    C_TYPE_MAP.iter().find(|(e, _)| *e == type_enum).map(|(_, s)| *s)
}

pub struct StructGenerator;

impl StructGenerator {
    pub fn write_all(
        executor: &mut Il2CppExecutor,
        metadata: &mut Metadata,
        il2cpp: &mut Il2Cpp,
        output_dir: &str,
    ) -> Result<()> {
        let output_path = Path::new(output_dir);
        Self::write_script_json(executor, metadata, il2cpp, &output_path.join("script.json"))?;
        Self::write_string_literal_json(metadata, &output_path.join("stringliteral.json"))?;
        Self::write_header(executor, metadata, il2cpp, &output_path.join("il2cpp.h"))?;
        Ok(())
    }

    fn write_script_json(
        executor: &mut Il2CppExecutor,
        metadata: &mut Metadata,
        il2cpp: &mut Il2Cpp,
        path: &Path,
    ) -> Result<()> {
        let mut script = ScriptJson::new();
        let mut addresses_set = HashSet::new();

        let image_defs = metadata.image_defs.clone();
        for image_def in &image_defs {
            let image_name = metadata.get_string_from_index(image_def.name_index)?;
            let type_end = image_def.type_start as usize + image_def.type_count as usize;

            for type_def_index in image_def.type_start as usize..type_end {
                let type_def = metadata.type_defs[type_def_index].clone();
                let type_name = executor.get_type_def_name(&type_def, type_def_index, metadata, il2cpp, true, false);

                let method_end = type_def.method_start as usize + type_def.method_count as usize;
                for method_index in type_def.method_start as usize..method_end {
                    let method_def = metadata.method_defs[method_index].clone();
                    if (method_def.flags as u32 & method_attributes::ABSTRACT) != 0 {
                        continue;
                    }
                    let method_pointer = il2cpp.get_method_pointer(&image_name, &method_def);
                    if method_pointer == 0 {
                        continue;
                    }
                    let rva = il2cpp.get_rva(method_pointer);
                    addresses_set.insert(rva);

                    let method_name = metadata.get_string_from_index(method_def.name_index as i32)?;
                    let signature = Self::get_method_signature(executor, metadata, il2cpp, &method_def);

                    script.script_methods.push(ScriptMethod {
                        address: rva,
                        name: method_name,
                        signature,
                        type_signature: type_name.clone(),
                    });

                    if let Some(spec_indices) = il2cpp.method_definition_method_specs.get(&method_index).cloned() {
                        for spec_idx in &spec_indices {
                            let spec_ptr = il2cpp.method_spec_generic_method_pointers.get(spec_idx).copied().unwrap_or(0);
                            if spec_ptr == 0 {
                                continue;
                            }
                            let spec_rva = il2cpp.get_rva(spec_ptr);
                            if addresses_set.contains(&spec_rva) {
                                continue;
                            }
                            addresses_set.insert(spec_rva);

                            let (spec_type_name, spec_method_name) = executor.get_method_spec_name(*spec_idx, metadata, il2cpp, true);

                            script.script_methods.push(ScriptMethod {
                                address: spec_rva,
                                name: spec_method_name,
                                signature: String::new(),
                                type_signature: spec_type_name,
                            });
                        }
                    }
                }
            }
        }

        if il2cpp.version < 27.0 {
            Self::add_metadata_usages(&mut script, &mut addresses_set, executor, metadata, il2cpp);
        }

        let mut sorted_addresses: Vec<u64> = addresses_set.into_iter().collect();
        sorted_addresses.sort_unstable();
        script.addresses = sorted_addresses;

        let json = script.to_json().map_err(|e| crate::error::Error::Other(e.to_string()))?;
        fs::write(path, json)?;
        Ok(())
    }

    fn add_metadata_usages(
        script: &mut ScriptJson,
        _addresses_set: &mut HashSet<u64>,
        executor: &mut Il2CppExecutor,
        metadata: &mut Metadata,
        il2cpp: &mut Il2Cpp,
    ) {
        if metadata.metadata_usage_dic.is_empty() {
            return;
        }

        let usage_dic = metadata.metadata_usage_dic.clone();
        for (usage_type, entries) in &usage_dic {
            for (dest_index, source_index) in entries {
                let dest = *dest_index as usize;
                if dest >= il2cpp.metadata_usages.len() {
                    continue;
                }
                let address = il2cpp.metadata_usages[dest];
                if address == 0 {
                    continue;
                }
                let rva = il2cpp.get_rva(address);

                let src = *source_index as usize;
                match *usage_type {
                    1 => {
                        if let Some(type_def) = metadata.type_defs.get(src).cloned() {
                            let td_idx = src;
                            let name = executor.get_type_def_name(&type_def, td_idx, metadata, il2cpp, true, true);
                            script.script_metadata.push(ScriptMetadata {
                                address: rva,
                                name: format!("{name}_TypeInfo"),
                            });
                        }
                    }
                    2 => {
                        if let Some(il2cpp_type) = il2cpp.types.get(src).cloned() {
                            let name = executor.get_type_name(&il2cpp_type, metadata, il2cpp, true, false);
                            script.script_metadata.push(ScriptMetadata {
                                address: rva,
                                name: format!("{name}_Type"),
                            });
                        }
                    }
                    3 => {
                        if let Some(method_def) = metadata.method_defs.get(src).cloned() {
                            if let Some(type_def) = metadata.type_defs.get(method_def.declaring_type as usize).cloned() {
                                let td_idx = method_def.declaring_type as usize;
                                let type_name = executor.get_type_def_name(&type_def, td_idx, metadata, il2cpp, true, true);
                                let method_name = metadata.get_string_from_index(method_def.name_index as i32)
                                    .unwrap_or_else(|_| "?".to_string());
                                script.script_metadata_methods.push(ScriptMetadataMethod {
                                    address: rva,
                                    name: format!("{type_name}.{method_name}"),
                                    method_address: 0,
                                });
                            }
                        }
                    }
                    5 => {
                        if let Ok(string_literal) = metadata.get_string_literal_from_index(src) {
                            script.script_strings.push(ScriptString {
                                address: rva,
                                value: string_literal,
                            });
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    fn write_string_literal_json(
        metadata: &mut Metadata,
        path: &Path,
    ) -> Result<()> {
        let mut entries = Vec::new();
        for i in 0..metadata.string_literals.len() {
            if let Ok(value) = metadata.get_string_literal_from_index(i) {
                entries.push(StringLiteralEntry { index: i, value });
            }
        }
        let json = serde_json::to_string_pretty(&entries)
            .map_err(|e| crate::error::Error::Other(e.to_string()))?;
        fs::write(path, json)?;
        Ok(())
    }

    fn write_header(
        executor: &mut Il2CppExecutor,
        metadata: &mut Metadata,
        il2cpp: &mut Il2Cpp,
        path: &Path,
    ) -> Result<()> {
        let mut buf = String::with_capacity(1 << 18);
        let mut generated_types = HashSet::new();

        writeln!(buf, "#ifndef IL2CPP_H").ok();
        writeln!(buf, "#define IL2CPP_H\n").ok();
        writeln!(buf, "#include <stdint.h>").ok();
        writeln!(buf, "#include <stdbool.h>\n").ok();

        Self::write_base_types(&mut buf);

        let type_defs = metadata.type_defs.clone();
        for type_def in &type_defs {
            let safe_name = Self::get_safe_type_name(type_def, metadata);
            if !safe_name.is_empty() && !generated_types.contains(&safe_name) {
                writeln!(buf, "struct {safe_name}_o;").ok();
                generated_types.insert(safe_name);
            }
        }
        writeln!(buf).ok();

        generated_types.clear();

        for (idx, type_def) in type_defs.iter().enumerate() {
            let safe_name = Self::get_safe_type_name(type_def, metadata);
            if safe_name.is_empty() || generated_types.contains(&safe_name) {
                continue;
            }
            generated_types.insert(safe_name.clone());

            if type_def.is_enum() {
                Self::write_enum_definition(&mut buf, executor, metadata, il2cpp, type_def, &safe_name);
                continue;
            }

            writeln!(buf, "typedef struct {safe_name}_o {{").ok();

            if type_def.parent_index >= 0 && !type_def.is_value_type() {
                if let Some(parent_type) = il2cpp.types.get(type_def.parent_index as usize).cloned() {
                    let parent_name = executor.get_type_name(&parent_type, metadata, il2cpp, false, false);
                    if parent_name != "object" && parent_name != "ValueType" {
                        writeln!(buf, "    {}_o _base;", sanitize_name(&parent_name)).ok();
                    } else {
                        writeln!(buf, "    Il2CppObject _base;").ok();
                    }
                }
            }

            let field_end = type_def.field_start as usize + type_def.field_count as usize;
            for i in type_def.field_start as usize..field_end {
                let field_def = metadata.field_defs[i].clone();
                let field_type = il2cpp.types[field_def.type_index as usize].clone();

                if (field_type.attrs & field_attributes::STATIC) != 0 {
                    continue;
                }

                let field_name = metadata.get_string_from_index(field_def.name_index)
                    .unwrap_or_else(|_| "field".to_string());
                let field_type_str = Self::get_c_type_name(&field_type, executor, metadata, il2cpp);
                let safe_field_name = sanitize_name(&field_name);

                let offset = il2cpp.get_field_offset_from_index(
                    idx, i - type_def.field_start as usize, i,
                    type_def.is_value_type(), false,
                );
                writeln!(buf, "    {field_type_str} {safe_field_name}; // 0x{offset:X}").ok();
            }

            writeln!(buf, "}} {safe_name}_o;\n").ok();
        }

        writeln!(buf, "\n#endif // IL2CPP_H").ok();
        fs::write(path, buf)?;
        Ok(())
    }

    fn write_base_types(buf: &mut String) {
        writeln!(buf, "typedef struct Il2CppObject {{").ok();
        writeln!(buf, "    void* klass;").ok();
        writeln!(buf, "    void* monitor;").ok();
        writeln!(buf, "}} Il2CppObject;\n").ok();

        writeln!(buf, "typedef struct System_String_o {{").ok();
        writeln!(buf, "    Il2CppObject _base;").ok();
        writeln!(buf, "    int32_t length;").ok();
        writeln!(buf, "    uint16_t chars[1];").ok();
        writeln!(buf, "}} System_String_o;\n").ok();

        writeln!(buf, "typedef struct Il2CppArray {{").ok();
        writeln!(buf, "    Il2CppObject _base;").ok();
        writeln!(buf, "    void* bounds;").ok();
        writeln!(buf, "    uintptr_t max_length;").ok();
        writeln!(buf, "}} Il2CppArray;\n").ok();
    }

    fn write_enum_definition(
        buf: &mut String,
        executor: &mut Il2CppExecutor,
        metadata: &mut Metadata,
        il2cpp: &Il2Cpp,
        type_def: &Il2CppTypeDefinition,
        safe_name: &str,
    ) {
        writeln!(buf, "typedef enum {safe_name} {{").ok();
        let field_end = type_def.field_start as usize + type_def.field_count as usize;

        for i in type_def.field_start as usize..field_end {
            let field_def = &metadata.field_defs[i];
            let field_type = &il2cpp.types[field_def.type_index as usize];
            if (field_type.attrs & field_attributes::LITERAL) == 0 {
                continue;
            }
            let field_name = metadata.get_string_from_index(field_def.name_index)
                .unwrap_or_else(|_| "value".to_string());
            let safe_field_name = sanitize_name(&field_name);

            if let Some(dv) = metadata.get_field_default_value(i as i32) {
                let dv = dv.clone();
                if dv.data_index != -1 {
                    if let Ok(val) = executor.try_get_default_value(dv.type_index, dv.data_index, metadata, il2cpp) {
                        writeln!(buf, "    {safe_name}_{safe_field_name} = {val},").ok();
                        continue;
                    }
                }
            }
            writeln!(buf, "    {safe_name}_{safe_field_name},").ok();
        }

        writeln!(buf, "}} {safe_name};\n").ok();
    }

    fn get_safe_type_name(type_def: &Il2CppTypeDefinition, metadata: &mut Metadata) -> String {
        let namespace = metadata.get_string_from_index(type_def.namespace_index)
            .unwrap_or_default();
        let mut name = metadata.get_string_from_index(type_def.name_index)
            .unwrap_or_default();

        if let Some(backtick) = name.find('`') {
            name.truncate(backtick);
        }

        let full_name = if namespace.is_empty() { name } else { format!("{namespace}_{name}") };
        sanitize_name(&full_name)
    }

    fn get_c_type_name(
        il2cpp_type: &Il2CppType,
        _executor: &mut Il2CppExecutor,
        metadata: &mut Metadata,
        _il2cpp: &mut Il2Cpp,
    ) -> String {
        let type_enum = Il2CppTypeEnum::from_u8(il2cpp_type.type_enum);

        if let Some(te) = type_enum {
            if let Some(c_type) = get_c_type(te) {
                return c_type.to_string();
            }

            match te {
                Il2CppTypeEnum::SzArray => return "Il2CppArray*".to_string(),
                Il2CppTypeEnum::Ptr => return "void*".to_string(),
                Il2CppTypeEnum::GenericInst => return "void*".to_string(),
                Il2CppTypeEnum::Class | Il2CppTypeEnum::ValueType => {
                    let klass_index = il2cpp_type.klass_index() as usize;
                    if let Some(type_def) = metadata.type_defs.get(klass_index).cloned() {
                        let safe_name = Self::get_safe_type_name(&type_def, metadata);
                        if type_def.is_value_type() {
                            return format!("{safe_name}_o");
                        } else {
                            return format!("{safe_name}_o*");
                        }
                    }
                    return "void*".to_string();
                }
                _ => {}
            }
        }

        "void*".to_string()
    }

    fn get_method_signature(
        executor: &mut Il2CppExecutor,
        metadata: &mut Metadata,
        il2cpp: &mut Il2Cpp,
        method_def: &Il2CppMethodDefinition,
    ) -> String {
        let return_type = il2cpp.types[method_def.return_type as usize].clone();
        let return_type_name = executor.get_type_name(&return_type, metadata, il2cpp, false, false);
        let method_name = metadata.get_string_from_index(method_def.name_index as i32)
            .unwrap_or_else(|_| "?".to_string());

        let mut params = Vec::new();
        for j in 0..method_def.parameter_count as usize {
            let param_def = metadata.parameter_defs[method_def.parameter_start as usize + j].clone();
            let param_type = il2cpp.types[param_def.type_index as usize].clone();
            let param_type_name = executor.get_type_name(&param_type, metadata, il2cpp, false, false);
            let param_name = metadata.get_string_from_index(param_def.name_index)
                .unwrap_or_else(|_| "param".to_string());
            params.push(format!("{param_type_name} {param_name}"));
        }

        format!("{return_type_name} {method_name}({})", params.join(", "))
    }
}

fn sanitize_name(name: &str) -> String {
    let mut result = String::with_capacity(name.len());
    for c in name.chars() {
        if c.is_alphanumeric() || c == '_' {
            result.push(c);
        } else if matches!(c, '.' | '/' | '<' | '>' | '[' | ']') {
            result.push('_');
        }
    }
    if result.starts_with(|c: char| c.is_ascii_digit()) {
        result.insert(0, '_');
    }
    result
}
