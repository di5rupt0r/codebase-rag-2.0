use super::languages::Language;
use serde::{Deserialize, Serialize};
use tree_sitter::{Language as TsLanguage, Node, Parser};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SymbolKind {
    Function,
    Method,
    Class,
    Struct,
    Enum,
    Interface,
    Trait,
    TypeAlias,
    Constant,
    Module,
    Import,
}

impl SymbolKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            SymbolKind::Function => "function",
            SymbolKind::Method => "method",
            SymbolKind::Class => "class",
            SymbolKind::Struct => "struct",
            SymbolKind::Enum => "enum",
            SymbolKind::Interface => "interface",
            SymbolKind::Trait => "trait",
            SymbolKind::TypeAlias => "type",
            SymbolKind::Constant => "constant",
            SymbolKind::Module => "module",
            SymbolKind::Import => "import",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolInfo {
    pub name: String,
    pub kind: SymbolKind,
    pub signature: String,
    pub docstring: Option<String>,
    pub parent_scope: Option<String>,
    pub line_start: usize,
    pub line_end: usize,
    pub byte_start: usize,
    pub byte_end: usize,
}

pub struct AstParser;

impl AstParser {
    pub fn get_tree_sitter_language(lang: Language) -> Option<TsLanguage> {
        match lang {
            Language::Rust => Some(tree_sitter_rust::language()),
            Language::Python => Some(tree_sitter_python::language()),
            Language::TypeScript => Some(tree_sitter_typescript::language_typescript()),
            Language::JavaScript => Some(tree_sitter_javascript::language()),
            Language::Go => Some(tree_sitter_go::language()),
            Language::Java => Some(tree_sitter_java::language()),
            Language::C => Some(tree_sitter_c::language()),
            Language::Cpp => Some(tree_sitter_cpp::language()),
            _ => None,
        }
    }

    pub fn parse_symbols(content: &str, lang: Language) -> Vec<SymbolInfo> {
        let ts_lang = match Self::get_tree_sitter_language(lang) {
            Some(l) => l,
            None => return Vec::new(),
        };

        let mut parser = Parser::new();
        if parser.set_language(&ts_lang).is_err() {
            return Vec::new();
        }

        let tree = match parser.parse(content, None) {
            Some(t) => t,
            None => return Vec::new(),
        };

        let mut symbols = Vec::new();
        let root_node = tree.root_node();
        Self::traverse_node(root_node, content, lang, None, &mut symbols);
        symbols
    }

    fn traverse_node(
        node: Node,
        content: &str,
        lang: Language,
        current_scope: Option<&str>,
        symbols: &mut Vec<SymbolInfo>,
    ) {
        let kind_str = node.kind();
        let (is_symbol, sym_kind, name_node) = match lang {
            Language::Rust => match kind_str {
                "function_item" => (true, SymbolKind::Function, node.child_by_field_name("name")),
                "struct_item" => (true, SymbolKind::Struct, node.child_by_field_name("name")),
                "enum_item" => (true, SymbolKind::Enum, node.child_by_field_name("name")),
                "trait_item" => (true, SymbolKind::Trait, node.child_by_field_name("name")),
                "impl_item" => {
                    let type_node = node.child_by_field_name("type");
                    (true, SymbolKind::Class, type_node)
                }
                "type_item" => (true, SymbolKind::TypeAlias, node.child_by_field_name("name")),
                "const_item" | "static_item" => (true, SymbolKind::Constant, node.child_by_field_name("name")),
                "mod_item" => (true, SymbolKind::Module, node.child_by_field_name("name")),
                "use_declaration" => (true, SymbolKind::Import, None),
                _ => (false, SymbolKind::Function, None),
            },
            Language::Python => match kind_str {
                "function_definition" => {
                    let sym_k = if current_scope.is_some() {
                        SymbolKind::Method
                    } else {
                        SymbolKind::Function
                    };
                    (true, sym_k, node.child_by_field_name("name"))
                }
                "class_definition" => (true, SymbolKind::Class, node.child_by_field_name("name")),
                "import_statement" | "import_from_statement" => (true, SymbolKind::Import, None),
                _ => (false, SymbolKind::Function, None),
            },
            Language::TypeScript | Language::JavaScript => match kind_str {
                "function_declaration" | "generator_function_declaration" => {
                    (true, SymbolKind::Function, node.child_by_field_name("name"))
                }
                "method_definition" => (true, SymbolKind::Method, node.child_by_field_name("name")),
                "class_declaration" => (true, SymbolKind::Class, node.child_by_field_name("name")),
                "interface_declaration" => (true, SymbolKind::Interface, node.child_by_field_name("name")),
                "type_alias_declaration" => (true, SymbolKind::TypeAlias, node.child_by_field_name("name")),
                "enum_declaration" => (true, SymbolKind::Enum, node.child_by_field_name("name")),
                "import_statement" => (true, SymbolKind::Import, None),
                _ => (false, SymbolKind::Function, None),
            },
            Language::Go => match kind_str {
                "function_declaration" => (true, SymbolKind::Function, node.child_by_field_name("name")),
                "method_declaration" => (true, SymbolKind::Method, node.child_by_field_name("name")),
                "type_declaration" => (true, SymbolKind::Struct, node.child_by_field_name("name")),
                "import_declaration" => (true, SymbolKind::Import, None),
                _ => (false, SymbolKind::Function, None),
            },
            Language::Java => match kind_str {
                "class_declaration" => (true, SymbolKind::Class, node.child_by_field_name("name")),
                "interface_declaration" => (true, SymbolKind::Interface, node.child_by_field_name("name")),
                "enum_declaration" => (true, SymbolKind::Enum, node.child_by_field_name("name")),
                "method_declaration" => (true, SymbolKind::Method, node.child_by_field_name("name")),
                "import_declaration" => (true, SymbolKind::Import, None),
                _ => (false, SymbolKind::Function, None),
            },
            Language::C | Language::Cpp => match kind_str {
                "function_definition" => (true, SymbolKind::Function, node.child_by_field_name("declarator")),
                "struct_specifier" => (true, SymbolKind::Struct, node.child_by_field_name("name")),
                "class_specifier" => (true, SymbolKind::Class, node.child_by_field_name("name")),
                "enum_specifier" => (true, SymbolKind::Enum, node.child_by_field_name("name")),
                "type_definition" => (true, SymbolKind::TypeAlias, node.child_by_field_name("declarator")),
                "preproc_include" => (true, SymbolKind::Import, None),
                _ => (false, SymbolKind::Function, None),
            },
            _ => (false, SymbolKind::Function, None),
        };

        let mut next_scope = current_scope.map(|s| s.to_string());

        if is_symbol {
            let name = if let Some(n) = name_node {
                content
                    .get(n.start_byte()..n.end_byte())
                    .unwrap_or("")
                    .trim()
                    .to_string()
            } else if sym_kind == SymbolKind::Import {
                content
                    .get(node.start_byte()..node.end_byte())
                    .unwrap_or("")
                    .trim()
                    .to_string()
            } else {
                kind_str.to_string()
            };

            let byte_start = node.start_byte();
            let byte_end = node.end_byte();
            let line_start = node.start_position().row + 1;
            let line_end = node.end_position().row + 1;

            // Extract first line or signature
            let node_text = content.get(byte_start..byte_end).unwrap_or("");
            let signature = node_text
                .lines()
                .next()
                .unwrap_or("")
                .trim()
                .to_string();

            if !name.is_empty() {
                symbols.push(SymbolInfo {
                    name: name.clone(),
                    kind: sym_kind,
                    signature,
                    docstring: None,
                    parent_scope: current_scope.map(|s| s.to_string()),
                    line_start,
                    line_end,
                    byte_start,
                    byte_end,
                });

                if matches!(sym_kind, SymbolKind::Class | SymbolKind::Struct | SymbolKind::Interface | SymbolKind::Trait | SymbolKind::Module) {
                    next_scope = Some(name);
                }
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            Self::traverse_node(child, content, lang, next_scope.as_deref(), symbols);
        }
    }
}
