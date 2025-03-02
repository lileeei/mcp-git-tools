use crate::tools::run_git_command;
use mcp_core::handler::{ToolError, ToolHandler};
use serde_json::{Value, json};

/// Git diff tool implementation
#[derive(Default)]
pub struct GitDiffTool;

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct GitDiffToolParams {
    #[schemars(description = "The path to the git repository")]
    repo_path: String,
    #[schemars(description = "The path to the file to diff")]
    path: Option<String>,
    #[schemars(description = "Whether to show staged changes")]
    staged: Option<bool>,
    #[schemars(description = "The commit to diff against")]
    commit: Option<String>,
}

#[async_trait::async_trait]
impl ToolHandler for GitDiffTool {
    fn name(&self) -> &'static str {
        "git_diff"
    }

    fn description(&self) -> &'static str {
        "Show changes between commits, commit and working tree, etc"
    }

    fn schema(&self) -> Value {
        mcp_core::handler::generate_schema::<GitDiffToolParams>()
            .expect("Failed to generate schema")
    }

    async fn call(&self, params: Value) -> Result<Value, ToolError> {
        let params: GitDiffToolParams =
            serde_json::from_value(params).map_err(|e| ToolError::ExecutionError(e.to_string()))?;

        git_diff(params.repo_path, params.path, params.staged, params.commit).await
    }
}

pub async fn git_diff(
    repo_path: String,
    path: Option<String>,
    staged: Option<bool>,
    commit: Option<String>,
) -> Result<Value, ToolError> {
    // Create basic command arguments
    let base_args = vec!["diff"];

    // Collect all arguments
    let mut cmd_args = Vec::new();
    cmd_args.extend_from_slice(&base_args);

    // Handle staged option
    if staged.unwrap_or(false) {
        cmd_args.push("--staged");
    }

    // Handle commit parameter
    if let Some(ref commit_ref) = commit {
        cmd_args.push(commit_ref);
    }

    // Handle path parameter
    if let Some(ref file_path) = path {
        cmd_args.push("--");
        cmd_args.push(file_path);
    }

    let diff_output = run_git_command(&repo_path, &cmd_args)?;

    Ok(json!({
        "changes": diff_output
    }))
}
