use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptMethod {
    #[serde(rename = "Address")]
    pub address: u64,
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Signature")]
    pub signature: String,
    #[serde(rename = "TypeSignature")]
    pub type_signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptString {
    #[serde(rename = "Address")]
    pub address: u64,
    #[serde(rename = "Value")]
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptMetadata {
    #[serde(rename = "Address")]
    pub address: u64,
    #[serde(rename = "Name")]
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptMetadataMethod {
    #[serde(rename = "Address")]
    pub address: u64,
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "MethodAddress")]
    pub method_address: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScriptJson {
    #[serde(rename = "ScriptMethod")]
    pub script_methods: Vec<ScriptMethod>,
    #[serde(rename = "ScriptString")]
    pub script_strings: Vec<ScriptString>,
    #[serde(rename = "ScriptMetadata")]
    pub script_metadata: Vec<ScriptMetadata>,
    #[serde(rename = "ScriptMetadataMethod")]
    pub script_metadata_methods: Vec<ScriptMetadataMethod>,
    #[serde(rename = "Addresses")]
    pub addresses: Vec<u64>,
}

impl ScriptJson {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StringLiteralEntry {
    pub index: usize,
    pub value: String,
}
