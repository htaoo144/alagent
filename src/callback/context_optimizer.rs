pub mod sliding_windows;

use crate::agent::llm_request::LlmRequest;
use crate::agent::event::ContentItem;
pub fn count_tokens(_model: &str, request: &LlmRequest)->usize{
    let bpe = tiktoken_rs::cl100k_base()
        .expect("tiktoken_rs::cl100k_base failed");
    
    let mut total = 0usize;
    
    for instruction in &request.instructions {
        total += 4+ bpe.encode_ordinary(instruction).len();
    }
    
    for item in &request.content {
        total += 4;
        total = match item {
            ContentItem::Message {content,..} => bpe.encode_ordinary(content).len(),
            ContentItem::ToolCall {name,arguments,..} => {bpe.encode_ordinary(name).len() + bpe.encode_ordinary(&arguments.to_string()).len()},
            ContentItem::ToolResult {content,..} => {bpe.encode_ordinary(content).len()},
        }
    }
    total
}
