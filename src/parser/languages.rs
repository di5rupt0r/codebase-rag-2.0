use std::path::Path;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Language {
    Rust,
    Python,
    TypeScript,
    JavaScript,
    Go,
    Java,
    C,
    Cpp,
    Markdown,
    Json,
    Yaml,
    Toml,
    Unknown,
}

impl Language {
    pub fn from_path<P: AsRef<Path>>(path: P) -> Self {
        let path = path.as_ref();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            match ext.to_lowercase().as_str() {
                "rs" => Language::Rust,
                "py" | "pyi" => Language::Python,
                "ts" | "tsx" | "mts" | "cts" => Language::TypeScript,
                "js" | "jsx" | "mjs" | "cjs" => Language::JavaScript,
                "go" => Language::Go,
                "java" => Language::Java,
                "c" | "h" => Language::C,
                "cpp" | "cxx" | "cc" | "hpp" | "hxx" => Language::Cpp,
                "md" | "markdown" => Language::Markdown,
                "json" => Language::Json,
                "yaml" | "yml" => Language::Yaml,
                "toml" => Language::Toml,
                _ => Language::Unknown,
            }
        } else {
            Language::Unknown
        }
    }

    pub fn is_ast_supported(&self) -> bool {
        matches!(
            self,
            Language::Rust
                | Language::Python
                | Language::TypeScript
                | Language::JavaScript
                | Language::Go
                | Language::Java
                | Language::C
                | Language::Cpp
        )
    }

    pub fn name(&self) -> &'static str {
        match self {
            Language::Rust => "rust",
            Language::Python => "python",
            Language::TypeScript => "typescript",
            Language::JavaScript => "javascript",
            Language::Go => "go",
            Language::Java => "java",
            Language::C => "c",
            Language::Cpp => "cpp",
            Language::Markdown => "markdown",
            Language::Json => "json",
            Language::Yaml => "yaml",
            Language::Toml => "toml",
            Language::Unknown => "unknown",
        }
    }
}
