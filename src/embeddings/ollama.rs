use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct OllamaEmbeddingClient {
    client: Client,
    model: String,
    base_url: String,
}

#[derive(Serialize)]
struct OllamaEmbedRequest<'a> {
    model: &'a str,
    prompt: &'a str,
}

#[derive(Deserialize)]
struct OllamaEmbedResponse {
    embedding: Vec<f32>,
}

impl OllamaEmbeddingClient {
    pub fn new(model: Option<String>, base_url: Option<String>) -> Self {
        Self {
            client: Client::new(),
            model: model.unwrap_or_else(|| "nomic-embed-text".to_string()),
            base_url: base_url.unwrap_or_else(|| "http://localhost:11434".to_string()),
        }
    }

    pub async fn embed_query(&self, text: &str) -> Result<Vec<f32>> {
        let url = format!("{}/api/embeddings", self.base_url.trim_end_matches('/'));
        let req = OllamaEmbedRequest {
            model: &self.model,
            prompt: text,
        };

        let res = self.client.post(&url).json(&req).send().await?;
        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            return Err(anyhow!("Ollama embedding failed [{}]: {}", status, body));
        }

        let resp: OllamaEmbedResponse = res.json().await?;
        Ok(resp.embedding)
    }

    pub async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let mut results = Vec::with_capacity(texts.len());
        for text in texts {
            results.push(self.embed_query(text).await?);
        }
        Ok(results)
    }
}
