pub const AGENT_MODEL:&str = "deepseek-v4-pro";
pub const EMBEDDING_MODEL:&str = "deepseek-v4-pro";

/// 允许通过环境变量 EMBEDDING_MODEL 覆盖嵌入模型（默认使用 EMBEDDING_MODEL 常量）
pub fn embedding_model() -> String {
    std::env::var("EMBEDDING_MODEL").unwrap_or_else(|_| EMBEDDING_MODEL.to_string())
}