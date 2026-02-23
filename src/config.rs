use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Config {
    pub dump_method: bool,
    pub dump_field: bool,
    pub dump_property: bool,
    pub dump_attribute: bool,
    pub dump_method_offset: bool,
    pub dump_field_offset: bool,
    pub dump_type_def_index: bool,
    pub generate_struct: bool,
    pub generate_dummy_dll: bool,
    pub require_any_key: bool,
    pub dummy_dll_add_token: bool,
    pub force_il2cpp_version: bool,
    pub force_version: f64,
    pub force_dump: bool,
    pub no_redirected_pointer: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            dump_method: true,
            dump_field: true,
            dump_property: true,
            dump_attribute: true,
            dump_method_offset: true,
            dump_field_offset: true,
            dump_type_def_index: true,
            generate_struct: true,
            generate_dummy_dll: true,
            require_any_key: true,
            dummy_dll_add_token: true,
            force_il2cpp_version: false,
            force_version: 24.3,
            force_dump: false,
            no_redirected_pointer: false,
        }
    }
}

impl Config {
    pub fn load_from_file(path: &str) -> std::result::Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = serde_json::from_str(&content)?;
        Ok(config)
    }

    pub fn save_to_file(&self, path: &str) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }
}
