use rmcp::schemars;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct GetPackageRequest {
    /// Nix package attribute, for example "ripgrep".
    pub attribute: String,
}
