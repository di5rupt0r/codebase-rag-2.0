use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::fusion::ScoredChunk;
use crate::parser::chunker::Chunker;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackedContextBlock {
    pub file_path: String,
    pub line_start: usize,
    pub line_end: usize,
    pub symbol_name: Option<String>,
    pub symbol_kind: Option<String>,
    pub content: String,
    pub rrf_score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackedContext {
    pub blocks: Vec<PackedContextBlock>,
    pub total_estimated_tokens: usize,
    pub formatted_text: String,
}

pub struct ContextPacker {
    max_tokens: usize,
}

impl ContextPacker {
    pub fn new(max_tokens: usize) -> Self {
        Self { max_tokens }
    }

    pub fn pack(&self, chunks: &[ScoredChunk]) -> PackedContext {
        let mut blocks = Vec::new();
        let mut total_tokens = 0;

        // Deduplicate overlapping chunks for the same file
        let mut seen_ranges: HashMap<String, Vec<(usize, usize)>> = HashMap::new();

        for chunk in chunks {
            // Check if chunk text is empty (e.g. from pure symbol table match without body)
            if chunk.content.is_empty() {
                continue;
            }

            let ranges = seen_ranges.entry(chunk.relative_path.clone()).or_default();

            // Check overlap
            let mut overlaps = false;
            for (s, e) in ranges.iter() {
                let overlap_start = chunk.line_start.max(*s);
                let overlap_end = chunk.line_end.min(*e);
                if overlap_start <= overlap_end {
                    let overlap_len = overlap_end - overlap_start + 1;
                    let chunk_len = chunk.line_end - chunk.line_start + 1;
                    if overlap_len as f32 / chunk_len as f32 > 0.6 {
                        overlaps = true;
                        break;
                    }
                }
            }

            if overlaps {
                continue;
            }

            let chunk_tokens = Chunker::estimate_tokens(&chunk.content);
            if total_tokens + chunk_tokens > self.max_tokens && !blocks.is_empty() {
                break;
            }

            ranges.push((chunk.line_start, chunk.line_end));
            total_tokens += chunk_tokens;

            blocks.push(PackedContextBlock {
                file_path: chunk.relative_path.clone(),
                line_start: chunk.line_start,
                line_end: chunk.line_end,
                symbol_name: chunk.symbol_name.clone(),
                symbol_kind: chunk.symbol_kind.clone(),
                content: chunk.content.clone(),
                rrf_score: chunk.rrf_score,
            });
        }

        let formatted_text = Self::format_blocks(&blocks);

        PackedContext {
            blocks,
            total_estimated_tokens: total_tokens,
            formatted_text,
        }
    }

    fn format_blocks(blocks: &[PackedContextBlock]) -> String {
        if blocks.is_empty() {
            return String::new();
        }

        let mut out = String::new();
        out.push_str("<!-- CODEBASE RAG CONTEXT -->\n");

        for (idx, block) in blocks.iter().enumerate() {
            let symbol_header = if let Some(ref sym) = block.symbol_name {
                let kind = block.symbol_kind.as_deref().unwrap_or("symbol");
                format!(" ({}: {})", kind, sym)
            } else {
                String::new()
            };

            out.push_str(&format!(
                "### [{}] {}:{}-{}{}\n",
                idx + 1,
                block.file_path,
                block.line_start,
                block.line_end,
                symbol_header
            ));

            out.push_str("```\n");
            out.push_str(&block.content);
            if !block.content.ends_with('\n') {
                out.push('\n');
            }
            out.push_str("```\n\n");
        }

        out.push_str("<!-- /CODEBASE RAG CONTEXT -->");
        out
    }
}
