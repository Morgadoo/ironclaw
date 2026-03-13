//! Project E Knowledge Base — hybrid FTS + pgvector ingestion and retrieval.

pub mod ingestion;
pub mod retrieval;

/// Default chunk size for text splitting.
pub const DEFAULT_CHUNK_SIZE: usize = 512;

/// Default overlap between consecutive chunks.
pub const DEFAULT_CHUNK_OVERLAP: usize = 64;
