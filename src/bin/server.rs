use mcp_git_tools::register_git_tools;
use mcp_git_tools::McpServerBuilder;
use tracing_subscriber::EnvFilter;
use mcp_server::ByteTransport;
// Use tokio's async stdin/stdout
use tokio::io::{stdin, stdout};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive("mcp_git_tools=debug".parse().unwrap())
                .add_directive("mcp_server=info".parse().unwrap()),
        )
        .init();

    // Create the server
    let mut builder = McpServerBuilder::new("git-tools", "1.0.0");
    
    // Register Git tools
    register_git_tools(&mut builder);
    
    // Build the server
    let server = builder.build();
    
    println!("Starting Git Tools MCP server...");
    
    // Create a ByteTransport using tokio's stdin/stdout
    let transport = ByteTransport::new(stdin(), stdout());
    
    // Run the server
    server.run(transport).await?;
    
    Ok(())
} 