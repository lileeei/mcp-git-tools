//! Unit tests for mcp-git-tools library

use mcp_git_tools::GitToolsHandler;
use std::process::Command;
use tempfile::tempdir;

fn create_test_repo(temp_dir: &tempfile::TempDir) -> String {
    let repo_path = temp_dir.path().join("test_repo");
    std::fs::create_dir(&repo_path).unwrap();

    Command::new("git")
        .args(["init"])
        .current_dir(&repo_path)
        .output()
        .unwrap();

    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(&repo_path)
        .output()
        .unwrap();

    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(&repo_path)
        .output()
        .unwrap();

    repo_path.to_str().unwrap().to_string()
}

#[tokio::test]
async fn test_git_tools_handler_creation() {
    let handler = GitToolsHandler::new();
    let handler2 = GitToolsHandler::default();

    assert_eq!(format!("{:?}", handler), format!("{:?}", handler2));
}

#[tokio::test]
async fn test_run_git_command_with_invalid_path() {
    let temp_dir = tempdir().unwrap();
    let non_existent_path = temp_dir.path().join("non_existent_repo");

    let result = mcp_git_tools::run_git_command(
        non_existent_path.to_str().unwrap(),
        &["status", "--porcelain"],
    );

    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("does not exist") || error_msg.contains("not a git repository"));
}

#[tokio::test]
async fn test_git_status_clean() {
    let temp_dir = tempdir().unwrap();
    let repo_path = create_test_repo(&temp_dir);
    let handler = GitToolsHandler::new();

    let result = handler.git_status(repo_path).await;
    assert!(result.is_ok());

    let call_result = result.unwrap();
    assert_eq!(call_result.content.len(), 1);
}

#[tokio::test]
async fn test_git_status_with_changes() {
    let temp_dir = tempdir().unwrap();
    let repo_path = create_test_repo(&temp_dir);
    let handler = GitToolsHandler::new();

    let test_file = temp_dir.path().join("test_repo/test.txt");
    std::fs::write(&test_file, "test content").unwrap();

    let result = handler.git_status(repo_path).await;
    assert!(result.is_ok());

    let call_result = result.unwrap();
    assert_eq!(call_result.content.len(), 1);
}

#[tokio::test]
async fn test_git_log() {
    let temp_dir = tempdir().unwrap();
    let repo_path = create_test_repo(&temp_dir);
    let handler = GitToolsHandler::new();

    let test_file = temp_dir.path().join("test_repo/test.txt");
    std::fs::write(&test_file, "initial content").unwrap();

    Command::new("git")
        .args(["add", "test.txt"])
        .current_dir(temp_dir.path().join("test_repo"))
        .output()
        .unwrap();

    Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(temp_dir.path().join("test_repo"))
        .output()
        .unwrap();

    let result = handler
        .git_log(repo_path, Some(5), None, None, None, None)
        .await;
    assert!(result.is_ok());

    let call_result = result.unwrap();
    assert_eq!(call_result.content.len(), 1);
}

#[tokio::test]
async fn test_git_branches() {
    let temp_dir = tempdir().unwrap();
    let repo_path = create_test_repo(&temp_dir);
    let handler = GitToolsHandler::new();

    let test_file = temp_dir.path().join("test_repo/test.txt");
    std::fs::write(&test_file, "initial content").unwrap();

    Command::new("git")
        .args(["add", "test.txt"])
        .current_dir(temp_dir.path().join("test_repo"))
        .output()
        .unwrap();

    Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(temp_dir.path().join("test_repo"))
        .output()
        .unwrap();

    let result = handler.git_branches(repo_path).await;

    if !result.is_ok() {
        let error = result.as_ref().unwrap_err();
        eprintln!("git_branches failed: {}", error);
    }

    assert!(result.is_ok());

    let call_result = result.unwrap();
    assert_eq!(call_result.content.len(), 1);
}

#[tokio::test]
async fn test_git_reset_parameter_parsing() {
    let temp_dir = tempdir().unwrap();
    let repo_path = create_test_repo(&temp_dir);
    let handler = GitToolsHandler::new();

    let test_file = temp_dir.path().join("test_repo/test.txt");
    std::fs::write(&test_file, "test content").unwrap();

    Command::new("git")
        .args(["add", "test.txt"])
        .current_dir(temp_dir.path().join("test_repo"))
        .output()
        .unwrap();

    let result = handler
        .git_reset(
            repo_path.clone(),
            Some("test.txt".to_string()),
            Some(false),
            None,
        )
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_git_reset_hard() {
    let temp_dir = tempdir().unwrap();
    let repo_path = create_test_repo(&temp_dir);
    let handler = GitToolsHandler::new();

    let test_file = temp_dir.path().join("test_repo/test.txt");
    std::fs::write(&test_file, "test content").unwrap();

    Command::new("git")
        .args(["add", "test.txt"])
        .current_dir(temp_dir.path().join("test_repo"))
        .output()
        .unwrap();

    Command::new("git")
        .args(["commit", "-m", "Add test file"])
        .current_dir(temp_dir.path().join("test_repo"))
        .output()
        .unwrap();

    std::fs::write(&test_file, "modified content").unwrap();

    let result = handler.git_reset(repo_path, None, Some(true), None).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_git_status_detailed_untracked() {
    let temp_dir = tempdir().unwrap();
    let repo_path = create_test_repo(&temp_dir);
    let handler = GitToolsHandler::new();

    let test_file = temp_dir.path().join("test_repo/test.txt");
    std::fs::write(&test_file, "test content").unwrap();

    let result = handler.git_status(repo_path).await;
    assert!(result.is_ok());

    let call_result = result.unwrap();
    assert_eq!(call_result.content.len(), 1);
}

#[tokio::test]
async fn test_git_status_detailed_staged() {
    let temp_dir = tempdir().unwrap();
    let repo_path = create_test_repo(&temp_dir);
    let handler = GitToolsHandler::new();

    let test_file = temp_dir.path().join("test_repo/test.txt");
    std::fs::write(&test_file, "test content").unwrap();

    Command::new("git")
        .args(["add", "test.txt"])
        .current_dir(temp_dir.path().join("test_repo"))
        .output()
        .unwrap();

    let result = handler.git_status(repo_path).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().content.len(), 1);
}

#[tokio::test]
async fn test_git_status_detailed_modified() {
    let temp_dir = tempdir().unwrap();
    let repo_path = create_test_repo(&temp_dir);
    let handler = GitToolsHandler::new();

    let test_file = temp_dir.path().join("test_repo/test.txt");
    std::fs::write(&test_file, "initial content").unwrap();

    Command::new("git")
        .args(["add", "test.txt"])
        .current_dir(temp_dir.path().join("test_repo"))
        .output()
        .unwrap();

    Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(temp_dir.path().join("test_repo"))
        .output()
        .unwrap();

    std::fs::write(&test_file, "modified content").unwrap();

    let result = handler.git_status(repo_path).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().content.len(), 1);
}

#[tokio::test]
async fn test_git_status_detailed_invalid_repo() {
    let temp_dir = tempdir().unwrap();
    let non_git_dir = temp_dir.path().join("not_a_repo");
    std::fs::create_dir(&non_git_dir).unwrap();
    let handler = GitToolsHandler::new();

    let result = handler
        .git_status(non_git_dir.to_str().unwrap().to_string())
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_git_log_with_author_filter() {
    let temp_dir = tempdir().unwrap();
    let repo_path = create_test_repo(&temp_dir);
    let handler = GitToolsHandler::new();

    let test_file = temp_dir.path().join("test_repo/test.txt");
    std::fs::write(&test_file, "initial content").unwrap();

    Command::new("git")
        .args(["add", "test.txt"])
        .current_dir(temp_dir.path().join("test_repo"))
        .output()
        .unwrap();

    Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(temp_dir.path().join("test_repo"))
        .output()
        .unwrap();

    let result = handler
        .git_log(repo_path, None, None, None, None, Some("Test".to_string()))
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_git_log_with_time_filter() {
    let temp_dir = tempdir().unwrap();
    let repo_path = create_test_repo(&temp_dir);
    let handler = GitToolsHandler::new();

    let test_file = temp_dir.path().join("test_repo/test.txt");
    std::fs::write(&test_file, "initial content").unwrap();

    Command::new("git")
        .args(["add", "test.txt"])
        .current_dir(temp_dir.path().join("test_repo"))
        .output()
        .unwrap();

    Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(temp_dir.path().join("test_repo"))
        .output()
        .unwrap();

    let result = handler
        .git_log(
            repo_path,
            None,
            None,
            Some("1 week ago".to_string()),
            None,
            None,
        )
        .await;
    if !result.is_ok() {
        eprintln!("Error: {:?}", result.as_ref().unwrap_err());
    }
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_git_log_with_branch() {
    let temp_dir = tempdir().unwrap();
    let repo_path = create_test_repo(&temp_dir);
    let handler = GitToolsHandler::new();

    let test_file = temp_dir.path().join("test_repo/test.txt");
    std::fs::write(&test_file, "initial content").unwrap();

    Command::new("git")
        .args(["add", "test.txt"])
        .current_dir(temp_dir.path().join("test_repo"))
        .output()
        .unwrap();

    Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(temp_dir.path().join("test_repo"))
        .output()
        .unwrap();

    let result = handler
        .git_log(repo_path, None, Some("HEAD".to_string()), None, None, None)
        .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_git_log_with_time_range() {
    let temp_dir = tempdir().unwrap();
    let repo_path = create_test_repo(&temp_dir);
    let handler = GitToolsHandler::new();

    let test_file = temp_dir.path().join("test_repo/test.txt");
    std::fs::write(&test_file, "initial content").unwrap();

    Command::new("git")
        .args(["add", "test.txt"])
        .current_dir(temp_dir.path().join("test_repo"))
        .output()
        .unwrap();

    Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(temp_dir.path().join("test_repo"))
        .output()
        .unwrap();

    let result = handler
        .git_log(
            repo_path,
            None,
            None,
            Some("1 year ago".to_string()),
            None,
            None,
        )
        .await;

    assert!(result.is_ok());
}
