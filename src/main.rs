mod cache;
mod clients;
mod error;
mod models;
mod server;
mod tools;

use rmcp::ServiceExt;
use server::NixMcpServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = NixMcpServer::new();

    let service = server.serve(rmcp::transport::stdio()).await?;

    service.waiting().await?;

    Ok(())
}
