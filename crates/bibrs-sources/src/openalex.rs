use crate::source::*;
use bibrs_core::model::EntryType;

/// OpenAlex API client.
pub struct OpenAlex {
    client: reqwest::Client,
    mailto: String,
    base_url: String,
}

impl OpenAlex {
    /// Creates a new OpenAlex client.
    pub fn new(mailto: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            mailto: mailto.to_string(),
            base_url: "https://api.openalex.org".to_string(),
        }
    }

    /// Creates a client with a custom base URL (for testing).
    pub fn with_base_url(mailto: &str, base_url: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            mailto: mailto.to_string(),
            base_url: base_url.to_string(),
        }
    }
}

impl BibSource for OpenAlex {
    fn id(&self) -> &'static str {
        "openalex"
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, SourceError> {
        let filter = if let Some(doi) = &query.doi {
            format!("filter=doi:{}", doi)
        } else if let Some(title) = &query.title {
            format!("filter=title.search:{}", urlencoded(title))
        } else if let Some(q) = &query.query {
            format!("search={}", urlencoded(q))
        } else {
            return Ok(Vec::new());
        };

        let mailto_param = if !self.mailto.is_empty() {
            format!("&mailto={}", self.mailto)
        } else {
            String::new()
        };

        let url = format!(
            "{}/works?{}&per_page={}{}",
            self.base_url, filter, query.max_results, mailto_param
        );

        tracing::info!(url = %url, "openalex query");

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

        let results = body["results"]
            .as_array()
            .cloned()
            .unwrap_or_default();

        Ok(results.iter().filter_map(parse_openalex_item).collect())
    }

    async fn fetch_by_id(&self, id: &str) -> Result<Option<SearchResult>, SourceError> {
        let query = SearchQuery {
            doi: Some(id.to_string()),
            ..Default::default()
        };
        let results = self.search(&query).await?;
        Ok(results.into_iter().next())
    }
}

fn parse_openalex_item(item: &serde_json::Value) -> Option<SearchResult> {
    let doi = item["doi"]
        .as_str()
        .unwrap_or("")
        .strip_prefix("https://doi.org/")
        .unwrap_or(item["doi"].as_str().unwrap_or(""))
        .to_string();

    let title = item["title"].as_str().unwrap_or("").to_string();

    let authors = item["authorships"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|a| a["author"]["display_name"].as_str().map(|s| s.to_string()))
                .collect::<Vec<_>>()
                .join(" and ")
        })
        .unwrap_or_default();

    let journal = item["primary_location"]["source"]["display_name"]
        .as_str()
        .unwrap_or("")
        .to_string();

    let year = item["publication_year"]
        .as_u64()
        .map(|y| y.to_string())
        .unwrap_or_default();

    let entry_type = match item["type"].as_str().unwrap_or("") {
        "journal-article" | "article" => EntryType::Article,
        "book" => EntryType::Book,
        "book-chapter" => EntryType::InBook,
        "proceedings-article" => EntryType::InProceedings,
        "dataset" => EntryType::Dataset,
        other if !other.is_empty() => EntryType::Other(other.to_string()),
        _ => EntryType::Misc,
    };

    let cite_key = doi
        .replace("10.", "")
        .replace('/', "_")
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .collect::<String>();

    let entry = build_entry(
        entry_type,
        if cite_key.is_empty() { "openalex" } else { &cite_key },
        vec![
            ("author", authors),
            ("title", title),
            ("journal", journal),
            ("year", year),
            ("doi", doi),
        ],
    );

    Some(SearchResult {
        entry,
        source: "openalex".to_string(),
        relevance: 0.0,
    })
}

fn urlencoded(s: &str) -> String {
    s.replace(' ', "+")
        .replace('&', "%26")
        .replace('=', "%3D")
}
