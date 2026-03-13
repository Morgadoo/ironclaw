//! BM25 index for module discovery.
//!
//! Lightweight full-text index over module metadata. Built at the start
//! of each Interpret phase from the live registry. Allows the LLM-as-Judge
//! to find relevant modules by query without receiving the full catalog.

use std::collections::HashMap;

use uuid::Uuid;

use project_e_core::types::ModuleMetadata;

/// BM25 index over module metadata.
pub struct ModuleBm25Index {
    /// (module_id, tokens, doc_length)
    docs: Vec<(Uuid, Vec<String>, usize)>,
    /// Precomputed IDF values per term.
    idf_cache: HashMap<String, f32>,
    /// Average document length.
    avg_dl: f32,
    /// BM25 k1 parameter (saturation).
    k1: f32,
    /// BM25 b parameter (length normalization).
    b: f32,
}

impl ModuleBm25Index {
    /// Build an index from the current module registry state.
    pub fn build(modules: &[ModuleMetadata]) -> Self {
        let mut docs = Vec::with_capacity(modules.len());
        let mut df: HashMap<String, usize> = HashMap::new();

        for module in modules {
            let tokens = tokenize_module(module);
            let token_set: std::collections::HashSet<&str> =
                tokens.iter().map(|s| s.as_str()).collect();
            for token in &token_set {
                *df.entry(token.to_string()).or_insert(0) += 1;
            }
            let len = tokens.len();
            docs.push((module.id, tokens, len));
        }

        let n = docs.len() as f32;
        let avg_dl = if docs.is_empty() {
            1.0
        } else {
            docs.iter().map(|(_, _, len)| *len as f32).sum::<f32>() / n
        };

        let idf_cache: HashMap<String, f32> = df
            .iter()
            .map(|(term, &doc_freq)| {
                let idf = ((n - doc_freq as f32 + 0.5) / (doc_freq as f32 + 0.5) + 1.0).ln();
                (term.clone(), idf.max(0.0))
            })
            .collect();

        Self {
            docs,
            idf_cache,
            avg_dl,
            k1: 1.5,
            b: 0.75,
        }
    }

    /// Query the index and return top-k module IDs ranked by BM25 score.
    pub fn query(&self, q: &str, k: usize) -> Vec<(Uuid, f32)> {
        let query_tokens = tokenize_text(q);

        let mut scores: Vec<(Uuid, f32)> = self
            .docs
            .iter()
            .map(|(id, doc_tokens, doc_len)| {
                let score = self.bm25_score(&query_tokens, doc_tokens, *doc_len);
                (*id, score)
            })
            .filter(|(_, score)| *score > 0.0)
            .collect();

        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scores.truncate(k);
        scores
    }

    /// Number of documents in the index.
    pub fn doc_count(&self) -> usize {
        self.docs.len()
    }

    fn bm25_score(&self, query_tokens: &[String], doc_tokens: &[String], doc_len: usize) -> f32 {
        let dl = doc_len as f32;
        let mut score = 0.0;

        for qt in query_tokens {
            let idf = self.idf_cache.get(qt).copied().unwrap_or(0.0);
            let tf = doc_tokens.iter().filter(|t| t == &qt).count() as f32;
            let numerator = tf * (self.k1 + 1.0);
            let denominator = tf + self.k1 * (1.0 - self.b + self.b * dl / self.avg_dl);
            score += idf * numerator / denominator;
        }

        score
    }
}

/// Tokenize a module's metadata into searchable terms.
fn tokenize_module(module: &ModuleMetadata) -> Vec<String> {
    let mut text = String::new();
    text.push_str(&module.name);
    text.push(' ');
    text.push_str(&module.description);
    text.push(' ');
    text.push_str(&module.category);
    for tag in &module.tags {
        text.push(' ');
        text.push_str(tag);
    }
    tokenize_text(&text)
}

/// Tokenize text by whitespace, underscores, and hyphens; lowercase.
fn tokenize_text(text: &str) -> Vec<String> {
    text.to_ascii_lowercase()
        .split(|c: char| c.is_whitespace() || c == '_' || c == '-' || c == '.')
        .filter(|s| s.len() > 1) // skip single chars
        .map(|s| s.to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use project_e_core::types::ModuleState;

    fn make_module(name: &str, desc: &str, tags: &[&str]) -> ModuleMetadata {
        ModuleMetadata {
            id: Uuid::new_v4(),
            name: name.to_string(),
            version: "0.1.0".to_string(),
            description: desc.to_string(),
            category: "test".to_string(),
            tags: tags.iter().map(|s| s.to_string()).collect(),
            state: ModuleState::Active,
            subscriptions: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn build_index_from_modules() {
        let modules = vec![
            make_module(
                "energy-tariff",
                "Monitors energy tariff changes",
                &["energy", "tariff"],
            ),
            make_module("weather-forecast", "Provides weather data", &["weather"]),
            make_module(
                "market-prices",
                "OMIE market price tracker",
                &["energy", "market"],
            ),
        ];

        let index = ModuleBm25Index::build(&modules);
        assert_eq!(index.doc_count(), 3);
    }

    #[test]
    fn query_returns_relevant_results() {
        let modules = vec![
            make_module(
                "energy-tariff",
                "Monitors energy tariff changes",
                &["energy", "tariff"],
            ),
            make_module(
                "weather-forecast",
                "Provides weather forecasting data",
                &["weather"],
            ),
            make_module(
                "market-prices",
                "OMIE market price tracker",
                &["energy", "market"],
            ),
        ];

        let index = ModuleBm25Index::build(&modules);
        let results = index.query("energy tariff", 10);

        assert!(!results.is_empty());
        // The energy-tariff module should rank first
        assert_eq!(results[0].0, modules[0].id);
    }

    #[test]
    fn query_with_no_matches_returns_empty() {
        let modules = vec![make_module(
            "energy-tariff",
            "Monitors tariff changes",
            &["energy"],
        )];

        let index = ModuleBm25Index::build(&modules);
        let results = index.query("zzzzzzz", 10);
        assert!(results.is_empty());
    }

    #[test]
    fn empty_index_returns_empty() {
        let index = ModuleBm25Index::build(&[]);
        assert_eq!(index.doc_count(), 0);
        assert!(index.query("anything", 10).is_empty());
    }

    #[test]
    fn top_k_limits_results() {
        let modules: Vec<ModuleMetadata> = (0..20)
            .map(|i| make_module(&format!("module-{i}"), "energy related module", &["energy"]))
            .collect();

        let index = ModuleBm25Index::build(&modules);
        let results = index.query("energy", 5);
        assert!(results.len() <= 5);
    }
}
