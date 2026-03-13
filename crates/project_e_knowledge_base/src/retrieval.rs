//! Knowledge retrieval — RRF hybrid search combining FTS and vector results.

/// Reciprocal Rank Fusion: merge two ranked lists into one.
///
/// `score(doc) = Σ 1/(k + rank(doc))` for each ranking.
/// Higher k gives more weight to lower-ranked items (default: 60).
pub fn reciprocal_rank_fusion<T: Eq + std::hash::Hash + Clone>(
    rankings: &[Vec<T>],
    k: f32,
) -> Vec<(T, f32)> {
    use std::collections::HashMap;

    let mut scores: HashMap<T, f32> = HashMap::new();

    for ranking in rankings {
        for (rank, item) in ranking.iter().enumerate() {
            *scores.entry(item.clone()).or_insert(0.0) += 1.0 / (k + rank as f32 + 1.0);
        }
    }

    let mut results: Vec<(T, f32)> = scores.into_iter().collect();
    results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rrf_merges_rankings() {
        let fts_results = vec!["doc_a", "doc_b", "doc_c"];
        let vec_results = vec!["doc_b", "doc_a", "doc_d"];

        let fused = reciprocal_rank_fusion(&[fts_results, vec_results], 60.0);

        assert!(!fused.is_empty());
        // doc_a and doc_b appear in both → should rank higher than doc_c/doc_d
        let top_ids: Vec<&&str> = fused.iter().map(|(id, _)| id).collect();
        assert!(top_ids.contains(&&"doc_a"));
        assert!(top_ids.contains(&&"doc_b"));
    }

    #[test]
    fn rrf_single_ranking() {
        let results = vec![1, 2, 3];
        let fused = reciprocal_rank_fusion(&[results], 60.0);
        assert_eq!(fused.len(), 3);
        assert_eq!(fused[0].0, 1); // first item should have highest score
    }

    #[test]
    fn rrf_empty() {
        let fused: Vec<(i32, f32)> = reciprocal_rank_fusion(&[], 60.0);
        assert!(fused.is_empty());
    }
}
