use rmcp::schemars;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct InspectFlakeRequest {
    /// Flake reference, for example "nixpkgs" or "github:owner/repo".
    pub reference: String,
}
