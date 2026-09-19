use std::collections::HashMap;
use serde_json::Value;
use crate::agent::event::ContentItem;
use crate::agent::llm_request::LlmRequest;

pub struct Compaction {
    keep_recent:usize,
}

impl Compaction {
    pub fn apply(&self, request: &mut LlmRequest) {
        let protection_from = request.content.len().saturating_sub(self.keep_recent);
        let mut call_args :HashMap<String,Value> = HashMap::new();

        for (idx, item) in request.content.iter_mut().enumerate() {
            match item {
                ContentItem::ToolCall { tool_call_id,name,arguments}=>{
                    call_args.insert(tool_call_id.clone(),arguments.clone());
                }
                ContentItem::ToolResult {tool_call_id,name,content,..}=>{
                    if idx >= protection_from {
                        continue;
                    }
                    let args = call_args.get(tool_call_id);
                    let replacement = match name.as_str() {
                        "read_file" => {
                            let path = args.and_then(|a|a.get("file_path"))
                                .and_then(Value::as_str)
                                .unwrap_or("unknow");
                            Some(format!("File '{path}' was already read,call read_file agent if you need it."))
                        }
                        // other like this
                        _=>None
                    };

                    if let Some(replacement) = replacement {
                        *content=replacement;
                    }
                }
                _ =>{}
            }
        }
    }
}