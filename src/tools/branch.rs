use crate::tools::run_git_command;
use mcp_core::handler::{ToolError, ToolHandler};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

/// Git branches tool implementation
#[derive(Default)]
pub struct GitBranchesTool;

#[derive(Deserialize, JsonSchema)]
struct GitBranchesToolParam {
    repo_path: String,
}

#[async_trait::async_trait]
impl ToolHandler for GitBranchesTool {
    fn name(&self) -> &'static str {
        "git_branches"
    }

    fn description(&self) -> &'static str {
        "List all branches in a git repository"
    }

    fn schema(&self) -> Value {
        mcp_core::handler::generate_schema::<GitBranchesToolParam>()
            .expect("Failed to generate schema")
    }

    async fn call(&self, params: Value) -> Result<Value, ToolError> {
        let params: GitBranchesToolParam =
            serde_json::from_value(params).map_err(|e| ToolError::ExecutionError(e.to_string()))?;

        git_branches(params.repo_path).await
    }
}

pub async fn git_branches(repo_path: String) -> Result<Value, ToolError> {
    let branch_output = run_git_command(&repo_path, &["branch", "--list"])?;

    // Parse branches, trim whitespace and identify current branch with asterisk
    let branches: Vec<String> = branch_output
        .lines()
        .map(|line| line.trim().to_string())
        .collect();

    // Get current branch separately
    let current_branch = run_git_command(&repo_path, &["rev-parse", "--abbrev-ref", "HEAD"])?
        .trim()
        .to_string();

    Ok(json!({
        "branches": branches,
        "current": current_branch
    }))
}
