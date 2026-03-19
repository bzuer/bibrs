use crate::source::*;
use bibrs_core::model::EntryType;

/// Google Books API client.
pub struct GoogleBooks {
    client: reqwest::Client,
    base_url: String,
}

impl GoogleBooks {
    /// Creates a new Google Books client.
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: "https://www.googleapis.com/books/v1".to_string(),
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

impl Default for GoogleBooks {
    fn default() -> Self {
        Self::new()
    }
}

impl BibSource for GoogleBooks {
    fn id(&self) -> &'static str {
        "google_books"
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, SourceError> {
        let q = if let Some(isbn) = &query.isbn {
            format!("isbn:{}", isbn)
        } else if let Some(title) = &query.title {
            if let Some(author) = &query.author {
                format!("intitle:{}+inauthor:{}", urlencoded(title), urlencoded(author))
            } else {
                format!("intitle:{}", urlencoded(title))
            }
        } else if let Some(q) = &query.query {
            urlencoded(q)
        } else {
            return Ok(Vec::new());
        };

        let url = format!(
            "{}/volumes?q={}&maxResults={}",
            self.base_url, q, query.max_results
        );

        tracing::info!(url = %url, "google_books query");

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

        let items = body["items"].as_array().cloned().unwrap_or_default();
        Ok(items.iter().filter_map(parse_google_books_item).collect())
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

fn parse_google_books_item(item: &serde_json::Value) -> Option<SearchResult> {
    let info = &item["volumeInfo"];

    let title = info["title"].as_str().unwrap_or("").to_string();

    let authors = info["authors"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|a| a.as_str().map(|s| s.to_string()))
                .collect::<Vec<_>>()
                .join(" and ")
        })
        .unwrap_or_default();

    let publisher = info["publisher"].as_str().unwrap_or("").to_string();
    let year = info["publishedDate"]
        .as_str()
        .and_then(|d| d.get(..4))
        .unwrap_or("")
        .to_string();

    let isbn = info["industryIdentifiers"]
        .as_array()
        .and_then(|arr| {
            arr.iter()
                .find(|id| id["type"].as_str() == Some("ISBN_13"))
                .or_else(|| arr.iter().find(|id| id["type"].as_str() == Some("ISBN_10")))
        })
        .and_then(|id| id["identifier"].as_str())
        .unwrap_or("")
        .to_string();

    let cite_key = format!(
        "gbooks_{}",
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
            ("author", authors),
            ("title", title),
            ("publisher", publisher),
            ("year", year),
            ("isbn", isbn),
        ],
    );

    Some(SearchResult {
        entry,
        source: "google_books".to_string(),
        relevance: 0.0,
    })
}

fn urlencoded(s: &str) -> String {
    s.replace(' ', "+")
        .replace('&', "%26")
        .replace('=', "%3D")
}
