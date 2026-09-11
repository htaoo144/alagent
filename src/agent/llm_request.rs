use crate::agent::context::ExecutionContext;
use crate::agent::event::ContentItem;
use async_trait::async_trait;

#[derive(Debug, Clone, Default)]
pub struct LlmRequest {
    pub instructions: Vec<String>,
    pub content: Vec<ContentItem>,
}

impl LlmRequest {
    pub fn append_instructions(&mut self, instructions: impl Into<String>) {
        self.instructions.push(instructions.into());
    }
}

#[async_trait::async_trait]
pub trait BeforeLlmCallback: Send + Sync {
    async fn call(&self, context: &mut ExecutionContext, request: &mut LlmRequest);
}
