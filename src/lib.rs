//! MCP Git Tools library - Provides Git functionality through the Model Context Protocol

use anyhow::Result;
use rmcp::{
    model::*,
    schemars, serde,
    serde_json::{self, json},
    ServerHandler,
};
use std::process::Command;
use std::sync::Arc;

/// Run a git command in the specified repository directory and return the output
pub fn run_git_command(repo_path: &str, args: &[&str]) -> Result<String> {
    // Validate repository path
    let path = std::path::Path::new(repo_path);
    if !path.exists() {
        anyhow::bail!("Repository path does not exist: {}", repo_path);
    }

    // Check if it's a git repository
    let git_dir = path.join(".git");
    if !git_dir.is_dir() {
        anyhow::bail!("Not a git repository: {}", repo_path);
    }

    let output = Command::new("git")
        .args(args)
        .current_dir(repo_path)
        .output()?;

    if !output.status.success() {
        anyhow::bail!(
            "Git command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Git tools handler - central struct for all git operations
#[derive(Clone, Default, Debug)]
pub struct GitToolsHandler;

impl GitToolsHandler {
    /// Create a new Git tools handler
    pub fn new() -> Self {
        Self
    }

    /// Get the status of a git repository
    pub async fn git_status(&self, repo_path: String) -> Result<CallToolResult> {
        let status_output = run_git_command(&repo_path, &["status", "--porcelain"])?;
        let status_lines: Vec<&str> = status_output.lines().collect();

        let result = json!({
            "status": status_lines,
            "is_clean": status_lines.is_empty()
        });

        Ok(CallToolResult::success(vec![Content::json(result)?]))
    }

    /// Get the commit history of a git repository
    pub async fn git_log(
        &self,
        repo_path: String,
        max_count: Option<u32>,
        branch: Option<String>,
    ) -> Result<CallToolResult> {
        let mut cmd_args: Vec<String> = vec![
            "log".to_string(),
            "--pretty=format:%H|%an|%ad|%s".to_string(),
        ];

        if let Some(count) = max_count {
            cmd_args.push("-n".to_string());
            cmd_args.push(count.to_string());
        }

        if let Some(b) = branch {
            cmd_args.push(b);
        }

        let cmd_args_refs: Vec<&str> = cmd_args.iter().map(|s| s.as_str()).collect();
        let log_output = run_git_command(&repo_path, &cmd_args_refs)?;

        let commits: Vec<serde_json::Value> = log_output
            .lines()
            .filter(|line: &&str| !line.is_empty())
            .map(|line: &str| {
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

        let result = json!({ "commits": commits });

        Ok(CallToolResult::success(vec![Content::json(result)?]))
    }

    /// List all branches in a git repository
    pub async fn git_branches(&self, repo_path: String) -> Result<CallToolResult> {
        let branch_output = run_git_command(&repo_path, &["branch", "--list"])?;

        let branches: Vec<String> = branch_output
            .lines()
            .map(|line: &str| line.trim().to_string())
            .collect();

        let current_branch = run_git_command(&repo_path, &["rev-parse", "--abbrev-ref", "HEAD"])?
            .trim()
            .to_string();

        let result = json!({
            "branches": branches,
            "current": current_branch
        });

        Ok(CallToolResult::success(vec![Content::json(result)?]))
    }

    /// Create a commit with the staged changes
    pub async fn git_commit(
        &self,
        repo_path: String,
        message: String,
        all: Option<bool>,
    ) -> Result<CallToolResult> {
        let mut args: Vec<String> = vec!["commit".to_string(), "-m".to_string(), message.clone()];

        if all.unwrap_or(false) {
            args.push("-a".to_string());
        }

        let cmd_args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        let commit_output = run_git_command(&repo_path, &cmd_args_refs)?;

        let commit_hash = run_git_command(&repo_path, &["rev-parse", "HEAD"])?
            .trim()
            .to_string();

        let result = json!({
            "success": true,
            "hash": commit_hash,
            "message": message,
            "output": commit_output
        });

        Ok(CallToolResult::success(vec![Content::json(result)?]))
    }

    /// Pull changes from a remote repository
    pub async fn git_pull(
        &self,
        repo_path: String,
        remote: Option<String>,
        branch: Option<String>,
    ) -> Result<CallToolResult> {
        let remote_name = remote.unwrap_or_else(|| "origin".to_string());

        let mut args: Vec<String> = vec!["pull".to_string(), remote_name.clone()];

        if let Some(branch_name) = branch {
            args.push(branch_name);
        }

        let cmd_args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        let pull_output = run_git_command(&repo_path, &cmd_args_refs)?;

        let result = json!({
            "success": true,
            "remote": remote_name,
            "output": pull_output.trim()
        });

        Ok(CallToolResult::success(vec![Content::json(result)?]))
    }

    /// Push local commits to a remote repository
    pub async fn git_push(
        &self,
        repo_path: String,
        remote: Option<String>,
        branch: Option<String>,
        force: Option<bool>,
    ) -> Result<CallToolResult> {
        let remote_name = remote.unwrap_or_else(|| "origin".to_string());

        let mut args: Vec<String> = vec!["push".to_string(), remote_name.clone()];

        if let Some(branch_name) = branch {
            args.push(branch_name);
        }

        if force.unwrap_or(false) {
            args.push("--force".to_string());
        }

        let cmd_args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        let push_output = run_git_command(&repo_path, &cmd_args_refs)?;

        let result = json!({
            "success": true,
            "remote": remote_name,
            "output": push_output.trim()
        });

        Ok(CallToolResult::success(vec![Content::json(result)?]))
    }

    /// Show changes between commits, commit and working tree, etc
    pub async fn git_diff(
        &self,
        repo_path: String,
        path: Option<String>,
        staged: Option<bool>,
        commit: Option<String>,
    ) -> Result<CallToolResult> {
        let mut cmd_args: Vec<String> = vec!["diff".to_string()];

        if staged.unwrap_or(false) {
            cmd_args.push("--staged".to_string());
        }

        if let Some(ref commit_ref) = commit {
            cmd_args.push(commit_ref.to_string());
        }

        if let Some(ref file_path) = path {
            cmd_args.push("--".to_string());
            cmd_args.push(file_path.to_string());
        }

        let cmd_args_refs: Vec<&str> = cmd_args.iter().map(|s| s.as_str()).collect();
        let diff_output = run_git_command(&repo_path, &cmd_args_refs)?;

        let result = json!({ "changes": diff_output });

        Ok(CallToolResult::success(vec![Content::json(result)?]))
    }

    /// Add file contents to the git staging area
    pub async fn git_add(
        &self,
        repo_path: String,
        path: Option<String>,
        update: Option<bool>,
        all: Option<bool>,
    ) -> Result<CallToolResult> {
        let path_value = path.unwrap_or_else(|| ".".to_string());
        let mut args: Vec<String> = vec!["add".to_string()];

        if update.unwrap_or(false) {
            args.push("--update".to_string());
        } else if all.unwrap_or(false) {
            args.push("--all".to_string());
        }

        args.push(path_value);

        let cmd_args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        let add_output = run_git_command(&repo_path, &cmd_args_refs)?;

        let status_output = run_git_command(&repo_path, &["status", "--porcelain"])?;
        let status_lines: Vec<&str> = status_output.lines().collect();

        let result = json!({
            "success": true,
            "message": if add_output.is_empty() { "Files staged successfully".to_string() } else { add_output },
            "status": status_lines
        });

        Ok(CallToolResult::success(vec![Content::json(result)?]))
    }

    /// Reset the staging area or working tree to a specified state
    pub async fn git_reset(
        &self,
        repo_path: String,
        path: Option<String>,
        hard: Option<bool>,
        target: Option<String>,
    ) -> Result<CallToolResult> {
        let mut args: Vec<String> = vec!["reset".to_string()];

        if hard.unwrap_or(false) {
            args.push("--hard".to_string());
        }

        if let Some(ref target_ref) = target {
            args.push(target_ref.to_string());
        }

        if let Some(ref p) = path {
            args.push("--".to_string());
            args.push(p.to_string());
        }

        let cmd_args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        let reset_output = run_git_command(&repo_path, &cmd_args_refs)?;

        let status_output = run_git_command(&repo_path, &["status", "--porcelain"])?;
        let status_lines: Vec<&str> = status_output.lines().collect();

        let result = json!({
            "success": true,
            "message": if reset_output.is_empty() {
                if hard.unwrap_or(false) {
                    "Hard reset performed successfully".to_string()
                } else {
                    "Files unstaged successfully".to_string()
                }
            } else {
                reset_output
            },
            "status": status_lines
        });

        Ok(CallToolResult::success(vec![Content::json(result)?]))
    }

    /// Get commits within a specified time range, optionally filtered by author and branch
    pub async fn git_time_filtered_log(
        &self,
        repo_path: String,
        since: String,
        until: Option<String>,
        author: Option<String>,
        branch: Option<String>,
    ) -> Result<CallToolResult> {
        let mut args: Vec<String> = vec![
            "log".to_string(),
            "--pretty=format:%H|%an|%ad|%s".to_string(),
        ];

        args.push(format!("--since={}", since));

        if let Some(ref until_date) = until {
            args.push(format!("--until={}", until_date));
        }

        if let Some(ref author_name) = author {
            args.push(format!("--author={}", author_name));
        }

        if let Some(ref branch_name) = branch {
            args.push(branch_name.to_string());
        }

        let cmd_args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        let log_output = run_git_command(&repo_path, &cmd_args_refs)?;

        let commits: Vec<serde_json::Value> = log_output
            .lines()
            .filter(|line: &&str| !line.is_empty())
            .map(|line: &str| {
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

        let result = json!({
            "commits": commits,
            "filters": {
                "since": since,
                "until": until,
                "author": author,
                "branch": branch
            }
        });

        Ok(CallToolResult::success(vec![Content::json(result)?]))
    }
}

impl ServerHandler for GitToolsHandler {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation::from_build_env(),
            instructions: Some("Git tools for managing git repositories".to_string()),
        }
    }

    fn list_tools(
        &self,
        _params: Option<PaginatedRequestParams>,
        _context: rmcp::service::RequestContext<rmcp::service::RoleServer>,
    ) -> impl std::future::Future<Output = Result<ListToolsResult, ErrorData>> + Send + '_ {
        Box::pin(async move {
            let tools = vec![
                Tool {
                    name: "git_status".into(),
                    title: None,
                    description: Some("Get the status of a git repository".into()),
                    input_schema: Arc::new(
                        serde_json::to_value(schemars::schema_for!(GitStatusParams))
                            .map_err(|e| ErrorData::internal_error(format!("Failed to generate schema: {}", e), None))?
                            .as_object()
                            .cloned()
                            .unwrap_or(serde_json::Map::new()),
                    ),
                    annotations: None,
                    icons: None,
                    meta: None,
                    output_schema: None,
                },
                Tool {
                    name: "git_log".into(),
                    title: None,
                    description: Some("Get the commit history of a git repository".into()),
                    input_schema: Arc::new(
                        serde_json::to_value(schemars::schema_for!(GitLogParams))
                            .map_err(|e| ErrorData::internal_error(format!("Failed to generate schema: {}", e), None))?
                            .as_object()
                            .cloned()
                            .unwrap_or(serde_json::Map::new()),
                    ),
                    annotations: None,
                    icons: None,
                    meta: None,
                    output_schema: None,
                },
                Tool {
                    name: "git_branches".into(),
                    title: None,
                    description: Some("List all branches in a git repository".into()),
                    input_schema: Arc::new(
                        serde_json::to_value(schemars::schema_for!(GitBranchesParams))
                            .map_err(|e| ErrorData::internal_error(format!("Failed to generate schema: {}", e), None))?
                            .as_object()
                            .cloned()
                            .unwrap_or(serde_json::Map::new()),
                    ),
                    annotations: None,
                    icons: None,
                    meta: None,
                    output_schema: None,
                },
                Tool {
                    name: "git_commit".into(),
                    title: None,
                    description: Some("Create a commit with the staged changes".into()),
                    input_schema: Arc::new(
                        serde_json::to_value(schemars::schema_for!(GitCommitParams))
                            .map_err(|e| ErrorData::internal_error(format!("Failed to generate schema: {}", e), None))?
                            .as_object()
                            .cloned()
                            .unwrap_or(serde_json::Map::new()),
                    ),
                    annotations: None,
                    icons: None,
                    meta: None,
                    output_schema: None,
                },
                Tool {
                    name: "git_pull".into(),
                    title: None,
                    description: Some("Pull changes from a remote repository".into()),
                    input_schema: Arc::new(
                        serde_json::to_value(schemars::schema_for!(GitPullParams))
                            .map_err(|e| ErrorData::internal_error(format!("Failed to generate schema: {}", e), None))?
                            .as_object()
                            .cloned()
                            .unwrap_or(serde_json::Map::new()),
                    ),
                    annotations: None,
                    icons: None,
                    meta: None,
                    output_schema: None,
                },
                Tool {
                    name: "git_push".into(),
                    title: None,
                    description: Some("Push local commits to a remote repository".into()),
                    input_schema: Arc::new(
                        serde_json::to_value(schemars::schema_for!(GitPushParams))
                            .map_err(|e| ErrorData::internal_error(format!("Failed to generate schema: {}", e), None))?
                            .as_object()
                            .cloned()
                            .unwrap_or(serde_json::Map::new()),
                    ),
                    annotations: None,
                    icons: None,
                    meta: None,
                    output_schema: None,
                },
                Tool {
                    name: "git_diff".into(),
                    title: None,
                    description: Some("Show changes between commits, commit and working tree, etc".into()),
                    input_schema: Arc::new(
                        serde_json::to_value(schemars::schema_for!(GitDiffParams))
                            .map_err(|e| ErrorData::internal_error(format!("Failed to generate schema: {}", e), None))?
                            .as_object()
                            .cloned()
                            .unwrap_or(serde_json::Map::new()),
                    ),
                    annotations: None,
                    icons: None,
                    meta: None,
                    output_schema: None,
                },
                Tool {
                    name: "git_add".into(),
                    title: None,
                    description: Some("Add file contents to the git staging area".into()),
                    input_schema: Arc::new(
                        serde_json::to_value(schemars::schema_for!(GitAddParams))
                            .map_err(|e| ErrorData::internal_error(format!("Failed to generate schema: {}", e), None))?
                            .as_object()
                            .cloned()
                            .unwrap_or(serde_json::Map::new()),
                    ),
                    annotations: None,
                    icons: None,
                    meta: None,
                    output_schema: None,
                },
                Tool {
                    name: "git_reset".into(),
                    title: None,
                    description: Some("Reset the staging area or working tree to a specified state".into()),
                    input_schema: Arc::new(
                        serde_json::to_value(schemars::schema_for!(GitResetParams))
                            .map_err(|e| ErrorData::internal_error(format!("Failed to generate schema: {}", e), None))?
                            .as_object()
                            .cloned()
                            .unwrap_or(serde_json::Map::new()),
                    ),
                    annotations: None,
                    icons: None,
                    meta: None,
                    output_schema: None,
                },
                Tool {
                    name: "git_time_filtered_log".into(),
                    title: None,
                    description: Some("Get commits within a specified time range, optionally filtered by author and branch".into()),
                    input_schema: Arc::new(
                        serde_json::to_value(schemars::schema_for!(GitTimeFilteredLogParams))
                            .map_err(|e| ErrorData::internal_error(format!("Failed to generate schema: {}", e), None))?
                            .as_object()
                            .cloned()
                            .unwrap_or(serde_json::Map::new()),
                    ),
                    annotations: None,
                    icons: None,
                    meta: None,
                    output_schema: None,
                },
            ];

            Ok(ListToolsResult {
                tools,
                next_cursor: None,
                meta: None,
            })
        })
    }

    fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: rmcp::service::RequestContext<rmcp::service::RoleServer>,
    ) -> impl std::future::Future<Output = Result<CallToolResult, ErrorData>> + Send + '_ {
        let name = request.name.clone();
        let handler = self.clone();

        Box::pin(async move {
            let args = request
                .arguments
                .ok_or_else(|| ErrorData::invalid_params("Missing arguments".to_string(), None))?;

            let result = match name.as_ref() {
                "git_status" => {
                    let params: GitStatusParams =
                        serde_json::from_value(serde_json::Value::Object(args)).map_err(|e| {
                            ErrorData::invalid_params(format!("Invalid parameters: {}", e), None)
                        })?;
                    handler.git_status(params.repo_path).await
                }
                "git_log" => {
                    let params: GitLogParams =
                        serde_json::from_value(serde_json::Value::Object(args)).map_err(|e| {
                            ErrorData::invalid_params(format!("Invalid parameters: {}", e), None)
                        })?;
                    handler
                        .git_log(params.repo_path, params.max_count, params.branch)
                        .await
                }
                "git_branches" => {
                    let params: GitBranchesParams =
                        serde_json::from_value(serde_json::Value::Object(args)).map_err(|e| {
                            ErrorData::invalid_params(format!("Invalid parameters: {}", e), None)
                        })?;
                    handler.git_branches(params.repo_path).await
                }
                "git_commit" => {
                    let params: GitCommitParams =
                        serde_json::from_value(serde_json::Value::Object(args)).map_err(|e| {
                            ErrorData::invalid_params(format!("Invalid parameters: {}", e), None)
                        })?;
                    handler
                        .git_commit(params.repo_path, params.message, params.all)
                        .await
                }
                "git_pull" => {
                    let params: GitPullParams =
                        serde_json::from_value(serde_json::Value::Object(args)).map_err(|e| {
                            ErrorData::invalid_params(format!("Invalid parameters: {}", e), None)
                        })?;
                    handler
                        .git_pull(params.repo_path, params.remote, params.branch)
                        .await
                }
                "git_push" => {
                    let params: GitPushParams =
                        serde_json::from_value(serde_json::Value::Object(args)).map_err(|e| {
                            ErrorData::invalid_params(format!("Invalid parameters: {}", e), None)
                        })?;
                    handler
                        .git_push(params.repo_path, params.remote, params.branch, params.force)
                        .await
                }
                "git_diff" => {
                    let params: GitDiffParams =
                        serde_json::from_value(serde_json::Value::Object(args)).map_err(|e| {
                            ErrorData::invalid_params(format!("Invalid parameters: {}", e), None)
                        })?;
                    handler
                        .git_diff(params.repo_path, params.path, params.staged, params.commit)
                        .await
                }
                "git_add" => {
                    let params: GitAddParams =
                        serde_json::from_value(serde_json::Value::Object(args)).map_err(|e| {
                            ErrorData::invalid_params(format!("Invalid parameters: {}", e), None)
                        })?;
                    handler
                        .git_add(params.repo_path, params.path, params.update, params.all)
                        .await
                }
                "git_reset" => {
                    let params: GitResetParams =
                        serde_json::from_value(serde_json::Value::Object(args)).map_err(|e| {
                            ErrorData::invalid_params(format!("Invalid parameters: {}", e), None)
                        })?;
                    handler
                        .git_reset(params.repo_path, params.path, params.hard, params.target)
                        .await
                }
                "git_time_filtered_log" => {
                    let params: GitTimeFilteredLogParams =
                        serde_json::from_value(serde_json::Value::Object(args)).map_err(|e| {
                            ErrorData::invalid_params(format!("Invalid parameters: {}", e), None)
                        })?;
                    handler
                        .git_time_filtered_log(
                            params.repo_path,
                            params.since,
                            params.until,
                            params.author,
                            params.branch,
                        )
                        .await
                }
                _ => return Err(ErrorData::method_not_found::<CallToolRequestMethod>()),
            };

            result.map_err(|e| {
                ErrorData::internal_error(format!("Tool execution failed: {}", e), None)
            })
        })
    }
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct GitStatusParams {
    repo_path: String,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct GitLogParams {
    repo_path: String,
    max_count: Option<u32>,
    branch: Option<String>,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct GitBranchesParams {
    repo_path: String,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct GitCommitParams {
    repo_path: String,
    message: String,
    all: Option<bool>,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct GitPullParams {
    repo_path: String,
    remote: Option<String>,
    branch: Option<String>,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct GitPushParams {
    repo_path: String,
    remote: Option<String>,
    branch: Option<String>,
    force: Option<bool>,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct GitDiffParams {
    repo_path: String,
    path: Option<String>,
    staged: Option<bool>,
    commit: Option<String>,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct GitAddParams {
    repo_path: String,
    path: Option<String>,
    update: Option<bool>,
    all: Option<bool>,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct GitResetParams {
    repo_path: String,
    path: Option<String>,
    hard: Option<bool>,
    target: Option<String>,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct GitTimeFilteredLogParams {
    repo_path: String,
    since: String,
    until: Option<String>,
    author: Option<String>,
    branch: Option<String>,
}
