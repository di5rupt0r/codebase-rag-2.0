use codebase_rag::parser::{AstParser, Chunker, Language, SymbolKind};

#[test]
fn test_language_detection() {
    assert_eq!(Language::from_path("src/main.rs"), Language::Rust);
    assert_eq!(Language::from_path("app/server.py"), Language::Python);
    assert_eq!(Language::from_path("frontend/App.tsx"), Language::TypeScript);
    assert_eq!(Language::from_path("backend/handler.go"), Language::Go);
    assert_eq!(Language::from_path("README.md"), Language::Markdown);
    assert_eq!(Language::from_path("config.json"), Language::Json);
}

#[test]
fn test_rust_ast_parsing() {
    let rust_code = r#"
pub struct UserConfig {
    pub name: String,
    pub timeout: u64,
}

impl UserConfig {
    pub fn new(name: String) -> Self {
        Self { name, timeout: 30 }
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }
}

pub fn calculate_hash(data: &[u8]) -> u64 {
    42
}
"#;

    let symbols = AstParser::parse_symbols(rust_code, Language::Rust);
    assert!(!symbols.is_empty());

    let struct_sym = symbols.iter().find(|s| s.name == "UserConfig" && s.kind == SymbolKind::Struct);
    assert!(struct_sym.is_some(), "UserConfig struct must be found");

    let fn_sym = symbols.iter().find(|s| s.name == "calculate_hash");
    assert!(fn_sym.is_some(), "calculate_hash function must be found");
}

#[test]
fn test_python_ast_parsing() {
    let py_code = r#"
class DataProcessor:
    def __init__(self, name: str):
        self.name = name

    def process(self, items: list) -> list:
        return [item.strip() for item in items]

def fetch_records():
    return []
"#;

    let symbols = AstParser::parse_symbols(py_code, Language::Python);
    assert!(!symbols.is_empty());

    let class_sym = symbols.iter().find(|s| s.name == "DataProcessor" && s.kind == SymbolKind::Class);
    assert!(class_sym.is_some(), "DataProcessor class must be found");

    let method_sym = symbols.iter().find(|s| s.name == "process");
    assert!(method_sym.is_some(), "process method must be found");

    let fn_sym = symbols.iter().find(|s| s.name == "fetch_records");
    assert!(fn_sym.is_some(), "fetch_records function must be found");
}

#[test]
fn test_typescript_ast_parsing() {
    let ts_code = r#"
interface SessionPayload {
    userId: string;
    token: string;
}

export class AuthService {
    constructor(private secret: string) {}

    async validateSession(token: string): Promise<boolean> {
        return token.length > 0;
    }
}
"#;

    let symbols = AstParser::parse_symbols(ts_code, Language::TypeScript);
    assert!(!symbols.is_empty());

    let iface_sym = symbols.iter().find(|s| s.name == "SessionPayload");
    assert!(iface_sym.is_some(), "SessionPayload interface must be found");

    let class_sym = symbols.iter().find(|s| s.name == "AuthService");
    assert!(class_sym.is_some(), "AuthService class must be found");
}

#[test]
fn test_chunker_ast_and_fallback() {
    let chunker = Chunker::new(50, 10);
    let rust_code = r#"
pub fn greet(name: &str) -> String {
    format!("Hello, {}", name)
}
"#;
    let (chunks, symbols) = chunker.chunk_file("src/greet.rs", "src/greet.rs", rust_code);
    assert!(!chunks.is_empty());
    assert!(!symbols.is_empty());
    assert_eq!(chunks[0].relative_path, "src/greet.rs");
    assert_eq!(chunks[0].symbol_name, Some("greet".to_string()));
}
