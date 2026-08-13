# alagent

（具体可查看master分支）

基于 Rust 与 OpenAI 兼容 API 的 LLM Agent 框架，支持工具调用、MCP 协议集成、回调机制与向量检索。

## 功能特性

- **Agent 运行时**：多轮工具调用循环，内置最大步数限制与执行上下文追踪
- **记忆模块**：对话历史存储（轮数上限自动裁剪）、JSON 持久化，支持多轮对话
- **事件系统**：完整记录每次执行的 message / tool_call / tool_result 事件
- **工具生态**：
  - `Calculator`：四则运算
  - `WebSearch`：基于 [Tavily](https://tavily.com/) 的网页搜索
  - `MCP`：通过 [rmcp](https://crates.io/crates/rmcp) 接入任意 MCP 服务，支持两种连接方式：
    - 本地子进程（stdio）：内置 `fs_mcp` 文件服务（文件读写/增删、目录管理、图片读取、通配符搜索、zip 解压）
    - 远程 Streamable HTTP：通过 URL + API Key（Bearer Token）连接外部 MCP 服务
- **回调机制**：
  - `BeforeToolCallBack`：工具执行前拦截（如危险操作人工审批）
  - `AfterToolCallBack`：工具执行后处理（如搜索结果向量压缩）
- **知识库**：定长分块、Embedding 向量化、Top-K 余弦相似度检索
- **结构化输出**：`ActionPlan` JSON Schema 约束输出

## 项目结构

```
src/
├── agent/               # Agent 运行时、执行上下文、事件系统、回调 trait
├── tools/               # 工具抽象与实现
│   ├── calculator.rs    # 计算器工具
│   ├── web_search.rs    # Tavily 网页搜索
│   └── mcp/             # MCP 客户端与工具封装
├── knowledge_base/      # 分块、Embedding、向量检索
├── callback/            # 审批、搜索压缩回调
├── memory.rs            # 对话记忆（多轮对话）
├── bin/fs_mcp.rs        # 文件系统 MCP 服务（stdio，11 个工具）
├── action_plan.rs       # ActionPlan 结构化输出
├── complete.rs          # 简化版对话补全循环
└── constant.rs          # 模型常量配置
examples/
├── agent_run.rs         # Agent 完整运行示例
├── search_web.rs        # 网页搜索 + 向量压缩示例
├── file_explorer.rs     # 文件探索 + 审批回调示例
├── multi_turn_chat.rs   # 多轮对话 + 记忆持久化示例
├── fs_mcp_demo.rs       # fs_mcp 工具集完整演示（无需 API key）
└── mcp_http.rs          # 远程 HTTP MCP 客户端（URL + API Key）
tests/
└── http_mcp.rs          # HTTP MCP 端到端集成测试
```

## 快速开始

### 环境要求

- Rust 2024 edition（`cargo 1.85+`）

### 配置环境变量

在项目根目录创建 `.env` 文件：

```env
OPENAI_API_KEY=your_api_key
OPENAI_BASE_URL=https://api.openai.com/v1   # 可选，兼容任意 OpenAI 兼容服务
TAVILY_API_KEY=your_tavily_api_key          # 使用 WebSearch 时需要
```

模型配置在 `src/constant.rs`：

```rust
pub const AGENT_MODEL: &str = "deepseek-v4-pro";
pub const EMBEDDING_MODEL: &str = "deepseek-v4-pro";
```

### 运行示例

```bash
# Agent 完整运行（工具调用 + 行动计划输出）
cargo run --example agent_run

# 网页搜索与向量检索压缩演示
cargo run --example search_web

# 文件探索 Agent（解压 zip、审批回调）
cargo run --example file_explorer

# 多轮对话（记忆 + 持久化）
cargo run --example multi_turn_chat

# fs_mcp 文件工具集演示（无需 API key）
cargo run --example fs_mcp_demo

# 远程 HTTP MCP 客户端
cargo run --example mcp_http -- <mcp_url> [api_key] [tool_name] [tool_args_json]

# HTTP MCP 集成测试
cargo test --test http_mcp

# 独立启动文件系统 MCP 服务
cargo run --bin fs_mcp
```

## 使用方式

### 创建 Agent

```rust
use std::sync::Arc;
use alagent::agent::runtime::Agent;
use alagent::callback::approval::ApprovalCallback;
use alagent::constant::AGENT_MODEL;
use alagent::tools::build_toolbox;

let toolbox = build_toolbox().await?;   // 内置工具 + MCP 工具
let agent = Agent::new(AGENT_MODEL, Some("系统指令"), &toolbox)
    .with_max_steps(10)
    .with_before_tool_callback(Arc::new(ApprovalCallback::new(["delete_file"])));

let result = agent.run("你的任务描述").await?;
println!("{}", result.output);
println!("执行了 {} 步，记录了 {} 条事件", result.context.current_step, result.context.event.len());
```

### 多轮对话（记忆模块）

```rust
use alagent::agent::runtime::Agent;
use alagent::memory::Memory;

let memory = Memory::new().with_max_turns(5);   // 最多保留 5 轮对话
let agent = Agent::new(model, Some(instructions), &toolbox)
    .with_memory(memory);

let first = agent.run("我叫小明").await?;       // 第一轮
let second = agent.run("我叫什么名字？").await?; // 第二轮，自动携带历史

agent.save_memory("chat_memory.json")?;         // 持久化
let restored = Memory::load("chat_memory.json")?;
```

`Memory` 会在每轮 `run()` 成功后自动记录 user/assistant 消息，超出 `max_turns` 时按 FIFO 裁剪。

### 连接远程 MCP 服务（HTTP）

```rust
use alagent::tools::mcp::client::McpClient;

// 带 API Key（以 Bearer Token 放入 Authorization 头）
let client = McpClient::connect_http("https://example.com/mcp", Some("sk-xxx")).await?;

// 不带认证
let client = McpClient::connect_http("http://localhost:3000/mcp", None).await?;

let tools = client.list_tools().await?;
let result = client.call_tool("read_file", serde_json::json!({"path": "C:/a.txt"})).await?;
```

设置环境变量 `MCP_HTTP_URL`（可选 `MCP_API_KEY`）后，`build_toolbox()` 会优先连接远程 HTTP MCP 服务，否则回退到本地 `fs_mcp` 子进程。

### 自定义工具

实现 `Tools` trait 即可接入工具箱：

```rust
#[async_trait::async_trait]
impl Tools for MyTool {
    fn name(&self) -> &str { "my_tool" }
    fn description(&self) -> &str { "工具描述" }
    fn parameters(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(MyArgs)).unwrap()
    }
    async fn execute(&self, args_json: &str, context: &ExecutionContext) -> anyhow::Result<String> {
        // 实现逻辑
        Ok("result".to_string())
    }
}
```

### 自定义回调

实现 `BeforeToolCallBack` / `AfterToolCallBack` trait，通过 `with_before_tool_callback` / `with_after_tool_callback` 注册：

- 返回 `Some(String)` 时短路工具执行（before）或替换工具结果（after）
- 返回 `None` 时按原流程继续

### 向量检索

```rust
use alagent::knowledge_base::{chunk::fixed_length_chunking, search::vector_search};

let chunks = fixed_length_chunking(&long_text, 500, 50);  // 分块（500 字，重叠 50 字）
let hits = vector_search("查询语句", &chunks, 3).await?;  // Top-3 相似块
```

## 依赖

| 依赖 | 用途 |
|------|------|
| async-openai | OpenAI 兼容 API 调用 |
| tokio | 异步运行时 |
| rmcp | MCP 协议客户端与服务端 |
| schemars | JSON Schema 生成 |
| tavily (HTTP API) | 网页搜索 |
| tiktoken-rs | Token 估算 |
| zip | zip 解压（fs_mcp） |

## License

MIT
