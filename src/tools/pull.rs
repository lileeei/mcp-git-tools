use crate::tools::run_git_command;
use async_trait::async_trait;
use mcp_core::handler::{ToolError, ToolHandler};
use schemars::{JsonSchema, schema_for};
use serde::Deserialize;
use serde_json::{Value, json};

/// Git pull tool implementation
#[derive(Debug, Default)]
pub struct GitPullTool;

#[derive(Deserialize, JsonSchema)]
struct GitPullToolParams {
    #[schemars(description = "The path to the git repository")]
    repo_path: String,
    #[schemars(description = "The remote to pull from")]
    remote: Option<String>,
    #[schemars(description = "The branch to pull")]
    branch: Option<String>,
}

#[async_trait]
impl ToolHandler for GitPullTool {
    fn name(&self) -> &'static str {
        "git_pull"
    }

    fn description(&self) -> &'static str {
        "Pull changes from a remote repository"
    }

    fn schema(&self) -> Value {
        serde_json::to_value(schema_for!(GitPullToolParams)).unwrap_or_default()
    }

    async fn call(&self, params: Value) -> Result<Value, ToolError> {
        let params: GitPullToolParams =
            serde_json::from_value(params).map_err(|e| ToolError::ExecutionError(e.to_string()))?;

        git_pull(params.repo_path, params.remote, params.branch).await
    }
}

pub async fn git_pull(
    repo_path: String,
    remote: Option<String>,
    branch: Option<String>,
) -> Result<Value, ToolError> {
    let remote_name = remote.unwrap_or_else(|| "origin".to_string());

    let mut args = Vec::new();
    args.push("pull");
    args.push(&remote_name);

    if let Some(ref branch_name) = branch {
        args.push(branch_name);
    }

    let pull_output = run_git_command(&repo_path, &args)?;

    Ok(json!({
        "success": true,
        "remote": remote_name,
        "output": pull_output.trim()
    }))
}
