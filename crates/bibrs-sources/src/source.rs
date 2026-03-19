use bibrs_core::model::{Entry, EntryType, FieldValue};
use indexmap::IndexMap;
use std::fmt;

/// Error from a bibliographic source.
#[derive(Debug)]
pub enum SourceError {
    Http(String),
    Parse(String),
    Cache(String),
}

impl fmt::Display for SourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Http(msg) => write!(f, "HTTP error: {}", msg),
            Self::Parse(msg) => write!(f, "parse error: {}", msg),
            Self::Cache(msg) => write!(f, "cache error: {}", msg),
        }
    }
}

impl std::error::Error for SourceError {}

/// Query parameters for searching external sources.
#[derive(Debug, Clone)]
pub struct SearchQuery {
    pub query: Option<String>,
    pub doi: Option<String>,
    pub isbn: Option<String>,
    pub author: Option<String>,
    pub title: Option<String>,
    pub max_results: usize,
}

impl Default for SearchQuery {
    fn default() -> Self {
        Self {
            query: None,
            doi: None,
            isbn: None,
            author: None,
            title: None,
            max_results: 10,
        }
    }
}

/// Result from an external source search.
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub entry: Entry,
    pub source: String,
    pub relevance: f64,
}

/// Trait for external bibliographic data sources.
#[allow(async_fn_in_trait)]
pub trait BibSource: Send + Sync {
    /// Source identifier.
    fn id(&self) -> &'static str;

    /// Searches the source with the given query.
    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, SourceError>;

    /// Fetches a single entry by identifier (DOI, ISBN, etc.).
    async fn fetch_by_id(&self, id: &str) -> Result<Option<SearchResult>, SourceError>;
}

/// Builds an Entry from common fields.
pub fn build_entry(
    entry_type: EntryType,
    cite_key: &str,
    fields: Vec<(&str, String)>,
) -> Entry {
    let mut map = IndexMap::new();
    for (k, v) in fields {
        if !v.is_empty() {
            map.insert(k.to_string(), FieldValue::Literal(v));
        }
    }
    Entry {
        entry_type,
        cite_key: cite_key.to_string(),
        fields: map,
        leading_comments: Vec::new(),
    }
}
