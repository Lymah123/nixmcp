use rmcp::schemars;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SearchPackagesRequest {
    /// Package name or search query.
    pub query: String,
}
