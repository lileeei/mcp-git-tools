use crate::tools::run_git_command;
use async_trait::async_trait;
use mcp_core::handler::{ToolError, ToolHandler};
use schemars::{JsonSchema, schema_for};
use serde::Deserialize;
use serde_json::{Value, json};

/// Git reset tool implementation
#[derive(Debug, Default)]
pub struct GitResetTool;

#[derive(Deserialize, JsonSchema)]
struct GitResetToolParams {
    #[schemars(description = "The path to the git repository")]
    repo_path: String,
    #[schemars(description = "The path(s) to reset, or patterns to match. Use '.' for all files.")]
    path: String,
    #[schemars(description = "Whether to perform a hard reset (WARNING: discards all local changes)")]
    #[serde(default)]
    hard: bool,
    #[schemars(description = "The commit or branch to reset to (defaults to HEAD)")]
    #[serde(default)]
    target: Option<String>,
}

#[async_trait]
impl ToolHandler for GitResetTool {
    fn name(&self) -> &'static str {
        "git_reset"
    }

    fn description(&self) -> &'static str {
        "Reset the staging area or working tree to a specified state"
    }

    fn schema(&self) -> Value {
        serde_json::to_value(schema_for!(GitResetToolParams)).unwrap_or_default()
    }

    async fn call(&self, params: Value) -> Result<Value, ToolError> {
        let params: GitResetToolParams =
            serde_json::from_value(params).map_err(|e| ToolError::ExecutionError(e.to_string()))?;

        git_reset(params.repo_path, params.path, params.hard, params.target).await
    }
}

pub async fn git_reset(
    repo_path: String,
    path: String,
    hard: bool,
    target: Option<String>,
) -> Result<Value, ToolError> {
    let mut args = vec!["reset"];

    if hard {
        args.push("--hard");
    }

    // If target is provided, add it
    if let Some(target_ref) = target.as_deref() {
        args.push(target_ref);
    }

    // Add the file path
    if !path.is_empty() && path != "." {
        args.push("--");
        args.push(&path);
    }
    
    let reset_output = run_git_command(&repo_path, &args)?;
    
    // Get the status after resetting
    let status_output = run_git_command(&repo_path, &["status", "--porcelain"])?;
    let status_lines: Vec<&str> = status_output.lines().collect();

    Ok(json!({
        "success": true,
        "message": if reset_output.is_empty() { 
            if hard {
                "Hard reset performed successfully"
            } else {
                "Files unstaged successfully"
            }
        } else { 
            &reset_output 
        },
        "status": status_lines
    }))
} 