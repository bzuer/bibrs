use bibrs_core::encoding::detect_and_convert;
use bibrs_core::parser::Parser;
use bibrs_normalize::dedup::find_duplicates;
use bibrs_normalize::fields;
use bibrs_normalize::names;

const REAL_BIB: &[u8] = include_bytes!("../../bibrs-core/tests/fixtures/real.bib");

#[test]
fn normalize_real_authors() {
    let enc = detect_and_convert(REAL_BIB);
    let result = Parser::parse(&enc.content);

    let mut parsed_count = 0;
    let mut failed = Vec::new();

    for entry in &result.bibliography.entries {
        if let Some(author_str) = entry.get_str("author") {
            let authors = names::parse_authors(author_str);
            for name in &authors {
                if name.last.is_empty() && !name.is_institutional {
                    failed.push((entry.cite_key.clone(), author_str.to_string()));
                }
            }
            parsed_count += authors.len();
        }
    }

    assert!(
        parsed_count > 5000,
        "expected >5000 author names, got {}",
        parsed_count
    );
    assert!(
        failed.len() < 20,
        "too many failures ({}/{}): {:?}",
        failed.len(),
        parsed_count,
        &failed[..failed.len().min(10)]
    );
}

#[test]
fn normalize_real_dois() {
    let enc = detect_and_convert(REAL_BIB);
    let result = Parser::parse(&enc.content);

    let mut doi_count = 0;
    for entry in &result.bibliography.entries {
        if let Some(doi) = entry.get_str("doi") {
            let normalized = fields::normalize_doi(doi);
            assert!(
                normalized.starts_with("10."),
                "DOI '{}' normalized to '{}' does not start with '10.'",
                doi,
                normalized
            );
            doi_count += 1;
        }
    }

    assert!(
        doi_count > 3000,
        "expected >3000 DOIs, got {}",
        doi_count
    );
}

#[test]
fn normalize_real_years() {
    let enc = detect_and_convert(REAL_BIB);
    let result = Parser::parse(&enc.content);

    let mut year_count = 0;
    let mut invalid = 0;
    for entry in &result.bibliography.entries {
        if let Some(year) = entry.get_str("year") {
            match fields::normalize_year(year) {
                Some(y) => {
                    assert!(y.len() == 4, "year '{}' normalized to '{}'", year, y);
                    year_count += 1;
                }
                None => invalid += 1,
            }
        }
    }

    assert!(
        year_count > 5000,
        "expected >5000 years, got {}",
        year_count
    );
    assert!(invalid < 50, "too many invalid years: {}", invalid);
}

#[test]
fn normalize_real_pages() {
    let enc = detect_and_convert(REAL_BIB);
    let result = Parser::parse(&enc.content);

    for entry in &result.bibliography.entries {
        if let Some(pages) = entry.get_str("pages") {
            let normalized = fields::normalize_pages(pages);
            assert!(
                !normalized.contains('–') && !normalized.contains('—'),
                "pages '{}' still has unicode dashes after normalization: '{}'",
                pages,
                normalized
            );
        }
    }
}

#[test]
fn dedup_real_file_finds_duplicates() {
    let enc = detect_and_convert(REAL_BIB);
    let result = Parser::parse(&enc.content);

    let groups = find_duplicates(&result.bibliography, 0.90);

    assert!(
        !groups.is_empty(),
        "expected some duplicates in a 5544-entry file"
    );
    assert!(
        groups.len() > 10,
        "expected >10 duplicate groups, got {}",
        groups.len()
    );
}

#[test]
fn dedup_real_file_performance() {
    let enc = detect_and_convert(REAL_BIB);
    let result = Parser::parse(&enc.content);

    let start = std::time::Instant::now();
    let _groups = find_duplicates(&result.bibliography, 0.90);
    let elapsed = start.elapsed();

    assert!(
        elapsed.as_secs() < 10,
        "dedup took too long: {:?}",
        elapsed
    );
}

#[test]
fn real_file_isbn_validation() {
    let enc = detect_and_convert(REAL_BIB);
    let result = Parser::parse(&enc.content);

    let mut total_isbn = 0;
    let mut valid = 0;
    for entry in &result.bibliography.entries {
        if let Some(isbn) = entry.get_str("isbn") {
            total_isbn += 1;
            if fields::validate_isbn(isbn).is_some() {
                valid += 1;
            }
        }
    }

    assert!(total_isbn > 500, "expected >500 ISBNs, got {}", total_isbn);
    assert!(
        valid as f64 / total_isbn as f64 > 0.5,
        "ISBN validation rate too low: {}/{}",
        valid,
        total_isbn
    );
}
