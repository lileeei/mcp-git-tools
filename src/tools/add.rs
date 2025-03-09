use crate::tools::run_git_command;
use async_trait::async_trait;
use mcp_core::handler::{ToolError, ToolHandler};
use schemars::{JsonSchema, schema_for};
use serde::Deserialize;
use serde_json::{Value, json};

/// Git add tool implementation
#[derive(Debug, Default)]
pub struct GitAddTool;

#[derive(Deserialize, JsonSchema)]
struct GitAddToolParams {
    #[schemars(description = "The path to the git repository")]
    repo_path: String,
    #[schemars(description = "The path(s) to add, or patterns to match. Use '.' for all files.")]
    path: String,
    #[schemars(description = "Whether to update, rather than add")]
    #[serde(default)]
    update: bool,
    #[schemars(description = "Whether to add all changes, including untracked files")]
    #[serde(default)]
    all: bool,
}

#[async_trait]
impl ToolHandler for GitAddTool {
    fn name(&self) -> &'static str {
        "git_add"
    }

    fn description(&self) -> &'static str {
        "Add file contents to the git staging area"
    }

    fn schema(&self) -> Value {
        serde_json::to_value(schema_for!(GitAddToolParams)).unwrap_or_default()
    }

    async fn call(&self, params: Value) -> Result<Value, ToolError> {
        let params: GitAddToolParams =
            serde_json::from_value(params).map_err(|e| ToolError::ExecutionError(e.to_string()))?;

        git_add(params.repo_path, params.path, params.update, params.all).await
    }
}

pub async fn git_add(
    repo_path: String,
    path: String,
    update: bool,
    all: bool,
) -> Result<Value, ToolError> {
    let mut args = vec!["add"];

    if update {
        args.push("--update");
    } else if all {
        args.push("--all");
    }

    args.push(&path);
    
    let add_output = run_git_command(&repo_path, &args)?;
    
    // Get the status after adding
    let status_output = run_git_command(&repo_path, &["status", "--porcelain"])?;
    let status_lines: Vec<&str> = status_output.lines().collect();

    Ok(json!({
        "success": true,
        "message": if add_output.is_empty() { "Files staged successfully" } else { &add_output },
        "status": status_lines
    }))
} 