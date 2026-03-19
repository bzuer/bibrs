use bibrs_core::encoding::detect_and_convert;
use bibrs_core::parser::Parser;
use bibrs_core::serializer::{serialize, SerializeConfig};

const REAL_BIB: &[u8] = include_bytes!("fixtures/real.bib");

#[test]
fn parse_real_file_no_panic() {
    let enc = detect_and_convert(REAL_BIB);
    let result = Parser::parse(&enc.content);
    assert!(
        result.bibliography.entries.len() > 5000,
        "expected >5000 entries, got {}",
        result.bibliography.entries.len()
    );
}

#[test]
fn parse_real_file_entry_count() {
    let enc = detect_and_convert(REAL_BIB);
    let result = Parser::parse(&enc.content);
    assert_eq!(result.bibliography.entries.len(), 5544);
}

#[test]
fn parse_real_file_zero_errors() {
    let enc = detect_and_convert(REAL_BIB);
    let result = Parser::parse(&enc.content);
    assert!(
        result.errors.is_empty(),
        "got {} parse errors: {:?}",
        result.errors.len(),
        result
            .errors
            .iter()
            .take(5)
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>()
    );
}

#[test]
fn parse_real_file_type_distribution() {
    let enc = detect_and_convert(REAL_BIB);
    let result = Parser::parse(&enc.content);
    let counts = result.bibliography.count_by_type();

    assert!(counts.get("article").copied().unwrap_or(0) > 3000);
    assert!(counts.get("book").copied().unwrap_or(0) > 1000);
    assert!(counts.get("incollection").copied().unwrap_or(0) > 500);
}

#[test]
fn roundtrip_real_file() {
    let enc = detect_and_convert(REAL_BIB);
    let r1 = Parser::parse(&enc.content);

    let serialized = serialize(&r1.bibliography, &SerializeConfig::default());
    let r2 = Parser::parse(&serialized);

    assert_eq!(
        r1.bibliography.entries.len(),
        r2.bibliography.entries.len(),
        "roundtrip lost entries: {} -> {}",
        r1.bibliography.entries.len(),
        r2.bibliography.entries.len()
    );

    for (a, b) in r1
        .bibliography
        .entries
        .iter()
        .zip(r2.bibliography.entries.iter())
    {
        assert_eq!(a.cite_key, b.cite_key, "cite key mismatch");
        assert_eq!(a.entry_type, b.entry_type, "type mismatch for {}", a.cite_key);
        assert_eq!(
            a.fields.len(),
            b.fields.len(),
            "field count mismatch for {}: {} vs {}",
            a.cite_key,
            a.fields.len(),
            b.fields.len()
        );
    }
}

#[test]
fn real_file_specific_entries() {
    let enc = detect_and_convert(REAL_BIB);
    let result = Parser::parse(&enc.content);
    let bib = &result.bibliography;

    let entry = bib.find_by_key("abbas2022").expect("entry abbas2022 not found");
    assert_eq!(entry.entry_type, bibrs_core::model::EntryType::Article);
    assert!(entry.get_str("title").unwrap().contains("Encanto"));
    assert_eq!(entry.get_str("year"), Some("2022"));
    assert!(entry.get_str("doi").unwrap().contains("10.21608"));

    let entry = bib
        .find_by_key("abraham1992")
        .expect("entry abraham1992 not found");
    assert_eq!(entry.entry_type, bibrs_core::model::EntryType::Book);
    assert_eq!(entry.get_str("isbn"), Some("0-252-01841-9"));

    let entry = bib
        .find_by_key("zuzanek2020")
        .expect("entry zuzanek2020 not found");
    assert_eq!(entry.entry_type, bibrs_core::model::EntryType::InCollection);
    assert!(entry.get_str("booktitle").is_some());
}

#[test]
fn real_file_encoding_is_utf8() {
    let enc = detect_and_convert(REAL_BIB);
    assert_eq!(enc.original, bibrs_core::encoding::DetectedEncoding::Utf8);
    assert!(enc.lossy.is_empty());
}
