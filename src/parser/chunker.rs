use super::ast::{AstParser, SymbolInfo, SymbolKind};
use super::languages::Language;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeChunk {
    pub id: String,
    pub file_path: PathBuf,
    pub relative_path: String,
    pub language: Language,
    pub symbol_name: Option<String>,
    pub symbol_kind: Option<SymbolKind>,
    pub signature: Option<String>,
    pub content: String,
    pub line_start: usize,
    pub line_end: usize,
    pub token_count_estimate: usize,
}

pub struct Chunker {
    target_lines: usize,
    overlap_lines: usize,
}

impl Chunker {
    pub fn new(target_lines: usize, overlap_lines: usize) -> Self {
        Self {
            target_lines: target_lines.max(10),
            overlap_lines: overlap_lines.min(target_lines / 2),
        }
    }

    pub fn chunk_file<P: AsRef<Path>>(
        &self,
        file_path: P,
        relative_path: &str,
        content: &str,
    ) -> (Vec<CodeChunk>, Vec<SymbolInfo>) {
        let path = file_path.as_ref();
        let lang = Language::from_path(path);
        let lines: Vec<&str> = content.lines().collect();

        if lines.is_empty() {
            return (Vec::new(), Vec::new());
        }

        if lang.is_ast_supported() {
            let symbols = AstParser::parse_symbols(content, lang);
            if !symbols.is_empty() {
                let chunks = self.chunk_from_ast(path, relative_path, lang, &lines, &symbols);
                return (chunks, symbols);
            }
        }

        // Fallback line/window chunker
        let chunks = self.chunk_sliding_window(path, relative_path, lang, &lines);
        (chunks, Vec::new())
    }

    fn chunk_from_ast(
        &self,
        file_path: &Path,
        relative_path: &str,
        lang: Language,
        lines: &[&str],
        symbols: &[SymbolInfo],
    ) -> Vec<CodeChunk> {
        let mut chunks = Vec::new();
        let total_lines = lines.len();

        for sym in symbols {
            if matches!(sym.kind, SymbolKind::Import) {
                continue;
            }

            let start = sym.line_start.saturating_sub(1);
            let end = sym.line_end.min(total_lines);

            if start >= end {
                continue;
            }

            let sym_lines = &lines[start..end];
            let sym_content = sym_lines.join("\n");
            let line_count = end - start;

            if line_count <= self.target_lines * 2 {
                let id = format!("{}:{}-{}", relative_path, sym.line_start, sym.line_end);
                let token_count = Self::estimate_tokens(&sym_content);
                chunks.push(CodeChunk {
                    id,
                    file_path: file_path.to_path_buf(),
                    relative_path: relative_path.to_string(),
                    language: lang,
                    symbol_name: Some(sym.name.clone()),
                    symbol_kind: Some(sym.kind),
                    signature: Some(sym.signature.clone()),
                    content: sym_content,
                    line_start: sym.line_start,
                    line_end: sym.line_end,
                    token_count_estimate: token_count,
                });
            } else {
                let mut chunk_start = start;
                while chunk_start < end {
                    let chunk_end = (chunk_start + self.target_lines).min(end);
                    let sub_lines = &lines[chunk_start..chunk_end];
                    let sub_content = sub_lines.join("\n");
                    let sub_start_line = chunk_start + 1;
                    let sub_end_line = chunk_end;

                    let id = format!("{}:{}-{}", relative_path, sub_start_line, sub_end_line);
                    let token_count = Self::estimate_tokens(&sub_content);

                    chunks.push(CodeChunk {
                        id,
                        file_path: file_path.to_path_buf(),
                        relative_path: relative_path.to_string(),
                        language: lang,
                        symbol_name: Some(sym.name.clone()),
                        symbol_kind: Some(sym.kind),
                        signature: Some(sym.signature.clone()),
                        content: sub_content,
                        line_start: sub_start_line,
                        line_end: sub_end_line,
                        token_count_estimate: token_count,
                    });

                    if chunk_end >= end {
                        break;
                    }
                    chunk_start += self.target_lines - self.overlap_lines;
                }
            }
        }

        if chunks.is_empty() {
            return self.chunk_sliding_window(file_path, relative_path, lang, lines);
        }

        chunks
    }

    fn chunk_sliding_window(
        &self,
        file_path: &Path,
        relative_path: &str,
        lang: Language,
        lines: &[&str],
    ) -> Vec<CodeChunk> {
        let mut chunks = Vec::new();
        let total_lines = lines.len();
        let mut start = 0;

        while start < total_lines {
            let end = (start + self.target_lines).min(total_lines);
            let chunk_lines = &lines[start..end];
            let chunk_content = chunk_lines.join("\n");
            let line_start = start + 1;
            let line_end = end;

            let id = format!("{}:{}-{}", relative_path, line_start, line_end);
            let token_count = Self::estimate_tokens(&chunk_content);

            chunks.push(CodeChunk {
                id,
                file_path: file_path.to_path_buf(),
                relative_path: relative_path.to_string(),
                language: lang,
                symbol_name: None,
                symbol_kind: None,
                signature: None,
                content: chunk_content,
                line_start,
                line_end,
                token_count_estimate: token_count,
            });

            if end >= total_lines {
                break;
            }
            start += self.target_lines - self.overlap_lines;
        }

        chunks
    }

    pub fn estimate_tokens(text: &str) -> usize {
        (text.len() + 3) / 4
    }
}
