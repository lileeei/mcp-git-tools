# MCP Git Tools (中文版)

Git 工具库，用于 Model Context Protocol (MCP)。

[English Documentation](README.md)

## 功能特性

此库提供了一系列 Git 操作工具，可以通过 MCP 协议进行调用：

- `git_status` - 获取仓库状态
- `git_branches` - 列出分支信息
- `git_log` - 获取提交历史，支持灵活的过滤选项（按时间、作者、分支、数量）
- `git_commit` - 创建新提交
- `git_pull` - 从远程拉取更改
- `git_push` - 推送更改到远程
- `git_diff` - 查看文件差异
- `git_add` - 将文件内容添加到暂存区
- `git_reset` - 重置暂存区或工作区到指定状态

## 安装

```bash
# 克隆仓库
git clone https://gitcode.com/lileeei/mcp-git-tools.git

# 进入目录
cd mcp-git-tools

# 构建
cargo build --release
```

## 使用方法

### 作为独立服务器运行

```bash
cargo run --release --bin mcp-git-server
```

这会启动一个 MCP 服务器，通过标准输入/输出与客户端交互。

### 使用 MCP Inspector 测试

1. 构建服务器：
```bash
cargo build --release
```

2. 启动服务器：
```bash
./target/release/mcp-git-server
```

3. 使用 MCP Inspector 测试工具：
   - 下载 MCP Inspector：https://github.com/modelcontextprotocol/inspector
   - 点击 "Add a server"
   - 输入：`/path/to/mcp-git-server/target/release/mcp-git-server`
   - 点击 "Add"
   - 使用 Inspector 界面测试每个工具

## 架构

此项目使用现代 RMCP SDK (v0.14.0)，采用清晰的架构设计：

### 核心组件

**GitToolsHandler**
- 所有 Git 操作的中央处理器
- 实现 RMCP SDK 的 `ServerHandler` trait
- 包含每个 Git 操作的方法

**ServerHandler 实现**
- `get_info()` - 提供服务器元数据和功能
- `list_tools()` - 列出所有可用工具及其 schema
- `call_tool()` - 将工具调用路由到相应的处理方法

### 工具实现模式

每个工具都作为 `GitToolsHandler` 的方法实现：

```rust
impl GitToolsHandler {
    #[tool(description = "获取 Git 仓库状态")]
    pub async fn git_status(
        &self,
        repo_path: String,
    ) -> Result<CallToolResult> {
        // 实现
    }
}
```

### 参数处理

所有工具参数使用 derive 宏进行自动 schema 生成：

```rust
#[derive(serde::Deserialize, schemars::JsonSchema)]
struct GitStatusParams {
    repo_path: String,
}
```

这提供了：
- 自动 JSON schema 生成
- 类型安全的参数解析
- 无需手动序列化/反序列化代码

### 传输层

使用 RMCP 的 `stdio()` 传输进行标准输入/输出通信。

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

获取提交历史，支持灵活的过滤选项。

**参数：**
- `repo_path` - Git 仓库路径
- `max_count` - (可选) 最大返回提交数量
- `branch` - (可选) 分支名称
- `since` - (可选) 开始日期（例如："2023-01-01"、"1 week ago"、"yesterday"）
- `until` - (可选) 结束日期（例如："2023-12-31"、"today"）
- `author` - (可选) 按特定作者过滤（匹配姓名或邮箱）

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

**使用示例：**
- 获取最近 10 条提交：`{"repo_path": ".", "max_count": 10}`
- 获取最近一周的提交：`{"repo_path": ".", "since": "1 week ago"}`
- 按作者过滤：`{"repo_path": ".", "author": "张三"}`
- 获取时间范围内的提交：`{"repo_path": ".", "since": "2023-01-01", "until": "2023-12-31"}`

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
  "changes": "diff --git a/file.txt b/file.txt\nindex 1234567..abcdefg 100644\n--- a/file.txt\n+++ b/file.txt\n@@ -1,3 +1,4 @@\n Line 1\nLine 2\nLine 3\n+New line\nLine 4"
}
```

### git_add

将文件内容添加到暂存区。

**参数：**
- `repo_path` - Git 仓库路径
- `path` - 要添加的文件路径或匹配模式。使用'.'表示所有文件
- `update` - (可选) 是否只更新已跟踪的文件
- `all` - (可选) 是否添加所有更改，包括未跟踪的文件

**返回：**
```json
{
  "success": true,
  "message": "Files staged successfully",
  "status": ["M file1.txt", "A file2.txt"]
}
```

### git_reset

重置暂存区或工作区到指定状态。

**参数：**
- `repo_path` - Git 仓库路径
- `path` - 要重置的文件路径或匹配模式。使用'.'表示所有文件
- `hard` - (可选) 是否执行硬重置（警告：会丢弃所有本地更改）
- `target` - (可选) 要重置到的提交或分支（默认为HEAD）

**返回：**
```json
{
  "success": true,
  "message": "Files unstaged successfully",
  "status": ["?? file1.txt", "?? file2.txt"]
}
```

## 开发

### 项目结构

```
mcp-git-tools/
├── Cargo.toml          # 依赖和项目配置
├── src/
│   ├── lib.rs          # 核心库，包含 GitToolsHandler
│   └── bin/
│       └── server.rs   # 服务器入口
├── tests/
│   └── lib_tests.rs   # 单元测试
```

### 构建

```bash
# Debug 构建
cargo build

# Release 构建
cargo build --release

# 运行测试
cargo test

# 运行服务器
cargo run --release --bin mcp-git-server
```

### 添加新工具

要添加新的 Git 工具：

1. 在 `src/lib.rs` 中添加参数结构体：
```rust
#[derive(serde::Deserialize, schemars::JsonSchema)]
struct GitMyToolParams {
    repo_path: String,
    // 其他参数...
}
```

2. 在 `GitToolsHandler` impl 块中添加方法：
```rust
#[tool(description = "工具描述")]
pub async fn git_my_tool(
    &self,
    repo_path: String,
    // 其他参数...
) -> Result<CallToolResult> {
    // 实现
}
```

3. 更新 `list_tools()` 方法，在返回的列表中包含新工具

## 依赖项

- `rmcp` v0.14.0 - 官方 RMCP SDK
- `tokio` - 异步运行时
- `serde` - 序列化框架
- `serde_json` - JSON 实现
- `anyhow` - 错误处理
- `tracing` - 日志框架
- `tempfile` - 测试工具

## 许可证

MIT 许可证
