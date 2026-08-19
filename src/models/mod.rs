use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Package {
    pub attribute: String,
    pub pname: String,
    pub version: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NixosOption {
    pub path: String,
    pub option_type: Option<String>,
    pub description: Option<String>,
    pub default: Option<serde_json::Value>,
}
