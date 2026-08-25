use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredChunk {
    pub chunk_id: String,
    pub relative_path: String,
    pub language: String,
    pub symbol_name: Option<String>,
    pub symbol_kind: Option<String>,
    pub signature: Option<String>,
    pub content: String,
    pub line_start: usize,
    pub line_end: usize,
    pub rrf_score: f32,
    pub bm25_score: Option<f32>,
    pub vector_score: Option<f32>,
    pub symbol_score: Option<f32>,
}

pub struct RankedListCandidate {
    pub chunk_id: String,
    pub relative_path: String,
    pub language: String,
    pub symbol_name: Option<String>,
    pub symbol_kind: Option<String>,
    pub signature: Option<String>,
    pub content: String,
    pub line_start: usize,
    pub line_end: usize,
    pub raw_score: f32,
}

pub fn reciprocal_rank_fusion(
    bm25_results: &[RankedListCandidate],
    bm25_weight: f32,
    vector_results: &[RankedListCandidate],
    vector_weight: f32,
    symbol_results: &[RankedListCandidate],
    symbol_weight: f32,
    k: usize,
    top_n: usize,
) -> Vec<ScoredChunk> {
    let k_f32 = k as f32;
    let mut chunk_map: HashMap<String, ScoredChunk> = HashMap::new();

    // 1. Process BM25
    for (rank, item) in bm25_results.iter().enumerate() {
        let rank_score = bm25_weight / (k_f32 + (rank + 1) as f32);
        let entry = chunk_map
            .entry(item.chunk_id.clone())
            .or_insert_with(|| ScoredChunk {
                chunk_id: item.chunk_id.clone(),
                relative_path: item.relative_path.clone(),
                language: item.language.clone(),
                symbol_name: item.symbol_name.clone(),
                symbol_kind: item.symbol_kind.clone(),
                signature: item.signature.clone(),
                content: item.content.clone(),
                line_start: item.line_start,
                line_end: item.line_end,
                rrf_score: 0.0,
                bm25_score: None,
                vector_score: None,
                symbol_score: None,
            });
        entry.rrf_score += rank_score;
        entry.bm25_score = Some(item.raw_score);
    }

    // 2. Process Vector
    for (rank, item) in vector_results.iter().enumerate() {
        let rank_score = vector_weight / (k_f32 + (rank + 1) as f32);
        let entry = chunk_map
            .entry(item.chunk_id.clone())
            .or_insert_with(|| ScoredChunk {
                chunk_id: item.chunk_id.clone(),
                relative_path: item.relative_path.clone(),
                language: item.language.clone(),
                symbol_name: item.symbol_name.clone(),
                symbol_kind: item.symbol_kind.clone(),
                signature: item.signature.clone(),
                content: item.content.clone(),
                line_start: item.line_start,
                line_end: item.line_end,
                rrf_score: 0.0,
                bm25_score: None,
                vector_score: None,
                symbol_score: None,
            });
        entry.rrf_score += rank_score;
        entry.vector_score = Some(item.raw_score);
    }

    // 3. Process Symbol
    for (rank, item) in symbol_results.iter().enumerate() {
        let rank_score = symbol_weight / (k_f32 + (rank + 1) as f32);
        let entry = chunk_map
            .entry(item.chunk_id.clone())
            .or_insert_with(|| ScoredChunk {
                chunk_id: item.chunk_id.clone(),
                relative_path: item.relative_path.clone(),
                language: item.language.clone(),
                symbol_name: item.symbol_name.clone(),
                symbol_kind: item.symbol_kind.clone(),
                signature: item.signature.clone(),
                content: item.content.clone(),
                line_start: item.line_start,
                line_end: item.line_end,
                rrf_score: 0.0,
                bm25_score: None,
                vector_score: None,
                symbol_score: None,
            });
        entry.rrf_score += rank_score;
        entry.symbol_score = Some(item.raw_score);
    }

    let mut fused: Vec<ScoredChunk> = chunk_map.into_values().collect();
    fused.sort_by(|a, b| {
        b.rrf_score
            .partial_cmp(&a.rrf_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    fused.truncate(top_n);
    fused
}
