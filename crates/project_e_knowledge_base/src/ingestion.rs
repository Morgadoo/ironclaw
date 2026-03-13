//! Knowledge ingestion — chunk, embed, and upsert scored data items.

use uuid::Uuid;

/// A text chunk ready for embedding and storage.
#[derive(Debug, Clone)]
pub struct TextChunk {
    pub id: Uuid,
    pub content: String,
    pub source_id: Uuid,
    pub chunk_index: usize,
    pub source_type: String,
}

/// Split text into overlapping chunks.
pub fn chunk_text(text: &str, chunk_size: usize, overlap: usize) -> Vec<String> {
    if text.is_empty() || chunk_size == 0 {
        return vec![];
    }

    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() {
        return vec![];
    }

    let mut chunks = Vec::new();
    let step = chunk_size.saturating_sub(overlap).max(1);
    let mut start = 0;

    while start < words.len() {
        let end = (start + chunk_size).min(words.len());
        let chunk = words[start..end].join(" ");
        chunks.push(chunk);

        if end >= words.len() {
            break;
        }
        start += step;
    }

    chunks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_empty_text() {
        assert!(chunk_text("", 100, 20).is_empty());
    }

    #[test]
    fn chunk_short_text() {
        let chunks = chunk_text("hello world", 100, 20);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0], "hello world");
    }

    #[test]
    fn chunk_with_overlap() {
        let text = (0..20)
            .map(|i| format!("word{i}"))
            .collect::<Vec<_>>()
            .join(" ");
        let chunks = chunk_text(&text, 10, 3);

        assert!(chunks.len() >= 2);
        // Verify overlap: last words of chunk 0 should appear in chunk 1
        let chunk0_words: Vec<&str> = chunks[0].split_whitespace().collect();
        let chunk1_words: Vec<&str> = chunks[1].split_whitespace().collect();
        // The last 3 words of chunk 0 should be the first 3 words of chunk 1
        assert_eq!(chunk0_words[7..10], chunk1_words[0..3]);
    }
}
