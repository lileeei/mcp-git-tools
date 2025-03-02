use crate::tools::run_git_command;
use async_trait::async_trait;
use mcp_core::handler::{ToolError, ToolHandler};
use schemars::{JsonSchema, schema_for};
use serde::Deserialize;
use serde_json::{Value, json};

/// Git status tool implementation
#[derive(Debug, Default)]
pub struct GitStatusTool;

#[derive(Deserialize, JsonSchema)]
struct GitStatusToolParams {
    #[schemars(description = "The path to the git repository")]
    repo_path: String,
}

#[async_trait]
impl ToolHandler for GitStatusTool {
    fn name(&self) -> &'static str {
        "git_status"
    }

    fn description(&self) -> &'static str {
        "Get the status of a git repository"
    }

    fn schema(&self) -> Value {
        serde_json::to_value(schema_for!(GitStatusToolParams)).unwrap_or_default()
    }

    async fn call(&self, params: Value) -> Result<Value, ToolError> {
        let params: GitStatusToolParams =
            serde_json::from_value(params).map_err(|e| ToolError::ExecutionError(e.to_string()))?;

        git_status(params.repo_path).await
    }
}

pub async fn git_status(repo_path: String) -> Result<Value, ToolError> {
    let status_output = run_git_command(&repo_path, &["status", "--porcelain"])?;
    let status_lines: Vec<&str> = status_output.lines().collect();

    Ok(json!({
        "status": status_lines,
        "is_clean": status_lines.is_empty()
    }))
}
