use tracing::Level;
use tracing_subscriber::FmtSubscriber;
use alagent::tools::mcp::client::McpClient;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::WARN)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    let args: Vec<String> = std::env::args().collect();
    let url = args
        .get(1)
        .cloned()
        .or_else(|| std::env::var("MCP_HTTP_URL").ok())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "用法: cargo run --example mcp_http -- <mcp_url> [api_key] [tool_name] [tool_args_json]\n\
                 例: cargo run --example mcp_http -- https://example.com/mcp sk-xxx read_file \"{{\\\"path\\\":\\\"C:/a.txt\\\"}}\""
            )
        })?;
    let api_key = args
        .get(2)
        .cloned()
        .or_else(|| std::env::var("MCP_API_KEY").ok());

    let client = McpClient::connect_http(&url, api_key.as_deref()).await?;

    let tools = client.list_tools().await?;
    println!("已连接远程 MCP 服务: {url}");
    println!("提供 {} 个工具:", tools.len());
    for tool in &tools {
        println!(
            "- {}: {}",
            tool.name,
            tool.description.clone().unwrap_or_default()
        );
    }

    if let Some(tool_name) = args.get(3) {
        let args_json = args.get(4).cloned().unwrap_or_else(|| "{}".to_string());
        let arguments: serde_json::Value = serde_json::from_str(&args_json)?;
        let result = client.call_tool(tool_name, arguments).await?;
        println!("\n[{tool_name}] 返回:\n{result}");
    }

    Ok(())
}
