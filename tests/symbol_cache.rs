#[path = "support/result.rs"]
mod result_support;

use cortex_plugin_dev::symbol_cache::{SymbolCache, content_hash};
use cortex_plugin_dev::treesitter::SupportedLanguage;
use result_support::ResultTestExt;

#[test]
fn content_hash_deterministic() {
    let h1 = content_hash("fn main() {}");
    let h2 = content_hash("fn main() {}");
    assert_eq!(h1, h2);
}

#[test]
fn content_hash_differs() {
    let h1 = content_hash("fn main() {}");
    let h2 = content_hash("fn other() {}");
    assert_ne!(h1, h2);
}

#[test]
fn cache_roundtrip() {
    let cache = SymbolCache::open_in_memory().or_panic();
    let source = "fn hello() {}";
    let entry = cache
        .get_or_parse("test.rs", source, SupportedLanguage::Rust)
        .or_panic();
    assert!(!entry.symbols.is_empty());
    assert!(entry.imports.is_empty());
    assert_eq!(cache.count(), 1);

    let entry2 = cache
        .get_or_parse("test.rs", source, SupportedLanguage::Rust)
        .or_panic();
    assert_eq!(entry.symbols.len(), entry2.symbols.len());
}

#[test]
fn invalidate_removes() {
    let cache = SymbolCache::open_in_memory().or_panic();
    cache
        .get_or_parse("test.rs", "fn a() {}", SupportedLanguage::Rust)
        .or_panic();
    assert_eq!(cache.count(), 1);
    cache.invalidate("test.rs").or_panic();
    assert_eq!(cache.count(), 0);
}
