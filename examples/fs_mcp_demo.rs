use std::path::Path;

use serde_json::json;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;
use alagent::tools::mcp::client::McpClient;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::WARN)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    let client = McpClient::connect().await?;

    let tools = client.list_tools().await?;
    println!("===== fs_mcp 提供 {} 个工具 =====", tools.len());
    for tool in &tools {
        println!(
            "- {}: {}",
            tool.name,
            tool.description.clone().unwrap_or_default()
        );
    }

    let root = std::env::temp_dir().join("fs_mcp_demo");
    let root_str = root.to_string_lossy().to_string();

    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root)?;

    println!("\n===== 演示开始（工作目录 {}）=====", root_str);

    let r = client
        .call_tool("create_dir", json!({"path": format!("{root_str}/notes")}))
        .await?;
    println!("\n[create_dir] {r}");

    let r = client
        .call_tool(
            "write_file",
            json!({
                "path": format!("{root_str}/notes/hello.txt"),
                "content": "你好，fs_mcp！\n这是第一行。\n这是第二行。"
            }),
        )
        .await?;
    println!("[write_file] {r}");

    let config = serde_json::to_string(&json!({"name": "demo", "version": 1}))?;
    let r = client
        .call_tool(
            "write_file",
            json!({
                "path": format!("{root_str}/config.json"),
                "content": config
            }),
        )
        .await?;
    println!("[write_file] {r}");

    let r = client
        .call_tool(
            "read_file",
            json!({"path": format!("{root_str}/notes/hello.txt")}),
        )
        .await?;
    println!("[read_file]\n{r}");

    let r = client
        .call_tool(
            "list_files",
            json!({"path": root_str.clone(), "recursive": true}),
        )
        .await?;
    println!("[list_files]\n{r}");

    let r = client
        .call_tool(
            "file_info",
            json!({"path": format!("{root_str}/notes/hello.txt")}),
        )
        .await?;
    println!("[file_info]\n{r}");

    let r = client
        .call_tool(
            "copy_file",
            json!({
                "from": format!("{root_str}/notes/hello.txt"),
                "to": format!("{root_str}/notes/hello_copy.txt")
            }),
        )
        .await?;
    println!("[copy_file] {r}");

    let r = client
        .call_tool(
            "move_file",
            json!({
                "from": format!("{root_str}/notes/hello_copy.txt"),
                "to": format!("{root_str}/notes/renamed.txt")
            }),
        )
        .await?;
    println!("[move_file] {r}");

    let r = client
        .call_tool(
            "search_files",
            json!({"root": root_str.clone(), "pattern": "**/*.txt"}),
        )
        .await?;
    println!("[search_files]\n{r}");

    let r = client
        .call_tool(
            "delete_file",
            json!({"path": root_str.clone(), "recursive": true}),
        )
        .await?;
    println!("[delete_file] {r}");

    let cleaned = !Path::new(&root_str).exists();
    println!("\n演示结束，临时目录已清理: {cleaned}");
    Ok(())
}
