pub mod gemini;
pub mod mock;
pub mod ollama;
pub mod openai;
pub mod voyage;

use anyhow::Result;

use crate::config::{AppConfig, EmbeddingProviderKind};
pub use gemini::GeminiEmbeddingClient;
pub use mock::MockEmbeddingClient;
pub use ollama::OllamaEmbeddingClient;
pub use openai::OpenAIEmbeddingClient;
pub use voyage::VoyageEmbeddingClient;

#[derive(Clone)]
pub enum EmbeddingEngine {
    Gemini(GeminiEmbeddingClient),
    OpenAI(OpenAIEmbeddingClient),
    Voyage(VoyageEmbeddingClient),
    Ollama(OllamaEmbeddingClient),
    Mock(MockEmbeddingClient),
}

impl EmbeddingEngine {
    pub fn from_config(config: &AppConfig) -> Self {
        let emb = &config.embedding;
        match emb.provider {
            EmbeddingProviderKind::Gemini => {
                if let Some(key) = &emb.api_key {
                    EmbeddingEngine::Gemini(GeminiEmbeddingClient::new(
                        key.clone(),
                        Some(emb.model.clone()),
                        emb.base_url.clone(),
                    ))
                } else {
                    EmbeddingEngine::Mock(MockEmbeddingClient::new(emb.dimensions))
                }
            }
            EmbeddingProviderKind::OpenAI => {
                if let Some(key) = &emb.api_key {
                    EmbeddingEngine::OpenAI(OpenAIEmbeddingClient::new(
                        key.clone(),
                        Some(emb.model.clone()),
                        emb.base_url.clone(),
                        emb.dimensions,
                    ))
                } else {
                    EmbeddingEngine::Mock(MockEmbeddingClient::new(emb.dimensions))
                }
            }
            EmbeddingProviderKind::Voyage => {
                if let Some(key) = &emb.api_key {
                    EmbeddingEngine::Voyage(VoyageEmbeddingClient::new(
                        key.clone(),
                        Some(emb.model.clone()),
                        emb.base_url.clone(),
                    ))
                } else {
                    EmbeddingEngine::Mock(MockEmbeddingClient::new(emb.dimensions))
                }
            }
            EmbeddingProviderKind::Ollama => {
                EmbeddingEngine::Ollama(OllamaEmbeddingClient::new(
                    Some(emb.model.clone()),
                    emb.base_url.clone(),
                ))
            }
            EmbeddingProviderKind::None => {
                EmbeddingEngine::Mock(MockEmbeddingClient::new(emb.dimensions))
            }
        }
    }

    pub async fn embed_query(&self, text: &str) -> Result<Vec<f32>> {
        match self {
            Self::Gemini(c) => c.embed_query(text).await,
            Self::OpenAI(c) => c.embed_query(text).await,
            Self::Voyage(c) => c.embed_query(text).await,
            Self::Ollama(c) => c.embed_query(text).await,
            Self::Mock(c) => c.embed_query(text).await,
        }
    }

    pub async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        match self {
            Self::Gemini(c) => c.embed_batch(texts).await,
            Self::OpenAI(c) => c.embed_batch(texts).await,
            Self::Voyage(c) => c.embed_batch(texts).await,
            Self::Ollama(c) => c.embed_batch(texts).await,
            Self::Mock(c) => c.embed_batch(texts).await,
        }
    }

    pub fn is_mock(&self) -> bool {
        matches!(self, Self::Mock(_))
    }
}
