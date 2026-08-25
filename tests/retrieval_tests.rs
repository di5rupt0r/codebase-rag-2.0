use codebase_rag::retrieval::{
    reciprocal_rank_fusion, ContextPacker, RankedListCandidate, ScoredChunk,
};

#[test]
fn test_reciprocal_rank_fusion() {
    let bm25 = vec![
        RankedListCandidate {
            chunk_id: "a.rs:1-10".to_string(),
            relative_path: "a.rs".to_string(),
            language: "rust".to_string(),
            symbol_name: Some("foo".to_string()),
            symbol_kind: Some("function".to_string()),
            signature: Some("fn foo()".to_string()),
            content: "fn foo() {}".to_string(),
            line_start: 1,
            line_end: 10,
            raw_score: 5.0,
        },
        RankedListCandidate {
            chunk_id: "b.rs:1-10".to_string(),
            relative_path: "b.rs".to_string(),
            language: "rust".to_string(),
            symbol_name: Some("bar".to_string()),
            symbol_kind: Some("function".to_string()),
            signature: Some("fn bar()".to_string()),
            content: "fn bar() {}".to_string(),
            line_start: 1,
            line_end: 10,
            raw_score: 3.0,
        },
    ];

    let vector = vec![
        RankedListCandidate {
            chunk_id: "b.rs:1-10".to_string(),
            relative_path: "b.rs".to_string(),
            language: "rust".to_string(),
            symbol_name: Some("bar".to_string()),
            symbol_kind: Some("function".to_string()),
            signature: Some("fn bar()".to_string()),
            content: "fn bar() {}".to_string(),
            line_start: 1,
            line_end: 10,
            raw_score: 0.95,
        },
        RankedListCandidate {
            chunk_id: "a.rs:1-10".to_string(),
            relative_path: "a.rs".to_string(),
            language: "rust".to_string(),
            symbol_name: Some("foo".to_string()),
            symbol_kind: Some("function".to_string()),
            signature: Some("fn foo()".to_string()),
            content: "fn foo() {}".to_string(),
            line_start: 1,
            line_end: 10,
            raw_score: 0.80,
        },
    ];

    let symbol = vec![];

    let fused = reciprocal_rank_fusion(&bm25, 1.0, &vector, 1.0, &symbol, 1.0, 60, 5);

    assert_eq!(fused.len(), 2);
    assert!(fused[0].rrf_score > 0.0);
    assert!(fused[1].rrf_score > 0.0);
}

#[test]
fn test_context_packer() {
    let packer = ContextPacker::new(4000);
    let chunks = vec![ScoredChunk {
        chunk_id: "src/auth.rs:10-25".to_string(),
        relative_path: "src/auth.rs".to_string(),
        language: "rust".to_string(),
        symbol_name: Some("authenticate".to_string()),
        symbol_kind: Some("function".to_string()),
        signature: Some("pub fn authenticate(token: &str) -> bool".to_string()),
        content: "pub fn authenticate(token: &str) -> bool {\n    !token.is_empty()\n}".to_string(),
        line_start: 10,
        line_end: 25,
        rrf_score: 0.035,
        bm25_score: Some(4.2),
        vector_score: Some(0.88),
        symbol_score: None,
    }];

    let packed = packer.pack(&chunks);
    assert_eq!(packed.blocks.len(), 1);
    assert!(packed.formatted_text.contains("src/auth.rs:10-25"));
    assert!(packed.formatted_text.contains("authenticate"));
    assert!(packed.formatted_text.contains("<!-- CODEBASE RAG CONTEXT -->"));
}
