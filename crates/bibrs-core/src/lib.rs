/// Application configuration (INI-based).
pub mod config;
/// Encoding detection and conversion utilities.
pub mod encoding;
/// Error types for parsing and I/O.
pub mod error;
/// Core data model: entries, fields, bibliography.
pub mod model;
/// BibTeX parser built on nom combinators.
pub mod parser;
/// Bibliography serialization back to BibTeX format.
pub mod serializer;
