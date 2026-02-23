use std::collections::HashMap;
use std::fmt::Write as FmtWrite;
use std::fs;
use std::path::Path;
use crate::error::Result;
use crate::il2cpp::base::*;
use crate::il2cpp::metadata::Metadata;
use crate::il2cpp::enums::*;
use crate::il2cpp::structures::*;
use crate::executor::Il2CppExecutor;
use crate::executor::custom_attribute_reader;
use crate::config::Config;

pub struct Il2CppDecompiler;

impl Il2CppDecompiler {
    pub fn decompile(
        executor: &mut Il2CppExecutor,
        metadata: &mut Metadata,
        il2cpp: &mut Il2Cpp,
        config: &Config,
        output_dir: &str,
    ) -> Result<()> {
        let output_path = Path::new(output_dir).join("dump.cs");
        let mut buf = String::with_capacity(1 << 20);

        let image_info: Vec<(usize, i32, i32)> = metadata.image_defs.iter().enumerate()
            .map(|(idx, img)| (idx, img.name_index, img.type_start))
            .collect();
        for (idx, name_index, type_start) in &image_info {
            let name = metadata.get_string_from_index(*name_index)?;
            writeln!(buf, "// Image {idx}: {name} - {type_start}").ok();
        }

        let image_defs = metadata.image_defs.clone();
        for (img_idx, image_def) in image_defs.iter().enumerate() {
            let image_name = metadata.get_string_from_index(image_def.name_index)?;
            let type_end = image_def.type_start as usize + image_def.type_count as usize;

            for type_def_index in image_def.type_start as usize..type_end {
                if let Err(e) = Self::dump_type(
                    &mut buf, executor, metadata, il2cpp, config,
                    type_def_index, img_idx, &image_name,
                ) {
                    writeln!(buf, "/*\n{e}\n*/\n}}").ok();
                }
            }
        }

        fs::write(output_path, buf)?;
        Ok(())
    }

    fn dump_type(
        buf: &mut String,
        executor: &mut Il2CppExecutor,
        metadata: &mut Metadata,
        il2cpp: &mut Il2Cpp,
        config: &Config,
        type_def_index: usize,
        image_index: usize,
        image_name: &str,
    ) -> Result<()> {
        let type_def = metadata.type_defs[type_def_index].clone();
        let mut extends = Vec::new();

        if type_def.parent_index >= 0 {
            if let Some(parent) = il2cpp.types.get(type_def.parent_index as usize).cloned() {
                let parent_name = executor.get_type_name(&parent, metadata, il2cpp, false, false);
                if !type_def.is_value_type() && !type_def.is_enum() && parent_name != "object" {
                    extends.push(parent_name);
                }
            }
        }

        if type_def.interfaces_count > 0 {
            for i in 0..type_def.interfaces_count as usize {
                let iface_idx = metadata.interface_indices[type_def.interfaces_start as usize + i];
                if let Some(iface) = il2cpp.types.get(iface_idx as usize).cloned() {
                    extends.push(executor.get_type_name(&iface, metadata, il2cpp, false, false));
                }
            }
        }

        let namespace = metadata.get_string_from_index(type_def.namespace_index)?;
        writeln!(buf, "\n// Namespace: {namespace}").ok();

        if config.dump_attribute {
            Self::write_custom_attributes(
                buf, executor, metadata, il2cpp,
                image_index, type_def.custom_attribute_index, type_def.token, "",
            );

            if (type_def.flags & type_attributes::SERIALIZABLE) != 0 {
                writeln!(buf, "[Serializable]").ok();
            }
        }

        let visibility = type_def.flags & type_attributes::VISIBILITY_MASK;
        buf.push_str(&get_type_visibility(visibility));

        if (type_def.flags & type_attributes::ABSTRACT) != 0
            && (type_def.flags & type_attributes::SEALED) != 0
        {
            buf.push_str("static ");
        } else if (type_def.flags & type_attributes::INTERFACE) == 0
            && (type_def.flags & type_attributes::ABSTRACT) != 0
        {
            buf.push_str("abstract ");
        } else if !type_def.is_value_type() && !type_def.is_enum()
            && (type_def.flags & type_attributes::SEALED) != 0
        {
            buf.push_str("sealed ");
        }

        if (type_def.flags & type_attributes::INTERFACE) != 0 {
            buf.push_str("interface ");
        } else if type_def.is_enum() {
            buf.push_str("enum ");
        } else if type_def.is_value_type() {
            buf.push_str("struct ");
        } else {
            buf.push_str("class ");
        }

        let type_name = executor.get_type_def_name(&type_def, type_def_index, metadata, il2cpp, false, true);
        buf.push_str(&type_name);

        if !extends.is_empty() {
            write!(buf, " : {}", extends.join(", ")).ok();
        }

        if config.dump_type_def_index {
            writeln!(buf, " // TypeDefIndex: {type_def_index}\n{{").ok();
        } else {
            writeln!(buf, "\n{{").ok();
        }

        if config.dump_field && type_def.field_count > 0 {
            Self::dump_fields(buf, executor, metadata, il2cpp, config, &type_def, type_def_index, image_index)?;
        }

        if config.dump_property && type_def.property_count > 0 {
            Self::dump_properties(buf, executor, metadata, il2cpp, config, &type_def, image_index)?;
        }

        if config.dump_method && type_def.method_count > 0 {
            Self::dump_methods(buf, executor, metadata, il2cpp, config, &type_def, image_name, image_index)?;
        }

        writeln!(buf, "}}").ok();
        Ok(())
    }

    fn dump_fields(
        buf: &mut String,
        executor: &mut Il2CppExecutor,
        metadata: &mut Metadata,
        il2cpp: &mut Il2Cpp,
        config: &Config,
        type_def: &Il2CppTypeDefinition,
        type_def_index: usize,
        image_index: usize,
    ) -> Result<()> {
        writeln!(buf, "\n\t// Fields").ok();
        let field_end = type_def.field_start as usize + type_def.field_count as usize;

        for i in type_def.field_start as usize..field_end {
            let field_def = metadata.field_defs[i].clone();
            let field_type = il2cpp.types[field_def.type_index as usize].clone();
            let mut is_static = false;
            let mut is_const = false;

            if config.dump_attribute {
                Self::write_custom_attributes(
                    buf, executor, metadata, il2cpp,
                    image_index, field_def.custom_attribute_index, field_def.token, "\t",
                );
            }

            buf.push('\t');

            let access = field_type.attrs & field_attributes::FIELD_ACCESS_MASK;
            buf.push_str(&get_field_visibility(access));

            if (field_type.attrs & field_attributes::LITERAL) != 0 {
                is_const = true;
                buf.push_str("const ");
            } else {
                if (field_type.attrs & field_attributes::STATIC) != 0 {
                    is_static = true;
                    buf.push_str("static ");
                }
                if (field_type.attrs & field_attributes::INIT_ONLY) != 0 {
                    buf.push_str("readonly ");
                }
            }

            let field_type_name = executor.get_type_name(&field_type, metadata, il2cpp, false, false);
            let field_name = metadata.get_string_from_index(field_def.name_index)?;
            write!(buf, "{field_type_name} {field_name}").ok();

            if let Some(dv) = metadata.get_field_default_value(i as i32) {
                let dv = dv.clone();
                if dv.data_index != -1 {
                    match executor.try_get_default_value(dv.type_index, dv.data_index, metadata, il2cpp) {
                        Ok(val) => write!(buf, " = {val}").ok(),
                        Err(offset) => write!(buf, " /*Metadata offset 0x{offset:X}*/").ok(),
                    };
                }
            }

            if config.dump_field_offset && !is_const {
                let offset = il2cpp.get_field_offset_from_index(
                    type_def_index,
                    i - type_def.field_start as usize,
                    i,
                    type_def.is_value_type(),
                    is_static,
                );
                writeln!(buf, "; // 0x{offset:X}").ok();
            } else {
                writeln!(buf, ";").ok();
            }
        }
        Ok(())
    }

    fn dump_properties(
        buf: &mut String,
        executor: &mut Il2CppExecutor,
        metadata: &mut Metadata,
        il2cpp: &mut Il2Cpp,
        config: &Config,
        type_def: &Il2CppTypeDefinition,
        image_index: usize,
    ) -> Result<()> {
        writeln!(buf, "\n\t// Properties").ok();
        let prop_end = type_def.property_start as usize + type_def.property_count as usize;

        for i in type_def.property_start as usize..prop_end {
            let property_def = metadata.property_defs[i].clone();

            if config.dump_attribute {
                Self::write_custom_attributes(
                    buf, executor, metadata, il2cpp,
                    image_index, property_def.custom_attribute_index, property_def.token, "\t",
                );
            }

            buf.push('\t');

            let property_type_name;
            if property_def.get >= 0 {
                let method_def = metadata.method_defs[type_def.method_start as usize + property_def.get as usize].clone();
                let mods = executor.get_modifiers(method_def.flags as u32).to_string();
                buf.push_str(&mods);
                let ret_type = il2cpp.types[method_def.return_type as usize].clone();
                property_type_name = executor.get_type_name(&ret_type, metadata, il2cpp, false, false);
            } else if property_def.set >= 0 {
                let method_def = metadata.method_defs[type_def.method_start as usize + property_def.set as usize].clone();
                let mods = executor.get_modifiers(method_def.flags as u32).to_string();
                buf.push_str(&mods);
                let param_def = metadata.parameter_defs[method_def.parameter_start as usize].clone();
                let param_type = il2cpp.types[param_def.type_index as usize].clone();
                property_type_name = executor.get_type_name(&param_type, metadata, il2cpp, false, false);
            } else {
                property_type_name = "object".to_string();
            }

            let property_name = metadata.get_string_from_index(property_def.name_index)?;
            write!(buf, "{property_type_name} {property_name} {{ ").ok();

            if property_def.get >= 0 { buf.push_str("get; "); }
            if property_def.set >= 0 { buf.push_str("set; "); }

            writeln!(buf, "}}").ok();
        }
        Ok(())
    }

    fn dump_methods(
        buf: &mut String,
        executor: &mut Il2CppExecutor,
        metadata: &mut Metadata,
        il2cpp: &mut Il2Cpp,
        config: &Config,
        type_def: &Il2CppTypeDefinition,
        image_name: &str,
        image_index: usize,
    ) -> Result<()> {
        writeln!(buf, "\n\t// Methods").ok();
        let method_end = type_def.method_start as usize + type_def.method_count as usize;

        for i in type_def.method_start as usize..method_end {
            writeln!(buf).ok();
            let method_def = metadata.method_defs[i].clone();
            let is_abstract = (method_def.flags as u32 & method_attributes::ABSTRACT) != 0;

            if config.dump_attribute {
                Self::write_custom_attributes(
                    buf, executor, metadata, il2cpp,
                    image_index, method_def.custom_attribute_index, method_def.token, "\t",
                );
            }

            if config.dump_method_offset {
                let method_pointer = il2cpp.get_method_pointer(image_name, &method_def);
                if !is_abstract && method_pointer > 0 {
                    let rva = il2cpp.get_rva(method_pointer);
                    let offset = il2cpp.map_vatr(method_pointer).unwrap_or(rva);
                    write!(buf, "\t// RVA: 0x{rva:X} Offset: 0x{offset:X} VA: 0x{method_pointer:X}").ok();
                } else {
                    write!(buf, "\t// RVA: -1 Offset: -1").ok();
                }
                if method_def.slot != 0xFFFF {
                    write!(buf, " Slot: {}", method_def.slot).ok();
                }
                writeln!(buf).ok();
            }

            buf.push('\t');
            let mods = executor.get_modifiers(method_def.flags as u32).to_string();
            buf.push_str(&mods);

            let return_type = il2cpp.types[method_def.return_type as usize].clone();
            let method_name_raw = metadata.get_string_from_index(method_def.name_index as i32)?;
            let mut method_name = method_name_raw;

            if method_def.generic_container_index >= 0 {
                if let Some(gc) = metadata.generic_containers.get(method_def.generic_container_index as usize) {
                    let gc = gc.clone();
                    let params = executor.get_generic_container_params(&gc, metadata);
                    method_name.push_str(&params);
                }
            }

            if return_type.byref == 1 {
                buf.push_str("ref ");
            }

            let return_type_name = executor.get_type_name(&return_type, metadata, il2cpp, false, false);
            write!(buf, "{return_type_name} {method_name}(").ok();

            let mut params = Vec::new();
            for j in 0..method_def.parameter_count as usize {
                let param_def = metadata.parameter_defs[method_def.parameter_start as usize + j].clone();
                let param_name = metadata.get_string_from_index(param_def.name_index)?;
                let param_type = il2cpp.types[param_def.type_index as usize].clone();
                let param_type_name = executor.get_type_name(&param_type, metadata, il2cpp, false, false);

                let mut param_str = String::new();

                if param_type.byref == 1 {
                    if (param_type.attrs & param_attributes::OUT) != 0
                        && (param_type.attrs & param_attributes::IN) == 0
                    {
                        param_str.push_str("out ");
                    } else if (param_type.attrs & param_attributes::OUT) == 0
                        && (param_type.attrs & param_attributes::IN) != 0
                    {
                        param_str.push_str("in ");
                    } else {
                        param_str.push_str("ref ");
                    }
                } else {
                    if (param_type.attrs & param_attributes::IN) != 0 {
                        param_str.push_str("[In] ");
                    }
                    if (param_type.attrs & param_attributes::OUT) != 0 {
                        param_str.push_str("[Out] ");
                    }
                }

                write!(param_str, "{param_type_name} {param_name}").ok();

                if let Some(dv) = metadata.get_parameter_default_value(method_def.parameter_start + j as i32) {
                    let dv = dv.clone();
                    if dv.data_index != -1 {
                        match executor.try_get_default_value(dv.type_index, dv.data_index, metadata, il2cpp) {
                            Ok(val) => write!(param_str, " = {val}").ok(),
                            Err(offset) => write!(param_str, " /*Metadata offset 0x{offset:X}*/").ok(),
                        };
                    }
                }

                params.push(param_str);
            }

            buf.push_str(&params.join(", "));

            if is_abstract {
                writeln!(buf, ");").ok();
            } else {
                writeln!(buf, ") {{ }}").ok();
            }

            if let Some(method_specs) = il2cpp.method_definition_method_specs.get(&i).cloned() {
                writeln!(buf, "\t/* GenericInstMethod :").ok();

                let mut groups: HashMap<u64, Vec<usize>> = HashMap::new();
                for spec_idx in &method_specs {
                    let ptr = il2cpp.method_spec_generic_method_pointers.get(spec_idx).copied().unwrap_or(0);
                    groups.entry(ptr).or_default().push(*spec_idx);
                }

                for (ptr, spec_indices) in &groups {
                    writeln!(buf, "\t|").ok();
                    if *ptr > 0 {
                        let rva = il2cpp.get_rva(*ptr);
                        writeln!(buf, "\t|-RVA: 0x{rva:X} VA: 0x{ptr:X}").ok();
                    } else {
                        writeln!(buf, "\t|-RVA: -1 Offset: -1").ok();
                    }

                    for spec_idx in spec_indices {
                        let (type_name, method_name) = executor.get_method_spec_name(*spec_idx, metadata, il2cpp, false);
                        writeln!(buf, "\t|-{type_name}.{method_name}").ok();
                    }
                }

                writeln!(buf, "\t*/").ok();
            }
        }
        Ok(())
    }

    fn write_custom_attributes(
        buf: &mut String,
        executor: &mut Il2CppExecutor,
        metadata: &mut Metadata,
        il2cpp: &mut Il2Cpp,
        image_index: usize,
        custom_attribute_index: i32,
        token: u32,
        padding: &str,
    ) {
        if il2cpp.version < 21.0 {
            return;
        }

        let attr_index = metadata.get_custom_attribute_index(image_index, custom_attribute_index, token);

        if let Some(attr_idx) = attr_index {
            if il2cpp.version < 29.0 {
                let method_pointer = if attr_idx < executor.custom_attribute_generators.len() {
                    executor.custom_attribute_generators[attr_idx]
                } else {
                    0
                };
                let rva = il2cpp.get_rva(method_pointer);
                let offset = il2cpp.map_vatr(method_pointer).unwrap_or(rva);

                if attr_idx < metadata.attribute_type_ranges.len() {
                    let attr_range = metadata.attribute_type_ranges[attr_idx].clone();
                    for j in 0..attr_range.count as usize {
                        let type_index = metadata.attribute_types[attr_range.start as usize + j];
                        if let Some(attr_type) = il2cpp.types.get(type_index as usize).cloned() {
                            let type_name = executor.get_type_name(&attr_type, metadata, il2cpp, false, false);
                            writeln!(buf, "{padding}[{type_name}] // RVA: 0x{rva:X} Offset: 0x{offset:X} VA: 0x{method_pointer:X}").ok();
                        }
                    }
                }
            } else {
                custom_attribute_reader::format_custom_attribute_data(buf, metadata, attr_idx, padding);
            }
        }
    }
}

fn get_type_visibility(visibility: u32) -> String {
    match visibility {
        type_attributes::PUBLIC | type_attributes::NESTED_PUBLIC => "public ".to_string(),
        type_attributes::NOT_PUBLIC | type_attributes::NESTED_FAM_AND_ASSEM | type_attributes::NESTED_ASSEMBLY => "internal ".to_string(),
        type_attributes::NESTED_PRIVATE => "private ".to_string(),
        type_attributes::NESTED_FAMILY => "protected ".to_string(),
        type_attributes::NESTED_FAM_OR_ASSEM => "protected internal ".to_string(),
        _ => String::new(),
    }
}

fn get_field_visibility(access: u32) -> String {
    match access {
        field_attributes::PRIVATE => "private ".to_string(),
        field_attributes::PUBLIC => "public ".to_string(),
        field_attributes::FAMILY => "protected ".to_string(),
        field_attributes::ASSEMBLY | field_attributes::FAM_AND_ASSEM => "internal ".to_string(),
        field_attributes::FAM_OR_ASSEM => "protected internal ".to_string(),
        _ => String::new(),
    }
}
