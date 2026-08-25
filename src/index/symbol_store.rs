use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};
use strsim::jaro_winkler;

use crate::parser::ast::{SymbolInfo, SymbolKind};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolRecord {
    pub name: String,
    pub kind: String,
    pub relative_path: String,
    pub signature: String,
    pub parent_scope: Option<String>,
    pub line_start: usize,
    pub line_end: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolSearchResult {
    pub symbol: SymbolRecord,
    pub score: f32,
}

#[derive(Default, Serialize, Deserialize)]
pub struct SymbolStore {
    symbols: Vec<SymbolRecord>,
    #[serde(skip)]
    file_path: Option<PathBuf>,
}

impl SymbolStore {
    pub fn open_or_create<P: AsRef<Path>>(dir: P) -> Result<Self> {
        let dir = dir.as_ref();
        std::fs::create_dir_all(dir)?;
        let path = dir.join("symbols.json");

        if path.exists() {
            let file = File::open(&path)?;
            let reader = BufReader::new(file);
            let mut store: Self = serde_json::from_reader(reader)?;
            store.file_path = Some(path);
            Ok(store)
        } else {
            Ok(Self {
                symbols: Vec::new(),
                file_path: Some(path),
            })
        }
    }

    pub fn create_in_ram() -> Self {
        Self {
            symbols: Vec::new(),
            file_path: None,
        }
    }

    pub fn add_symbols(&mut self, relative_path: &str, symbols: &[SymbolInfo]) {
        self.delete_file(relative_path);

        for sym in symbols {
            if matches!(sym.kind, SymbolKind::Import) {
                continue;
            }

            self.symbols.push(SymbolRecord {
                name: sym.name.clone(),
                kind: sym.kind.as_str().to_string(),
                relative_path: relative_path.to_string(),
                signature: sym.signature.clone(),
                parent_scope: sym.parent_scope.clone(),
                line_start: sym.line_start,
                line_end: sym.line_end,
            });
        }
    }

    pub fn delete_file(&mut self, relative_path: &str) {
        self.symbols.retain(|s| s.relative_path != relative_path);
    }

    pub fn save(&self) -> Result<()> {
        if let Some(path) = &self.file_path {
            let file = File::create(path)?;
            let writer = BufWriter::new(file);
            serde_json::to_writer(writer, self)?;
        }
        Ok(())
    }

    pub fn search(&self, query: &str, limit: usize) -> Vec<SymbolSearchResult> {
        let query_lower = query.to_lowercase();
        let mut results = Vec::new();

        for sym in &self.symbols {
            let sym_lower = sym.name.to_lowercase();

            let score = if sym_lower == query_lower {
                1.0f32
            } else if sym_lower.starts_with(&query_lower) {
                0.9f32
            } else if sym_lower.contains(&query_lower) {
                0.8f32
            } else {
                let similarity = jaro_winkler(&query_lower, &sym_lower) as f32;
                if similarity > 0.75 {
                    similarity * 0.7
                } else {
                    0.0
                }
            };

            if score > 0.0 {
                results.push(SymbolSearchResult {
                    symbol: sym.clone(),
                    score,
                });
            }
        }

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(limit);
        results
    }

    pub fn get_all_symbols(&self) -> &[SymbolRecord] {
        &self.symbols
    }

    pub fn count(&self) -> usize {
        self.symbols.len()
    }
}
