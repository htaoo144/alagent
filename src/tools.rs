use async_openai::types::chat::{ChatCompletionTool, ChatCompletionTools, FunctionObjectArgs};
use schemars::schema_for;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use async_openai::types::audio::AudioResponseFormat::Json;
use crate::agent::context::ExecutionContext;
use crate::tools::calculator::{calculation, CalculatorArgs};
use crate::tools::mcp::client::McpClient;
use crate::tools::mcp::tool::McpTool;
use crate::tools::web_search::{web_search, WebSearchArgs};


pub mod web_search;
pub mod calculator;
pub mod mcp;


pub type ToolBox=HashMap<String,Box<dyn Tools>>;

pub async fn build_toolbox() -> anyhow::Result<ToolBox> {
    let mut tools:Vec<Box<dyn Tools>> = vec![Box::new(CalculatorTool), Box::new(WebSearchTool)];

    // 优先通过 MCP_HTTP_URL 连接远程 MCP 服务（可选 MCP_API_KEY），否则启动本地 fs_mcp 子进程
    let mcp_client = match std::env::var("MCP_HTTP_URL") {
        Ok(url) if !url.is_empty() => {
            let api_key = std::env::var("MCP_API_KEY").ok();
            tracing::info!("connecting to remote MCP server: {url}");
            Arc::new(McpClient::connect_http(&url, api_key.as_deref()).await?)
        }
        _ => Arc::new(McpClient::connect().await?),
    };
    for tool in mcp_client.list_tools().await? {
        tools.push(Box::new(McpTool::new(mcp_client.clone(), tool)));
    }

    let mut toolbox:ToolBox = HashMap::new();
    for tool in tools {
        let name = tool.name().to_string();
        if toolbox.insert(name.clone(), tool).is_some() {
            tracing::warn!("duplicate tool name `{name}`, the later one overrides the former");
        }
    }
    Ok(toolbox)
}

#[async_trait::async_trait]
// trait 实现
pub trait Tools:Send+Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters(&self)->Value;

    async fn execute(&self,args_json:&str,context:&ExecutionContext)->anyhow::Result<String>;

    fn definition(&self)->anyhow::Result<ChatCompletionTools>{
        let function = FunctionObjectArgs::default()
            .name(self.name())
            .description(self.description())
            .parameters(self.parameters())
            .build()
            .map_err(|_|anyhow::anyhow!("no function definition"))?;

        Ok(ChatCompletionTools::Function(ChatCompletionTool{
            function,
        }))
    }
}

// trait 实现案例
pub struct CalculatorTool;
pub struct WebSearchTool;
#[async_trait::async_trait]
impl Tools for CalculatorTool {
    fn name(&self) -> &str {
        "Calculator"
    }
    fn description(&self) -> &str {
        "Calculator"
    }
    fn parameters(&self)->Value {
        serde_json::to_value(schema_for!(CalculatorArgs))
            .expect("Can't convert CalculatorArgs to JSON")
    }
    async fn execute(&self,args_json:&str,_context:&ExecutionContext)->anyhow::Result<String>{
        let args:CalculatorArgs =serde_json::from_str(args_json)?;
        let result= calculation(&args.operator,args.first_number,args.second_number);
        match result {
            Ok(result) => Ok(result.to_string()),
            Err(error) => Ok(format!("{:#?}", error)),
        }

    }

}


// pub fn calculator_tool_description()->async_openai::types::chat::ChatCompletionTools{
//     ChatCompletionTools::Function(ChatCompletionTool{
//         function:FunctionObjectArgs::default()
//             .name("calculator")
//             .description("Calculator")
//             .parameters(json!({
//                 "type": "object",
//                 "properties":{
//                     "operator":{
//                     "type":"string",
//                     "description":" ",
//                     "enum":["add","subtract","multiply"],
//                 },
//                 "first-number":{
//                     "type":"number",
//                     "description":" ",
//                 },
//                 "seconde_number":{
//                     "type":"number",
//                     "description":" ",
//                 }
//             },
//                 "request":["operator","first-number","second-number"],
//             }))
//             .build()
//             .unwrap()
//     })
// }


#[async_trait::async_trait]
impl Tools for WebSearchTool {
    fn name(&self) -> &str {
        "WebSearch"
    }
    fn description(&self) -> &str {
        "Web search"
    }
    fn parameters(&self)->Value {
        serde_json::to_value(schema_for!(WebSearchArgs))
            .expect("Can't convert WebSearchArgs to JSON")
    }
    async fn execute(&self,args_json:&str,_context:&ExecutionContext)->anyhow::Result<String>{
        let args:WebSearchArgs =serde_json::from_str(args_json)?;
        let result = web_search(args).await;
        match result {
            Ok(result) => Ok(serde_json::to_string(&result)?),
            Err(error) => Ok(format!("{:#?}", error)),
        }
    }
}