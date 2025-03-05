use crate::tools::run_git_command;
use mcp_core::{ToolError, handler::ToolHandler};
use schemars::{schema_for, JsonSchema};
use serde::Deserialize;
use serde_json::{Value, json};

/// Git time filtered log tool implementation
#[derive(Default, Debug)]
pub struct GitTimeFilteredLogTool;

#[derive(Debug, Deserialize, JsonSchema)]
struct GitTimeFilteredLogToolParams {
    #[schemars(description = "The path to the git repository")]
    repo_path: String,
    #[schemars(description = "The start date")]
    since: String,        // Start date (e.g., "2023-01-01", "1 week ago", "yesterday")
    #[schemars(description = "The end date")]
    #[serde(default)]
    until: String, // End date, optional (e.g., "2023-01-31", "today")
    #[schemars(description = "The author to filter by")]
    #[serde(default)]
    author: String, // Filter by author, optional
    #[schemars(description = "The branch to filter by")]
    #[serde(default)]
    branch: String, // Filter by branch, optional
}

#[async_trait::async_trait]
impl ToolHandler for GitTimeFilteredLogTool {
    #[doc = " The name of the tool"]
    fn name(&self) -> &'static str {
        "git_time_filtered_log"
    }

    fn description(&self) -> &'static str {
        "Get commits within a specified time range, optionally filtered by author and branch"
    }

    fn schema(&self) -> Value {
        serde_json::to_value(schema_for!(GitTimeFilteredLogToolParams)).unwrap_or_default()
    }

    async fn call(
        &self,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, mcp_core::handler::ToolError> {
        let params: GitTimeFilteredLogToolParams =
            serde_json::from_value(params).map_err(|e| ToolError::ExecutionError(e.to_string()))?;

        let until = if params.until.is_empty() {
            None
        } else {
            Some(params.until)
        };

        let author = if params.author.is_empty() {
            None
        } else {
            Some(params.author)
        };
        
        let branch = if params.branch.is_empty() {
            None
        } else {
            Some(params.branch)
        };

        git_time_filtered_log(
            params.repo_path,
            params.since,
            until,
            author,
            branch
        ).await
    }
}

pub async fn git_time_filtered_log(
    repo_path: String,
    since: String,
    until: Option<String>,
    author: Option<String>,
    branch: Option<String>,
) -> Result<Value, ToolError> {
    // Store the basic command parts
    let mut args = Vec::new();
    args.push("log".to_string());
    args.push("--pretty=format:%H|%an|%ad|%s".to_string());
    
    // Add time range filters
    args.push(format!("--since={}", since));
    
    if let Some(until_date) = &until {
        args.push(format!("--until={}", until_date));
    }
    
    // Add author filter if specified
    if let Some(author_name) = &author {
        args.push(format!("--author={}", author_name));
    }
    
    // Add branch if specified
    if let Some(branch_name) = &branch {
        args.push(branch_name.clone());
    }
    
    // Convert String arguments to &str for run_git_command
    let cmd_args: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    
    // Execute git command with the collected arguments
    let log_output = run_git_command(&repo_path, &cmd_args)?;
    
    // Parse the output into structured data
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
    
    Ok(json!({
        "commits": commits,
        "filters": {
            "since": since,
            "until": until,
            "author": author,
            "branch": branch
        }
    }))
} 