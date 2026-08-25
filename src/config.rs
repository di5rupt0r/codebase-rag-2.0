use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EmbeddingProviderKind {
    Gemini,
    OpenAI,
    Voyage,
    Ollama,
    None,
}

impl Default for EmbeddingProviderKind {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    pub provider: EmbeddingProviderKind,
    pub model: String,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub dimensions: Option<usize>,
    pub batch_size: usize,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            provider: EmbeddingProviderKind::None,
            model: "text-embedding-004".to_string(),
            api_key: None,
            base_url: None,
            dimensions: None,
            batch_size: 32,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalConfig {
    pub top_k: usize,
    pub bm25_weight: f32,
    pub vector_weight: f32,
    pub symbol_weight: f32,
    pub rrf_k: usize,
    pub max_context_tokens: usize,
}

impl Default for RetrievalConfig {
    fn default() -> Self {
        Self {
            top_k: 10,
            bm25_weight: 1.0,
            vector_weight: 1.0,
            symbol_weight: 1.5,
            rrf_k: 60,
            max_context_tokens: 8000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexConfig {
    pub max_file_size_bytes: u64,
    pub chunk_target_lines: usize,
    pub chunk_overlap_lines: usize,
    pub ignore_patterns: Vec<String>,
}

impl Default for IndexConfig {
    fn default() -> Self {
        Self {
            max_file_size_bytes: 1024 * 1024, // 1MB
            chunk_target_lines: 60,
            chunk_overlap_lines: 10,
            ignore_patterns: vec![
                "**/.git/**".to_string(),
                "**/target/**".to_string(),
                "**/node_modules/**".to_string(),
                "**/dist/**".to_string(),
                "**/build/**".to_string(),
                "**/.venv/**".to_string(),
                "**/__pycache__/**".to_string(),
                "**/.codebase-rag/**".to_string(),
            ],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub project_root: PathBuf,
    pub storage_dir: PathBuf,
    pub embedding: EmbeddingConfig,
    pub retrieval: RetrievalConfig,
    pub index: IndexConfig,
}

impl AppConfig {
    pub fn new<P: AsRef<Path>>(project_root: P) -> Self {
        let root = project_root.as_ref().to_path_buf();
        let storage_dir = root.join(".codebase-rag");

        let mut config = Self {
            project_root: root,
            storage_dir,
            embedding: EmbeddingConfig::default(),
            retrieval: RetrievalConfig::default(),
            index: IndexConfig::default(),
        };

        config.load_env_and_file();
        config
    }

    pub fn load_env_and_file(&mut self) {
        let config_file = self.project_root.join(".codebase-rag.toml");
        if config_file.exists() {
            if let Ok(contents) = std::fs::read_to_string(&config_file) {
                if let Ok(parsed) = toml::from_str::<Self>(&contents) {
                    self.embedding = parsed.embedding;
                    self.retrieval = parsed.retrieval;
                    self.index = parsed.index;
                }
            }
        }

        // Environment variable overrides
        if let Ok(val) = std::env::var("CODEBASE_RAG_PROVIDER") {
            self.embedding.provider = match val.to_lowercase().as_str() {
                "gemini" => EmbeddingProviderKind::Gemini,
                "openai" => EmbeddingProviderKind::OpenAI,
                "voyage" => EmbeddingProviderKind::Voyage,
                "ollama" => EmbeddingProviderKind::Ollama,
                _ => EmbeddingProviderKind::None,
            };
        }

        if let Ok(val) = std::env::var("GEMINI_API_KEY") {
            if self.embedding.provider == EmbeddingProviderKind::None {
                self.embedding.provider = EmbeddingProviderKind::Gemini;
                self.embedding.model = "text-embedding-004".to_string();
            }
            if self.embedding.provider == EmbeddingProviderKind::Gemini {
                self.embedding.api_key = Some(val);
            }
        }

        if let Ok(val) = std::env::var("OPENAI_API_KEY") {
            if self.embedding.provider == EmbeddingProviderKind::None {
                self.embedding.provider = EmbeddingProviderKind::OpenAI;
                self.embedding.model = "text-embedding-3-small".to_string();
            }
            if self.embedding.provider == EmbeddingProviderKind::OpenAI {
                self.embedding.api_key = Some(val);
            }
        }

        if let Ok(val) = std::env::var("VOYAGE_API_KEY") {
            if self.embedding.provider == EmbeddingProviderKind::None {
                self.embedding.provider = EmbeddingProviderKind::Voyage;
                self.embedding.model = "voyage-code-3".to_string();
            }
            if self.embedding.provider == EmbeddingProviderKind::Voyage {
                self.embedding.api_key = Some(val);
            }
        }

        if let Ok(val) = std::env::var("OLLAMA_HOST") {
            self.embedding.base_url = Some(val);
            if self.embedding.provider == EmbeddingProviderKind::None {
                self.embedding.provider = EmbeddingProviderKind::Ollama;
                self.embedding.model = "nomic-embed-text".to_string();
            }
        }

        if let Ok(val) = std::env::var("CODEBASE_RAG_MODEL") {
            self.embedding.model = val;
        }

        if let Ok(val) = std::env::var("CODEBASE_RAG_TOP_K") {
            if let Ok(parsed) = val.parse::<usize>() {
                self.retrieval.top_k = parsed;
            }
        }
    }
}
