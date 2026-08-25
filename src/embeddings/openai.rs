use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct OpenAIEmbeddingClient {
    client: Client,
    api_key: String,
    model: String,
    base_url: String,
    dimensions: Option<usize>,
}

#[derive(Serialize)]
struct OpenAIEmbedRequest<'a> {
    model: &'a str,
    input: Vec<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dimensions: Option<usize>,
}

#[derive(Deserialize)]
struct OpenAIEmbeddingData {
    embedding: Vec<f32>,
    index: usize,
}

#[derive(Deserialize)]
struct OpenAIEmbedResponse {
    data: Vec<OpenAIEmbeddingData>,
}

impl OpenAIEmbeddingClient {
    pub fn new(
        api_key: String,
        model: Option<String>,
        base_url: Option<String>,
        dimensions: Option<usize>,
    ) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model: model.unwrap_or_else(|| "text-embedding-3-small".to_string()),
            base_url: base_url.unwrap_or_else(|| "https://api.openai.com/v1".to_string()),
            dimensions,
        }
    }

    pub async fn embed_query(&self, text: &str) -> Result<Vec<f32>> {
        let results = self.embed_batch(&[text.to_string()]).await?;
        results
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("Empty embedding returned"))
    }

    pub async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        let url = format!("{}/embeddings", self.base_url.trim_end_matches('/'));
        let input_refs: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();

        let req = OpenAIEmbedRequest {
            model: &self.model,
            input: input_refs,
            dimensions: self.dimensions,
        };

        let res = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&req)
            .send()
            .await?;

        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            return Err(anyhow!("OpenAI embedding failed [{}]: {}", status, body));
        }

        let mut resp: OpenAIEmbedResponse = res.json().await?;
        resp.data.sort_by_key(|d| d.index);
        Ok(resp.data.into_iter().map(|d| d.embedding).collect())
    }
}
