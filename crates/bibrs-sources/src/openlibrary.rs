use crate::source::*;
use bibrs_core::model::EntryType;

/// OpenLibrary API client.
pub struct OpenLibrary {
    client: reqwest::Client,
    base_url: String,
}

impl OpenLibrary {
    /// Creates a new OpenLibrary client.
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: "https://openlibrary.org".to_string(),
        }
    }

    /// Creates a client with a custom base URL (for testing).
    pub fn with_base_url(base_url: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.to_string(),
        }
    }
}

impl Default for OpenLibrary {
    fn default() -> Self {
        Self::new()
    }
}

impl BibSource for OpenLibrary {
    fn id(&self) -> &'static str {
        "openlibrary"
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, SourceError> {
        let url = if let Some(isbn) = &query.isbn {
            format!("{}/isbn/{}.json", self.base_url, isbn)
        } else {
            let q = query
                .query
                .as_deref()
                .or(query.title.as_deref())
                .unwrap_or("");
            format!(
                "{}/search.json?q={}&limit={}",
                self.base_url,
                urlencoded(q),
                query.max_results
            )
        };

        tracing::info!(url = %url, "openlibrary query");

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| SourceError::Http(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(SourceError::Http(format!("status {}", resp.status())));
        }

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| SourceError::Parse(e.to_string()))?;

        if query.isbn.is_some() {
            match parse_openlibrary_edition(&body) {
                Some(result) => Ok(vec![result]),
                None => Ok(Vec::new()),
            }
        } else {
            let docs = body["docs"].as_array().cloned().unwrap_or_default();
            Ok(docs.iter().filter_map(parse_openlibrary_doc).collect())
        }
    }

    async fn fetch_by_id(&self, isbn: &str) -> Result<Option<SearchResult>, SourceError> {
        let query = SearchQuery {
            isbn: Some(isbn.to_string()),
            max_results: 1,
            ..Default::default()
        };
        let results = self.search(&query).await?;
        Ok(results.into_iter().next())
    }
}

fn parse_openlibrary_doc(doc: &serde_json::Value) -> Option<SearchResult> {
    let title = doc["title"].as_str().unwrap_or("").to_string();

    let authors = doc["author_name"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|a| a.as_str().map(|s| s.to_string()))
                .collect::<Vec<_>>()
                .join(" and ")
        })
        .unwrap_or_default();

    let publisher = doc["publisher"]
        .as_array()
        .and_then(|a| a.first())
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let year = doc["first_publish_year"]
        .as_u64()
        .map(|y| y.to_string())
        .unwrap_or_default();

    let isbn = doc["isbn"]
        .as_array()
        .and_then(|a| a.first())
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let cite_key = format!("olib_{}", doc["key"].as_str().unwrap_or("unknown")
        .replace('/', "_"));

    let entry = build_entry(
        EntryType::Book,
        &cite_key,
        vec![
            ("author", authors),
            ("title", title),
            ("publisher", publisher),
            ("year", year),
            ("isbn", isbn),
        ],
    );

    Some(SearchResult {
        entry,
        source: "openlibrary".to_string(),
        relevance: 0.0,
    })
}

fn parse_openlibrary_edition(item: &serde_json::Value) -> Option<SearchResult> {
    let title = item["title"].as_str().unwrap_or("").to_string();

    let publishers = item["publishers"]
        .as_array()
        .and_then(|a| a.first())
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let year = item["publish_date"].as_str().unwrap_or("").to_string();

    let cite_key = format!(
        "olib_{}",
        title
            .split_whitespace()
            .next()
            .unwrap_or("unknown")
            .to_lowercase()
            .chars()
            .filter(|c| c.is_alphanumeric())
            .collect::<String>()
    );

    let entry = build_entry(
        EntryType::Book,
        &cite_key,
        vec![
            ("title", title),
            ("publisher", publishers),
            ("year", year),
        ],
    );

    Some(SearchResult {
        entry,
        source: "openlibrary".to_string(),
        relevance: 0.0,
    })
}

fn urlencoded(s: &str) -> String {
    s.replace(' ', "+")
        .replace('&', "%26")
        .replace('=', "%3D")
}
