use std::collections::{HashMap, HashSet};
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
use crate::disassembler::{self, Disassembler, DisassemblyContext};

pub struct Il2CppDecompiler;

impl Il2CppDecompiler {
    pub fn decompile<L: FnMut(&str)>(
        executor: &mut Il2CppExecutor,
        metadata: &mut Metadata,
        il2cpp: &mut Il2Cpp,
        config: &Config,
        output_dir: &str,
        mut logger: L,
    ) -> Result<()> {
        let output_path = Path::new(output_dir).join("dump.cs");
        let mut buf = String::with_capacity(1 << 20);

        let disasm = if config.dump_disassembly {
            let arch = il2cpp.detect_architecture();
            logger(&format!("Disassembly enabled — architecture: {arch}"));
            let sorted_addrs = il2cpp.build_sorted_method_addresses();
            logger(&format!("Building method address map ({} methods)...", sorted_addrs.len()));

            let mut rva_to_name: HashMap<u64, String> = HashMap::new();
            let image_defs_clone = metadata.image_defs.clone();
            for image_def in &image_defs_clone {
                let image_name = metadata.get_string_from_index(image_def.name_index).unwrap_or_default();
                let type_end = image_def.type_start as usize + image_def.type_count as usize;
                for type_def_index in image_def.type_start as usize..type_end {
                    let td = metadata.type_defs[type_def_index].clone();
                    let type_name = metadata.get_string_from_index(td.name_index).unwrap_or_default();
                    let method_end = td.method_start as usize + td.method_count as usize;
                    for mi in td.method_start as usize..method_end {
                        let method_def = metadata.method_defs[mi].clone();
                        let method_ptr = il2cpp.get_method_pointer(&image_name, &method_def);
                        if method_ptr > 0 {
                            let rva = il2cpp.get_rva(method_ptr);
                            let method_name = metadata.get_string_from_index(method_def.name_index as i32).unwrap_or_default();
                            rva_to_name.insert(rva, format!("{type_name}.{method_name}"));
                        }
                    }
                }
            }

            let spec_pointers: Vec<(usize, u64)> = il2cpp.method_spec_generic_method_pointers
                .iter().map(|(k, v)| (*k, *v)).collect();
            let mut generic_spec_count = 0usize;
            for (spec_idx, ptr) in spec_pointers {
                if ptr == 0 { continue; }
                let rva = il2cpp.get_rva(ptr);
                if rva_to_name.contains_key(&rva) { continue; }
                let (tn, mn) = executor.get_method_spec_name(spec_idx, metadata, il2cpp, false);
                rva_to_name.insert(rva, format!("{tn}.{mn}"));
                generic_spec_count += 1;
            }
            if generic_spec_count > 0 {
                logger(&format!("Indexed {} generic method specializations", generic_spec_count));
            }

            let mut disassembler = Disassembler::new(arch);
            disassembler.set_method_names(rva_to_name);

            let mut string_table: HashMap<u32, String> = HashMap::new();
            let total_string_lits = metadata.string_literals.len();
            for i in 0..total_string_lits {
                if let Ok(s) = metadata.get_string_literal_from_index(i) {
                    string_table.insert(i as u32, s);
                }
            }
            if !string_table.is_empty() {
                disassembler.set_string_literal_table(string_table);
            }

            if il2cpp.version >= 27.0 {
                Self::build_v27_annotations(&mut disassembler, executor, metadata, il2cpp, &image_defs_clone);
            } else if il2cpp.version > 16.0 {
                Self::build_legacy_annotations(&mut disassembler, executor, metadata, il2cpp, &image_defs_clone);
            }

            let detected = Self::detect_string_new_wrapper(&disassembler, il2cpp, &sorted_addrs, total_string_lits, config);
            for rva in &detected {
                disassembler.add_string_new_wrapper_rva(*rva);
            }
            if !detected.is_empty() {
                let preview: Vec<String> = detected.iter().take(3).map(|r| format!("0x{:X}", r)).collect();
                logger(&format!("Detected {} il2cpp_string_new_wrapper candidate(s): {}", detected.len(), preview.join(", ")));
            }

            let box_helpers = Self::detect_box_helpers(&disassembler, il2cpp, &sorted_addrs, config);
            for rva in &box_helpers {
                disassembler.add_box_helper_rva(*rva);
            }
            if !box_helpers.is_empty() {
                let preview: Vec<String> = box_helpers.iter().take(3).map(|r| format!("0x{:X}", r)).collect();
                logger(&format!("Detected {} box/object_new helper candidate(s): {}", box_helpers.len(), preview.join(", ")));
            }

            let box_helper_set: HashSet<u64> = box_helpers.iter().copied().collect();
            let unbox_helpers = Self::detect_unbox_helpers(&disassembler, il2cpp, &sorted_addrs, config, &box_helper_set);
            for rva in &unbox_helpers {
                disassembler.add_unbox_helper_rva(*rva);
            }
            if !unbox_helpers.is_empty() {
                let preview: Vec<String> = unbox_helpers.iter().take(3).map(|r| format!("0x{:X}", r)).collect();
                logger(&format!("Detected {} unbox helper candidate(s): {}", unbox_helpers.len(), preview.join(", ")));
            }

            let ann_count = disassembler.annotation_count();
            if ann_count > 0 {
                logger(&format!("Built {} metadata annotations (strings, types, methods, fields)", ann_count));
            }

            Some((disassembler, sorted_addrs))
        } else {
            None
        };

        let image_info: Vec<(usize, i32, i32)> = metadata.image_defs.iter().enumerate()
            .map(|(idx, img)| (idx, img.name_index, img.type_start))
            .collect();
        for (idx, name_index, type_start) in &image_info {
            let name = metadata.get_string_from_index(*name_index)?;
            writeln!(buf, "// Image {idx}: {name} - {type_start}").ok();
        }

        let split_root = if config.split_dump_per_type {
            let p = Path::new(output_dir).join("DiffableCs");
            if p.exists() {
                fs::remove_dir_all(&p).ok();
            }
            fs::create_dir_all(&p)?;
            Some(p)
        } else {
            None
        };

        let mut created_dirs = HashSet::new();
        let mut split_file_outputs = Vec::new();

        let image_defs = metadata.image_defs.clone();
        for (img_idx, image_def) in image_defs.iter().enumerate() {
            let image_name = metadata.get_string_from_index(image_def.name_index)?;
            let type_end = image_def.type_start as usize + image_def.type_count as usize;

            for type_def_index in image_def.type_start as usize..type_end {
                // Classic dump.cs (flat structure, exactly matching original)
                let flat_disasm = if config.dump_disassembly_target == 0 || config.dump_disassembly_target == 1 {
                    &disasm
                } else {
                    &None
                };

                if let Err(e) = Self::dump_type(
                    &mut buf, executor, metadata, il2cpp, config,
                    type_def_index, img_idx, &image_name, "", false, flat_disasm,
                ) {
                    writeln!(buf, "/*\n{e}\n*/\n}}").ok();
                }

                // Diffable C# Split output (combined nested structure)
                if let Some(ref root) = split_root {
                    let type_def_ref = &metadata.type_defs[type_def_index];
                    
                    // Skip nested types (they will be recursed inside their parent)
                    if type_def_ref.declaring_type_index >= 0 {
                        continue;
                    }

                    let namespace_idx = type_def_ref.namespace_index;
                    let name_idx = type_def_ref.name_index;
                    
                    let namespace = metadata.get_string_from_index(namespace_idx).unwrap_or_default();
                    let type_name = metadata.get_string_from_index(name_idx).unwrap_or_default();

                    let assembly_name = image_name.trim_end_matches(".dll");
                    let dir = if namespace.is_empty() {
                        root.join(assembly_name)
                    } else {
                        root.join(assembly_name).join(namespace.replace('.', std::path::MAIN_SEPARATOR_STR))
                    };
                    
                    let dir_str = dir.to_string_lossy().to_string();
                    if !created_dirs.contains(&dir_str) {
                        fs::create_dir_all(&dir)?;
                        created_dirs.insert(dir_str);
                    }

                    let safe_name = type_name
                        .replace('<', "_")
                        .replace('>', "_")
                        .replace('|', "_")
                        .replace('/', "_")
                        .replace('\\', "_")
                        .replace(':', "_")
                        .replace('*', "_")
                        .replace('?', "_")
                        .replace('"', "_");
                    let file_path = dir.join(format!("{safe_name}.cs"));

                    let mut type_buf = String::with_capacity(4096);
                    let split_disasm = if config.dump_disassembly_target == 0 || config.dump_disassembly_target == 2 {
                        &disasm
                    } else {
                        &None
                    };

                    if let Err(e) = Self::dump_type(
                        &mut type_buf, executor, metadata, il2cpp, config,
                        type_def_index, img_idx, &image_name, "", true, split_disasm,
                    ) {
                        writeln!(type_buf, "/*\n{e}\n*/\n}}").ok();
                    }
                    split_file_outputs.push((file_path, type_buf));
                }
            }
        }

        use rayon::prelude::*;
        split_file_outputs.par_iter().for_each(|(file_path, content)| {
            if let Err(e) = fs::write(file_path, content) {
                eprintln!("WARNING: Failed to write diffable cs: {e}");
            }
        });

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
        indent: &str,
        dump_nested: bool,
        disasm: &Option<(Disassembler, Vec<u64>)>,
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

        if buf.is_empty() {
            if config.dump_assembly_name {
                writeln!(buf, "{indent}// Dll : {image_name}").ok();
            } else {
                buf.push_str(indent);
            }
        } else {
            if config.dump_assembly_name {
                writeln!(buf, "\n{indent}// Dll : {image_name}").ok();
            } else {
                writeln!(buf, "\n{indent}").ok();
            }
        }
        writeln!(buf, "{indent}// Namespace: {namespace}").ok();

        if config.dump_attribute {
            Self::write_custom_attributes(
                buf, executor, metadata, il2cpp,
                image_index, type_def.custom_attribute_index, type_def.token, indent,
            );

            if (type_def.flags & type_attributes::SERIALIZABLE) != 0 {
                writeln!(buf, "{indent}[Serializable]").ok();
            }
        }

        let visibility = type_def.flags & type_attributes::VISIBILITY_MASK;
        buf.push_str(indent);
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

        let type_name_raw = executor.get_type_def_name(&type_def, type_def_index, metadata, il2cpp, false, true);
        
        // Strip parent namespace prefixes from nested classes for clean inner definitions
        let display_name = if type_def.declaring_type_index >= 0 {
            let parts: Vec<&str> = type_name_raw.split('.').collect();
            parts.last().unwrap_or(&type_name_raw.as_str()).to_string()
        } else {
            type_name_raw
        };
        
        buf.push_str(&display_name);

        if !extends.is_empty() {
            write!(buf, " : {}", extends.join(", ")).ok();
        }

        if config.dump_type_def_index {
            writeln!(buf, " // TypeDefIndex: {type_def_index}\n{indent}{{").ok();
        } else {
            writeln!(buf, "\n{indent}{{").ok();
        }

        if config.dump_field && type_def.field_count > 0 {
            Self::dump_fields(buf, executor, metadata, il2cpp, config, &type_def, type_def_index, image_index, indent)?;
        }

        if config.dump_property && type_def.property_count > 0 {
            Self::dump_properties(buf, executor, metadata, il2cpp, config, &type_def, image_index, indent)?;
        }

        if config.dump_method && type_def.method_count > 0 {
            Self::dump_methods(buf, executor, metadata, il2cpp, config, &type_def, image_name, image_index, indent, disasm, type_def_index)?;
        }

        writeln!(buf, "{indent}}}").ok();

        if dump_nested && type_def.nested_type_count > 0 {
            for i in 0..type_def.nested_type_count as usize {
                let nested_idx = metadata.nested_type_indices[type_def.nested_types_start as usize + i];
                // Pass the original non-indented string to avoid horizontal spacing
                if let Err(e) = Self::dump_type(buf, executor, metadata, il2cpp, config, nested_idx as usize, image_index, image_name, indent, true, disasm) {
                    writeln!(buf, "/* Error dumping nested type: {e} */").ok();
                }
            }
        }

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
        indent: &str,
    ) -> Result<()> {
        writeln!(buf, "\n{indent}\t// Fields").ok();
        let field_end = type_def.field_start as usize + type_def.field_count as usize;

        for i in type_def.field_start as usize..field_end {
            let field_def = metadata.field_defs[i].clone();
            let field_type = il2cpp.types[field_def.type_index as usize].clone();
            let mut is_static = false;
            let mut is_const = false;

            if config.dump_attribute {
                let attr_indent = format!("{indent}\t");
                Self::write_custom_attributes(
                    buf, executor, metadata, il2cpp,
                    image_index, field_def.custom_attribute_index, field_def.token, &attr_indent,
                );
            }

            buf.push_str(indent);
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
        indent: &str,
    ) -> Result<()> {
        writeln!(buf, "\n{indent}\t// Properties").ok();
        let prop_end = type_def.property_start as usize + type_def.property_count as usize;

        for i in type_def.property_start as usize..prop_end {
            let property_def = metadata.property_defs[i].clone();

            if config.dump_attribute {
                let attr_indent = format!("{indent}\t");
                Self::write_custom_attributes(
                    buf, executor, metadata, il2cpp,
                    image_index, property_def.custom_attribute_index, property_def.token, &attr_indent,
                );
            }

            buf.push_str(indent);
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
        indent: &str,
        disasm: &Option<(Disassembler, Vec<u64>)>,
        type_def_index: usize,
    ) -> Result<()> {
        writeln!(buf, "\n{indent}\t// Methods").ok();
        let method_end = type_def.method_start as usize + type_def.method_count as usize;

        let field_ctx = if disasm.is_some() {
            let mut ctx = DisassemblyContext::new();

            if type_def.field_count > 0 {
                let field_end = type_def.field_start as usize + type_def.field_count as usize;
                for fi in type_def.field_start as usize..field_end {
                    let fd = metadata.field_defs[fi].clone();
                    let ft = il2cpp.types.get(fd.type_index as usize).cloned();
                    let is_static = ft.as_ref().map(|t| (t.attrs & crate::il2cpp::enums::field_attributes::STATIC) != 0).unwrap_or(false);
                    if is_static {
                        continue;
                    }
                    let offset = il2cpp.get_field_offset_from_index(
                        type_def_index,
                        fi - type_def.field_start as usize,
                        fi,
                        type_def.is_value_type(),
                        false,
                    );
                    if offset > 0 {
                        if let Ok(name) = metadata.get_string_from_index(fd.name_index) {
                            ctx.field_offsets.insert(offset, name);
                        }
                    }
                }
            }

            if type_def.vtable_count > 0 {
                let ptr_size = if il2cpp.is_32bit { 4i32 } else { 8i32 };
                for vi in 0..type_def.vtable_count as usize {
                    let vtable_index = type_def.vtable_start as usize + vi;
                    if vtable_index >= metadata.vtable_methods.len() { break; }

                    let encoded = metadata.vtable_methods[vtable_index];
                    let usage = (encoded & 0xE0000000) >> 29;
                    let index = if metadata.version >= 27.0 {
                        (encoded & 0x1FFFFFFE) >> 1
                    } else {
                        encoded & 0x1FFFFFFF
                    };

                    let method_name = if usage == 6 {
                        if (index as usize) < il2cpp.method_specs.len() {
                            let (tn, mn) = executor.get_method_spec_name(index as usize, metadata, il2cpp, false);
                            Some(format!("{}.{}", tn, mn))
                        } else { None }
                    } else {
                        if let Some(md) = metadata.method_defs.get(index as usize).cloned() {
                            if md.slot != 0xFFFF {
                                let declaring_idx = md.declaring_type as usize;
                                let tn = if let Some(td) = metadata.type_defs.get(declaring_idx).cloned() {
                                    metadata.get_string_from_index(td.name_index).unwrap_or_default()
                                } else { "?".to_string() };
                                let mn = metadata.get_string_from_index(md.name_index as i32).unwrap_or_default();
                                Some(format!("{}.{}", tn, mn))
                            } else { None }
                        } else { None }
                    };

                    if let Some(name) = method_name {
                        let vtable_byte_offset = (vi as i32) * ptr_size;
                        ctx.vtable_methods.insert(vtable_byte_offset, name);
                    }
                }
            }

            Some(ctx)
        } else {
            None
        };

        for i in type_def.method_start as usize..method_end {
            writeln!(buf).ok();
            let method_def = metadata.method_defs[i].clone();
            let is_abstract = (method_def.flags as u32 & method_attributes::ABSTRACT) != 0;

            if config.dump_attribute {
                let attr_indent = format!("{indent}\t");
                Self::write_custom_attributes(
                    buf, executor, metadata, il2cpp,
                    image_index, method_def.custom_attribute_index, method_def.token, &attr_indent,
                );
            }

            if config.dump_method_offset {
                let method_pointer = il2cpp.get_method_pointer(image_name, &method_def);
                if is_abstract {
                    write!(buf, "{indent}\t// ").ok();
                } else if method_pointer > 0 {
                    let rva = il2cpp.get_rva(method_pointer);
                    let offset = il2cpp.map_vatr(method_pointer).unwrap_or(rva);
                    write!(buf, "{indent}\t// RVA: 0x{rva:X} Offset: 0x{offset:X} VA: 0x{method_pointer:X}").ok();
                } else {
                    write!(buf, "{indent}\t// RVA: -1 Offset: -1").ok();
                }
                if method_def.slot != 0xFFFF {
                    write!(buf, " Slot: {}", method_def.slot).ok();
                }
                writeln!(buf).ok();
            }

            buf.push_str(indent);
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
            } else if let Some((disassembler, sorted_addrs)) = disasm {
                let method_pointer = il2cpp.get_method_pointer(image_name, &method_def);
                if method_pointer > 0 {
                    let rva = il2cpp.get_rva(method_pointer);
                    let body_size = il2cpp.get_method_body_size(rva, sorted_addrs);
                    if let Some(bytes) = il2cpp.read_bytes_at_rva(rva, body_size) {
                        let method_ctx = if let Some(ref base_ctx) = field_ctx {
                            let mut ctx = DisassemblyContext::new();
                            ctx.field_offsets = base_ctx.field_offsets.clone();
                            ctx.string_literals = base_ctx.string_literals.clone();
                            ctx.type_names = base_ctx.type_names.clone();
                            ctx.method_refs = base_ctx.method_refs.clone();
                            ctx.field_refs = base_ctx.field_refs.clone();
                            ctx.vtable_methods = base_ctx.vtable_methods.clone();

                            let is_static = (method_def.flags as u32 & method_attributes::STATIC) != 0;
                            let is_arm = matches!(disassembler.arch(), disassembler::Architecture::Arm64 | disassembler::Architecture::Arm32);
                            let is_arm64 = matches!(disassembler.arch(), disassembler::Architecture::Arm64);

                            if is_arm64 || is_arm {
                                let mut reg_slot = if is_static { 0usize } else {
                                    ctx.register_names.insert("x0".to_string(), "this".to_string());
                                    ctx.register_names.insert("x19".to_string(), "this".to_string());
                                    1
                                };

                                for j in 0..method_def.parameter_count as usize {
                                    if reg_slot > 7 { break; }
                                    let param_def = metadata.parameter_defs[method_def.parameter_start as usize + j].clone();
                                    if let Ok(pname) = metadata.get_string_from_index(param_def.name_index) {
                                        ctx.register_names.insert(format!("x{}", reg_slot), pname);
                                    }
                                    reg_slot += 1;
                                }
                            } else {
                                if !is_static {
                                    ctx.register_names.insert("ecx".to_string(), "this".to_string());
                                    ctx.register_names.insert("rcx".to_string(), "this".to_string());
                                }
                            }

                            Some(ctx)
                        } else {
                            None
                        };

                        let asm_block = disassembler.format_method_body(
                            &bytes, rva, config.max_disassembly_instructions, indent,
                            method_ctx.as_ref().or(field_ctx.as_ref()),
                            config.dump_disassembly_hex_bytes,
                            config.dump_disassembly_field_names,
                            config.dump_disassembly_annotations,
                            config.dump_disassembly_cfg,
                        );
                        if !asm_block.is_empty() {
                            writeln!(buf, ") {{").ok();
                            buf.push_str(&asm_block);
                            writeln!(buf, "{indent}\t}}").ok();
                        } else {
                            writeln!(buf, ") {{ }}").ok();
                        }
                    } else {
                        writeln!(buf, ") {{ }}").ok();
                    }
                } else {
                    writeln!(buf, ") {{ }}").ok();
                }
            } else {
                writeln!(buf, ") {{ }}").ok();
            }

            if let Some(method_specs) = il2cpp.method_definition_method_specs.get(&i).cloned() {
                writeln!(buf, "{indent}\t/* GenericInstMethod :").ok();

                let mut groups: HashMap<u64, Vec<usize>> = HashMap::new();
                for spec_idx in &method_specs {
                    let ptr = il2cpp.method_spec_generic_method_pointers.get(spec_idx).copied().unwrap_or(0);
                    groups.entry(ptr).or_default().push(*spec_idx);
                }

                for (ptr, spec_indices) in &groups {
                    writeln!(buf, "{indent}\t|").ok();
                    if *ptr > 0 {
                        let rva = il2cpp.get_rva(*ptr);
                        let offset = il2cpp.map_vatr(*ptr).unwrap_or(rva);
                        writeln!(buf, "{indent}\t|-RVA: 0x{rva:X} Offset: 0x{offset:X} VA: 0x{ptr:X}").ok();
                    } else {
                        writeln!(buf, "{indent}\t|-RVA: 0x0 Offset: 0x0").ok();
                    }

                    for spec_idx in spec_indices {
                        let (type_name, method_name) = executor.get_method_spec_name(*spec_idx, metadata, il2cpp, false);
                        writeln!(buf, "{indent}\t|-{type_name}.{method_name}").ok();
                    }
                }

                writeln!(buf, "{indent}\t*/").ok();
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

    fn detect_box_helpers(
        disassembler: &Disassembler,
        il2cpp: &Il2Cpp,
        sorted_addrs: &[u64],
        config: &Config,
    ) -> Vec<u64> {
        if sorted_addrs.is_empty() {
            return Vec::new();
        }

        let arch = disassembler.arch();
        if !matches!(arch, disassembler::Architecture::Arm64) {
            return Vec::new();
        }

        let sample_count = sorted_addrs.len().min(2000);
        let stride = (sorted_addrs.len() / sample_count.max(1)).max(1);

        let mut total_calls: HashMap<u64, u32> = HashMap::new();
        let mut typeinfo_arg_calls: HashMap<u64, u32> = HashMap::new();

        let mut sampled = 0usize;
        let mut idx = 0usize;
        while idx < sorted_addrs.len() && sampled < sample_count {
            let rva = sorted_addrs[idx];
            idx += stride;

            let body_size = il2cpp.get_method_body_size(rva, sorted_addrs);
            let probe = body_size.min(config.max_disassembly_instructions.saturating_mul(4).max(256));
            let bytes = match il2cpp.read_bytes_at_rva(rva, probe) {
                Some(b) => b,
                None => continue,
            };

            let insns = disassembler.disassemble(&bytes, rva, config.max_disassembly_instructions.min(120));
            if insns.is_empty() { continue; }
            sampled += 1;

            let prop = disassembler::analyze_propagation(&insns, arch);

            for insn in &insns {
                if !insn.is_call { continue; }
                let target = match insn.call_target { Some(t) => t, None => continue };
                if disassembler.has_method_name(target) { continue; }

                *total_calls.entry(target).or_insert(0) += 1;

                if let Some(&va) = prop.call_arg_x0_va.get(&insn.address) {
                    if disassembler.type_name_at_va(va).is_some() {
                        *typeinfo_arg_calls.entry(target).or_insert(0) += 1;
                    }
                }
            }
        }

        let mut candidates: Vec<(u64, u32, u32)> = typeinfo_arg_calls.iter()
            .map(|(&rva, &hits)| (rva, hits, *total_calls.get(&rva).unwrap_or(&0)))
            .filter(|(_, hits, total)| *hits >= 8 && *total >= 8 && (*hits as f32 / *total as f32) >= 0.8)
            .collect();
        candidates.sort_by(|a, b| b.1.cmp(&a.1));
        candidates.truncate(4);
        candidates.into_iter().map(|(rva, _, _)| rva).collect()
    }

    fn detect_unbox_helpers(
        disassembler: &Disassembler,
        il2cpp: &Il2Cpp,
        sorted_addrs: &[u64],
        config: &Config,
        exclude: &HashSet<u64>,
    ) -> Vec<u64> {
        if sorted_addrs.is_empty() {
            return Vec::new();
        }

        let arch = disassembler.arch();
        if !matches!(arch, disassembler::Architecture::Arm64) {
            return Vec::new();
        }

        let sample_count = sorted_addrs.len().min(2000);
        let stride = (sorted_addrs.len() / sample_count.max(1)).max(1);

        let mut total_calls: HashMap<u64, u32> = HashMap::new();
        let mut x1_typeinfo_calls: HashMap<u64, u32> = HashMap::new();

        let mut sampled = 0usize;
        let mut idx = 0usize;
        while idx < sorted_addrs.len() && sampled < sample_count {
            let rva = sorted_addrs[idx];
            idx += stride;

            let body_size = il2cpp.get_method_body_size(rva, sorted_addrs);
            let probe = body_size.min(config.max_disassembly_instructions.saturating_mul(4).max(256));
            let bytes = match il2cpp.read_bytes_at_rva(rva, probe) {
                Some(b) => b,
                None => continue,
            };

            let insns = disassembler.disassemble(&bytes, rva, config.max_disassembly_instructions.min(120));
            if insns.is_empty() { continue; }
            sampled += 1;

            let prop = disassembler::analyze_propagation(&insns, arch);

            for insn in &insns {
                if !insn.is_call { continue; }
                let target = match insn.call_target { Some(t) => t, None => continue };
                if disassembler.has_method_name(target) { continue; }
                if exclude.contains(&target) { continue; }

                *total_calls.entry(target).or_insert(0) += 1;

                let x1_is_type = prop.call_arg_x1_va.get(&insn.address)
                    .and_then(|&va| disassembler.type_name_at_va(va))
                    .is_some();
                let x0_is_type = prop.call_arg_x0_va.get(&insn.address)
                    .and_then(|&va| disassembler.type_name_at_va(va))
                    .is_some();

                if x1_is_type && !x0_is_type {
                    *x1_typeinfo_calls.entry(target).or_insert(0) += 1;
                }
            }
        }

        let mut candidates: Vec<(u64, u32, u32)> = x1_typeinfo_calls.iter()
            .map(|(&rva, &hits)| (rva, hits, *total_calls.get(&rva).unwrap_or(&0)))
            .filter(|(_, hits, total)| *hits >= 6 && *total >= 6 && (*hits as f32 / *total as f32) >= 0.75)
            .collect();
        candidates.sort_by(|a, b| b.1.cmp(&a.1));
        candidates.truncate(3);
        candidates.into_iter().map(|(rva, _, _)| rva).collect()
    }

    fn detect_string_new_wrapper(
        disassembler: &Disassembler,
        il2cpp: &Il2Cpp,
        sorted_addrs: &[u64],
        total_string_lits: usize,
        config: &Config,
    ) -> Vec<u64> {
        if total_string_lits == 0 || sorted_addrs.is_empty() {
            return Vec::new();
        }

        let arch = disassembler.arch();
        if !matches!(arch, disassembler::Architecture::Arm64) {
            return Vec::new();
        }

        let sample_count = sorted_addrs.len().min(2000);
        let stride = (sorted_addrs.len() / sample_count.max(1)).max(1);

        let mut total_calls: HashMap<u64, u32> = HashMap::new();
        let mut string_arg_calls: HashMap<u64, u32> = HashMap::new();

        let mut sampled = 0usize;
        let mut idx = 0usize;
        while idx < sorted_addrs.len() && sampled < sample_count {
            let rva = sorted_addrs[idx];
            idx += stride;

            let body_size = il2cpp.get_method_body_size(rva, sorted_addrs);
            let probe = body_size.min(config.max_disassembly_instructions.saturating_mul(4).max(256));
            let bytes = match il2cpp.read_bytes_at_rva(rva, probe) {
                Some(b) => b,
                None => continue,
            };

            let insns = disassembler.disassemble(&bytes, rva, config.max_disassembly_instructions.min(120));
            if insns.is_empty() { continue; }
            sampled += 1;

            let prop = disassembler::analyze_propagation(&insns, arch);

            for insn in &insns {
                if !insn.is_call { continue; }
                let target = match insn.call_target { Some(t) => t, None => continue };
                if disassembler.has_method_name(target) { continue; }

                *total_calls.entry(target).or_insert(0) += 1;

                if let Some(&w0) = prop.call_arg_w0.get(&insn.address) {
                    if (w0 as usize) < total_string_lits {
                        *string_arg_calls.entry(target).or_insert(0) += 1;
                    }
                }
            }
        }

        let mut candidates: Vec<(u64, u32, u32)> = string_arg_calls.iter()
            .map(|(&rva, &hits)| (rva, hits, *total_calls.get(&rva).unwrap_or(&0)))
            .filter(|(_, hits, total)| *hits >= 5 && *total >= 5 && (*hits as f32 / *total as f32) >= 0.6)
            .collect();
        candidates.sort_by(|a, b| b.1.cmp(&a.1));
        candidates.truncate(2);
        candidates.into_iter().map(|(rva, _, _)| rva).collect()
    }

    fn build_legacy_annotations(
        disassembler: &mut Disassembler,
        executor: &mut Il2CppExecutor,
        metadata: &mut Metadata,
        il2cpp: &mut Il2Cpp,
        image_defs: &[Il2CppImageDefinition],
    ) {
        if metadata.metadata_usage_dic.is_empty() { return; }
        let usage_dic = metadata.metadata_usage_dic.clone();

        let _type_def_image_map: HashMap<usize, String> = {
            let mut m = HashMap::new();
            for img in image_defs {
                let name = metadata.get_string_from_index(img.name_index).unwrap_or_default();
                let end = img.type_start as usize + img.type_count as usize;
                for ti in img.type_start as usize..end {
                    m.insert(ti, name.clone());
                }
            }
            m
        };

        for (usage_type, entries) in &usage_dic {
            for (dest_index, source_index) in entries {
                let dest = *dest_index as usize;
                if dest >= il2cpp.metadata_usages.len() { continue; }
                let address = il2cpp.metadata_usages[dest];
                if address == 0 { continue; }
                let rva = il2cpp.get_rva(address);
                let src = *source_index as usize;

                match *usage_type {
                    1 => {
                        if src < il2cpp.types.len() {
                            let type_ref = il2cpp.types[src].clone();
                            let type_name = executor.get_type_name(&type_ref, metadata, il2cpp, true, false);
                            disassembler.add_type_info(rva, type_name);
                            Self::register_static_fields(disassembler, metadata, il2cpp, &type_ref, rva);
                        }
                    }
                    2 => {
                        if src < il2cpp.types.len() {
                            let type_ref = il2cpp.types[src].clone();
                            let type_name = executor.get_type_name(&type_ref, metadata, il2cpp, true, false);
                            disassembler.add_type_info(rva, type_name);
                            Self::register_static_fields(disassembler, metadata, il2cpp, &type_ref, rva);
                        }
                    }
                    3 => {
                        if let Some(method_def) = metadata.method_defs.get(src).cloned() {
                            if let Some(type_def) = metadata.type_defs.get(method_def.declaring_type as usize).cloned() {
                                let td_idx = method_def.declaring_type as usize;
                                let type_name = executor.get_type_def_name(&type_def, td_idx, metadata, il2cpp, true, true);
                                let method_name = metadata.get_string_from_index(method_def.name_index as i32).unwrap_or_default();
                                disassembler.add_method_ref(rva, format!("{}.{}()", type_name, method_name));
                            }
                        }
                    }
                    4 => {
                        if src < metadata.field_refs.len() {
                            let field_ref = metadata.field_refs[src].clone();
                            if (field_ref.type_index as usize) < il2cpp.types.len() {
                                let il2cpp_type = il2cpp.types[field_ref.type_index as usize].clone();
                                let type_name = executor.get_type_name(&il2cpp_type, metadata, il2cpp, true, false);
                                let klass_idx = il2cpp_type.klass_index() as usize;
                                if let Some(td) = metadata.type_defs.get(klass_idx) {
                                    let field_idx = td.field_start as usize + field_ref.field_index as usize;
                                    if let Some(fd) = metadata.field_defs.get(field_idx) {
                                        let field_name = metadata.get_string_from_index(fd.name_index).unwrap_or_default();
                                        disassembler.add_field_ref(rva, format!("{}.{}", type_name, field_name));
                                    }
                                }
                            }
                        }
                    }
                    5 => {
                        if let Ok(string_literal) = metadata.get_string_literal_from_index(src) {
                            if !string_literal.is_empty() {
                                disassembler.add_string_literal(rva, string_literal);
                            }
                        }
                    }
                    6 => {
                        if src < il2cpp.method_specs.len() {
                            let (spec_type_name, spec_method_name) = executor.get_method_spec_name(src, metadata, il2cpp, true);
                            disassembler.add_method_ref(rva, format!("{}.{}()", spec_type_name, spec_method_name));
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    fn register_static_fields(
        disassembler: &mut Disassembler,
        metadata: &mut Metadata,
        il2cpp: &mut Il2Cpp,
        type_ref: &Il2CppType,
        type_info_rva: u64,
    ) {
        let klass_idx = type_ref.klass_index() as usize;
        let type_def = match metadata.type_defs.get(klass_idx).cloned() {
            Some(td) => td,
            None => return,
        };
        if type_def.field_count == 0 { return; }

        let type_short_name = metadata.get_string_from_index(type_def.name_index).unwrap_or_default();
        let field_end = type_def.field_start as usize + type_def.field_count as usize;

        for fi in type_def.field_start as usize..field_end {
            let fd = match metadata.field_defs.get(fi).cloned() {
                Some(f) => f,
                None => continue,
            };
            let ft = match il2cpp.types.get(fd.type_index as usize).cloned() {
                Some(t) => t,
                None => continue,
            };
            let is_static = (ft.attrs & crate::il2cpp::enums::field_attributes::STATIC) != 0;
            if !is_static { continue; }
            let is_literal = (ft.attrs & crate::il2cpp::enums::field_attributes::LITERAL) != 0;
            if is_literal { continue; }

            let offset = il2cpp.get_field_offset_from_index(
                klass_idx,
                fi - type_def.field_start as usize,
                fi,
                type_def.is_value_type(),
                true,
            );
            if offset <= 0 { continue; }

            if let Ok(field_name) = metadata.get_string_from_index(fd.name_index) {
                disassembler.add_static_field(type_info_rva, &type_short_name, offset as i64, &field_name);
            }
        }
    }

    fn build_v27_annotations(
        disassembler: &mut Disassembler,
        executor: &mut Il2CppExecutor,
        metadata: &mut Metadata,
        il2cpp: &mut Il2Cpp,
        image_defs: &[Il2CppImageDefinition],
    ) {
        let pointer_size = if il2cpp.is_32bit { 4u64 } else { 8u64 };
        let data_sections = il2cpp.data_sections.clone();

        let _type_def_image_map: HashMap<usize, String> = {
            let mut m = HashMap::new();
            for img in image_defs {
                let name = metadata.get_string_from_index(img.name_index).unwrap_or_default();
                let end = img.type_start as usize + img.type_count as usize;
                for ti in img.type_start as usize..end {
                    m.insert(ti, name.clone());
                }
            }
            m
        };

        for sec in &data_sections {
            let sec_end = std::cmp::min(sec.offset_end, il2cpp.stream.len() as u64).saturating_sub(pointer_size);
            let mut pos = sec.offset;

            while pos < sec_end {
                il2cpp.stream.set_position(pos);
                let metadata_value = if il2cpp.is_32bit {
                    il2cpp.stream.read_u32().unwrap_or(0) as u64
                } else {
                    il2cpp.stream.read_u64().unwrap_or(0)
                };
                let saved_pos = il2cpp.stream.position();
                pos = saved_pos;

                if metadata_value >= u32::MAX as u64 { continue; }
                let encoded_token = metadata_value as u32;
                let usage = (encoded_token & 0xE0000000) >> 29;
                if usage == 0 || usage > 6 { continue; }
                let decoded_index = (encoded_token & 0x1FFFFFFE) >> 1;
                let expected = ((usage << 29) | (decoded_index << 1)) + 1;
                if metadata_value != expected as u64 { continue; }

                let addr = pos - pointer_size;
                let va = il2cpp.map_rtva(addr);
                if va == 0 { continue; }
                let rva = il2cpp.get_rva(va);

                match usage {
                    1 => {
                        if (decoded_index as usize) < il2cpp.types.len() {
                            let type_ref = il2cpp.types[decoded_index as usize].clone();
                            let type_name = executor.get_type_name(&type_ref, metadata, il2cpp, true, false);
                            disassembler.add_type_info(rva, type_name);
                            Self::register_static_fields(disassembler, metadata, il2cpp, &type_ref, rva);
                        }
                    }
                    2 => {
                        if (decoded_index as usize) < il2cpp.types.len() {
                            let type_ref = il2cpp.types[decoded_index as usize].clone();
                            let type_name = executor.get_type_name(&type_ref, metadata, il2cpp, true, false);
                            disassembler.add_type_info(rva, type_name);
                            Self::register_static_fields(disassembler, metadata, il2cpp, &type_ref, rva);
                        }
                    }
                    3 => {
                        if let Some(method_def) = metadata.method_defs.get(decoded_index as usize).cloned() {
                            if let Some(type_def) = metadata.type_defs.get(method_def.declaring_type as usize).cloned() {
                                let td_idx = method_def.declaring_type as usize;
                                let type_name = executor.get_type_def_name(&type_def, td_idx, metadata, il2cpp, true, true);
                                let method_name = metadata.get_string_from_index(method_def.name_index as i32).unwrap_or_default();
                                disassembler.add_method_ref(rva, format!("{}.{}()", type_name, method_name));
                            }
                        }
                    }
                    4 => {
                        if (decoded_index as usize) < metadata.field_refs.len() {
                            let field_ref = metadata.field_refs[decoded_index as usize].clone();
                            if (field_ref.type_index as usize) < il2cpp.types.len() {
                                let il2cpp_type = il2cpp.types[field_ref.type_index as usize].clone();
                                let type_name = executor.get_type_name(&il2cpp_type, metadata, il2cpp, true, false);
                                let klass_idx = il2cpp_type.klass_index() as usize;
                                if let Some(td) = metadata.type_defs.get(klass_idx) {
                                    let field_idx = td.field_start as usize + field_ref.field_index as usize;
                                    if let Some(fd) = metadata.field_defs.get(field_idx) {
                                        let field_name = metadata.get_string_from_index(fd.name_index).unwrap_or_default();
                                        disassembler.add_field_ref(rva, format!("{}.{}", type_name, field_name));
                                    }
                                }
                            }
                        }
                    }
                    5 => {
                        if let Ok(string_literal) = metadata.get_string_literal_from_index(decoded_index as usize) {
                            if !string_literal.is_empty() {
                                disassembler.add_string_literal(rva, string_literal);
                            }
                        }
                    }
                    6 => {
                        if (decoded_index as usize) < il2cpp.method_specs.len() {
                            let (spec_type_name, spec_method_name) = executor.get_method_spec_name(decoded_index as usize, metadata, il2cpp, true);
                            disassembler.add_method_ref(rva, format!("{}.{}()", spec_type_name, spec_method_name));
                        }
                    }
                    _ => {}
                }

                if il2cpp.stream.position() != saved_pos {
                    il2cpp.stream.set_position(saved_pos);
                }
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
