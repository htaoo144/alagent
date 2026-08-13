use rmcp::model::{CallToolRequestParams, Tool};
use rmcp::service::RunningService;
use rmcp::service::RoleClient;
use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
use rmcp::transport::StreamableHttpClientTransport;
use rmcp::ServiceExt;
use rmcp::transport::TokioChildProcess;
use tokio::process::Command;

pub struct McpClient{
    service :RunningService<RoleClient,()>
}

impl McpClient {
    pub async fn connect() ->anyhow::Result<Self>{
        let command = match option_env!("CARGO_BIN_EXE_fs_mcp") {
            Some(bin) => Command::new(bin),
            None => {
                tracing::warn!("fs_mcp binary path unavailable at compile time, falling back to `cargo run`");
                let mut cmd = Command::new("cargo");
                cmd.args(["run", "--quiet", "--bin", "fs_mcp"]);
                cmd
            }
        };
        let service = ()
            .serve(TokioChildProcess::new(command)?)
            .await?;

        Ok(McpClient{service})
    }

    /// 通过 Streamable HTTP 连接远程 MCP 服务
    ///
    /// - `url`: 远程 MCP 端点，如 "https://example.com/mcp"
    /// - `api_key`: 可选，提供时以 Bearer Token 形式放入 Authorization 请求头
    pub async fn connect_http(url:&str, api_key:Option<&str>) -> anyhow::Result<Self> {
        let mut config = StreamableHttpClientTransportConfig::with_uri(url);
        if let Some(key) = api_key.filter(|k| !k.is_empty()) {
            config = config.auth_header(key);
        }
        let transport = StreamableHttpClientTransport::from_config(config);
        let service = ().serve(transport).await?;

        Ok(McpClient{service})
    }

    pub async fn list_tools(&self)-> anyhow::Result<Vec<Tool>> {
        let result = self.service.list_tools(Default::default()).await?;
        Ok(result.tools)
    }


    pub async fn call_tool(&self,name:&str,arguments:serde_json::Value)-> anyhow::Result<String> {
        let params = CallToolRequestParams::new(name.to_string())
            .with_arguments(arguments.as_object().cloned().unwrap_or_default());

        let result = self.service.call_tool(params).await?;

        let text = result
            .content
            .iter()
            .filter_map(|block| block.as_text().map(|text|text.text.clone()))
            .collect::<Vec<String>>()
            .join("\n\n");

        if text.is_empty() {
            Ok(format!(
                "[tool `{name}` returned {} non-text content block(s), which are not supported]",
                result.content.len()
            ))
        } else {
            Ok(text)
        }
    }
}
