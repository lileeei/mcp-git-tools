//! MCP Git Tools library - Provides Git functionality through the Model Context Protocol

pub mod tools;

use mcp_core::handler::ToolHandler;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

// Import the correct components
use mcp_core::{
    content::Content,
    handler::{PromptError, ResourceError, ToolError},
    prompt::Prompt,
    protocol::ServerCapabilities,
    resource::Resource,
    tool::Tool,
};
use mcp_server::Server;
use mcp_server::router::{CapabilitiesBuilder, Router, RouterService};
use serde_json::Value;

/// Define our own ServerBuilder struct to register tools
pub struct McpServerBuilder {
    name: String,
    version: String,
    tools: Vec<Arc<dyn ToolHandler>>,
}

impl McpServerBuilder {
    /// Create a new server builder
    pub fn new(name: &str, version: &str) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            tools: Vec::new(),
        }
    }

    /// Add tools to the server
    pub fn add_tool(&mut self, tool: Arc<dyn ToolHandler>) -> &mut Self {
        self.tools.push(tool);
        self
    }

    /// Build the server
    pub fn build(self) -> Server<RouterService<GitToolsRouter>> {
        // Create router and add tools
        let capabilities = CapabilitiesBuilder::new().with_tools(false).build();

        // Create service
        let router = GitToolsRouter {
            name: self.name,
            version: self.version,
            tools: self.tools,
            capabilities,
        };

        // Create and return server
        Server::new(RouterService(router))
    }
}

/// Create Git tools router
#[derive(Clone)]
pub struct GitToolsRouter {
    name: String,
    version: String,
    tools: Vec<Arc<dyn ToolHandler>>,
    capabilities: ServerCapabilities,
}

impl Router for GitToolsRouter {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn instructions(&self) -> String {
        "Git tools for managing git repositories".to_string()
    }

    fn capabilities(&self) -> ServerCapabilities {
        self.capabilities.clone()
    }

    fn list_tools(&self) -> Vec<Tool> {
        self.tools
            .iter()
            .map(|tool| Tool {
                name: tool.name().to_string(),
                description: tool.description().to_string(),
                input_schema: tool.schema(),
            })
            .collect()
    }

    fn call_tool(
        &self,
        tool_name: &str,
        arguments: Value,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Content>, ToolError>> + Send + 'static>> {
        let tools = self.tools.clone();
        let tool_name = tool_name.to_string();

        Box::pin(async move {
            for tool in tools {
                if tool.name() == tool_name {
                    let result = tool.call(arguments).await?;
                    return Ok(vec![Content::text(result.to_string())]);
                }
            }

            Err(ToolError::NotFound(format!(
                "Tool '{}' not found",
                tool_name
            )))
        })
    }

    fn list_resources(&self) -> Vec<Resource> {
        Vec::new() // No resources in this implementation
    }

    fn read_resource(
        &self,
        _uri: &str,
    ) -> Pin<Box<dyn Future<Output = Result<String, ResourceError>> + Send + 'static>> {
        Box::pin(async {
            Err(ResourceError::NotFound(
                "Resources not supported".to_string(),
            ))
        })
    }

    fn list_prompts(&self) -> Vec<Prompt> {
        Vec::new() // No prompts in this implementation
    }

    fn get_prompt(
        &self,
        prompt_name: &str,
    ) -> Pin<Box<dyn Future<Output = Result<String, PromptError>> + Send + 'static>> {
        let prompt_name = prompt_name.to_string();
        Box::pin(async move {
            Err(PromptError::NotFound(format!(
                "Prompt '{}' not found",
                prompt_name
            )))
        })
    }
}

/// Register all Git tools
pub fn register_git_tools(builder: &mut McpServerBuilder) -> &mut McpServerBuilder {
    builder.add_tool(Arc::new(tools::GitStatusTool));
    builder.add_tool(Arc::new(tools::GitBranchesTool));
    builder.add_tool(Arc::new(tools::GitLogTool));
    builder.add_tool(Arc::new(tools::GitCommitTool));
    builder.add_tool(Arc::new(tools::GitPullTool));
    builder.add_tool(Arc::new(tools::GitPushTool));
    builder.add_tool(Arc::new(tools::GitDiffTool));
    builder.add_tool(Arc::new(tools::GitTimeFilteredLogTool));
    builder
}

/// Get a list of all Git tools
pub fn get_all_git_tools() -> Vec<Arc<dyn ToolHandler>> {
    vec![
        Arc::new(tools::GitStatusTool),
        Arc::new(tools::GitBranchesTool),
        Arc::new(tools::GitLogTool),
        Arc::new(tools::GitCommitTool),
        Arc::new(tools::GitPullTool),
        Arc::new(tools::GitPushTool),
        Arc::new(tools::GitDiffTool),
        Arc::new(tools::GitTimeFilteredLogTool),
    ]
}
