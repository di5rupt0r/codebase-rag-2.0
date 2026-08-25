use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::fusion::{reciprocal_rank_fusion, RankedListCandidate, ScoredChunk};
use crate::config::RetrievalConfig;
use crate::embeddings::EmbeddingEngine;
use crate::index::{SymbolStore, TantivyStore, VectorStore};

pub struct HybridRetriever {
    tantivy_store: Arc<RwLock<TantivyStore>>,
    vector_store: Arc<RwLock<VectorStore>>,
    symbol_store: Arc<RwLock<SymbolStore>>,
    embedding_engine: Arc<EmbeddingEngine>,
    config: RetrievalConfig,
}

impl HybridRetriever {
    pub fn new(
        tantivy_store: Arc<RwLock<TantivyStore>>,
        vector_store: Arc<RwLock<VectorStore>>,
        symbol_store: Arc<RwLock<SymbolStore>>,
        embedding_engine: Arc<EmbeddingEngine>,
        config: RetrievalConfig,
    ) -> Self {
        Self {
            tantivy_store,
            vector_store,
            symbol_store,
            embedding_engine,
            config,
        }
    }

    pub async fn retrieve(&self, query: &str, top_k: Option<usize>) -> Result<Vec<ScoredChunk>> {
        let k = top_k.unwrap_or(self.config.top_k);
        let candidate_limit = k * 3;

        // 1. BM25 search
        let bm25_candidates: Vec<RankedListCandidate> = {
            let store = self.tantivy_store.read().await;
            match store.search(query, candidate_limit) {
                Ok(results) => results
                    .into_iter()
                    .map(|r| RankedListCandidate {
                        chunk_id: r.chunk_id,
                        relative_path: r.relative_path,
                        language: r.language,
                        symbol_name: r.symbol_name,
                        symbol_kind: r.symbol_kind,
                        signature: r.signature,
                        content: r.content,
                        line_start: r.line_start,
                        line_end: r.line_end,
                        raw_score: r.score,
                    })
                    .collect(),
                Err(_) => Vec::new(),
            }
        };

        // 2. Vector search (generate embedding for query)
        let vector_candidates: Vec<RankedListCandidate> = {
            let query_emb = match self.embedding_engine.embed_query(query).await {
                Ok(emb) => emb,
                Err(_) => Vec::new(),
            };

            if !query_emb.is_empty() {
                let store = self.vector_store.read().await;
                store
                    .search(&query_emb, candidate_limit)
                    .into_iter()
                    .map(|r| RankedListCandidate {
                        chunk_id: r.chunk_id,
                        relative_path: r.relative_path,
                        language: r.language,
                        symbol_name: r.symbol_name,
                        symbol_kind: r.symbol_kind,
                        signature: r.signature,
                        content: r.content,
                        line_start: r.line_start,
                        line_end: r.line_end,
                        raw_score: r.score,
                    })
                    .collect()
            } else {
                Vec::new()
            }
        };

        // 3. Symbol Search (exact and fuzzy match)
        let symbol_candidates: Vec<RankedListCandidate> = {
            let store = self.symbol_store.read().await;
            let sym_results = store.search(query, candidate_limit);

            sym_results
                .into_iter()
                .map(|r| RankedListCandidate {
                    chunk_id: format!("{}:{}-{}", r.symbol.relative_path, r.symbol.line_start, r.symbol.line_end),
                    relative_path: r.symbol.relative_path,
                    language: "unknown".to_string(),
                    symbol_name: Some(r.symbol.name),
                    symbol_kind: Some(r.symbol.kind),
                    signature: Some(r.symbol.signature),
                    content: String::new(),
                    line_start: r.symbol.line_start,
                    line_end: r.symbol.line_end,
                    raw_score: r.score,
                })
                .collect()
        };

        // 4. Reciprocal Rank Fusion
        let fused = reciprocal_rank_fusion(
            &bm25_candidates,
            self.config.bm25_weight,
            &vector_candidates,
            self.config.vector_weight,
            &symbol_candidates,
            self.config.symbol_weight,
            self.config.rrf_k,
            k,
        );

        Ok(fused)
    }
}
