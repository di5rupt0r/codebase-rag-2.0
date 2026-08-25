use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::*;
use tantivy::{Index, IndexReader, IndexWriter, ReloadPolicy};

use crate::parser::CodeChunk;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LexicalSearchResult {
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

pub struct TantivyStore {
    index: Index,
    reader: IndexReader,
    _schema: Schema,
    f_chunk_id: Field,
    f_relative_path: Field,
    f_language: Field,
    f_symbol_name: Field,
    f_symbol_kind: Field,
    f_signature: Field,
    f_content: Field,
    f_line_start: Field,
    f_line_end: Field,
}

impl TantivyStore {
    pub fn open_or_create<P: AsRef<Path>>(dir: P) -> Result<Self> {
        let dir = dir.as_ref();
        std::fs::create_dir_all(dir)?;

        let mut schema_builder = Schema::builder();
        let f_chunk_id = schema_builder.add_text_field("chunk_id", STRING | STORED);
        let f_relative_path = schema_builder.add_text_field("relative_path", STRING | STORED);
        let f_language = schema_builder.add_text_field("language", STRING | STORED);
        let f_symbol_name = schema_builder.add_text_field("symbol_name", TEXT | STORED);
        let f_symbol_kind = schema_builder.add_text_field("symbol_kind", STRING | STORED);
        let f_signature = schema_builder.add_text_field("signature", TEXT | STORED);
        let f_content = schema_builder.add_text_field("content", TEXT | STORED);
        let f_line_start = schema_builder.add_u64_field("line_start", INDEXED | STORED);
        let f_line_end = schema_builder.add_u64_field("line_end", INDEXED | STORED);

        let schema = schema_builder.build();

        let index = match Index::open_in_dir(dir) {
            Ok(idx) => idx,
            Err(_) => Index::create_in_dir(dir, schema.clone())?,
        };

        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::Manual)
            .try_into()?;

        Ok(Self {
            index,
            reader,
            _schema: schema,
            f_chunk_id,
            f_relative_path,
            f_language,
            f_symbol_name,
            f_symbol_kind,
            f_signature,
            f_content,
            f_line_start,
            f_line_end,
        })
    }

    pub fn create_in_ram() -> Result<Self> {
        let mut schema_builder = Schema::builder();
        let f_chunk_id = schema_builder.add_text_field("chunk_id", STRING | STORED);
        let f_relative_path = schema_builder.add_text_field("relative_path", STRING | STORED);
        let f_language = schema_builder.add_text_field("language", STRING | STORED);
        let f_symbol_name = schema_builder.add_text_field("symbol_name", TEXT | STORED);
        let f_symbol_kind = schema_builder.add_text_field("symbol_kind", STRING | STORED);
        let f_signature = schema_builder.add_text_field("signature", TEXT | STORED);
        let f_content = schema_builder.add_text_field("content", TEXT | STORED);
        let f_line_start = schema_builder.add_u64_field("line_start", INDEXED | STORED);
        let f_line_end = schema_builder.add_u64_field("line_end", INDEXED | STORED);

        let schema = schema_builder.build();
        let index = Index::create_in_ram(schema.clone());
        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::Manual)
            .try_into()?;

        Ok(Self {
            index,
            reader,
            _schema: schema,
            f_chunk_id,
            f_relative_path,
            f_language,
            f_symbol_name,
            f_symbol_kind,
            f_signature,
            f_content,
            f_line_start,
            f_line_end,
        })
    }

    pub fn get_writer(&self, heap_size_mb: usize) -> Result<IndexWriter> {
        Ok(self.index.writer(heap_size_mb * 1024 * 1024)?)
    }

    pub fn add_chunks(&self, writer: &mut IndexWriter, chunks: &[CodeChunk]) -> Result<()> {
        for chunk in chunks {
            let mut doc = TantivyDocument::default();
            doc.add_text(self.f_chunk_id, &chunk.id);
            doc.add_text(self.f_relative_path, &chunk.relative_path);
            doc.add_text(self.f_language, chunk.language.name());

            if let Some(name) = &chunk.symbol_name {
                doc.add_text(self.f_symbol_name, name);
            }
            if let Some(kind) = &chunk.symbol_kind {
                doc.add_text(self.f_symbol_kind, kind.as_str());
            }
            if let Some(sig) = &chunk.signature {
                doc.add_text(self.f_signature, sig);
            }

            doc.add_text(self.f_content, &chunk.content);
            doc.add_u64(self.f_line_start, chunk.line_start as u64);
            doc.add_u64(self.f_line_end, chunk.line_end as u64);

            writer.add_document(doc)?;
        }
        Ok(())
    }

    pub fn delete_file(&self, writer: &mut IndexWriter, relative_path: &str) -> Result<()> {
        let term = Term::from_field_text(self.f_relative_path, relative_path);
        writer.delete_term(term);
        Ok(())
    }

    pub fn search(&self, query_str: &str, limit: usize) -> Result<Vec<LexicalSearchResult>> {
        self.reader.reload()?;
        let searcher = self.reader.searcher();
        let mut query_parser = QueryParser::for_index(
            &self.index,
            vec![self.f_symbol_name, self.f_signature, self.f_content],
        );

        query_parser.set_field_boost(self.f_symbol_name, 3.0);
        query_parser.set_field_boost(self.f_signature, 2.0);
        query_parser.set_field_boost(self.f_content, 1.0);

        let sanitized = sanitize_query(query_str);
        let query = match query_parser.parse_query(&sanitized) {
            Ok(q) => q,
            Err(_) => match query_parser.parse_query(query_str) {
                Ok(q) => q,
                Err(_) => {
                    let words: Vec<&str> = query_str
                        .split(|c: char| !c.is_alphanumeric() && c != '_')
                        .filter(|w| !w.is_empty())
                        .collect();
                    if words.is_empty() {
                        return Ok(Vec::new());
                    }
                    let fallback_str = words.join(" ");
                    query_parser
                        .parse_query(&fallback_str)
                        .map_err(|e| anyhow!("Failed to parse query: {}", e))?
                }
            },
        };

        let top_docs = searcher.search(&query, &TopDocs::with_limit(limit))?;
        let mut results = Vec::with_capacity(top_docs.len());

        for (score, doc_address) in top_docs {
            let doc: TantivyDocument = searcher.doc(doc_address)?;

            let chunk_id = doc
                .get_first(self.f_chunk_id)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let relative_path = doc
                .get_first(self.f_relative_path)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let language = doc
                .get_first(self.f_language)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let symbol_name = doc
                .get_first(self.f_symbol_name)
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let symbol_kind = doc
                .get_first(self.f_symbol_kind)
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let signature = doc
                .get_first(self.f_signature)
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let content = doc
                .get_first(self.f_content)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let line_start = doc
                .get_first(self.f_line_start)
                .and_then(|v| v.as_u64())
                .unwrap_or(1) as usize;

            let line_end = doc
                .get_first(self.f_line_end)
                .and_then(|v| v.as_u64())
                .unwrap_or(1) as usize;

            results.push(LexicalSearchResult {
                chunk_id,
                relative_path,
                language,
                symbol_name,
                symbol_kind,
                signature,
                content,
                line_start,
                line_end,
                score,
            });
        }

        Ok(results)
    }
}

fn sanitize_query(query: &str) -> String {
    let mut words = Vec::new();
    for token in query.split_whitespace() {
        let clean: String = token
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_' || *c == ':')
            .collect();
        if !clean.is_empty() {
            words.push(clean);
        }
    }
    words.join(" ")
}
