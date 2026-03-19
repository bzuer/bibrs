use crate::source::*;
use bibrs_core::model::EntryType;
/// CrossRef API client.
pub struct CrossRef {
    client: reqwest::Client,
    mailto: String,
    base_url: String,
}

impl CrossRef {
    /// Creates a new CrossRef client.
    pub fn new(mailto: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            mailto: mailto.to_string(),
            base_url: "https://api.crossref.org".to_string(),
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

    fn build_headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        if !self.mailto.is_empty() {
            if let Ok(val) =
                reqwest::header::HeaderValue::from_str(&format!("mailto:{}", self.mailto))
            {
                headers.insert("User-Agent", val);
            }
        }
        headers
    }
}

impl BibSource for CrossRef {
    fn id(&self) -> &'static str {
        "crossref"
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, SourceError> {
        let url = if let Some(doi) = &query.doi {
            format!("{}/works/{}", self.base_url, doi)
        } else {
            let q = query
                .query
                .as_deref()
                .or(query.title.as_deref())
                .unwrap_or("");
            format!(
                "{}/works?query={}&rows={}",
                self.base_url,
                urlencoded(q),
                query.max_results
            )
        };

        tracing::info!(url = %url, "crossref query");

        let resp = self
            .client
            .get(&url)
            .headers(self.build_headers())
            .send()
            .await
            .map_err(|e| SourceError::Http(e.to_string()))?;

        let status = resp.status();
        if !status.is_success() {
            return Err(SourceError::Http(format!("status {}", status)));
        }

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| SourceError::Parse(e.to_string()))?;

        if query.doi.is_some() {
            let item = &body["message"];
            match parse_crossref_item(item) {
                Some(result) => Ok(vec![result]),
                None => Ok(Vec::new()),
            }
        } else {
            let items = body["message"]["items"]
                .as_array()
                .cloned()
                .unwrap_or_default();
            Ok(items.iter().filter_map(parse_crossref_item).collect())
        }
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

fn parse_crossref_item(item: &serde_json::Value) -> Option<SearchResult> {
    let doi = item["DOI"].as_str().unwrap_or("").to_string();
    let title = item["title"]
        .as_array()
        .and_then(|a| a.first())
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let authors = item["author"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|a| {
                    let family = a["family"].as_str()?;
                    let given = a["given"].as_str().unwrap_or("");
                    if given.is_empty() {
                        Some(family.to_string())
                    } else {
                        Some(format!("{}, {}", family, given))
                    }
                })
                .collect::<Vec<_>>()
                .join(" and ")
        })
        .unwrap_or_default();

    let journal = item["container-title"]
        .as_array()
        .and_then(|a| a.first())
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let year = item["published"]["date-parts"]
        .as_array()
        .and_then(|a| a.first())
        .and_then(|parts| parts.as_array())
        .and_then(|parts| parts.first())
        .and_then(|y| y.as_u64())
        .map(|y| y.to_string())
        .unwrap_or_default();

    let volume = item["volume"].as_str().unwrap_or("").to_string();
    let pages = item["page"].as_str().unwrap_or("").to_string();
    let issn = item["ISSN"]
        .as_array()
        .and_then(|a| a.first())
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let entry_type = match item["type"].as_str().unwrap_or("") {
        "journal-article" => EntryType::Article,
        "book" => EntryType::Book,
        "book-chapter" => EntryType::InBook,
        "proceedings-article" => EntryType::InProceedings,
        "report" => EntryType::Report,
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
        &cite_key,
        vec![
            ("author", authors),
            ("title", title),
            ("journal", journal),
            ("year", year),
            ("volume", volume),
            ("pages", pages),
            ("doi", doi.clone()),
            ("issn", issn),
        ],
    );

    Some(SearchResult {
        entry,
        source: "crossref".to_string(),
        relevance: item["score"].as_f64().unwrap_or(0.0),
    })
}

fn urlencoded(s: &str) -> String {
    s.replace(' ', "+")
        .replace('&', "%26")
        .replace('=', "%3D")
        .replace('?', "%3F")
}

