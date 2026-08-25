use anyhow::Result;
use ignore::WalkBuilder;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

use super::file_tracker::FileTracker;
use super::symbol_store::SymbolStore;
use super::tantivy_store::TantivyStore;
use super::vector_store::VectorStore;
use crate::config::AppConfig;
use crate::embeddings::EmbeddingEngine;
use crate::parser::Chunker;

pub struct IndexEngine {
    config: AppConfig,
    tantivy_store: Arc<RwLock<TantivyStore>>,
    vector_store: Arc<RwLock<VectorStore>>,
    symbol_store: Arc<RwLock<SymbolStore>>,
    file_tracker: Arc<RwLock<FileTracker>>,
    embedding_engine: Arc<EmbeddingEngine>,
    chunker: Chunker,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IndexStats {
    pub total_files: usize,
    pub total_chunks: usize,
    pub total_symbols: usize,
    pub total_vectors: usize,
    pub storage_dir: String,
    pub embedding_provider: String,
    pub embedding_model: String,
}

impl IndexEngine {
    pub fn new(config: AppConfig) -> Result<Self> {
        let storage_dir = &config.storage_dir;
        std::fs::create_dir_all(storage_dir)?;

        let tantivy_dir = storage_dir.join("tantivy");
        let tantivy_store = Arc::new(RwLock::new(TantivyStore::open_or_create(&tantivy_dir)?));
        let vector_store = Arc::new(RwLock::new(VectorStore::open_or_create(storage_dir)?));
        let symbol_store = Arc::new(RwLock::new(SymbolStore::open_or_create(storage_dir)?));
        let file_tracker = Arc::new(RwLock::new(FileTracker::open_or_create(storage_dir)?));
        let embedding_engine = Arc::new(EmbeddingEngine::from_config(&config));
        let chunker = Chunker::new(
            config.index.chunk_target_lines,
            config.index.chunk_overlap_lines,
        );

        Ok(Self {
            config,
            tantivy_store,
            vector_store,
            symbol_store,
            file_tracker,
            embedding_engine,
            chunker,
        })
    }

    pub fn new_in_ram(config: AppConfig) -> Result<Self> {
        let tantivy_store = Arc::new(RwLock::new(TantivyStore::create_in_ram()?));
        let vector_store = Arc::new(RwLock::new(VectorStore::create_in_ram()));
        let symbol_store = Arc::new(RwLock::new(SymbolStore::create_in_ram()));
        let file_tracker = Arc::new(RwLock::new(FileTracker::default()));
        let embedding_engine = Arc::new(EmbeddingEngine::from_config(&config));
        let chunker = Chunker::new(
            config.index.chunk_target_lines,
            config.index.chunk_overlap_lines,
        );

        Ok(Self {
            config,
            tantivy_store,
            vector_store,
            symbol_store,
            file_tracker,
            embedding_engine,
            chunker,
        })
    }

    pub fn scan_workspace(&self, root: &Path) -> Vec<PathBuf> {
        let mut builder = WalkBuilder::new(root);
        builder.hidden(false);
        builder.git_ignore(true);
        builder.git_global(true);
        builder.git_exclude(true);

        let mut files = Vec::new();
        for entry in builder.build().filter_map(|e| e.ok()) {
            if entry.file_type().map_or(false, |ft| ft.is_file()) {
                let path = entry.path().to_path_buf();
                let rel_str = path.to_string_lossy();

                if rel_str.contains("/.git/")
                    || rel_str.contains("/target/")
                    || rel_str.contains("/node_modules/")
                    || rel_str.contains("/.codebase-rag/")
                {
                    continue;
                }

                if let Ok(meta) = entry.metadata() {
                    if meta.len() <= self.config.index.max_file_size_bytes {
                        files.push(path);
                    }
                }
            }
        }
        files
    }

    pub async fn index_all(&self, show_progress: bool) -> Result<IndexStats> {
        let project_root = self.config.project_root.clone();
        let files = self.scan_workspace(&project_root);

        let changes = {
            let tracker = self.file_tracker.read().await;
            tracker.detect_changes(&project_root, &files)?
        };

        if changes.is_empty() {
            info!("Codebase index is up-to-date. No changes detected.");
            return self.get_stats().await;
        }

        info!(
            "Indexing changes: {} added, {} modified, {} deleted",
            changes.added.len(),
            changes.modified.len(),
            changes.deleted.len()
        );

        // 1. Handle Deletions
        if !changes.deleted.is_empty() || !changes.modified.is_empty() {
            let tantivy = self.tantivy_store.write().await;
            let mut vector = self.vector_store.write().await;
            let mut symbol = self.symbol_store.write().await;
            let mut tracker = self.file_tracker.write().await;
            let mut writer = tantivy.get_writer(50)?;

            for del_rel in &changes.deleted {
                tantivy.delete_file(&mut writer, del_rel)?;
                vector.delete_file(del_rel);
                symbol.delete_file(del_rel);
                tracker.remove_file(del_rel);
            }

            for mod_path in &changes.modified {
                let rel_path = mod_path
                    .strip_prefix(&project_root)
                    .unwrap_or(mod_path)
                    .to_string_lossy()
                    .to_string();
                tantivy.delete_file(&mut writer, &rel_path)?;
                vector.delete_file(&rel_path);
                symbol.delete_file(&rel_path);
                tracker.remove_file(&rel_path);
            }

            writer.commit()?;
        }

        // 2. Parse & Chunk Added/Modified Files
        let mut files_to_process = Vec::new();
        files_to_process.extend(changes.added);
        files_to_process.extend(changes.modified);

        let pb = if show_progress {
            let p = ProgressBar::new(files_to_process.len() as u64);
            p.set_style(
                ProgressStyle::default_bar()
                    .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} files ({msg})")
                    .unwrap()
                    .progress_chars("#>-"),
            );
            Some(p)
        } else {
            None
        };

        let mut all_chunks = Vec::new();
        let mut file_records = Vec::new();

        for file_path in files_to_process {
            let rel_path = file_path
                .strip_prefix(&project_root)
                .unwrap_or(&file_path)
                .to_string_lossy()
                .to_string();

            if let Some(ref p) = pb {
                p.set_message(rel_path.clone());
            }

            let content = match std::fs::read_to_string(&file_path) {
                Ok(c) => c,
                Err(_) => {
                    if let Some(ref p) = pb {
                        p.inc(1);
                    }
                    continue;
                }
            };

            let bytes = content.as_bytes();
            let hash = FileTracker::compute_sha256(bytes);
            let metadata = std::fs::metadata(&file_path)?;
            let mtime_secs = metadata
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let size_bytes = metadata.len();

            let (chunks, symbols) = self.chunker.chunk_file(&file_path, &rel_path, &content);

            {
                let mut sym_store = self.symbol_store.write().await;
                sym_store.add_symbols(&rel_path, &symbols);
            }

            file_records.push((rel_path, hash, mtime_secs, size_bytes, chunks.len()));
            all_chunks.extend(chunks);

            if let Some(ref p) = pb {
                p.inc(1);
            }
        }

        if let Some(p) = pb {
            p.finish_with_message("Parsing & AST Chunking Complete");
        }

        // 3. Batch Embeddings & Vector Storage
        if !all_chunks.is_empty() {
            let texts: Vec<String> = all_chunks
                .iter()
                .map(|c| {
                    if let Some(ref sig) = c.signature {
                        format!("{}\n{}", sig, c.content)
                    } else {
                        c.content.clone()
                    }
                })
                .collect();

            let batch_size = self.config.embedding.batch_size;
            let mut embeddings = Vec::with_capacity(texts.len());

            for chunk_slice in texts.chunks(batch_size) {
                let emb_res = self.embedding_engine.embed_batch(chunk_slice).await?;
                embeddings.extend(emb_res);
            }

            // Insert into Vector Store & Tantivy Store
            {
                let mut vec_store = self.vector_store.write().await;
                for (chunk, emb) in all_chunks.iter().zip(embeddings) {
                    vec_store.add_chunk_with_embedding(chunk, emb);
                }
            }

            {
                let tantivy = self.tantivy_store.write().await;
                let mut writer = tantivy.get_writer(50)?;
                tantivy.add_chunks(&mut writer, &all_chunks)?;
                writer.commit()?;
            }
        }

        // 4. Update File Tracker
        {
            let mut tracker = self.file_tracker.write().await;
            for (rel_path, hash, mtime, size, chunk_count) in file_records {
                tracker.record_file(&rel_path, hash, mtime, size, chunk_count);
            }
        }

        // 5. Persist Stores
        self.vector_store.read().await.save()?;
        self.symbol_store.read().await.save()?;
        self.file_tracker.read().await.save()?;

        self.get_stats().await
    }

    pub async fn get_stats(&self) -> Result<IndexStats> {
        let total_files = self.file_tracker.read().await.count();
        let total_chunks = self.file_tracker.read().await.total_chunks();
        let total_symbols = self.symbol_store.read().await.count();
        let total_vectors = self.vector_store.read().await.count();

        Ok(IndexStats {
            total_files,
            total_chunks,
            total_symbols,
            total_vectors,
            storage_dir: self.config.storage_dir.to_string_lossy().to_string(),
            embedding_provider: format!("{:?}", self.config.embedding.provider),
            embedding_model: self.config.embedding.model.clone(),
        })
    }

    pub fn get_tantivy_store(&self) -> Arc<RwLock<TantivyStore>> {
        self.tantivy_store.clone()
    }

    pub fn get_vector_store(&self) -> Arc<RwLock<VectorStore>> {
        self.vector_store.clone()
    }

    pub fn get_symbol_store(&self) -> Arc<RwLock<SymbolStore>> {
        self.symbol_store.clone()
    }

    pub fn get_embedding_engine(&self) -> Arc<EmbeddingEngine> {
        self.embedding_engine.clone()
    }

    pub fn get_config(&self) -> &AppConfig {
        &self.config
    }
}
