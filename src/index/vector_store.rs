use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};

use crate::parser::CodeChunk;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorDocument {
    pub chunk_id: String,
    pub relative_path: String,
    pub language: String,
    pub symbol_name: Option<String>,
    pub symbol_kind: Option<String>,
    pub signature: Option<String>,
    pub content: String,
    pub line_start: usize,
    pub line_end: usize,
    pub embedding: Vec<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorSearchResult {
    pub chunk_id: String,
    pub relative_path: String,
    pub language: String,
    pub symbol_name: Option<String>,
    pub symbol_kind: Option<String>,
    pub signature: Option<String>,
    pub content: String,
    pub line_start: usize,
    pub line_end: usize,
    pub score: f32,
}

#[derive(Default, Serialize, Deserialize)]
pub struct VectorStore {
    documents: Vec<VectorDocument>,
    #[serde(skip)]
    file_path: Option<PathBuf>,
}

impl VectorStore {
    pub fn open_or_create<P: AsRef<Path>>(dir: P) -> Result<Self> {
        let dir = dir.as_ref();
        std::fs::create_dir_all(dir)?;
        let path = dir.join("vectors.json");

        if path.exists() {
            let file = File::open(&path)?;
            let reader = BufReader::new(file);
            let mut store: Self = serde_json::from_reader(reader)?;
            store.file_path = Some(path);
            Ok(store)
        } else {
            Ok(Self {
                documents: Vec::new(),
                file_path: Some(path),
            })
        }
    }

    pub fn create_in_ram() -> Self {
        Self {
            documents: Vec::new(),
            file_path: None,
        }
    }

    pub fn add_chunk_with_embedding(&mut self, chunk: &CodeChunk, embedding: Vec<f32>) {
        // Normalize embedding for fast dot product
        let normalized = normalize_vector(embedding);

        // Remove existing chunk if already present
        self.documents.retain(|d| d.chunk_id != chunk.id);

        self.documents.push(VectorDocument {
            chunk_id: chunk.id.clone(),
            relative_path: chunk.relative_path.clone(),
            language: chunk.language.name().to_string(),
            symbol_name: chunk.symbol_name.clone(),
            symbol_kind: chunk.symbol_kind.map(|k| k.as_str().to_string()),
            signature: chunk.signature.clone(),
            content: chunk.content.clone(),
            line_start: chunk.line_start,
            line_end: chunk.line_end,
            embedding: normalized,
        });
    }

    pub fn delete_file(&mut self, relative_path: &str) {
        self.documents.retain(|d| d.relative_path != relative_path);
    }

    pub fn save(&self) -> Result<()> {
        if let Some(path) = &self.file_path {
            let file = File::create(path)?;
            let writer = BufWriter::new(file);
            serde_json::to_writer(writer, self)?;
        }
        Ok(())
    }

    pub fn search(&self, query_vector: &[f32], limit: usize) -> Vec<VectorSearchResult> {
        if self.documents.is_empty() || query_vector.is_empty() {
            return Vec::new();
        }

        let norm_query = normalize_vector(query_vector.to_vec());

        let mut scored: Vec<(f32, &VectorDocument)> = self
            .documents
            .iter()
            .map(|doc| {
                let score = dot_product(&norm_query, &doc.embedding);
                (score, doc)
            })
            .collect();

        // Sort descending by cosine similarity score
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        scored
            .into_iter()
            .take(limit)
            .map(|(score, doc)| VectorSearchResult {
                chunk_id: doc.chunk_id.clone(),
                relative_path: doc.relative_path.clone(),
                language: doc.language.clone(),
                symbol_name: doc.symbol_name.clone(),
                symbol_kind: doc.symbol_kind.clone(),
                signature: doc.signature.clone(),
                content: doc.content.clone(),
                line_start: doc.line_start,
                line_end: doc.line_end,
                score,
            })
            .collect()
    }

    pub fn count(&self) -> usize {
        self.documents.len()
    }
}

fn normalize_vector(mut v: Vec<f32>) -> Vec<f32> {
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for val in v.iter_mut() {
            *val /= norm;
        }
    }
    v
}

#[inline]
fn dot_product(a: &[f32], b: &[f32]) -> f32 {
    let len = a.len().min(b.len());
    a[..len]
        .iter()
        .zip(&b[..len])
        .map(|(x, y)| x * y)
        .sum()
}
