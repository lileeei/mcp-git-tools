use crate::tools::run_git_command;
use async_trait::async_trait;
use mcp_core::handler::{ToolError, ToolHandler};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

/// Git push tool implementation
#[derive(Debug, Default)]
pub struct GitPushTool;

#[derive(Deserialize, JsonSchema)]
struct GitPushToolParams {
    #[schemars(description = "The path to the git repository")]
    repo_path: String,
    #[schemars(description = "The remote to push to")]
    #[serde(default)]
    remote: String,
    #[schemars(description = "The branch to push")]
    #[serde(default)]
    branch: String,
    #[schemars(description = "Whether to force push")]
    #[serde(default)]
    force: bool,
}

#[async_trait]
impl ToolHandler for GitPushTool {
    fn name(&self) -> &'static str {
        "git_push"
    }

    fn description(&self) -> &'static str {
        "Push local commits to a remote repository"
    }

    fn schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(GitPushToolParams)).unwrap_or_default()
    }

    async fn call(&self, params: Value) -> Result<Value, ToolError> {
        let params: GitPushToolParams =
            serde_json::from_value(params).map_err(|e| ToolError::ExecutionError(e.to_string()))?;

        let remote = if params.remote.is_empty() {
            None
        } else {
            Some(params.remote)
        };

        let branch = if params.branch.is_empty() {
            None
        } else {
            Some(params.branch)
        };
        
        let force = if params.force {
            Some(true)
        } else {
            None
        };

        git_push(params.repo_path, remote, branch, force).await
    }
}

pub async fn git_push(
    repo_path: String,
    remote: Option<String>,
    branch: Option<String>,
    force: Option<bool>,
) -> Result<Value, ToolError> {
    let remote_name = remote.unwrap_or_else(|| "origin".to_string());

    let mut args = Vec::new();
    args.push("push");
    args.push(&remote_name);

    if let Some(ref branch_name) = branch {
        args.push(branch_name);
    }

    if force.unwrap_or(false) {
        args.push("--force");
    }

    let push_output = run_git_command(&repo_path, &args)?;

    Ok(json!({
        "success": true,
        "remote": remote_name,
        "output": push_output.trim()
    }))
}
