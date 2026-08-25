use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct GeminiEmbeddingClient {
    client: Client,
    api_key: String,
    model: String,
    base_url: String,
}

#[derive(Serialize)]
struct ContentPart {
    text: String,
}

#[derive(Serialize)]
struct Content {
    parts: Vec<ContentPart>,
}

#[derive(Serialize)]
struct EmbedContentRequest {
    model: String,
    content: Content,
}

#[derive(Serialize)]
struct BatchEmbedContentsRequest {
    requests: Vec<EmbedContentRequest>,
}

#[derive(Deserialize)]
struct ValuesHolder {
    values: Vec<f32>,
}

#[derive(Deserialize)]
struct BatchEmbedContentsResponse {
    embeddings: Vec<ValuesHolder>,
}

#[derive(Deserialize)]
struct SingleEmbedContentResponse {
    embedding: ValuesHolder,
}

impl GeminiEmbeddingClient {
    pub fn new(api_key: String, model: Option<String>, base_url: Option<String>) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model: model.unwrap_or_else(|| "text-embedding-004".to_string()),
            base_url: base_url.unwrap_or_else(|| "https://generativelanguage.googleapis.com".to_string()),
        }
    }

    pub async fn embed_query(&self, text: &str) -> Result<Vec<f32>> {
        let url = format!(
            "{}/v1beta/models/{}:embedContent?key={}",
            self.base_url, self.model, self.api_key
        );

        let req = EmbedContentRequest {
            model: format!("models/{}", self.model),
            content: Content {
                parts: vec![ContentPart {
                    text: text.to_string(),
                }],
            },
        };

        let res = self.client.post(&url).json(&req).send().await?;
        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            return Err(anyhow!("Gemini embedding failed [{}]: {}", status, body));
        }

        let resp: SingleEmbedContentResponse = res.json().await?;
        Ok(resp.embedding.values)
    }

    pub async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        let url = format!(
            "{}/v1beta/models/{}:batchEmbedContents?key={}",
            self.base_url, self.model, self.api_key
        );

        let requests = texts
            .iter()
            .map(|t| EmbedContentRequest {
                model: format!("models/{}", self.model),
                content: Content {
                    parts: vec![ContentPart { text: t.clone() }],
                },
            })
            .collect();

        let req = BatchEmbedContentsRequest { requests };
        let res = self.client.post(&url).json(&req).send().await?;
        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            return Err(anyhow!("Gemini batch embedding failed [{}]: {}", status, body));
        }

        let resp: BatchEmbedContentsResponse = res.json().await?;
        Ok(resp.embeddings.into_iter().map(|e| e.values).collect())
    }
}
