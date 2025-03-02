use crate::tools::run_git_command;
use async_trait::async_trait;
use mcp_core::handler::{ToolError, ToolHandler};
use schemars::{JsonSchema, schema_for};
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Debug, Default)]
pub struct GitCommitTool;

#[derive(Deserialize, JsonSchema)]
struct GitCommitToolParams {
    #[schemars(description = "The path to the git repository")]
    repo_path: String,
    #[schemars(description = "The commit message")]
    message: String,
    #[schemars(description = "Whether to add all changes")]
    #[serde(default)]
    all: Option<bool>,
}

#[async_trait]
impl ToolHandler for GitCommitTool {
    fn name(&self) -> &'static str {
        "git_commit"
    }

    fn description(&self) -> &'static str {
        "Create a commit with the staged changes"
    }

    fn schema(&self) -> Value {
        serde_json::to_value(schema_for!(GitCommitToolParams)).unwrap_or_default()
    }

    async fn call(&self, params: Value) -> Result<Value, ToolError> {
        let params: GitCommitToolParams =
            serde_json::from_value(params).map_err(|e| ToolError::ExecutionError(e.to_string()))?;

        git_commit(params.repo_path, params.message, params.all).await
    }
}

pub async fn git_commit(
    repo_path: String,
    message: String,
    all: Option<bool>,
) -> Result<Value, ToolError> {
    let mut args = vec!["commit", "-m", &message];

    if all.unwrap_or(false) {
        args.push("-a");
    }

    let commit_output = run_git_command(&repo_path, &args)?;

    // Get the commit hash of the latest commit
    let commit_hash = run_git_command(&repo_path, &["rev-parse", "HEAD"])?
        .trim()
        .to_string();

    Ok(json!({
        "success": true,
        "hash": commit_hash,
        "message": message,
        "output": commit_output
    }))
}
