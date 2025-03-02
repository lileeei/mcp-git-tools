use crate::tools::run_git_command;
use mcp_core::{ToolError, handler::ToolHandler};
use serde_json::{Value, json};

/// Git log tool implementation
#[derive(Default)]
pub struct GitLogTool;

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct GitLogToolParams {
    #[schemars(description = "The path to the git repository")]
    repo_path: String,
    #[schemars(description = "The maximum number of commits to return")]
    #[serde(default)]
    max_count: Option<u32>,
    #[schemars(description = "The branch to filter commits by")]
    #[serde(default)]
    branch: Option<String>,
}

#[async_trait::async_trait]
impl ToolHandler for GitLogTool {
    #[doc = " The name of the tool"]
    fn name(&self) -> &'static str {
        "git_log"
    }

    fn description(&self) -> &'static str {
        "Get the commit history of a git repository"
    }

    fn schema(&self) -> Value {
        mcp_core::handler::generate_schema::<GitLogToolParams>().expect("Failed to generate schema")
    }

    async fn call(
        &self,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, mcp_core::handler::ToolError> {
        let params: GitLogToolParams =
            serde_json::from_value(params).map_err(|e| ToolError::ExecutionError(e.to_string()))?;

        git_log(params.repo_path, params.max_count, params.branch).await
    }
}


pub async fn git_log(
    repo_path: String,
    max_count: Option<u32>,
    branch: Option<String>,
) -> Result<Value, ToolError> {
    // Create basic command arguments
    let base_args = vec!["log", "--pretty=format:%H|%an|%ad|%s"];

    // Collect all arguments
    let mut cmd_args = Vec::new();
    cmd_args.extend_from_slice(&base_args);

    // Handle max_count option
    if let Some(count) = max_count {
        let count_str = count.to_string();
        cmd_args.push("-n");
        cmd_args.push(&count_str[..]);

        // Ensure branch parameter doesn't cause lifetime issues
        if let Some(ref b) = branch {
            cmd_args.push(b);
        }

        let log_output = run_git_command(&repo_path, &cmd_args)?;

        let commits: Vec<Value> = log_output
            .lines()
            .filter(|line| !line.is_empty())
            .map(|line| {
                let parts: Vec<&str> = line.split('|').collect();
                if parts.len() >= 4 {
                    json!({
                        "hash": parts[0],
                        "author": parts[1],
                        "date": parts[2],
                        "message": parts[3]
                    })
                } else {
                    json!({ "raw": line })
                }
            })
            .collect();

        return Ok(json!({ "commits": commits }));
    }

    // If no max_count
    if let Some(ref b) = branch {
        cmd_args.push(b);
    }

    let log_output = run_git_command(&repo_path, &cmd_args)?;

    let commits: Vec<Value> = log_output
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| {
            let parts: Vec<&str> = line.split('|').collect();
            if parts.len() >= 4 {
                json!({
                    "hash": parts[0],
                    "author": parts[1],
                    "date": parts[2],
                    "message": parts[3]
                })
            } else {
                json!({ "raw": line })
            }
        })
        .collect();

    Ok(json!({ "commits": commits }))
}
