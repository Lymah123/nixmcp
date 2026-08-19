use rmcp::schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetOptionRequest {
    /// NixOS option path, for example "services.postgresql.enable".
    pub path: String,
}
