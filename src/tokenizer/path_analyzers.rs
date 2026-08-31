//! Which analyzer cuts the text under a JSON path.
//!
//! A JSON field holds a whole document, and the paths inside it are not all
//! the same kind of text: one is a title in English, the next a Japanese
//! body, the next an identifier that must not be cut at all. The tokenizer
//! named in the field's options answers for the field as a whole; this says
//! what a single path inside it is cut with instead.
//!
//! Paths are written the way a mapping writes them -- `user.name` -- and are
//! translated to the separator the index uses when a segment writer takes its
//! copy. Nothing here is read while a document is being indexed: a writer
//! resolves every name once, when it is created.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use common::json_path_writer::JSON_PATH_SEGMENT_SEP;

/// The analyzer chosen for each path of each JSON field.
///
/// Cloning shares the same table, so a change made through one handle is seen
/// by the next segment writer created from any of them.
#[derive(Clone, Default)]
pub struct PathAnalyzerManager {
    by_field: Arc<RwLock<HashMap<String, HashMap<String, String>>>>,
}

impl PathAnalyzerManager {
    /// Cut `path` inside `field` with the tokenizer registered under
    /// `tokenizer_name`. Replaces any analyzer set for that path before.
    pub fn set(&self, field: &str, path: &str, tokenizer_name: &str) {
        self.by_field
            .write()
            .unwrap()
            .entry(field.to_string())
            .or_default()
            .insert(path.to_string(), tokenizer_name.to_string());
    }

    /// Forget every path of `field`; its paths fall back to the field's own
    /// tokenizer again.
    pub fn clear_field(&self, field: &str) {
        self.by_field.write().unwrap().remove(field);
    }

    /// Forget every field.
    pub fn clear(&self) {
        self.by_field.write().unwrap().clear();
    }

    /// The tokenizer named for one path, if there is one.
    pub fn get(&self, field: &str, path: &str) -> Option<String> {
        self.by_field.read().unwrap().get(field)?.get(path).cloned()
    }

    /// The paths of one field, keyed the way the index writes them, for a
    /// writer about to resolve them into analyzers.
    pub(crate) fn paths_of(&self, field: &str) -> Vec<(String, String)> {
        let by_field = self.by_field.read().unwrap();
        let Some(paths) = by_field.get(field) else {
            return Vec::new();
        };
        paths
            .iter()
            .map(|(path, tokenizer)| (indexed_path(path), tokenizer.clone()))
            .collect()
    }
}

/// A mapping writes `user.name`; the index writes the same path with its own
/// separator between the segments.
fn indexed_path(path: &str) -> String {
    path.replace('.', unsafe {
        std::str::from_utf8_unchecked(&[JSON_PATH_SEGMENT_SEP])
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_path_is_kept_per_field() {
        let manager = PathAnalyzerManager::default();
        manager.set("doc", "title", "en");
        manager.set("other", "title", "ja");
        assert_eq!(manager.get("doc", "title").as_deref(), Some("en"));
        assert_eq!(manager.get("other", "title").as_deref(), Some("ja"));
        assert_eq!(manager.get("doc", "body"), None);
    }

    #[test]
    fn a_writer_sees_the_path_the_index_spells() {
        let manager = PathAnalyzerManager::default();
        manager.set("doc", "user.name", "keyword");
        assert_eq!(
            manager.paths_of("doc"),
            vec![("user\u{1}name".to_string(), "keyword".to_string())]
        );
    }

    #[test]
    fn clearing_a_field_leaves_the_others() {
        let manager = PathAnalyzerManager::default();
        manager.set("doc", "title", "en");
        manager.set("other", "title", "ja");
        manager.clear_field("doc");
        assert!(manager.paths_of("doc").is_empty());
        assert_eq!(manager.get("other", "title").as_deref(), Some("ja"));
    }
}
