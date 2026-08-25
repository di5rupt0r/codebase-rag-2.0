use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct VoyageEmbeddingClient {
    client: Client,
    api_key: String,
    model: String,
    base_url: String,
}

#[derive(Serialize)]
struct VoyageEmbedRequest<'a> {
    model: &'a str,
    input: Vec<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    input_type: Option<&'a str>,
}

#[derive(Deserialize)]
struct VoyageEmbeddingData {
    embedding: Vec<f32>,
    index: usize,
}

#[derive(Deserialize)]
struct VoyageEmbedResponse {
    data: Vec<VoyageEmbeddingData>,
}

impl VoyageEmbeddingClient {
    pub fn new(api_key: String, model: Option<String>, base_url: Option<String>) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model: model.unwrap_or_else(|| "voyage-code-3".to_string()),
            base_url: base_url.unwrap_or_else(|| "https://api.voyageai.com/v1".to_string()),
        }
    }

    pub async fn embed_query(&self, text: &str) -> Result<Vec<f32>> {
        let url = format!("{}/embeddings", self.base_url.trim_end_matches('/'));
        let req = VoyageEmbedRequest {
            model: &self.model,
            input: vec![text],
            input_type: Some("query"),
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
            return Err(anyhow!("Voyage embedding failed [{}]: {}", status, body));
        }

        let mut resp: VoyageEmbedResponse = res.json().await?;
        resp.data.sort_by_key(|d| d.index);
        resp.data
            .into_iter()
            .next()
            .map(|d| d.embedding)
            .ok_or_else(|| anyhow!("Empty embedding returned from Voyage"))
    }

    pub async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        let url = format!("{}/embeddings", self.base_url.trim_end_matches('/'));
        let input_refs: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();

        let req = VoyageEmbedRequest {
            model: &self.model,
            input: input_refs,
            input_type: Some("document"),
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
            return Err(anyhow!("Voyage embedding failed [{}]: {}", status, body));
        }

        let mut resp: VoyageEmbedResponse = res.json().await?;
        resp.data.sort_by_key(|d| d.index);
        Ok(resp.data.into_iter().map(|d| d.embedding).collect())
    }
}
