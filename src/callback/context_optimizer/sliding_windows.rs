
use crate::agent::context::ExecutionContext;
use crate::agent::event::ContentItem;
use crate::agent::llm_request::{BeforeLlmCallback, LlmRequest};
use crate::callback::context_optimizer::count_tokens;

pub struct SlidingWindow {
    pub model:String,
    pub tokens_threshold:usize,
    pub window_size:usize,
}


#[async_trait::async_trait]
impl BeforeLlmCallback for SlidingWindow {
    async fn call(&self, context: &mut ExecutionContext, request: &mut LlmRequest) {
        if count_tokens(&self.model,request)<self.tokens_threshold{
            return;
        }
        if request.content.len() <= self.window_size{
            return;
        }
        
        let user_idx = request
            .content
            .iter()
            .position(|item| matches!(item, ContentItem::Message { role,..} if role == "user"));
        let Some(user_idx)= user_idx else { return };
        
        let preserved = request.content[..=user_idx].to_vec();
        
        let mut remaining = request.content[user_idx+1..].to_vec();
        if remaining.len() > self.tokens_threshold {
            let cut = remaining.len() - self.window_size;
            remaining.split_off(cut);
        }
        
        request.content = preserved.into_iter().chain(remaining).collect();
    }
}