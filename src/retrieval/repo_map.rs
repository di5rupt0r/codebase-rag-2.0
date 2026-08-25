use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::index::SymbolStore;
use crate::parser::chunker::Chunker;

pub struct RepoMapGenerator {
    symbol_store: Arc<RwLock<SymbolStore>>,
}

impl RepoMapGenerator {
    pub fn new(symbol_store: Arc<RwLock<SymbolStore>>) -> Self {
        Self { symbol_store }
    }

    pub async fn generate_map(&self, max_tokens: usize) -> String {
        let store = self.symbol_store.read().await;
        let all_symbols = store.get_all_symbols();

        let mut file_symbols: BTreeMap<String, Vec<&crate::index::SymbolRecord>> = BTreeMap::new();
        for sym in all_symbols {
            file_symbols
                .entry(sym.relative_path.clone())
                .or_default()
                .push(sym);
        }

        let mut output = String::new();
        output.push_str("# Repository Structural Skeleton & Symbol Map\n\n");

        let mut current_tokens = Chunker::estimate_tokens(&output);

        for (file_path, symbols) in file_symbols {
            let mut file_block = format!("## {}\n", file_path);

            for sym in symbols {
                let scope_prefix = if let Some(ref scope) = sym.parent_scope {
                    format!("{}::", scope)
                } else {
                    String::new()
                };

                let sym_line = format!(
                    "  - [{}] {}{}: L{}-L{}\n    `{}`\n",
                    sym.kind, scope_prefix, sym.name, sym.line_start, sym.line_end, sym.signature
                );
                file_block.push_str(&sym_line);
            }
            file_block.push('\n');

            let block_tokens = Chunker::estimate_tokens(&file_block);
            if current_tokens + block_tokens > max_tokens {
                output.push_str("... (truncated to fit token budget)\n");
                break;
            }

            output.push_str(&file_block);
            current_tokens += block_tokens;
        }

        output
    }
}
