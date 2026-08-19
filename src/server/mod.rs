use rmcp::{
    ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
};

use crate::{
    clients::nix::NixClient,
    tools::{
        get_option::GetOptionRequest, get_package::GetPackageRequest,
        inspect_flake::InspectFlakeRequest, search_packages::SearchPackagesRequest,
    },
};

#[derive(Debug, Clone)]
pub struct NixMcpServer {
    tool_router: ToolRouter<Self>,
    nix_client: NixClient,
}

impl NixMcpServer {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
            nix_client: NixClient::new(),
        }
    }
}

impl Default for NixMcpServer {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_router]
impl NixMcpServer {
    /// Search the Nix package collection using the local Nix installation.
    #[tool(
        name = "search_packages",
        description = "Search the Nix package collection using the local Nix installation."
    )]
    pub async fn search_packages(
        &self,
        Parameters(SearchPackagesRequest { query }): Parameters<SearchPackagesRequest>,
    ) -> Result<String, String> {
        let packages = self.nix_client.search_packages(&query).await?;

        serde_json::to_string(&packages)
            .map_err(|error| format!("Failed to serialize search results: {error}"))
    }

    /// Get information about a specific Nix package.
    #[tool(
        name = "get_package",
        description = "Get information about a specific Nix package."
    )]
    pub async fn get_package(
        &self,
        Parameters(GetPackageRequest { attribute }): Parameters<GetPackageRequest>,
    ) -> Result<String, String> {
        let package = self.nix_client.get_package(&attribute).await?;

        serde_json::to_string(&package)
            .map_err(|error| format!("Failed to serialize package: {error}"))
    }

    /// Inspect the outputs exposed by a Nix flake.
    #[tool(
        name = "inspect_flake",
        description = "Inspect the outputs exposed by a Nix flake."
    )]
    pub async fn inspect_flake(
        &self,
        Parameters(InspectFlakeRequest { reference }): Parameters<InspectFlakeRequest>,
    ) -> Result<String, String> {
        let flake = self.nix_client.inspect_flake(&reference).await?;

        serde_json::to_string(&flake)
            .map_err(|error| format!("Failed to serialize flake information: {error}"))
    }

    /// Get metadata about a NixOS configuration option.
    #[tool(
        name = "get_option",
        description = "Get metadata about a NixOS configuration option."
    )]
    pub async fn get_option(
        &self,
        Parameters(GetOptionRequest { path }): Parameters<GetOptionRequest>,
    ) -> Result<String, String> {
        let option = self.nix_client.get_option(&path).await?;

        serde_json::to_string(&option)
            .map_err(|error| format!("Failed to serialize NixOS option: {error}"))
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for NixMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_instructions("NixMCP provides live access to Nix package and system information.")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn search_packages_executes_nix() {
        let server = NixMcpServer::new();

        let result = server
            .search_packages(Parameters(SearchPackagesRequest {
                query: "ripgrep".to_string(),
            }))
            .await;

        assert!(result.is_ok(), "search failed: {result:?}");

        let output = result.unwrap();

        assert!(output.contains("\"ripgrep\""));
    }

    #[tokio::test]
    async fn inspect_flake_executes_nix() {
        let server = NixMcpServer::new();

        let result = server
            .inspect_flake(Parameters(InspectFlakeRequest {
                reference: "nixpkgs".to_string(),
            }))
            .await;

        assert!(result.is_ok(), "flake inspection failed: {result:?}");

        let output = result.unwrap();

        assert!(output.contains("\"nixosModules\""));
    }
}

#[cfg(test)]
mod mcp_tests {
    use super::*;
    use rmcp::{ClientHandler, ServiceExt};

    #[derive(Debug, Clone, Default)]
    struct TestClient;

    impl ClientHandler for TestClient {
        fn get_info(&self) -> rmcp::model::ClientInfo {
            rmcp::model::ClientInfo::default()
        }
    }

    #[tokio::test]
    async fn inspect_flake_mcp_call_works() -> anyhow::Result<()> {
        let (server_transport, client_transport) = tokio::io::duplex(16 * 1024);

        let server = NixMcpServer::new();

        let server_handle = tokio::spawn(async move {
            server.serve(server_transport).await?.waiting().await?;

            anyhow::Ok(())
        });

        let client = TestClient.serve(client_transport).await?;

        let result = client
            .call_tool(
                rmcp::model::CallToolRequestParams::new("inspect_flake").with_arguments(
                    serde_json::json!({
                        "reference": "nixpkgs"
                    })
                    .as_object()
                    .unwrap()
                    .clone(),
                ),
            )
            .await?;

        let text = result
            .content
            .first()
            .and_then(|content| content.as_text())
            .map(|text| text.text.as_str())
            .expect("Expected text content");

        assert!(text.contains("\"nixosModules\""));

        client.cancel().await?;
        server_handle.await??;

        Ok(())
    }

    #[tokio::test]
    async fn search_packages_mcp_call_works() -> anyhow::Result<()> {
        let (server_transport, client_transport) = tokio::io::duplex(16 * 1024);

        let server = NixMcpServer::new();

        let server_handle = tokio::spawn(async move {
            server.serve(server_transport).await?.waiting().await?;

            anyhow::Ok(())
        });

        let client = TestClient.serve(client_transport).await?;

        let result = client
            .call_tool(
                rmcp::model::CallToolRequestParams::new("search_packages").with_arguments(
                    serde_json::json!({
                        "query": "ripgrep"
                    })
                    .as_object()
                    .unwrap()
                    .clone(),
                ),
            )
            .await?;

        let text = result
            .content
            .first()
            .and_then(|content| content.as_text())
            .map(|text| text.text.as_str())
            .expect("Expected text content");

        assert!(text.contains("\"ripgrep\""));

        client.cancel().await?;
        server_handle.await??;

        Ok(())
    }

    #[tokio::test]
    async fn get_package_mcp_call_works() -> anyhow::Result<()> {
        let (server_transport, client_transport) = tokio::io::duplex(16 * 1024);

        let server = NixMcpServer::new();

        let server_handle = tokio::spawn(async move {
            server.serve(server_transport).await?.waiting().await?;

            anyhow::Ok(())
        });

        let client = TestClient.serve(client_transport).await?;

        let result = client
            .call_tool(
                rmcp::model::CallToolRequestParams::new("get_package").with_arguments(
                    serde_json::json!({
                        "attribute": "ripgrep"
                    })
                    .as_object()
                    .unwrap()
                    .clone(),
                ),
            )
            .await?;

        let text = result
            .content
            .first()
            .and_then(|content| content.as_text())
            .map(|text| text.text.as_str())
            .expect("Expected text content");

        assert!(text.contains("\"pname\":\"ripgrep\""));
        assert!(text.contains("\"version\":\"15.2.0\""));

        client.cancel().await?;
        server_handle.await??;

        Ok(())
    }

    #[tokio::test]
    async fn get_option_mcp_call_works() -> anyhow::Result<()> {
        let (server_transport, client_transport) = tokio::io::duplex(16 * 1024);

        let server = NixMcpServer::new();

        let server_handle = tokio::spawn(async move {
            server.serve(server_transport).await?.waiting().await?;

            anyhow::Ok(())
        });

        let client = TestClient.serve(client_transport).await?;

        let result = client
            .call_tool(
                rmcp::model::CallToolRequestParams::new("get_option").with_arguments(
                    serde_json::json!({
                        "path": "services.postgresql.enable"
                    })
                    .as_object()
                    .unwrap()
                    .clone(),
                ),
            )
            .await?;

        let text = result
            .content
            .first()
            .and_then(|content| content.as_text())
            .map(|text| text.text.as_str())
            .expect("Expected text content");

        assert!(text.contains("\"path\":\"services.postgresql.enable\""));
        assert!(text.contains("Whether to enable PostgreSQL Server."));
        assert!(text.contains("\"option_type\":\"bool\""));
        assert!(text.contains("\"default\":false"));

        client.cancel().await?;
        server_handle.await??;

        Ok(())
    }
}
