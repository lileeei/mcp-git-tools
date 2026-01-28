use mcp_git_tools::GitToolsHandler;
use rmcp::{ServiceExt, transport::stdio};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging - output to stderr to avoid interfering with MCP protocol
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive("mcp_git_tools=debug".parse()?)
                .add_directive("rmcp=warn".parse()?),
        )
        .init();

    // Create a Git tools handler
    let handler = GitToolsHandler::new();

    eprintln!("Starting Git Tools MCP server...");

    // Create transport and serve
    let transport = stdio();

    eprintln!("Starting server transport...");

    // Serve the handler - this will block until the client disconnects
    let server = match handler.serve(transport).await {
        Ok(s) => {
            eprintln!("Server started successfully, waiting for MCP client connection...");
            s
        }
        Err(e) => {
            tracing::error!("Failed to start server: {:?}", e);
            return Err(e.into());
        }
    };

    // Wait for server to finish
    let result = server.waiting().await;

    if let Err(e) = result {
        tracing::error!("Server error: {:?}", e);
    }

    Ok(())
}
