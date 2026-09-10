use rmcp::schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SearchOptionsRequest {
    /// Search query for NixOS option paths, for example "postgresql".
    pub query: String,
}
