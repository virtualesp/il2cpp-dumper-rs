use std::collections::{HashMap, HashSet, BTreeMap};
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
use super::header_constants;

static KEYWORDS: &[&str] = &[
    "klass", "monitor", "register", "_cs", "auto", "friend", "template",
    "flat", "default", "_ds", "interrupt", "unsigned", "signed", "asm",
    "if", "case", "break", "continue", "do", "new", "_", "short",
    "union", "class", "namespace",
];

static SPECIAL_KEYWORDS: &[&str] = &["inline", "near", "far"];

struct StructFieldInfo {
    field_type_name: String,
    field_name: String,
    is_value_type: bool,
    is_custom_type: bool,
}

struct StructVTableMethodInfo {
    method_name: String,
}

struct StructRGCTXInfo {
    rgctx_type: i32,
    type_name: Option<String>,
    class_name: Option<String>,
    method_name: Option<String>,
}

struct StructInfo {
    type_name: String,
    is_value_type: bool,
    parent: Option<String>,
    fields: Vec<StructFieldInfo>,
    static_fields: Vec<StructFieldInfo>,
    vtable_methods: Vec<Option<StructVTableMethodInfo>>,
    rgctxs: Vec<StructRGCTXInfo>,
}

pub struct StructGenerator;

/// Mutable context for header generation (il2cpp.h).
/// Holds state that grows during generic class discovery.
struct HeaderGenCtx {
    /// Maps generic_class pointer → specialized struct name (e.g., "List_1_System_Int32")
    generic_class_struct_name_dic: HashMap<u64, String>,
    /// HashSet for dedup of struct names
    struct_name_hash_set: HashSet<String>,
    /// Newly discovered generic class pointers found during field parsing
    newly_discovered: Vec<u64>,
}

impl StructGenerator {
    pub fn write_all(
        executor: &mut Il2CppExecutor,
        metadata: &mut Metadata,
        il2cpp: &mut Il2Cpp,
        output_dir: &str,
    ) -> Result<()> {
        let output_path = Path::new(output_dir);

        let script_json = Self::build_script_json(executor, metadata, il2cpp)?;
        let string_literal_json = Self::build_string_literal_json(metadata)?;
        let header = Self::build_header(executor, metadata, il2cpp)?;

        use rayon::prelude::*;
        let writes: Vec<(&str, &[u8])> = vec![
            ("script.json", script_json.as_bytes()),
            ("stringliteral.json", string_literal_json.as_bytes()),
            ("il2cpp.h", header.as_bytes()),
        ];
        writes.par_iter().for_each(|(name, data)| {
            let path = output_path.join(name);
            if let Err(e) = fs::write(&path, data) {
                eprintln!("WARNING: Failed to write {name}: {e}");
            }
        });

        Ok(())
    }

    fn build_script_json(
        executor: &mut Il2CppExecutor,
        metadata: &mut Metadata,
        il2cpp: &mut Il2Cpp,
    ) -> Result<String> {
        let mut script = ScriptJson::new();
        let mut addresses_set: HashSet<u64> = HashSet::new();
        let mut method_info_cache: HashSet<u64> = HashSet::new();

        let struct_name_dic = Self::build_struct_name_dic(executor, metadata, il2cpp);
        let type_def_image_names = Self::build_type_def_image_names(metadata);

        let image_defs = metadata.image_defs.clone();
        for image_def in &image_defs {
            let image_name = metadata.get_string_from_index(image_def.name_index)?;
            let type_end = image_def.type_start as usize + image_def.type_count as usize;

            for type_def_index in image_def.type_start as usize..type_end {
                let type_def = metadata.type_defs[type_def_index].clone();
                let type_name = executor.get_type_def_name(&type_def, type_def_index, metadata, il2cpp, true, true);

                let method_end = type_def.method_start as usize + type_def.method_count as usize;
                for method_index in type_def.method_start as usize..method_end {
                    let method_def = metadata.method_defs[method_index].clone();
                    let method_name_raw = metadata.get_string_from_index(method_def.name_index as i32)?;
                    let method_pointer = il2cpp.get_method_pointer(&image_name, &method_def);

                    if method_pointer > 0 {
                        let rva = il2cpp.get_rva(method_pointer);
                        addresses_set.insert(rva);
                        let method_full_name = format!("{}$${}", type_name, method_name_raw);
                        let (signature, type_sig) = Self::build_method_signature(
                            executor, metadata, il2cpp, &method_def, &type_def,
                            &method_full_name, &struct_name_dic, None,
                        );
                        script.script_methods.push(ScriptMethod {
                            address: rva,
                            name: method_full_name,
                            signature,
                            type_signature: type_sig,
                        });
                    }

                    if let Some(spec_indices) = il2cpp.method_definition_method_specs.get(&method_index).cloned() {
                        for spec_idx in &spec_indices {
                            let spec_ptr = il2cpp.method_spec_generic_method_pointers.get(spec_idx).copied().unwrap_or(0);
                            if spec_ptr == 0 { continue; }
                            let spec_rva = il2cpp.get_rva(spec_ptr);
                            addresses_set.insert(spec_rva);

                            let _ = method_info_cache.insert(spec_ptr);

                            let (spec_type_name, spec_method_name) = executor.get_method_spec_name(*spec_idx, metadata, il2cpp, true);
                            let method_full_name = format!("{}$${}", spec_type_name, spec_method_name);

                            let (class_inst, method_inst) = executor.get_method_spec_generic_context(*spec_idx, il2cpp);
                            let generic_context = Il2CppGenericContext { class_inst, method_inst };

                            let (signature, type_sig) = Self::build_method_signature(
                                executor, metadata, il2cpp, &method_def, &type_def,
                                &method_full_name, &struct_name_dic, Some(&generic_context),
                            );

                            script.script_methods.push(ScriptMethod {
                                address: spec_rva,
                                name: method_full_name,
                                signature,
                                type_signature: type_sig,
                            });
                        }
                    }
                }
            }
        }

        Self::collect_all_addresses(&mut addresses_set, executor, il2cpp);

        if il2cpp.version >= 27.0 {
            Self::scan_v27_metadata_usages(&mut script, executor, metadata, il2cpp, &struct_name_dic, &type_def_image_names);
        } else if il2cpp.version > 16.0 {
            Self::add_metadata_usages(&mut script, executor, metadata, il2cpp, &struct_name_dic, &type_def_image_names);
        }

        let mut sorted_addresses: Vec<u64> = addresses_set.into_iter().filter(|a| *a > 0).collect();
        sorted_addresses.sort_unstable();
        script.addresses = sorted_addresses;

        let json = script.to_json().map_err(|e| crate::error::Error::Other(e.to_string()))?;
        Ok(json)
    }

    fn collect_all_addresses(
        addresses_set: &mut HashSet<u64>,
        executor: &Il2CppExecutor,
        il2cpp: &Il2Cpp,
    ) {
        if il2cpp.version >= 24.2 {
            for pointers in il2cpp.code_gen_module_method_pointers.values() {
                for ptr in pointers {
                    if *ptr > 0 { addresses_set.insert(il2cpp.get_rva(*ptr)); }
                }
            }
        } else {
            for ptr in &il2cpp.method_pointers {
                if *ptr > 0 { addresses_set.insert(il2cpp.get_rva(*ptr)); }
            }
        }

        for ptr in &il2cpp.generic_method_pointers {
            if *ptr > 0 { addresses_set.insert(il2cpp.get_rva(*ptr)); }
        }
        for ptr in &il2cpp.invoker_pointers {
            if *ptr > 0 { addresses_set.insert(il2cpp.get_rva(*ptr)); }
        }

        if il2cpp.version < 29.0 {
            for ptr in &executor.custom_attribute_generators {
                if *ptr > 0 { addresses_set.insert(il2cpp.get_rva(*ptr)); }
            }
        }

        if il2cpp.version >= 22.0 {
            for ptr in &il2cpp.reverse_pinvoke_wrappers {
                if *ptr > 0 { addresses_set.insert(il2cpp.get_rva(*ptr)); }
            }
            for ptr in &il2cpp.unresolved_virtual_call_pointers {
                if *ptr > 0 { addresses_set.insert(il2cpp.get_rva(*ptr)); }
            }
        }
    }

    fn build_method_signature(
        executor: &mut Il2CppExecutor,
        metadata: &mut Metadata,
        il2cpp: &mut Il2Cpp,
        method_def: &Il2CppMethodDefinition,
        type_def: &Il2CppTypeDefinition,
        method_full_name: &str,
        struct_name_dic: &HashMap<usize, String>,
        generic_context: Option<&Il2CppGenericContext>,
    ) -> (String, String) {
        let mut type_signature_parts: Vec<Il2CppTypeEnum> = Vec::new();

        let method_return_type = il2cpp.types[method_def.return_type as usize].clone();
        let return_type_c = Self::parse_type(&method_return_type, struct_name_dic, executor, metadata, il2cpp, generic_context, None);
        let return_c = if method_return_type.byref == 1 {
            type_signature_parts.push(Il2CppTypeEnum::Ptr);
            format!("{}*", return_type_c)
        } else {
            let te = Il2CppTypeEnum::from_u8(method_return_type.type_enum).unwrap_or(Il2CppTypeEnum::Void);
            type_signature_parts.push(if method_return_type.byref == 1 { Il2CppTypeEnum::Ptr } else { te });
            return_type_c
        };

        let mut param_strs = Vec::new();

        let is_static = (method_def.flags as u32 & method_attributes::STATIC) != 0;
        if !is_static {
            let byval_type = il2cpp.types[type_def.byval_type_index as usize].clone();
            let this_type = Self::parse_type(
                &il2cpp.types[type_def.byval_type_index as usize].clone(),
                struct_name_dic, executor, metadata, il2cpp, generic_context, None,
            );
            let te = Il2CppTypeEnum::from_u8(byval_type.type_enum)
                .unwrap_or(Il2CppTypeEnum::Object);
            type_signature_parts.push(te);
            param_strs.push(format!("{} __this", this_type));
        } else if il2cpp.version <= 24.0 {
            type_signature_parts.push(Il2CppTypeEnum::Ptr);
            param_strs.push("Il2CppObject* __this".to_string());
        }

        for j in 0..method_def.parameter_count as usize {
            let param_def = metadata.parameter_defs[method_def.parameter_start as usize + j].clone();
            let param_name = metadata.get_string_from_index(param_def.name_index)
                .unwrap_or_else(|_| "param".to_string());
            let param_type = il2cpp.types[param_def.type_index as usize].clone();
            let param_c_type = Self::parse_type(&param_type, struct_name_dic, executor, metadata, il2cpp, generic_context, None);
            let (param_c, sig_type) = if param_type.byref == 1 {
                (format!("{}*", param_c_type), Il2CppTypeEnum::Ptr)
            } else {
                let te = Il2CppTypeEnum::from_u8(param_type.type_enum).unwrap_or(Il2CppTypeEnum::Object);
                (param_c_type, te)
            };
            type_signature_parts.push(sig_type);
            param_strs.push(format!("{} {}", param_c, fix_name(&param_name)));
        }

        type_signature_parts.push(Il2CppTypeEnum::Ptr);
        param_strs.push("const MethodInfo* method".to_string());

        let signature = format!("{} {} ({});",
            return_c, fix_name(method_full_name), param_strs.join(", "));
        let type_sig = get_method_type_signature(&type_signature_parts);

        (signature, type_sig)
    }

    fn resolve_generic_type_var(
        il2cpp: &mut Il2Cpp,
        inst_addr: u64,
        param_num: u32,
    ) -> Option<Il2CppType> {
        if inst_addr == 0 { return None; }
        let generic_inst = il2cpp.read_generic_inst(inst_addr).ok()?;
        let pointers = il2cpp.read_ptr_array(generic_inst.type_argv, generic_inst.type_argc).ok()?;
        let pointer = *pointers.get(param_num as usize)?;
        il2cpp.get_il2cpp_type(pointer).cloned()
    }

    fn parse_type(
        il2cpp_type: &Il2CppType,
        struct_name_dic: &HashMap<usize, String>,
        executor: &mut Il2CppExecutor,
        metadata: &mut Metadata,
        il2cpp: &mut Il2Cpp,
        context: Option<&Il2CppGenericContext>,
        mut hdr_ctx: Option<&mut HeaderGenCtx>,
    ) -> String {
        let te = Il2CppTypeEnum::from_u8(il2cpp_type.type_enum);
        match te {
            Some(Il2CppTypeEnum::Void) => "void".to_string(),
            Some(Il2CppTypeEnum::Boolean) => "bool".to_string(),
            Some(Il2CppTypeEnum::Char) => "uint16_t".to_string(),
            Some(Il2CppTypeEnum::I1) => "int8_t".to_string(),
            Some(Il2CppTypeEnum::U1) => "uint8_t".to_string(),
            Some(Il2CppTypeEnum::I2) => "int16_t".to_string(),
            Some(Il2CppTypeEnum::U2) => "uint16_t".to_string(),
            Some(Il2CppTypeEnum::I4) => "int32_t".to_string(),
            Some(Il2CppTypeEnum::U4) => "uint32_t".to_string(),
            Some(Il2CppTypeEnum::I8) => "int64_t".to_string(),
            Some(Il2CppTypeEnum::U8) => "uint64_t".to_string(),
            Some(Il2CppTypeEnum::R4) => "float".to_string(),
            Some(Il2CppTypeEnum::R8) => "double".to_string(),
            Some(Il2CppTypeEnum::String) => "System_String_o*".to_string(),
            Some(Il2CppTypeEnum::I) => "intptr_t".to_string(),
            Some(Il2CppTypeEnum::U) => "uintptr_t".to_string(),
            Some(Il2CppTypeEnum::Object) | Some(Il2CppTypeEnum::TypedByRef) => "Il2CppObject*".to_string(),
            Some(Il2CppTypeEnum::ValueType) => {
                let klass_idx = il2cpp_type.klass_index() as usize;
                if let Some(td) = metadata.type_defs.get(klass_idx) {
                    if td.is_enum() {
                        if let Some(elem_type) = il2cpp.types.get(td.element_type_index as usize).cloned() {
                            return Self::parse_type(&elem_type, struct_name_dic, executor, metadata, il2cpp, context, hdr_ctx);
                        }
                    }
                    if let Some(sn) = struct_name_dic.get(&klass_idx) {
                        return format!("{}_o", sn);
                    }
                }
                "Il2CppObject*".to_string()
            }
            Some(Il2CppTypeEnum::Class) => {
                let klass_idx = il2cpp_type.klass_index() as usize;
                if let Some(sn) = struct_name_dic.get(&klass_idx) {
                    format!("{}_o*", sn)
                } else {
                    "Il2CppObject*".to_string()
                }
            }
            Some(Il2CppTypeEnum::SzArray) | Some(Il2CppTypeEnum::Array) => {
                if il2cpp_type.datapoint != 0 {
                    if let Some(element_type) = il2cpp.types.get(il2cpp_type.datapoint as usize).cloned() {
                        let elem_struct_name = Self::get_il2cpp_struct_name(&element_type, struct_name_dic, il2cpp, context);
                        return format!("{}_array*", elem_struct_name);
                    }
                }
                "Il2CppArray*".to_string()
            }
            Some(Il2CppTypeEnum::GenericInst) => {
                let generic_class_ptr = il2cpp_type.generic_class();
                if generic_class_ptr != 0 {
                    if let Ok(generic_class) = il2cpp.read_generic_class(generic_class_ptr) {
                        if let Some((type_def, td_idx)) = executor.get_generic_class_type_definition(&generic_class, metadata, il2cpp) {
                            // Use specialized name from hdr_ctx if available, fallback to base name
                            let type_struct_name = if let Some(ctx) = hdr_ctx.as_deref_mut() {
                                if let Some(name) = ctx.generic_class_struct_name_dic.get(&generic_class_ptr) {
                                    let name = name.clone();
                                    // Add to newly_discovered if this is a new unique name
                                    if ctx.struct_name_hash_set.insert(name.clone()) {
                                        ctx.newly_discovered.push(generic_class_ptr);
                                    }
                                    Some(name)
                                } else {
                                    struct_name_dic.get(&td_idx).cloned()
                                }
                            } else {
                                struct_name_dic.get(&td_idx).cloned()
                            };
                            if let Some(sn) = type_struct_name {
                                if type_def.is_value_type() {
                                    if type_def.is_enum() {
                                        if let Some(elem) = il2cpp.types.get(type_def.element_type_index as usize).cloned() {
                                            return Self::parse_type(&elem, struct_name_dic, executor, metadata, il2cpp, context, None);
                                        }
                                    }
                                    return format!("{}_o", sn);
                                } else {
                                    return format!("{}_o*", sn);
                                }
                            }
                        }
                    }
                }
                "Il2CppObject*".to_string()
            }
            Some(Il2CppTypeEnum::Var) => {
                if let Some(ctx) = context {
                    let generic_param = executor.get_generic_parameter_from_type(il2cpp_type, metadata, il2cpp);
                    if let Some(gp) = generic_param {
                        if let Some(resolved) = Self::resolve_generic_type_var(il2cpp, ctx.class_inst, gp.num as u32) {
                            return Self::parse_type(&resolved, struct_name_dic, executor, metadata, il2cpp, None, None);
                        }
                    }
                }
                "Il2CppObject*".to_string()
            }
            Some(Il2CppTypeEnum::MVar) => {
                if let Some(ctx) = context {
                    let generic_param = executor.get_generic_parameter_from_type(il2cpp_type, metadata, il2cpp);
                    if let Some(gp) = generic_param {
                        // C# issue #687: if method_inst == 0 && class_inst != 0, fall back to VAR
                        if ctx.method_inst == 0 && ctx.class_inst != 0 {
                            if let Some(resolved) = Self::resolve_generic_type_var(il2cpp, ctx.class_inst, gp.num as u32) {
                                return Self::parse_type(&resolved, struct_name_dic, executor, metadata, il2cpp, None, None);
                            }
                        } else {
                            if let Some(resolved) = Self::resolve_generic_type_var(il2cpp, ctx.method_inst, gp.num as u32) {
                                return Self::parse_type(&resolved, struct_name_dic, executor, metadata, il2cpp, None, None);
                            }
                        }
                    }
                }
                "Il2CppObject*".to_string()
            }
            Some(Il2CppTypeEnum::Ptr) => {
                if il2cpp_type.datapoint != 0 {
                    if let Some(ori_type) = il2cpp.types.get(il2cpp_type.datapoint as usize).cloned() {
                        let inner = Self::parse_type(&ori_type, struct_name_dic, executor, metadata, il2cpp, context, hdr_ctx);
                        return format!("{}*", inner);
                    }
                }
                "void*".to_string()
            }
            _ => "Il2CppObject*".to_string(),
        }
    }

    fn build_struct_name_dic(
        executor: &mut Il2CppExecutor,
        metadata: &mut Metadata,
        il2cpp: &mut Il2Cpp,
    ) -> HashMap<usize, String> {
        let mut dic = HashMap::new();
        let mut name_set: HashSet<String> = HashSet::new();

        let image_defs = metadata.image_defs.clone();
        for image_def in &image_defs {
            let type_end = image_def.type_start as usize + image_def.type_count as usize;
            for type_index in image_def.type_start as usize..type_end {
                let type_def = metadata.type_defs[type_index].clone();
                let type_name = executor.get_type_def_name(&type_def, type_index, metadata, il2cpp, true, true);
                let struct_name = fix_name(&type_name);
                let unique = get_unique_name(&struct_name, &mut name_set);
                dic.insert(type_index, unique);
            }
        }
        dic
    }

    fn build_type_def_image_names(metadata: &mut Metadata) -> HashMap<usize, String> {
        let mut dic = HashMap::new();
        let image_defs = metadata.image_defs.clone();
        for image_def in &image_defs {
            let image_name = metadata.get_string_from_index(image_def.name_index).unwrap_or_default();
            let type_end = image_def.type_start as usize + image_def.type_count as usize;
            for type_index in image_def.type_start as usize..type_end {
                dic.insert(type_index, image_name.clone());
            }
        }
        dic
    }

    fn add_metadata_usages(
        script: &mut ScriptJson,
        executor: &mut Il2CppExecutor,
        metadata: &mut Metadata,
        il2cpp: &mut Il2Cpp,
        struct_name_dic: &HashMap<usize, String>,
        _type_def_image_names: &HashMap<usize, String>,
    ) {
        if metadata.metadata_usage_dic.is_empty() { return; }
        let usage_dic = metadata.metadata_usage_dic.clone();

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
                            let sig = if let Some(sn) = Self::get_struct_name_for_type(&type_ref, struct_name_dic, metadata) {
                                format!("{}_c*", fix_name(&sn))
                            } else {
                                "Il2CppClass*".to_string()
                            };
                            script.script_metadata.push(ScriptMetadata {
                                address: rva,
                                name: format!("{}_TypeInfo", type_name),
                                signature: Some(sig),
                            });
                        }
                    }
                    2 => {
                        if src < il2cpp.types.len() {
                            let type_ref = il2cpp.types[src].clone();
                            let type_name = executor.get_type_name(&type_ref, metadata, il2cpp, true, false);
                            script.script_metadata.push(ScriptMetadata {
                                address: rva,
                                name: format!("{}_var", type_name),
                                signature: Some("Il2CppType*".to_string()),
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
                                let image_name = _type_def_image_names.get(&td_idx).cloned().unwrap_or_default();
                                let method_pointer = il2cpp.get_method_pointer(&image_name, &method_def);
                                let method_address = if method_pointer > 0 { il2cpp.get_rva(method_pointer) } else { 0 };
                                script.script_metadata_methods.push(ScriptMetadataMethod {
                                    address: rva,
                                    name: format!("Method${}.{}()", type_name, method_name),
                                    method_address,
                                });
                            }
                        }
                    }
                    4 => {
                        if src < metadata.field_refs.len() {
                            let field_ref = metadata.field_refs[src].clone();
                            let il2cpp_type = il2cpp.types[field_ref.type_index as usize].clone();
                            let type_name = executor.get_type_name(&il2cpp_type, metadata, il2cpp, true, false);
                            let klass_idx = il2cpp_type.klass_index() as usize;
                            if let Some(td) = metadata.type_defs.get(klass_idx) {
                                let field_idx = td.field_start as usize + field_ref.field_index as usize;
                                if let Some(fd) = metadata.field_defs.get(field_idx) {
                                    let field_name = metadata.get_string_from_index(fd.name_index).unwrap_or_default();
                                    script.script_metadata.push(ScriptMetadata {
                                        address: rva,
                                        name: format!("Field${}.{}", type_name, field_name),
                                        signature: None,
                                    });
                                }
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
                    6 => {
                        if src < il2cpp.method_specs.len() {
                            let _method_spec = il2cpp.method_specs[src].clone();
                            let (spec_type_name, spec_method_name) = executor.get_method_spec_name(src, metadata, il2cpp, true);
                            let method_address = il2cpp.method_spec_generic_method_pointers
                                .get(&src).copied()
                                .filter(|p| *p > 0)
                                .map(|p| il2cpp.get_rva(p))
                                .unwrap_or(0);
                            script.script_metadata_methods.push(ScriptMetadataMethod {
                                address: rva,
                                name: format!("Method${}.{}()", spec_type_name, spec_method_name),
                                method_address,
                            });
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    fn scan_v27_metadata_usages(
        script: &mut ScriptJson,
        executor: &mut Il2CppExecutor,
        metadata: &mut Metadata,
        il2cpp: &mut Il2Cpp,
        struct_name_dic: &HashMap<usize, String>,
        type_def_image_names: &HashMap<usize, String>,
    ) {
        let pointer_size = if il2cpp.is_32bit { 4u64 } else { 8u64 };
        let data_sections = il2cpp.data_sections.clone();

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
                            let sig = if let Some(sn) = Self::get_struct_name_for_type(&type_ref, struct_name_dic, metadata) {
                                if sn.ends_with("_array") {
                                    "Il2CppClass*".to_string()
                                } else {
                                    format!("{}_c*", fix_name(&sn))
                                }
                            } else {
                                "Il2CppClass*".to_string()
                            };
                            script.script_metadata.push(ScriptMetadata {
                                address: rva,
                                name: format!("{}_TypeInfo", type_name),
                                signature: Some(sig),
                            });
                        }
                    }
                    2 => {
                        if (decoded_index as usize) < il2cpp.types.len() {
                            let type_ref = il2cpp.types[decoded_index as usize].clone();
                            let type_name = executor.get_type_name(&type_ref, metadata, il2cpp, true, false);
                            script.script_metadata.push(ScriptMetadata {
                                address: rva,
                                name: format!("{}_var", type_name),
                                signature: Some("Il2CppType*".to_string()),
                            });
                        }
                    }
                    3 => {
                        if let Some(method_def) = metadata.method_defs.get(decoded_index as usize).cloned() {
                            if let Some(type_def) = metadata.type_defs.get(method_def.declaring_type as usize).cloned() {
                                let td_idx = method_def.declaring_type as usize;
                                let type_name = executor.get_type_def_name(&type_def, td_idx, metadata, il2cpp, true, true);
                                let method_name = metadata.get_string_from_index(method_def.name_index as i32)
                                    .unwrap_or_else(|_| "?".to_string());
                                let image_name = type_def_image_names.get(&td_idx).cloned().unwrap_or_default();
                                let method_pointer = il2cpp.get_method_pointer(&image_name, &method_def);
                                let method_address = if method_pointer > 0 { il2cpp.get_rva(method_pointer) } else { 0 };
                                script.script_metadata_methods.push(ScriptMetadataMethod {
                                    address: rva,
                                    name: format!("Method${}.{}()", type_name, method_name),
                                    method_address,
                                });
                            }
                        }
                    }
                    4 => {
                        if (decoded_index as usize) < metadata.field_refs.len() {
                            let field_ref = metadata.field_refs[decoded_index as usize].clone();
                            if (field_ref.type_index as usize) >= il2cpp.types.len() { continue; }
                            let il2cpp_type = il2cpp.types[field_ref.type_index as usize].clone();
                            let type_name = executor.get_type_name(&il2cpp_type, metadata, il2cpp, true, false);
                            let klass_idx = il2cpp_type.klass_index() as usize;
                            if let Some(td) = metadata.type_defs.get(klass_idx) {
                                let field_idx = td.field_start as usize + field_ref.field_index as usize;
                                if let Some(fd) = metadata.field_defs.get(field_idx) {
                                    let field_name = metadata.get_string_from_index(fd.name_index).unwrap_or_default();
                                    script.script_metadata.push(ScriptMetadata {
                                        address: rva,
                                        name: format!("Field${}.{}", type_name, field_name),
                                        signature: None,
                                    });
                                }
                            }
                        }
                    }
                    5 => {
                        if let Ok(string_literal) = metadata.get_string_literal_from_index(decoded_index as usize) {
                            script.script_strings.push(ScriptString {
                                address: rva,
                                value: string_literal,
                            });
                        }
                    }
                    6 => {
                        if (decoded_index as usize) < il2cpp.method_specs.len() {
                            let (spec_type_name, spec_method_name) = executor.get_method_spec_name(decoded_index as usize, metadata, il2cpp, true);
                            let method_address = il2cpp.method_spec_generic_method_pointers
                                .get(&(decoded_index as usize)).copied()
                                .filter(|p| *p > 0)
                                .map(|p| il2cpp.get_rva(p))
                                .unwrap_or(0);
                            script.script_metadata_methods.push(ScriptMetadataMethod {
                                address: rva,
                                name: format!("Method${}.{}()", spec_type_name, spec_method_name),
                                method_address,
                            });
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

    fn get_struct_name_for_type(
        il2cpp_type: &Il2CppType,
        struct_name_dic: &HashMap<usize, String>,
        _metadata: &Metadata,
    ) -> Option<String> {
        let te = Il2CppTypeEnum::from_u8(il2cpp_type.type_enum)?;
        match te {
            Il2CppTypeEnum::Class | Il2CppTypeEnum::ValueType => {
                let klass_idx = il2cpp_type.klass_index() as usize;
                struct_name_dic.get(&klass_idx).cloned()
            }
            _ => None,
        }
    }

    fn get_il2cpp_struct_name(
        il2cpp_type: &Il2CppType,
        struct_name_dic: &HashMap<usize, String>,
        il2cpp: &mut Il2Cpp,
        context: Option<&Il2CppGenericContext>,
    ) -> String {
        let te = Il2CppTypeEnum::from_u8(il2cpp_type.type_enum);
        match te {
            Some(Il2CppTypeEnum::Void) | Some(Il2CppTypeEnum::Boolean) | Some(Il2CppTypeEnum::Char) |
            Some(Il2CppTypeEnum::I1) | Some(Il2CppTypeEnum::U1) | Some(Il2CppTypeEnum::I2) |
            Some(Il2CppTypeEnum::U2) | Some(Il2CppTypeEnum::I4) | Some(Il2CppTypeEnum::U4) |
            Some(Il2CppTypeEnum::I8) | Some(Il2CppTypeEnum::U8) | Some(Il2CppTypeEnum::R4) |
            Some(Il2CppTypeEnum::R8) | Some(Il2CppTypeEnum::String) | Some(Il2CppTypeEnum::TypedByRef) |
            Some(Il2CppTypeEnum::I) | Some(Il2CppTypeEnum::U) | Some(Il2CppTypeEnum::Object) |
            Some(Il2CppTypeEnum::ValueType) | Some(Il2CppTypeEnum::Class) => {
                let klass_idx = il2cpp_type.klass_index() as usize;
                struct_name_dic.get(&klass_idx).cloned().unwrap_or_else(|| "System_Object".to_string())
            }
            Some(Il2CppTypeEnum::Ptr) => {
                if il2cpp_type.datapoint != 0 {
                    if let Some(ori_type) = il2cpp.types.get(il2cpp_type.datapoint as usize).cloned() {
                        return Self::get_il2cpp_struct_name(&ori_type, struct_name_dic, il2cpp, context);
                    }
                }
                "System_Object".to_string()
            }
            Some(Il2CppTypeEnum::SzArray) => {
                if il2cpp_type.datapoint != 0 {
                    if let Some(element_type) = il2cpp.types.get(il2cpp_type.datapoint as usize).cloned() {
                        let elem_name = Self::get_il2cpp_struct_name(&element_type, struct_name_dic, il2cpp, context);
                        return format!("{}_array", elem_name);
                    }
                }
                "System_Object".to_string()
            }
            Some(Il2CppTypeEnum::Array) => {
                if il2cpp_type.datapoint != 0 {
                    if let Some(element_type) = il2cpp.types.get(il2cpp_type.datapoint as usize).cloned() {
                        let elem_name = Self::get_il2cpp_struct_name(&element_type, struct_name_dic, il2cpp, context);
                        return format!("{}_array", elem_name);
                    }
                }
                "System_Object".to_string()
            }
            Some(Il2CppTypeEnum::GenericInst) => {
                let klass_idx = il2cpp_type.klass_index() as usize;
                struct_name_dic.get(&klass_idx).cloned().unwrap_or_else(|| "System_Object".to_string())
            }
            Some(Il2CppTypeEnum::Var) => {
                if let Some(ctx) = context {
                    if let Some(resolved) = Self::resolve_generic_type_var(il2cpp, ctx.class_inst, il2cpp_type.datapoint as u32) {
                        return Self::get_il2cpp_struct_name(&resolved, struct_name_dic, il2cpp, None);
                    }
                }
                "System_Object".to_string()
            }
            Some(Il2CppTypeEnum::MVar) => {
                if let Some(ctx) = context {
                    if let Some(resolved) = Self::resolve_generic_type_var(il2cpp, ctx.method_inst, il2cpp_type.datapoint as u32) {
                        return Self::get_il2cpp_struct_name(&resolved, struct_name_dic, il2cpp, None);
                    }
                }
                "System_Object".to_string()
            }
            _ => "System_Object".to_string(),
        }
    }

    fn parse_array_class_struct(
        buf: &mut String,
        element_type: &Il2CppType,
        struct_name_dic: &HashMap<usize, String>,
        executor: &mut Il2CppExecutor,
        metadata: &mut Metadata,
        il2cpp: &mut Il2Cpp,
        context: Option<&Il2CppGenericContext>,
    ) {
        let struct_name = Self::get_il2cpp_struct_name(element_type, struct_name_dic, il2cpp, context);
        let element_c_type = Self::parse_type(element_type, struct_name_dic, executor, metadata, il2cpp, context, None);
        writeln!(buf, "struct {}_array {{", struct_name).ok();
        writeln!(buf, "\tIl2CppObject obj;").ok();
        writeln!(buf, "\tIl2CppArrayBounds *bounds;").ok();
        writeln!(buf, "\til2cpp_array_size_t max_length;").ok();
        writeln!(buf, "\t{} m_Items[65535];", element_c_type).ok();
        writeln!(buf, "}};").ok();
    }

    fn build_string_literal_json(metadata: &mut Metadata) -> Result<String> {
        let mut entries = Vec::new();
        for i in 0..metadata.string_literals.len() {
            if let Ok(value) = metadata.get_string_literal_from_index(i) {
                entries.push(StringLiteralEntry { index: i, value });
            }
        }
        let json = serde_json::to_string_pretty(&entries)
            .map_err(|e| crate::error::Error::Other(e.to_string()))?;
        Ok(json)
    }

    fn build_header(
        executor: &mut Il2CppExecutor,
        metadata: &mut Metadata,
        il2cpp: &mut Il2Cpp,
    ) -> Result<String> {
        let version = il2cpp.version;
        let version_header = match header_constants::get_version_header(version) {
            Some(h) => h,
            None => {
                eprintln!("WARNING: IL2CPP version [{version}] does not support generating .h files");
                return Ok(String::new());
            }
        };

        let struct_name_dic = Self::build_struct_name_dic(executor, metadata, il2cpp);
        let type_def_image_names = Self::build_type_def_image_names(metadata);

        let mut struct_info_list: Vec<StructInfo> = Vec::new();
        let mut array_class_header = String::with_capacity(1 << 14);
        let mut array_class_set: HashSet<String> = HashSet::new();

        // Build genericClassStructNameDic (C# lines 58-73)
        let mut generic_class_struct_name_dic: HashMap<u64, String> = HashMap::new();
        let mut name_generic_class_dic: HashMap<String, Il2CppType> = HashMap::new();
        let mut generic_class_list: Vec<u64> = Vec::new();
        let struct_name_hash_set: HashSet<String> = struct_name_dic.values().cloned().collect();
        {
            let types_clone = il2cpp.types.clone();
            for il2cpp_type in &types_clone {
                let te = Il2CppTypeEnum::from_u8(il2cpp_type.type_enum);
                if te != Some(Il2CppTypeEnum::GenericInst) { continue; }
                let generic_class_ptr = il2cpp_type.generic_class();
                if generic_class_ptr == 0 { continue; }
                let generic_class = match il2cpp.read_generic_class(generic_class_ptr) {
                    Ok(gc) => gc,
                    Err(_) => continue,
                };
                let type_def_result = executor.get_generic_class_type_definition(&generic_class, metadata, il2cpp);
                let (type_def, td_idx) = match type_def_result {
                    Some(td) => td,
                    None => continue,
                };
                let type_base_name = match struct_name_dic.get(&td_idx) {
                    Some(n) => n.clone(),
                    None => continue,
                };
                let type_to_replace_name = fix_name(&executor.get_type_def_name(&type_def, td_idx, metadata, il2cpp, true, true));
                let type_replace_name = fix_name(&executor.get_type_name(il2cpp_type, metadata, il2cpp, true, false));
                let type_struct_name = type_base_name.replace(&type_to_replace_name, &type_replace_name);
                name_generic_class_dic.insert(type_struct_name.clone(), il2cpp_type.clone());
                generic_class_struct_name_dic.insert(generic_class_ptr, type_struct_name);
            }
        }

        let type_defs = metadata.type_defs.clone();
        for (type_index, type_def) in type_defs.iter().enumerate() {
            let type_name = match struct_name_dic.get(&type_index) {
                Some(n) => n.clone(),
                None => continue,
            };

            let mut info = StructInfo {
                type_name,
                is_value_type: type_def.is_value_type(),
                parent: None,
                fields: Vec::new(),
                static_fields: Vec::new(),
                vtable_methods: Vec::new(),
                rgctxs: Vec::new(),
            };

            Self::add_parent(il2cpp, type_def, &struct_name_dic, metadata, &mut info);
            Self::add_fields(executor, metadata, il2cpp, type_def, &struct_name_dic, &mut info, &mut array_class_header, &mut array_class_set, None, None);
            Self::add_vtable_methods(metadata, il2cpp, type_def, &mut info);
            Self::add_rgctx(executor, metadata, il2cpp, type_def, type_index, &type_def_image_names, &mut info);

            struct_info_list.push(info);
        }

        // Process generic class instances using fixpoint loop
        // C# uses a self-expanding for loop: for(int i=0; i<genericClassList.Count; i++)
        // where ParseType can add new entries during iteration.
        let mut hdr_ctx = HeaderGenCtx {
            generic_class_struct_name_dic: generic_class_struct_name_dic.clone(),
            struct_name_hash_set: struct_name_hash_set,
            newly_discovered: Vec::new(),
        };

        // Build initial list from all GENERICINST types in il2cpp.types
        for il2cpp_type in il2cpp.types.clone().iter() {
            let te = Il2CppTypeEnum::from_u8(il2cpp_type.type_enum);
            if te != Some(Il2CppTypeEnum::GenericInst) { continue; }
            let generic_class_ptr = il2cpp_type.generic_class();
            if generic_class_ptr == 0 { continue; }
            if !hdr_ctx.generic_class_struct_name_dic.contains_key(&generic_class_ptr) { continue; }
            let type_struct_name = hdr_ctx.generic_class_struct_name_dic[&generic_class_ptr].clone();
            if !hdr_ctx.struct_name_hash_set.insert(type_struct_name) { continue; }
            generic_class_list.push(generic_class_ptr);
        }

        // Fixpoint loop: process generic classes, discovering new ones as field types are parsed
        let mut processed = 0;
        loop {
            let current_len = generic_class_list.len();
            if processed >= current_len { break; }

            for idx in processed..current_len {
                let pointer = generic_class_list[idx];
                let generic_class = match il2cpp.read_generic_class(pointer) {
                    Ok(gc) => gc,
                    Err(_) => continue,
                };
                let type_def_result = executor.get_generic_class_type_definition(&generic_class, metadata, il2cpp);
                let (type_def, _td_idx) = match type_def_result {
                    Some(td) => td,
                    None => continue,
                };
                let type_struct_name = match hdr_ctx.generic_class_struct_name_dic.get(&pointer) {
                    Some(n) => n.clone(),
                    None => continue,
                };
                let mut info = StructInfo {
                    type_name: type_struct_name,
                    is_value_type: type_def.is_value_type(),
                    parent: None,
                    fields: Vec::new(),
                    static_fields: Vec::new(),
                    vtable_methods: Vec::new(),
                    rgctxs: Vec::new(),
                };
                let context = Il2CppGenericContext {
                    class_inst: generic_class.context.class_inst,
                    method_inst: generic_class.context.method_inst,
                };
                Self::add_parent(il2cpp, &type_def, &struct_name_dic, metadata, &mut info);
                Self::add_fields(executor, metadata, il2cpp, &type_def, &struct_name_dic, &mut info, &mut array_class_header, &mut array_class_set, Some(&context), Some(&mut hdr_ctx));
                Self::add_vtable_methods(metadata, il2cpp, &type_def, &mut info);
                struct_info_list.push(info);
            }

            processed = current_len;

            // Drain newly discovered generic classes into the main list
            let new_ptrs: Vec<u64> = hdr_ctx.newly_discovered.drain(..).collect();
            generic_class_list.extend(new_ptrs);
        }



        let struct_info_by_name: HashMap<String, usize> = struct_info_list.iter().enumerate()
            .map(|(i, info)| (format!("{}_o", info.type_name), i))
            .collect();

        let mut struct_cache: HashSet<usize> = HashSet::new();
        let mut header_struct = String::with_capacity(1 << 18);
        for i in 0..struct_info_list.len() {
            Self::recursion_struct_info(i, &struct_info_list, &struct_info_by_name, &mut struct_cache, &mut header_struct, il2cpp.is_32bit, il2cpp.is_pe);
        }

        let mut method_info_header = String::with_capacity(1 << 16);

        let mut method_info_cache: HashSet<u64> = HashSet::new();

        let image_defs = metadata.image_defs.clone();
        for image_def in &image_defs {
            let image_name = metadata.get_string_from_index(image_def.name_index).unwrap_or_default();
            let type_end = image_def.type_start as usize + image_def.type_count as usize;
            for type_def_index in image_def.type_start as usize..type_end {
                let type_def = metadata.type_defs[type_def_index].clone();
                let struct_type_name = match struct_name_dic.get(&type_def_index) {
                    Some(n) => n.clone(),
                    None => continue,
                };
                let method_end = type_def.method_start as usize + type_def.method_count as usize;
                for method_index in type_def.method_start as usize..method_end {
                    let method_def = metadata.method_defs[method_index].clone();
                    if let Some(spec_indices) = il2cpp.method_definition_method_specs.get(&method_index).cloned() {
                        for spec_idx in &spec_indices {
                            if *spec_idx >= il2cpp.method_specs.len() { continue; }
                            let _method_spec = il2cpp.method_specs[*spec_idx].clone();
                            // Note: do NOT filter on method_index_index < 0 here.
                            // C# generates MethodInfo for ALL method specs with genericMethodPointer > 0,
                            // including those with only class-level generics (method_index_index == -1).

                            let generic_method_pointer = il2cpp.method_spec_generic_method_pointers.get(spec_idx).copied().unwrap_or(0);
                            if generic_method_pointer == 0 { continue; }
                            let method_info_rva = il2cpp.get_rva(generic_method_pointer);
                            let method_info_name = format!("MethodInfo_{:X}", method_info_rva);

                            let (_spec_type_name, _spec_method_name) = executor.get_method_spec_name(*spec_idx, metadata, il2cpp, true);

                            let method_rgctxs = Self::collect_rgctx_info_for_method(
                                executor, metadata, il2cpp, &image_name, &method_def,
                            );
                            if method_info_cache.insert(generic_method_pointer) {
                                Self::generate_method_info(&mut method_info_header, &method_info_name, &struct_type_name, &method_rgctxs, il2cpp.version);
                            }
                        }
                    }
                }
            }
        }

        let mut buf = String::with_capacity(1 << 20);
        write!(buf, "#include <stdint.h>\n#include <stdbool.h>\n\n").ok();
        buf.push_str(header_constants::generic_header());
        buf.push_str(version_header);
        buf.push_str(&header_struct);
        buf.push_str(&array_class_header);
        buf.push_str(&method_info_header);

        Ok(buf)
    }

    fn add_parent(
        il2cpp: &Il2Cpp,
        type_def: &Il2CppTypeDefinition,
        struct_name_dic: &HashMap<usize, String>,
        _metadata: &Metadata,
        info: &mut StructInfo,
    ) {
        if type_def.is_value_type() || type_def.is_enum() { return; }
        if type_def.parent_index < 0 { return; }
        if let Some(parent) = il2cpp.types.get(type_def.parent_index as usize) {
            let te = Il2CppTypeEnum::from_u8(parent.type_enum);
            if te == Some(Il2CppTypeEnum::Object) { return; }
            let klass_idx = parent.klass_index() as usize;
            if let Some(sn) = struct_name_dic.get(&klass_idx) {
                info.parent = Some(sn.clone());
            }
        }
    }

    fn add_fields(
        executor: &mut Il2CppExecutor,
        metadata: &mut Metadata,
        il2cpp: &mut Il2Cpp,
        type_def: &Il2CppTypeDefinition,
        struct_name_dic: &HashMap<usize, String>,
        info: &mut StructInfo,
        array_class_header: &mut String,
        array_class_set: &mut HashSet<String>,
        context: Option<&Il2CppGenericContext>,
        mut hdr_ctx: Option<&mut HeaderGenCtx>,
    ) {
        if type_def.field_count == 0 { return; }
        let field_end = type_def.field_start as usize + type_def.field_count as usize;
        let mut field_name_cache: HashSet<String> = HashSet::new();

        for i in type_def.field_start as usize..field_end {
            let field_def = metadata.field_defs[i].clone();
            let field_type = il2cpp.types[field_def.type_index as usize].clone();

            if (field_type.attrs & field_attributes::LITERAL) != 0 { continue; }

            let te = Il2CppTypeEnum::from_u8(field_type.type_enum);
            if te == Some(Il2CppTypeEnum::SzArray) || te == Some(Il2CppTypeEnum::Array) {
                if field_type.datapoint != 0 {
                    if let Some(element_type) = il2cpp.types.get(field_type.datapoint as usize).cloned() {
                        let elem_struct_name = Self::get_il2cpp_struct_name(&element_type, struct_name_dic, il2cpp, context);
                        let array_struct_name = format!("{}_array", elem_struct_name);
                        if array_class_set.insert(array_struct_name) {
                            Self::parse_array_class_struct(array_class_header, &element_type, struct_name_dic, executor, metadata, il2cpp, context);
                        }
                    }
                }
            }

            let field_type_name = Self::parse_type(&field_type, struct_name_dic, executor, metadata, il2cpp, context, hdr_ctx.as_deref_mut());
            let mut field_name = fix_name(&metadata.get_string_from_index(field_def.name_index).unwrap_or_else(|_| "field".to_string()));
            if !field_name_cache.insert(field_name.clone()) {
                field_name = format!("_{}_{}", i - type_def.field_start as usize, field_name);
            }

            let is_vt = Self::is_value_type_check(&field_type, metadata, il2cpp, executor);
            let is_ct = Self::is_custom_type_check(&field_type, metadata, il2cpp, executor);

            let field_info = StructFieldInfo {
                field_type_name,
                field_name,
                is_value_type: is_vt,
                is_custom_type: is_ct,
            };

            if (field_type.attrs & field_attributes::STATIC) != 0 {
                info.static_fields.push(field_info);
            } else {
                info.fields.push(field_info);
            }
        }
    }

    fn is_value_type_check(il2cpp_type: &Il2CppType, metadata: &Metadata, il2cpp: &mut Il2Cpp, executor: &Il2CppExecutor) -> bool {
        match Il2CppTypeEnum::from_u8(il2cpp_type.type_enum) {
            Some(Il2CppTypeEnum::ValueType) => {
                let klass_idx = il2cpp_type.klass_index() as usize;
                if let Some(td) = metadata.type_defs.get(klass_idx) {
                    return !td.is_enum();
                }
                false
            }
            Some(Il2CppTypeEnum::GenericInst) => {
                let generic_class_ptr = il2cpp_type.generic_class();
                if generic_class_ptr != 0 {
                    if let Ok(generic_class) = il2cpp.read_generic_class(generic_class_ptr) {
                        if let Some((td, _)) = executor.get_generic_class_type_definition(&generic_class, metadata, il2cpp) {
                            return td.is_value_type() && !td.is_enum();
                        }
                    }
                }
                false
            }
            _ => false,
        }
    }

    fn is_custom_type_check(il2cpp_type: &Il2CppType, metadata: &Metadata, il2cpp: &mut Il2Cpp, executor: &Il2CppExecutor) -> bool {
        match Il2CppTypeEnum::from_u8(il2cpp_type.type_enum) {
            Some(Il2CppTypeEnum::Ptr) => {
                if il2cpp_type.datapoint != 0 {
                    if let Some(ori) = il2cpp.types.get(il2cpp_type.datapoint as usize).cloned() {
                        return Self::is_custom_type_check(&ori, metadata, il2cpp, executor);
                    }
                }
                false
            }
            Some(Il2CppTypeEnum::String) | Some(Il2CppTypeEnum::Class)
            | Some(Il2CppTypeEnum::Array) | Some(Il2CppTypeEnum::SzArray) => true,
            Some(Il2CppTypeEnum::ValueType) => {
                let klass_idx = il2cpp_type.klass_index() as usize;
                if let Some(td) = metadata.type_defs.get(klass_idx) {
                    if td.is_enum() {
                        if let Some(elem) = il2cpp.types.get(td.element_type_index as usize).cloned() {
                            return Self::is_custom_type_check(&elem, metadata, il2cpp, executor);
                        }
                    }
                    return true;
                }
                false
            }
            Some(Il2CppTypeEnum::GenericInst) => {
                let generic_class_ptr = il2cpp_type.generic_class();
                if generic_class_ptr != 0 {
                    if let Ok(generic_class) = il2cpp.read_generic_class(generic_class_ptr) {
                        if let Some((td, _)) = executor.get_generic_class_type_definition(&generic_class, metadata, il2cpp) {
                            if td.is_enum() {
                                if let Some(elem) = il2cpp.types.get(td.element_type_index as usize).cloned() {
                                    return Self::is_custom_type_check(&elem, metadata, il2cpp, executor);
                                }
                            }
                            return true;
                        }
                    }
                }
                true
            }
            _ => false,
        }
    }

    fn add_vtable_methods(
        metadata: &mut Metadata,
        il2cpp: &Il2Cpp,
        type_def: &Il2CppTypeDefinition,
        info: &mut StructInfo,
    ) {
        let mut dic: BTreeMap<u16, String> = BTreeMap::new();

        for i in 0..type_def.vtable_count as usize {
            let vtable_index = type_def.vtable_start as usize + i;
            if vtable_index >= metadata.vtable_methods.len() { continue; }

            let encoded = metadata.vtable_methods[vtable_index];
            let usage = (encoded & 0xE0000000) >> 29;
            // v27+ uses different index encoding: (encoded & 0x1FFFFFFEU) >> 1
            let index = if metadata.version >= 27.0 {
                (encoded & 0x1FFFFFFE) >> 1
            } else {
                encoded & 0x1FFFFFFF
            };

            let method_def = if usage == 6 {
                if (index as usize) < il2cpp.method_specs.len() {
                    let spec = &il2cpp.method_specs[index as usize];
                    metadata.method_defs.get(spec.method_definition_index as usize).cloned()
                } else {
                    None
                }
            } else {
                metadata.method_defs.get(index as usize).cloned()
            };

            if let Some(md) = method_def {
                if md.slot != 0xFFFF {
                    let name = metadata.get_string_from_index(md.name_index as i32).unwrap_or_else(|_| "unknown".to_string());
                    dic.insert(md.slot, fix_name(&name));
                }
            }
        }

        if !dic.is_empty() {
            let max_slot = *dic.keys().last().unwrap() as usize;
            let mut vtable_vec: Vec<Option<StructVTableMethodInfo>> = Vec::with_capacity(max_slot + 1);
            for i in 0..=max_slot {
                if let Some(name) = dic.get(&(i as u16)) {
                    vtable_vec.push(Some(StructVTableMethodInfo { method_name: name.clone() }));
                } else {
                    vtable_vec.push(None);
                }
            }
            info.vtable_methods = vtable_vec;
        }
    }

    fn add_rgctx(
        executor: &mut Il2CppExecutor,
        metadata: &mut Metadata,
        il2cpp: &mut Il2Cpp,
        type_def: &Il2CppTypeDefinition,
        _type_index: usize,
        type_def_image_names: &HashMap<usize, String>,
        info: &mut StructInfo,
    ) {
        let type_def_idx = _type_index;
        let image_name = type_def_image_names.get(&type_def_idx).cloned().unwrap_or_default();
        let collection = executor.get_rgctx_definition_for_type(&image_name, type_def, metadata, il2cpp);
        if let Some(definitions) = collection {
            for def in &definitions {
                let rgctx_type_val = def.rgctx_type();
                let mut rgctx_info = StructRGCTXInfo {
                    rgctx_type: rgctx_type_val,
                    type_name: None,
                    class_name: None,
                    method_name: None,
                };
                // For v27.2+, data is at data_ptr (needs MapVATR), for older, it's inline
                let rgctx_data_type_index: Option<i32> = if il2cpp.version >= 27.2 {
                    if def.data_ptr != 0 {
                        if let Ok(offset) = il2cpp.map_vatr(def.data_ptr as u64) {
                            il2cpp.stream.set_position(offset);
                            il2cpp.stream.read_i32().ok()
                        } else { None }
                    } else { None }
                } else {
                    def.data.as_ref().map(|d| d.rgctx_data_dummy)
                };
                if let Some(data_index) = rgctx_data_type_index {
                    match rgctx_type_val {
                        1 => {
                            let type_idx = data_index as usize;
                            if type_idx < il2cpp.types.len() {
                                let il2cpp_type = il2cpp.types[type_idx].clone();
                                let name = executor.get_type_name(&il2cpp_type, metadata, il2cpp, true, false);
                                rgctx_info.type_name = Some(fix_name(&name));
                            }
                        }
                        2 => {
                            let type_idx = data_index as usize;
                            if type_idx < il2cpp.types.len() {
                                let il2cpp_type = il2cpp.types[type_idx].clone();
                                let name = executor.get_type_name(&il2cpp_type, metadata, il2cpp, true, false);
                                rgctx_info.class_name = Some(fix_name(&name));
                            }
                        }
                        3 => {
                            let method_idx = data_index as usize;
                            if method_idx < il2cpp.method_specs.len() {
                                let (type_name, method_name) = executor.get_method_spec_name(method_idx, metadata, il2cpp, true);
                                rgctx_info.method_name = Some(fix_name(&format!("{}.{}", type_name, method_name)));
                            }
                        }
                        _ => {}
                    }
                }
                info.rgctxs.push(rgctx_info);
            }
        }
    }

    fn recursion_struct_info(
        idx: usize,
        list: &[StructInfo],
        name_map: &HashMap<String, usize>,
        cache: &mut HashSet<usize>,
        buf: &mut String,
        is_32bit: bool,
        is_pe: bool,
    ) {
        if !cache.insert(idx) { return; }
        let info = &list[idx];

        if let Some(parent_name) = &info.parent {
            let parent_key = format!("{}_o", parent_name);
            if let Some(&parent_idx) = name_map.get(&parent_key) {
                Self::recursion_struct_info(parent_idx, list, name_map, cache, buf, is_32bit, is_pe);
            }
            writeln!(buf, "struct {}_Fields : {}_Fields {{", info.type_name, parent_name).ok();
        } else {
            if is_pe && !info.is_value_type {
                if is_32bit {
                    writeln!(buf, "struct __declspec(align(4)) {}_Fields {{", info.type_name).ok();
                } else {
                    writeln!(buf, "struct __declspec(align(8)) {}_Fields {{", info.type_name).ok();
                }
            } else {
                writeln!(buf, "struct {}_Fields {{", info.type_name).ok();
            }
        }

        for field in &info.fields {
            if field.is_value_type {
                let field_key = &field.field_type_name;
                if let Some(&field_idx) = name_map.get(field_key) {
                    Self::recursion_struct_info(field_idx, list, name_map, cache, buf, is_32bit, is_pe);
                }
            }
            if field.is_custom_type {
                writeln!(buf, "\tstruct {} {};", field.field_type_name, field.field_name).ok();
            } else {
                writeln!(buf, "\t{} {};", field.field_type_name, field.field_name).ok();
            }
        }
        writeln!(buf, "}};").ok();

        if !info.rgctxs.is_empty() {
            writeln!(buf, "struct {}_RGCTXs {{", info.type_name).ok();
            for (i, rgctx) in info.rgctxs.iter().enumerate() {
                match rgctx.rgctx_type {
                    1 => {
                        let tn = rgctx.type_name.as_deref().unwrap_or("unknown");
                        writeln!(buf, "\tIl2CppType* _{}_{};", i, tn).ok();
                    }
                    2 => {
                        let cn = rgctx.class_name.as_deref().unwrap_or("unknown");
                        writeln!(buf, "\tIl2CppClass* _{}_{};", i, cn).ok();
                    }
                    3 => {
                        let mn = rgctx.method_name.as_deref().unwrap_or("unknown");
                        writeln!(buf, "\tMethodInfo* _{}_{};", i, mn).ok();
                    }
                    _ => {}
                }
            }
            writeln!(buf, "}};").ok();
        }

        if !info.vtable_methods.is_empty() {
            writeln!(buf, "struct {}_VTable {{", info.type_name).ok();
            for (i, method) in info.vtable_methods.iter().enumerate() {
                write!(buf, "\tVirtualInvokeData _{}_", i).ok();
                if let Some(m) = method {
                    write!(buf, "{}", m.method_name).ok();
                } else {
                    write!(buf, "unknown").ok();
                }
                writeln!(buf, ";").ok();
            }
            writeln!(buf, "}};").ok();
        }

        writeln!(buf, "struct {}_c {{", info.type_name).ok();
        writeln!(buf, "\tIl2CppClass_1 _1;").ok();
        if !info.static_fields.is_empty() {
            writeln!(buf, "\tstruct {}_StaticFields* static_fields;", info.type_name).ok();
        } else {
            writeln!(buf, "\tvoid* static_fields;").ok();
        }
        if !info.rgctxs.is_empty() {
            writeln!(buf, "\t{}_RGCTXs* rgctx_data;", info.type_name).ok();
        } else {
            writeln!(buf, "\tIl2CppRGCTXData* rgctx_data;").ok();
        }
        writeln!(buf, "\tIl2CppClass_2 _2;").ok();
        if !info.vtable_methods.is_empty() {
            writeln!(buf, "\t{}_VTable vtable;", info.type_name).ok();
        } else {
            writeln!(buf, "\tVirtualInvokeData vtable[32];").ok();
        }
        writeln!(buf, "}};").ok();

        writeln!(buf, "struct {}_o {{", info.type_name).ok();
        if !info.is_value_type {
            writeln!(buf, "\t{}_c *klass;", info.type_name).ok();
            writeln!(buf, "\tvoid *monitor;").ok();
        }
        writeln!(buf, "\t{}_Fields fields;", info.type_name).ok();
        writeln!(buf, "}};").ok();

        if !info.static_fields.is_empty() {
            writeln!(buf, "struct {}_StaticFields {{", info.type_name).ok();
            for field in &info.static_fields {
                if field.is_value_type {
                    let field_key = &field.field_type_name;
                    if let Some(&field_idx) = name_map.get(field_key) {
                        Self::recursion_struct_info(field_idx, list, name_map, cache, buf, is_32bit, is_pe);
                    }
                }
                if field.is_custom_type {
                    writeln!(buf, "\tstruct {} {};", field.field_type_name, field.field_name).ok();
                } else {
                    writeln!(buf, "\t{} {};", field.field_type_name, field.field_name).ok();
                }
            }
            writeln!(buf, "}};").ok();
        }
    }

    fn collect_rgctx_info_for_method(
        executor: &mut Il2CppExecutor,
        metadata: &mut Metadata,
        il2cpp: &mut Il2Cpp,
        image_name: &str,
        method_def: &Il2CppMethodDefinition,
    ) -> Vec<StructRGCTXInfo> {
        let mut rgctxs = Vec::new();
        let collection = executor.get_rgctx_definition_for_method(image_name, method_def, metadata, il2cpp);
        if let Some(definitions) = collection {
            for def in &definitions {
                let rgctx_type_val = def.rgctx_type();
                let mut rgctx_info = StructRGCTXInfo {
                    rgctx_type: rgctx_type_val,
                    type_name: None,
                    class_name: None,
                    method_name: None,
                };
                if let Some(data) = &def.data {
                    match rgctx_type_val {
                        1 => {
                            let type_idx = data.type_index() as usize;
                            if type_idx < il2cpp.types.len() {
                                let il2cpp_type = il2cpp.types[type_idx].clone();
                                let name = executor.get_type_name(&il2cpp_type, metadata, il2cpp, true, false);
                                rgctx_info.type_name = Some(fix_name(&name));
                            }
                        }
                        2 => {
                            let type_idx = data.type_index() as usize;
                            if type_idx < il2cpp.types.len() {
                                let il2cpp_type = il2cpp.types[type_idx].clone();
                                let name = executor.get_type_name(&il2cpp_type, metadata, il2cpp, true, false);
                                rgctx_info.class_name = Some(fix_name(&name));
                            }
                        }
                        3 => {
                            let method_idx = data.method_index() as usize;
                            if method_idx < il2cpp.method_specs.len() {
                                let (type_name, method_name) = executor.get_method_spec_name(method_idx, metadata, il2cpp, true);
                                rgctx_info.method_name = Some(fix_name(&format!("{}.{}", type_name, method_name)));
                            }
                        }
                        _ => {}
                    }
                }
                rgctxs.push(rgctx_info);
            }
        }
        rgctxs
    }

    fn generate_method_info(
        buf: &mut String,
        method_info_name: &str,
        struct_type_name: &str,
        rgctxs: &[StructRGCTXInfo],
        version: f64,
    ) {
        if !rgctxs.is_empty() {
            writeln!(buf, "struct {}_RGCTXs {{", method_info_name).ok();
            for (i, rgctx) in rgctxs.iter().enumerate() {
                match rgctx.rgctx_type {
                    1 => {
                        let tn = rgctx.type_name.as_deref().unwrap_or("unknown");
                        writeln!(buf, "\tIl2CppType* _{}_{};", i, tn).ok();
                    }
                    2 => {
                        let cn = rgctx.class_name.as_deref().unwrap_or("unknown");
                        writeln!(buf, "\tIl2CppClass* _{}_{};", i, cn).ok();
                    }
                    3 => {
                        let mn = rgctx.method_name.as_deref().unwrap_or("unknown");
                        writeln!(buf, "\tMethodInfo* _{}_{};", i, mn).ok();
                    }
                    _ => {}
                }
            }
            writeln!(buf, "}};").ok();
        }

        writeln!(buf, "struct {} {{", method_info_name).ok();
        writeln!(buf, "\tIl2CppMethodPointer methodPointer;").ok();
        if version >= 29.0 {
            writeln!(buf, "\tIl2CppMethodPointer virtualMethodPointer;").ok();
            writeln!(buf, "\tInvokerMethod invoker_method;").ok();
        } else {
            writeln!(buf, "\tvoid* invoker_method;").ok();
        }
        writeln!(buf, "\tconst char* name;").ok();
        if version <= 24.0 {
            writeln!(buf, "\t{}_c *declaring_type;", struct_type_name).ok();
        } else {
            writeln!(buf, "\t{}_c *klass;", struct_type_name).ok();
        }
        writeln!(buf, "\tconst Il2CppType *return_type;").ok();
        if version >= 29.0 {
            writeln!(buf, "\tconst Il2CppType** parameters;").ok();
        } else {
            writeln!(buf, "\tconst void* parameters;").ok();
        }
        if !rgctxs.is_empty() {
            writeln!(buf, "\tconst {}_RGCTXs* rgctx_data;", method_info_name).ok();
        } else {
            writeln!(buf, "\tconst Il2CppRGCTXData* rgctx_data;").ok();
        }
        writeln!(buf, "\tunion").ok();
        writeln!(buf, "\t{{").ok();
        writeln!(buf, "\t\tconst void* genericMethod;").ok();
        if version >= 27.0 {
            writeln!(buf, "\t\tconst void* genericContainerHandle;").ok();
        } else {
            writeln!(buf, "\t\tconst void* genericContainer;").ok();
        }
        writeln!(buf, "\t}};").ok();
        if version <= 24.0 {
            writeln!(buf, "\tint32_t customAttributeIndex;").ok();
        }
        writeln!(buf, "\tuint32_t token;").ok();
        writeln!(buf, "\tuint16_t flags;").ok();
        writeln!(buf, "\tuint16_t iflags;").ok();
        writeln!(buf, "\tuint16_t slot;").ok();
        writeln!(buf, "\tuint8_t parameters_count;").ok();
        writeln!(buf, "\tuint8_t bitflags;").ok();
        writeln!(buf, "}};").ok();
    }
}

fn fix_name(name: &str) -> String {
    if KEYWORDS.contains(&name) {
        return format!("_{}", name);
    }
    if SPECIAL_KEYWORDS.contains(&name) {
        return format!("_{}_", name);
    }

    let first_char = name.chars().next();
    let starts_with_digit = first_char.map(|c| c.is_ascii_digit()).unwrap_or(false);

    if starts_with_digit {
        return format!("_{}", name);
    }

    let mut result = String::with_capacity(name.len());
    for c in name.chars() {
        if c.is_ascii_alphanumeric() || c == '_' {
            result.push(c);
        } else {
            result.push('_');
        }
    }
    result
}

fn get_unique_name(name: &str, set: &mut HashSet<String>) -> String {
    let mut fix = name.to_string();
    let mut i = 1;
    while !set.insert(fix.clone()) {
        fix = format!("{}_{}", name, i);
        i += 1;
    }
    fix
}

fn get_method_type_signature(types: &[Il2CppTypeEnum]) -> String {
    let mut sig = String::with_capacity(types.len());
    for te in types {
        sig.push(match te {
            Il2CppTypeEnum::Void => 'v',
            Il2CppTypeEnum::Boolean | Il2CppTypeEnum::Char
            | Il2CppTypeEnum::I1 | Il2CppTypeEnum::U1
            | Il2CppTypeEnum::I2 | Il2CppTypeEnum::U2
            | Il2CppTypeEnum::I4 | Il2CppTypeEnum::U4 => 'i',
            Il2CppTypeEnum::I8 | Il2CppTypeEnum::U8 => 'j',
            Il2CppTypeEnum::R4 => 'f',
            Il2CppTypeEnum::R8 => 'd',
            _ => 'i',
        });
    }
    sig
}
