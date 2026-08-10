use async_openai::types::chat::{ChatCompletionTool, ChatCompletionTools, FunctionObjectArgs};
use schemars::schema_for;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
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
    
    let mcp_client = Arc::new(McpClient::connect().await?);
    for tool in mcp_client.list_tools().await? {
        tools.push(Box::new(McpTool::new(mcp_client.clone(), tool)));
    }

    Ok(tools
        .into_iter()
        .map(|t|(t.name().to_string(),t))
        .collect())

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
        let args:CalculatorArgs =serde_json::from_str(&args_json)?;
        let result= calculation(&args.operator,args.first_number,args.second_number);
        match result {
            Ok(result) => Ok(result.to_string()),
            Err(error) => Ok(format!("{:#?}", error)),
        }

    }

}

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
        let args:WebSearchArgs =serde_json::from_str(&args_json)?;
        let result = web_search(args).await;
        match result {
            Ok(result) => Ok(serde_json::to_string(&result)?),
            Err(error) => Ok(format!("{:#?}", error)),
        }
    }
}