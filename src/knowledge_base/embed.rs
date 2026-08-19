use anyhow::Context;
use async_openai::config::OpenAIConfig;
use async_openai::types::embeddings::{CreateEmbeddingRequestArgs, EmbeddingInput};

const EMBED_BATCH_SIZE: usize = 128;

/// embedding 端点可单独配置：
/// - 设置了 EMBEDDING_BASE_URL 时，embedding 请求走该端点（可配 EMBEDDING_API_KEY）
/// - 未设置时回退到默认客户端（与聊天共用 OPENAI_BASE_URL / OPENAI_API_KEY）
/// 例如 DeepSeek 不支持 embeddings，可指向硅基流动：EMBEDDING_BASE_URL=https://api.siliconflow.cn/v1
fn embedding_client() -> async_openai::Client<OpenAIConfig> {
    match std::env::var("EMBEDDING_BASE_URL") {
        Ok(base_url) if !base_url.trim().is_empty() => {
            let mut config = OpenAIConfig::new().with_api_base(base_url);
            if let Ok(api_key) = std::env::var("EMBEDDING_API_KEY") {
                if !api_key.trim().is_empty() {
                    config = config.with_api_key(api_key);
                }
            }
            async_openai::Client::with_config(config)
        }
        _ => async_openai::Client::new(),
    }
}

pub async fn embed_texts(text:&[String], model:&str) -> anyhow::Result<Vec<Vec<f32>>> {
    if text.is_empty() {
        return Ok(vec![]);
    }
    let client= embedding_client();

    let mut all_embeddings = Vec::with_capacity(text.len());
    for batch in text.chunks(EMBED_BATCH_SIZE) {
        let request = CreateEmbeddingRequestArgs::default()
            .model(model)
            .input(EmbeddingInput::StringArray(batch.to_vec()))
            .build()?;

        let response = client.embeddings().create(request).await.context(
            format!("embedding 请求失败（model={model}）。请确认 EMBEDDING_BASE_URL 指向支持 embeddings 的服务，且 EMBEDDING_MODEL 有效"),
        )?;

        let mut data = response.data;
        data.sort_by_key(|embedding| embedding.index);

        all_embeddings.extend(
            data.into_iter()
                .map(|embedding| embedding.embedding)
        );
    }

    Ok(all_embeddings)
}

pub async fn embed_text(text:&str,model:&str) -> anyhow::Result<Vec<f32>> {
    let owned = [text.to_string()];
    let mut vectors = embed_texts(&owned, model).await?;
    vectors.pop()
        .ok_or_else(|| anyhow::anyhow!("No embedding found"))
}