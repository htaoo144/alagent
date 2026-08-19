use serde_json::Value;
use crate::agent::context::ExecutionContext;
use crate::agent::event::{ContentItem, ToolResultStatus};
use crate::knowledge_base::chunk::fixed_length_chunking;
use crate::knowledge_base::search::vector_search;
use crate::agent::callback::AfterToolCallBack;

const COMPRESS_THRESHOLD: usize = 2000;
const CHUNK_SIZE: usize = 500;
const CHUNK_OVERLAP: usize = 50;
const TOP_K: usize = 3;

pub struct SearchCompressorCallback;

#[async_trait::async_trait]
impl AfterToolCallBack for SearchCompressorCallback {
    async fn call(
        &self,
        context: &ExecutionContext,
        tool_call_id: &str,
        tool_name: &str,
        status: ToolResultStatus,
        content: &str,
    ) -> Option<(ToolResultStatus, String)> {
        if tool_name != "WebSearch" || status != ToolResultStatus::Success {
            return None;
        }
        if content.len() < COMPRESS_THRESHOLD {
            return None;
        }

        let query = extract_query(context, tool_call_id)?;

        let chunks = fixed_length_chunking(content, CHUNK_SIZE, CHUNK_OVERLAP);

        if chunks.is_empty() {
            return None;
        }

        tracing::info!(
            "🔍 Compressing web_search result: {} chars → chunking into {} pieces...",
            content.len(),
            chunks.len(),
        );

        match vector_search(&query, &chunks, TOP_K).await {
            Ok(hits) => {
                let compressed = hits
                    .into_iter()
                    .map(|hit| hit.text)
                    .collect::<Vec<_>>()
                    .join("\n\n");
                tracing::info!(
                    "Compression complete: {} chars → {} chars (top {} of {} chunks)",
                    content.len(),
                    compressed.len(),
                    TOP_K,
                    chunks.len(),
                );
                Some((status, compressed))
            }
            Err(err) => {
                // embedding 失败时降级为朴素压缩：取前 TOP_K 个 chunk，保证压缩不整体跳过
                tracing::warn!("Vector compression failed ({err}), fallback to first {TOP_K} chunks");
                let fallback = chunks
                    .into_iter()
                    .take(TOP_K)
                    .collect::<Vec<_>>()
                    .join("\n\n");
                Some((status, fallback))
            }
        }
    }
}


fn extract_query(context: &ExecutionContext, tool_call_id: &str) -> Option<String> {
    context
        .event
        .iter()
        .flat_map(|event| &event.content)
        .find_map(|item| match item {
            ContentItem::ToolCall {
                tool_call_id: id,
                name,
                arguments,
            } if id == tool_call_id && name == "WebSearch" => arguments
                .get("query")
                .and_then(Value::as_str)
                .map(str::to_owned),
            _ => None,
        })
}