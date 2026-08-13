use std::collections::HashMap;

use async_openai::types::chat::{ChatCompletionTool, ChatCompletionTools, FunctionObjectArgs};
use serde_json::Value;

#[async_trait::async_trait]
pub trait Tools : Sync + Send {
    fn name(&self) -> &str;

    fn description(&self) -> &str;

    // fn version(&self) -> &str;

    fn parameters(&self)->Value;

    async fn execute(&self,args_json:&str)->anyhow::Result<String>;

    fn definition(&self)->anyhow::Result<ChatCompletionTools>{
        let function = FunctionObjectArgs::default()
            .name(self.name())
            .description(self.description())
            .parameters(self.parameters())
            .build()
            .map_err(|_| anyhow::anyhow!("Could not compile function {}", self.name()))?;

        Ok(ChatCompletionTools::Function(ChatCompletionTool{
            function,
        }))
    }
}