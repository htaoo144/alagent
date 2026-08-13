use async_openai::types::embeddings::{CreateEmbeddingRequestArgs, EmbeddingInput};

const EMBED_BATCH_SIZE: usize = 128;

pub async fn embed_texts(text:&[String], model:&str) -> anyhow::Result<Vec<Vec<f32>>> {
    if text.is_empty() {
        return Ok(vec![]);
    }
    let client= async_openai::Client::new();

    let mut all_embeddings = Vec::with_capacity(text.len());
    for batch in text.chunks(EMBED_BATCH_SIZE) {
        let request = CreateEmbeddingRequestArgs::default()
            .model(model)
            .input(EmbeddingInput::StringArray(batch.to_vec()))
            .build()?;

        let response = client.embeddings().create(request).await?;

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