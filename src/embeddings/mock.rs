use anyhow::Result;
use sha2::{Digest, Sha256};

#[derive(Clone)]
pub struct MockEmbeddingClient {
    dimension: usize,
}

impl MockEmbeddingClient {
    pub fn new(dimension: Option<usize>) -> Self {
        Self {
            dimension: dimension.unwrap_or(384),
        }
    }

    pub fn generate_vector(&self, text: &str) -> Vec<f32> {
        let mut vec = vec![0.0f32; self.dimension];
        let words: Vec<&str> = text.split_whitespace().collect();

        for (i, word) in words.iter().enumerate() {
            let mut hasher = Sha256::new();
            hasher.update(word.as_bytes());
            let result = hasher.finalize();
            for j in 0..self.dimension {
                let byte = result[j % 32];
                let weight = 1.0 / ((i + 1) as f32).sqrt();
                vec[j] += ((byte as f32 / 255.0) - 0.5) * weight;
            }
        }

        // L2 Normalize
        let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for v in vec.iter_mut() {
                *v /= norm;
            }
        }

        vec
    }

    pub async fn embed_query(&self, text: &str) -> Result<Vec<f32>> {
        Ok(self.generate_vector(text))
    }

    pub async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        Ok(texts.iter().map(|t| self.generate_vector(t)).collect())
    }
}
