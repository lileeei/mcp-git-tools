# MCP Git Tools

Git tool integration library for the Model Context Protocol (MCP).

[中文文档](README_CN.md)

## Features

This library provides a set of Git operations that can be called through the Model Context Protocol:

- `git_status` - Get the status of a repository
- `git_branches` - List branch information
- `git_log` - Get commit history
- `git_time_filtered_log` - Get commits within a specific time range
- `git_commit` - Create a new commit
- `git_pull` - Pull changes from remote
- `git_push` - Push changes to remote
- `git_diff` - View file differences

## Installation

```bash
# Clone the repository
git clone https://your-repository-url/mcp-git-tools.git

# Navigate to the directory
cd mcp-git-tools

# Build
cargo build
```

## Usage

### Run as a standalone server

```bash
cargo run --bin mcp-git-server
```

This starts an MCP server that interacts with clients through standard input/output.

### Use in an MCP client

```rust
use mcp_client::{
    client::{ClientCapabilities, ClientInfo, McpClient},
    StdioTransport, Transport, McpService,
};
use std::collections::HashMap;
use std::time::Duration;

// Create a connection to the Git tools server
let transport = StdioTransport::new(
    "path/to/mcp-git-server", 
    vec![], 
    HashMap::new()
);

// Start the transport
let handle = transport.start().await?;
let service = McpService::with_timeout(handle, Duration::from_secs(10));
let mut client = McpClient::new(service);

// Initialize the client
client.initialize(
    ClientInfo {
        name: "my-client".into(),
        version: "1.0.0".into(),
    },
    ClientCapabilities::default(),
).await?;

// Call the git_status tool
let status = client
    .call_tool("git_status", serde_json::json!({ "repo_path": "/path/to/repo" }))
    .await?;

println!("Git status: {:?}", status);
```

### Integrate into your own MCP server

```rust
use mcp_git_tools::register_git_tools;
use mcp_server::McpServerBuilder;

// Create a server
let mut builder = McpServerBuilder::new("my-server", "1.0.0");

// Register Git tools
register_git_tools(&mut builder);

// Add other tools...

// Build the server
let server = builder.build();
```

## Tool Details

### git_status

Get the status of a repository.

**Parameters:**
- `repo_path` - Path to the Git repository

**Returns:**
```json
{
  "status": ["M file1.txt", "?? file2.txt"],
  "is_clean": false
}
```

### git_branches

List all branches.

**Parameters:**
- `repo_path` - Path to the Git repository

**Returns:**
```json
{
  "branches": ["* main", "develop", "feature/new-feature"],
  "current": "main"
}
```

### git_log

Get commit history.

**Parameters:**
- `repo_path` - Path to the Git repository
- `max_count` - (optional) Maximum number of commits to return
- `branch` - (optional) Branch name

**Returns:**
```json
{
  "commits": [
    {
      "hash": "abcd1234",
      "author": "User Name",
      "date": "Mon Aug 1 10:00:00 2023 +0800",
      "message": "Initial commit"
    }
  ]
}
```

### git_time_filtered_log

Get commits within a specified time range, optionally filtered by author and branch.

**Parameters:**
- `repo_path` - Path to the Git repository
- `since` - Start date (e.g., "2023-01-01", "1 week ago", "yesterday")
- `until` - (optional) End date (e.g., "2023-01-31", "today")
- `author` - (optional) Filter by specific author
- `branch` - (optional) Branch name

**Returns:**
```json
{
  "commits": [
    {
      "hash": "abcd1234",
      "author": "User Name",
      "date": "Mon Aug 1 10:00:00 2023 +0800",
      "message": "Initial commit"
    }
  ],
  "filters": {
    "since": "1 week ago",
    "until": "today",
    "author": "User Name",
    "branch": "main"
  }
}
```

### git_commit

Create a new commit.

**Parameters:**
- `repo_path` - Path to the Git repository
- `message` - Commit message
- `all` - (optional) Whether to automatically stage modified files

**Returns:**
```json
{
  "success": true,
  "hash": "abcd1234",
  "message": "feat: Add new feature",
  "output": "[main abcd1234] feat: Add new feature\n 1 file changed, 10 insertions(+), 2 deletions(-)"
}
```

### git_pull

Pull changes from remote.

**Parameters:**
- `repo_path` - Path to the Git repository
- `remote` - (optional) Remote name, defaults to "origin"
- `branch` - (optional) Branch name

**Returns:**
```json
{
  "success": true,
  "remote": "origin",
  "output": "Updating abcd1234..efgh5678\nFast-forward\n file1.txt | 2 +-\n 1 file changed, 1 insertion(+), 1 deletion(-)"
}
```

### git_push

Push changes to remote.

**Parameters:**
- `repo_path` - Path to the Git repository
- `remote` - (optional) Remote name, defaults to "origin"
- `branch` - (optional) Branch name
- `force` - (optional) Whether to force push

**Returns:**
```json
{
  "success": true,
  "remote": "origin",
  "output": "To github.com:user/repo.git\n   abcd1234..efgh5678  main -> main"
}
```

### git_diff

View file differences.

**Parameters:**
- `repo_path` - Path to the Git repository
- `path` - (optional) Path to file or directory
- `staged` - (optional) Whether to show staged changes
- `commit` - (optional) Commit to compare against

**Returns:**
```json
{
  "changes": "diff --git a/file1.txt b/file1.txt\nindex abcd1234..efgh5678 100644\n--- a/file1.txt\n+++ b/file1.txt\n@@ -1,5 +1,5 @@\n Line 1\n-Line 2\n+Modified Line 2\n Line 3"
}
```

## License

MIT License 