# MCP Git Tools

Git tools implementation for the Model Context Protocol (MCP).

[中文文档](README_CN.md)

## Features

This library provides a set of Git operations that can be called through the Model Context Protocol:

- `git_status` - Get the status of a repository
- `git_branches` - List branch information
- `git_log` - Get commit history with flexible filtering options (by time, author, branch, count)
- `git_commit` - Create a new commit
- `git_pull` - Pull changes from remote
- `git_push` - Push changes to remote
- `git_diff` - View file differences
- `git_add` - Add file contents to the staging area
- `git_reset` - Reset the staging area or working tree to a specified state

## Installation

```bash
# Clone the repository
git clone https://gitcode.com/lileeei/mcp-git-tools.git

# Navigate to the directory
cd mcp-git-tools

# Build
cargo build --release
```

## Usage

### Run as a standalone server

```bash
cargo run --release --bin mcp-git-server
```

This starts an MCP server that interacts with clients through standard input/output.

### Test with MCP Inspector

1. Build the server:
```bash
cargo build --release
```

2. Start the server:
```bash
./target/release/mcp-git-server
```

3. Use MCP Inspector to test the tools:
   - Download MCP Inspector: https://github.com/modelcontextprotocol/inspector
   - Click "Add a server"
   - Enter: `/path/to/mcp-git-server/target/release/mcp-git-server`
   - Click "Add"
   - Test each tool using the Inspector interface

## Architecture

This project uses the modern RMCP SDK (v0.14.0) with a clean architecture:

### Core Components

**GitToolsHandler**
- Central handler struct for all Git operations
- Implements `ServerHandler` trait from RMCP SDK
- Contains methods for each Git operation

**ServerHandler Implementation**
- `get_info()` - Provides server metadata and capabilities
- `list_tools()` - Lists all available tools with their schemas
- `call_tool()` - Routes tool calls to appropriate handler methods

### Tool Implementation Pattern

Each tool is implemented as a method on `GitToolsHandler`:

```rust
impl GitToolsHandler {
    #[tool(description = "Get the status of a git repository")]
    pub async fn git_status(
        &self,
        repo_path: String,
    ) -> Result<CallToolResult> {
        // Implementation
    }
}
```

### Parameter Handling

All tool parameters use derive macros for automatic schema generation:

```rust
#[derive(serde::Deserialize, schemars::JsonSchema)]
struct GitStatusParams {
    repo_path: String,
}
```

This provides:
- Automatic JSON schema generation
- Type-safe parameter parsing
- No manual serialization/deserialization code

### Transport

Uses RMCP's `stdio()` transport for standard input/output communication.

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

Get commit history with flexible filtering options.

**Parameters:**
- `repo_path` - Path to Git repository
- `max_count` - (optional) Maximum number of commits to return
- `branch` - (optional) Branch name
- `since` - (optional) Start date (e.g., "2023-01-01", "1 week ago", "yesterday")
- `until` - (optional) End date (e.g., "2023-01-31", "today")
- `author` - (optional) Filter by specific author (matches name or email)

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

**Examples:**
- Get recent 10 commits: `{"repo_path": ".", "max_count": 10}`
- Get commits from last week: `{"repo_path": ".", "since": "1 week ago"}`
- Get commits by author: `{"repo_path": ".", "author": "John"}`
- Get commits from a time range: `{"repo_path": ".", "since": "2023-01-01", "until": "2023-12-31"}`

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
  "changes": "diff --git a/file.txt b/file.txt\nindex 1234567..abcdefg 100644\n--- a/file.txt\n+++ b/file.txt\n@@ -1,3 +1,4 @@\n Line 1\nLine 2\nLine 3\n+New line\nLine 4"
}
```

### git_add

Add file contents to the staging area.

**Parameters:**
- `repo_path` - Path to the Git repository
- `path` - Path(s) to add, or patterns to match. Use '.' for all files.
- `update` - (optional) Whether to update, rather than add
- `all` - (optional) Whether to add all changes, including untracked files

**Returns:**
```json
{
  "success": true,
  "message": "Files staged successfully",
  "status": ["M file1.txt", "A file2.txt"]
}
```

### git_reset

Reset the staging area or working tree to a specified state.

**Parameters:**
- `repo_path` - Path to the Git repository
- `path` - Path(s) to reset, or patterns to match. Use '.' for all files.
- `hard` - (optional) Whether to perform a hard reset (WARNING: discards all local changes)
- `target` - (optional) The commit or branch to reset to (defaults to HEAD)

**Returns:**
```json
{
  "success": true,
  "message": "Files unstaged successfully",
  "status": ["?? file1.txt", "?? file2.txt"]
}
```

## Development

### Project Structure

```
mcp-git-tools/
├── Cargo.toml          # Dependencies and project configuration
├── src/
│   ├── lib.rs          # Core library with GitToolsHandler
│   └── bin/
│       └── server.rs   # Server entry point
├── tests/
│   └── lib_tests.rs   # Unit tests
```

### Building

```bash
# Debug build
cargo build

# Release build
cargo build --release

# Run tests
cargo test

# Run server
cargo run --release --bin mcp-git-server
```

### Adding New Tools

To add a new Git tool:

1. Add the parameter struct in `src/lib.rs`:
```rust
#[derive(serde::Deserialize, schemars::JsonSchema)]
struct GitMyToolParams {
    repo_path: String,
    // other parameters...
}
```

2. Add the method in the `GitToolsHandler` impl block:
```rust
#[tool(description = "Description of your tool")]
pub async fn git_my_tool(
    &self,
    repo_path: String,
    // other parameters...
) -> Result<CallToolResult> {
    // Implementation
}
```

3. Update the `list_tools()` method to include your new tool in the returned list

## Dependencies

- `rmcp` v0.14.0 - The official RMCP SDK
- `tokio` - Async runtime
- `serde` - Serialization framework
- `serde_json` - JSON implementation
- `anyhow` - Error handling
- `tracing` - Logging framework
- `tempfile` - Testing utilities

## License

MIT License
