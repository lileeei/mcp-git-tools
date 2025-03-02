//! Git tools implementations

pub mod branch;
pub mod commit;
pub mod diff;
pub mod log;
pub mod pull;
pub mod push;
pub mod status;
pub mod time_filtered_log;

// Re-export all tools to make them publicly accessible
pub use status::GitStatusTool;
pub use branch::GitBranchesTool;
pub use log::GitLogTool;
pub use commit::GitCommitTool;
pub use pull::GitPullTool;
pub use push::GitPushTool;
pub use diff::GitDiffTool;
pub use time_filtered_log::GitTimeFilteredLogTool;

use mcp_core::handler::ToolError;
use std::process::Command;

/// Helper function to run Git commands and handle errors
pub(crate) fn run_git_command(repo_path: &str, args: &[&str]) -> Result<String, ToolError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .args(args)
        .output()
        .map_err(|e| ToolError::ExecutionError(format!("Failed to execute git: {}", e)))?;

    if !output.status.success() {
        return Err(ToolError::ExecutionError(format!(
            "Git command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
