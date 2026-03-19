use bibrs_sources::source::*;
use bibrs_sources::crossref::CrossRef;
use bibrs_sources::openalex::OpenAlex;
use wiremock::matchers::{method, path, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn crossref_fetch_by_doi() {
    let mock_server = MockServer::start().await;

    let body = include_str!("fixtures/api/crossref_work.json");

    Mock::given(method("GET"))
        .and(path_regex("/works/.*"))
        .respond_with(ResponseTemplate::new(200).set_body_string(body))
        .mount(&mock_server)
        .await;

    let client = CrossRef::with_base_url("test@example.com", &mock_server.uri());
    let result = client.fetch_by_id("10.1145/1327452.1327492").await.unwrap();

    assert!(result.is_some());
    let sr = result.unwrap();
    assert_eq!(sr.source, "crossref");
    assert_eq!(sr.entry.get_str("doi"), Some("10.1145/1327452.1327492"));
    assert!(sr.entry.get_str("author").unwrap().contains("Dean"));
    assert!(sr.entry.get_str("title").unwrap().contains("MapReduce"));
    assert_eq!(sr.entry.get_str("year"), Some("2008"));
    assert_eq!(sr.entry.get_str("volume"), Some("51"));
}

#[tokio::test]
async fn crossref_search() {
    let mock_server = MockServer::start().await;

    let body = r#"{
        "status": "ok",
        "message": {
            "items": [{
                "DOI": "10.1000/test",
                "type": "journal-article",
                "title": ["Test Article"],
                "author": [{"given": "John", "family": "Doe"}],
                "container-title": ["Test Journal"],
                "published": {"date-parts": [[2023]]},
                "score": 10.0
            }]
        }
    }"#;

    Mock::given(method("GET"))
        .and(path("/works"))
        .respond_with(ResponseTemplate::new(200).set_body_string(body))
        .mount(&mock_server)
        .await;

    let client = CrossRef::with_base_url("test@example.com", &mock_server.uri());
    let query = SearchQuery {
        query: Some("test".into()),
        ..Default::default()
    };
    let results = client.search(&query).await.unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].entry.get_str("doi"), Some("10.1000/test"));
}

#[tokio::test]
async fn openalex_search() {
    let mock_server = MockServer::start().await;

    let body = include_str!("fixtures/api/openalex_works.json");

    Mock::given(method("GET"))
        .and(path("/works"))
        .respond_with(ResponseTemplate::new(200).set_body_string(body))
        .mount(&mock_server)
        .await;

    let client = OpenAlex::with_base_url("test@example.com", &mock_server.uri());
    let query = SearchQuery {
        title: Some("Attention is all you need".into()),
        ..Default::default()
    };
    let results = client.search(&query).await.unwrap();

    assert_eq!(results.len(), 1);
    let entry = &results[0].entry;
    assert!(entry.get_str("title").unwrap().contains("Attention"));
    assert!(entry.get_str("author").unwrap().contains("Vaswani"));
    assert_eq!(entry.get_str("year"), Some("2017"));
}

#[tokio::test]
async fn crossref_handles_404() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&mock_server)
        .await;

    let client = CrossRef::with_base_url("test@example.com", &mock_server.uri());
    let result = client.fetch_by_id("10.1000/nonexistent").await;
    assert!(result.is_err());
}
