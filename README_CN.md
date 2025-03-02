# MCP Git Tools (中文版)

Git 工具集成库，用于 Model Context Protocol (MCP)。

[English Documentation](README.md)

## 功能特性

此库提供了一系列 Git 操作工具，可以通过 MCP 协议进行调用：

- `git_status` - 获取仓库状态
- `git_branches` - 列出分支信息
- `git_log` - 获取提交历史
- `git_time_filtered_log` - 获取指定时间范围内的提交
- `git_commit` - 创建新提交
- `git_pull` - 从远程拉取更改
- `git_push` - 推送更改到远程
- `git_diff` - 查看文件差异

## 安装

```bash
# 克隆仓库
git clone https://your-repository-url/mcp-git-tools.git

# 进入目录
cd mcp-git-tools

# 构建
cargo build
```

## 使用方法

### 作为独立服务器运行

```bash
cargo run --bin mcp-git-server
```

这会启动一个 MCP 服务器，通过标准输入/输出与客户端交互。

### 在 MCP 客户端中使用

```rust
use mcp_client::{
    client::{ClientCapabilities, ClientInfo, McpClient},
    StdioTransport, Transport, McpService,
};
use std::collections::HashMap;
use std::time::Duration;

// 创建到 Git 工具服务器的连接
let transport = StdioTransport::new(
    "path/to/mcp-git-server", 
    vec![], 
    HashMap::new()
);

// 启动 transport
let handle = transport.start().await?;
let service = McpService::with_timeout(handle, Duration::from_secs(10));
let mut client = McpClient::new(service);

// 初始化客户端
client.initialize(
    ClientInfo {
        name: "my-client".into(),
        version: "1.0.0".into(),
    },
    ClientCapabilities::default(),
).await?;

// 调用 git_status 工具
let status = client
    .call_tool("git_status", serde_json::json!({ "repo_path": "/path/to/repo" }))
    .await?;

println!("Git status: {:?}", status);
```

### 集成到自己的 MCP 服务器

```rust
use mcp_git_tools::register_git_tools;
use mcp_server::McpServerBuilder;

// 创建服务器
let mut builder = McpServerBuilder::new("my-server", "1.0.0");

// 注册 Git 工具
register_git_tools(&mut builder);

// 添加其他工具...

// 构建服务器
let server = builder.build();
```

## 工具详情

### git_status

获取仓库状态。

**参数：**
- `repo_path` - Git 仓库路径

**返回：**
```json
{
  "status": ["M file1.txt", "?? file2.txt"],
  "is_clean": false
}
```

### git_branches

列出所有分支。

**参数：**
- `repo_path` - Git 仓库路径

**返回：**
```json
{
  "branches": ["* main", "develop", "feature/new-feature"],
  "current": "main"
}
```

### git_log

获取提交历史。

**参数：**
- `repo_path` - Git 仓库路径
- `max_count` - (可选) 最大提交数量
- `branch` - (可选) 分支名称

**返回：**
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

获取指定时间范围内的提交，可选择按作者和分支进行过滤。

**参数：**
- `repo_path` - Git 仓库路径
- `since` - 开始日期（例如："2023-01-01"、"1 week ago"、"yesterday"）
- `until` - (可选) 结束日期（例如："2023-01-31"、"today"）
- `author` - (可选) 按特定作者过滤
- `branch` - (可选) 分支名称

**返回：**
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

创建新提交。

**参数：**
- `repo_path` - Git 仓库路径
- `message` - 提交信息
- `all` - (可选) 是否自动暂存已修改文件

**返回：**
```json
{
  "success": true,
  "hash": "abcd1234",
  "message": "feat: Add new feature",
  "output": "[main abcd1234] feat: Add new feature\n 1 file changed, 10 insertions(+), 2 deletions(-)"
}
```

### git_pull

从远程拉取更改。

**参数：**
- `repo_path` - Git 仓库路径
- `remote` - (可选) 远程名称，默认为 "origin"
- `branch` - (可选) 分支名称

**返回：**
```json
{
  "success": true,
  "remote": "origin",
  "output": "Updating abcd1234..efgh5678\nFast-forward\n file1.txt | 2 +-\n 1 file changed, 1 insertion(+), 1 deletion(-)"
}
```

### git_push

推送更改到远程。

**参数：**
- `repo_path` - Git 仓库路径
- `remote` - (可选) 远程名称，默认为 "origin"
- `branch` - (可选) 分支名称
- `force` - (可选) 是否强制推送

**返回：**
```json
{
  "success": true,
  "remote": "origin",
  "output": "To github.com:user/repo.git\n   abcd1234..efgh5678  main -> main"
}
```

### git_diff

查看文件差异。

**参数：**
- `repo_path` - Git 仓库路径
- `path` - (可选) 文件或目录路径
- `staged` - (可选) 是否查看暂存区差异
- `commit` - (可选) 要比较的提交

**返回：**
```json
{
  "changes": "diff --git a/file1.txt b/file1.txt\nindex abcd1234..efgh5678 100644\n--- a/file1.txt\n+++ b/file1.txt\n@@ -1,5 +1,5 @@\n Line 1\n-Line 2\n+Modified Line 2\n Line 3"
}
```

## 许可证

MIT 许可证 