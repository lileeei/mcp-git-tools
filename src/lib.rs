//! MCP Git Tools library - Provides Git functionality through the Model Context Protocol

use anyhow::Result;
use chrono::{TimeZone, Utc};
use gix::bstr::ByteSlice;
use gix::date;
use gix::prelude::ObjectIdExt;
use gix::status;
use rmcp::{
    ServerHandler,
    model::*,
    schemars, serde,
    serde_json::{self, json},
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
        let repo = gix::open(&repo_path)
            .map_err(|e| anyhow::anyhow!("Failed to open repository: {}", e))?;

        let platform = repo
            .status(gix::progress::Discard)?
            .untracked_files(status::UntrackedFiles::Files);

        let mut iter = platform.into_iter(Vec::new())?;

        let mut status_lines = Vec::new();

        for item in &mut iter {
            let item = item?;
            match item {
                status::Item::TreeIndex(change) => {
                    let (location, _, _, _) = change.fields();
                    let status_char = match change {
                        gix::diff::index::Change::Addition { .. } => "A",
                        gix::diff::index::Change::Deletion { .. } => "D",
                        gix::diff::index::Change::Modification { .. } => "M",
                        gix::diff::index::Change::Rewrite { .. } => "R",
                    };
                    status_lines.push(format!("{}  {}", status_char, location));
                }
                status::Item::IndexWorktree(item) => match item {
                    status::index_worktree::Item::Modification {
                        rela_path, status, ..
                    } => {
                        let status_str = entry_status_to_porcelain(&status);
                        status_lines.push(format!("{:2}  {}", status_str, rela_path));
                    }
                    status::index_worktree::Item::DirectoryContents {
                        entry,
                        collapsed_directory_status,
                        ..
                    } => {
                        if collapsed_directory_status.is_none() {
                            let path = &entry.rela_path;
                            let slash = if entry
                                .disk_kind
                                .unwrap_or(gix::dir::entry::Kind::File)
                                .is_dir()
                            {
                                "/"
                            } else {
                                ""
                            };
                            status_lines.push(format!("?? {}{}", path, slash));
                        }
                    }
                    status::index_worktree::Item::Rewrite {
                        source,
                        dirwalk_entry,
                        ..
                    } => {
                        status_lines.push(format!(
                            "R  {} → {}",
                            source.rela_path(),
                            dirwalk_entry.rela_path
                        ));
                    }
                },
            }
        }

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
        since: Option<String>,
        until: Option<String>,
        author: Option<String>,
    ) -> Result<CallToolResult> {
        let repo = gix::open(&repo_path)
            .map_err(|e| anyhow::anyhow!("Failed to open repository: {}", e))?;

        let start_commit = if let Some(branch_name) = &branch {
            let spec = repo
                .rev_parse(branch_name.as_str())
                .map_err(|e| anyhow::anyhow!("Failed to parse branch '{}': {}", branch_name, e))?;
            spec.single()
                .ok_or_else(|| anyhow::anyhow!("Invalid branch reference: {}", branch_name))?
                .object()
                .map_err(|e| anyhow::anyhow!("Failed to find commit: {}", e))?
                .into_commit()
        } else {
            repo.head_commit()
                .map_err(|e| anyhow::anyhow!("Failed to get HEAD commit: {}", e))?
        };

        let commit = start_commit;

        let platform = commit.ancestors();

        let mut commits = Vec::new();

        let mut iter = platform
            .all()
            .map_err(|e| anyhow::anyhow!("Failed to create commit iterator: {}", e))?;

        while let Some(info) = iter.next() {
            let info = info.map_err(|e| anyhow::anyhow!("Failed to iterate commits: {}", e))?;

            if let Some(count) = max_count {
                if commits.len() >= count as usize {
                    break;
                }
            }

            let commit = repo
                .find_object(info.id.attach(&repo))
                .map_err(|e| anyhow::anyhow!("Failed to find commit: {}", e))?
                .into_commit();

            let decoded = commit
                .decode()
                .map_err(|e| anyhow::anyhow!("Failed to decode commit: {}", e))?;

            let author_info = decoded
                .author()
                .map_err(|e| anyhow::anyhow!("Failed to parse author: {}", e))?;

            let author_time = author_info
                .time()
                .map_err(|e| anyhow::anyhow!("Failed to get commit time: {}", e))?;

            if let Some(since_str) = &since {
                let now = std::time::SystemTime::now();
                let since_time = date::parse(since_str, Some(now)).map_err(|e| {
                    anyhow::anyhow!("Failed to parse since time '{}': {}", since_str, e)
                })?;
                if author_time.seconds < since_time.seconds {
                    continue;
                }
            }

            if let Some(until_str) = &until {
                let now = std::time::SystemTime::now();
                let until_time = date::parse(until_str, Some(now)).map_err(|e| {
                    anyhow::anyhow!("Failed to parse until time '{}': {}", until_str, e)
                })?;
                if author_time.seconds > until_time.seconds {
                    continue;
                }
            }

            if let Some(author_filter) = &author {
                let author_name = author_info.name.to_str().unwrap_or("");
                let author_email = author_info.email.to_str().unwrap_or("");
                if !author_name.contains(author_filter) && !author_email.contains(author_filter) {
                    continue;
                }
            }

            let message = commit
                .message()
                .map_err(|e| anyhow::anyhow!("Failed to parse commit message: {}", e))?;

            let date_str = format_git_date(&author_time);

            commits.push(json!({
                "hash": commit.id().to_string(),
                "author": author_info.name.to_str().unwrap_or("").to_string(),
                "author_date": date_str,
                "message_title": message.title.to_string()
            }));
        }

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

    /// Show detailed information about a specific commit
    pub async fn git_show(
        &self,
        repo_path: String,
        commit_id: String,
    ) -> Result<CallToolResult> {
        let repo = gix::open(&repo_path)
            .map_err(|e| anyhow::anyhow!("Failed to open repository: {}", e))?;

        let commit = repo
            .rev_parse(commit_id.as_str())
            .map_err(|e| anyhow::anyhow!("Failed to parse commit id '{}': {}", commit_id, e))?
            .single()
            .ok_or_else(|| anyhow::anyhow!("Invalid commit id: {}", commit_id))?
            .object()
            .map_err(|e| anyhow::anyhow!("Failed to find commit: {}", e))?
            .into_commit();

        let decoded = commit
            .decode()
            .map_err(|e| anyhow::anyhow!("Failed to decode commit: {}", e))?;

        let author_info = decoded
            .author()
            .map_err(|e| anyhow::anyhow!("Failed to parse author: {}", e))?;

        let author_time = author_info
            .time()
            .map_err(|e| anyhow::anyhow!("Failed to get author time: {}", e))?;

        let committer_info = decoded
            .committer()
            .map_err(|e| anyhow::anyhow!("Failed to parse committer: {}", e))?;

        let committer_time = committer_info
            .time()
            .map_err(|e| anyhow::anyhow!("Failed to get committer time: {}", e))?;

        let message = commit
            .message()
            .map_err(|e| anyhow::anyhow!("Failed to parse commit message: {}", e))?;

        let author_date_str = format_git_date(&author_time);
        let committer_date_str = format_git_date(&committer_time);

        let result = json!({
            "hash": commit.id().to_string(),
            "author": author_info.name.to_str().unwrap_or("").to_string(),
            "email": author_info.email.to_str().unwrap_or("").to_string(),
            "date": author_date_str,
            "committer": committer_info.name.to_str().unwrap_or("").to_string(),
            "committer_email": committer_info.email.to_str().unwrap_or("").to_string(),
            "committer_date": committer_date_str,
            "message_title": message.title.to_string(),
            "message_body": message.body.and_then(|b| b.to_str().ok()).unwrap_or("").to_string(),
            "tree": commit.tree().ok().map(|t| t.id().to_string()),
            "parents": commit.parent_ids().map(|id| id.to_string()).collect::<Vec<_>>()
        });

        Ok(CallToolResult::success(vec![Content::json(result)?]))
    }

    pub async fn git_worktree_list(&self, repo_path: String) -> Result<CallToolResult> {
        let output = run_git_command(&repo_path, &["worktree", "list", "--porcelain"])?;

        let worktrees: Vec<serde_json::Value> = output
            .split("\n\n")
            .filter(|s| !s.is_empty())
            .map(|block| {
                let mut worktree = serde_json::Map::new();
                for line in block.lines() {
                    if let Some((key, value)) = line.split_once(' ') {
                        worktree.insert(key.to_string(), json!(value));
                    }
                }
                json!(worktree)
            })
            .collect();

        let result = json!({
            "worktrees": worktrees
        });

        Ok(CallToolResult::success(vec![Content::json(result)?]))
    }

    pub async fn git_worktree_add(
        &self,
        repo_path: String,
        path: String,
        branch: Option<String>,
        commit_ref: Option<String>,
        checkout: Option<bool>,
    ) -> Result<CallToolResult> {
        let mut args: Vec<String> = vec!["worktree".to_string(), "add".to_string()];

        if !checkout.unwrap_or(true) {
            args.push("--no-checkout".to_string());
        }

        if let Some(branch_name) = &branch {
            args.push("-b".to_string());
            args.push(branch_name.clone());
        }

        args.push(path.clone());

        if let Some(ref_commit) = &commit_ref {
            args.push(ref_commit.clone());
        }

        let cmd_args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        let output = run_git_command(&repo_path, &cmd_args_refs)?;

        let result = json!({
            "success": true,
            "path": path,
            "branch": branch,
            "output": output.trim()
        });

        Ok(CallToolResult::success(vec![Content::json(result)?]))
    }

    pub async fn git_worktree_remove(
        &self,
        repo_path: String,
        path: String,
        force: Option<bool>,
    ) -> Result<CallToolResult> {
        let mut args: Vec<String> = vec!["worktree".to_string(), "remove".to_string()];

        if force.unwrap_or(false) {
            args.push("--force".to_string());
        }

        args.push(path.clone());

        let cmd_args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        let output = run_git_command(&repo_path, &cmd_args_refs)?;

        let result = json!({
            "success": true,
            "path": path,
            "force": force,
            "output": output.trim()
        });

        Ok(CallToolResult::success(vec![Content::json(result)?]))
    }
}

/// 格式化 git 日期为标准格式
fn format_git_date(time: &gix::date::Time) -> String {
    if let Some(dt) = Utc.timestamp_opt(time.seconds, 0).single() {
        let offset_hours = time.offset.abs() / 3600;
        let offset_minutes = (time.offset.abs() % 3600) / 60;
        let sign = if time.offset >= 0 { '+' } else { '-' };
        dt.format("%a %b %e %H:%M:%S %Y ").to_string()
            + &format!("{}{:02}{:02}", sign, offset_hours, offset_minutes)
    } else {
        format!("{}", time.seconds)
    }
}

/// 将EntryStatus转换为porcelain格式
fn entry_status_to_porcelain(
    status: &gix::status::plumbing::index_as_worktree::EntryStatus<(), gix::submodule::Status>,
) -> String {
    use gix::status::plumbing::index_as_worktree::EntryStatus::*;

    match status {
        Conflict { .. } => "UU".to_string(),
        Change(change) => match change {
            gix::status::plumbing::index_as_worktree::Change::Removed => " D".to_string(),
            gix::status::plumbing::index_as_worktree::Change::Type { .. } => " T".to_string(),
            gix::status::plumbing::index_as_worktree::Change::Modification {
                executable_bit_changed: true,
                content_change: _,
                ..
            } => " M".to_string(),
            gix::status::plumbing::index_as_worktree::Change::Modification {
                executable_bit_changed: false,
                content_change: None,
                ..
            } => "  ".to_string(),
            gix::status::plumbing::index_as_worktree::Change::Modification {
                executable_bit_changed: false,
                content_change: Some(_),
                ..
            } => " M".to_string(),
            _ => "??".to_string(),
        },
        NeedsUpdate(_) => "  ".to_string(),
        IntentToAdd => "A ".to_string(),
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
                    description: Some("Get commit history of a git repository with flexible filtering options (by time, author, branch, count)".into()),
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
                    name: "git_show".into(),
                    title: None,
                    description: Some("Show detailed information about a specific commit by commit id".into()),
                    input_schema: Arc::new(
                        serde_json::to_value(schemars::schema_for!(GitShowParams))
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
                    name: "git_worktree_list".into(),
                    title: None,
                    description: Some("List all worktrees in a git repository".into()),
                    input_schema: Arc::new(
                        serde_json::to_value(schemars::schema_for!(GitWorktreeListParams))
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
                    name: "git_worktree_add".into(),
                    title: None,
                    description: Some("Add a new worktree to a git repository".into()),
                    input_schema: Arc::new(
                        serde_json::to_value(schemars::schema_for!(GitWorktreeAddParams))
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
                        .git_log(
                            params.repo_path,
                            params.max_count,
                            params.branch,
                            params.since,
                            params.until,
                            params.author,
                        )
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
                "git_show" => {
                    let params: GitShowParams =
                        serde_json::from_value(serde_json::Value::Object(args)).map_err(|e| {
                            ErrorData::invalid_params(format!("Invalid parameters: {}", e), None)
                        })?;
                    handler.git_show(params.repo_path, params.commit_id).await
                }
                "git_worktree_list" => {
                    let params: GitWorktreeListParams =
                        serde_json::from_value(serde_json::Value::Object(args)).map_err(|e| {
                            ErrorData::invalid_params(format!("Invalid parameters: {}", e), None)
                        })?;
                    handler.git_worktree_list(params.repo_path).await
                }
                "git_worktree_add" => {
                    let params: GitWorktreeAddParams =
                        serde_json::from_value(serde_json::Value::Object(args)).map_err(|e| {
                            ErrorData::invalid_params(format!("Invalid parameters: {}", e), None)
                        })?;
                    handler
                        .git_worktree_add(
                            params.repo_path,
                            params.path,
                            params.branch,
                            params.commit_ref,
                            params.checkout,
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
    since: Option<String>,
    until: Option<String>,
    author: Option<String>,
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
struct GitShowParams {
    repo_path: String,
    commit_id: String,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct GitWorktreeListParams {
    repo_path: String,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct GitWorktreeAddParams {
    repo_path: String,
    path: String,
    branch: Option<String>,
    commit_ref: Option<String>,
    checkout: Option<bool>,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct GitWorktreeRemoveParams {
    repo_path: String,
    path: String,
    force: Option<bool>,
}
