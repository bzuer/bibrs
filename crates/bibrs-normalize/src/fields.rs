/// Normalizes a DOI by stripping common prefixes.
///
/// Removes `https://doi.org/`, `http://doi.org/`, `https://dx.doi.org/`,
/// `doi:`, and surrounding whitespace. Result: `10.xxxx/yyyy`.
pub fn normalize_doi(doi: &str) -> String {
    let s = doi.trim();
    let s = s
        .strip_prefix("https://doi.org/")
        .or_else(|| s.strip_prefix("http://doi.org/"))
        .or_else(|| s.strip_prefix("https://dx.doi.org/"))
        .or_else(|| s.strip_prefix("http://dx.doi.org/"))
        .or_else(|| s.strip_prefix("doi:"))
        .or_else(|| s.strip_prefix("DOI:"))
        .unwrap_or(s);
    s.trim().to_string()
}

/// Normalizes page ranges to BibTeX double-dash format.
///
/// `15-20` -> `15--20`, `15 – 20` -> `15--20`, `15—20` -> `15--20`.
pub fn normalize_pages(pages: &str) -> String {
    let s = pages.trim();
    let s = s.replace(" -- ", "--");
    let s = s.replace(" – ", "--");
    let s = s.replace(" — ", "--");
    let s = s.replace('–', "--");
    let s = s.replace('—', "--");

    if s.contains("--") {
        return s;
    }

    s.replace('-', "--")
}

/// Validates and normalizes an ISSN (strips hyphens, validates check digit).
///
/// Returns `Some(normalized)` if valid, `None` if invalid.
pub fn validate_issn(issn: &str) -> Option<String> {
    let digits: Vec<char> = issn.chars().filter(|c| c.is_ascii_digit() || *c == 'X' || *c == 'x').collect();
    if digits.len() != 8 {
        return None;
    }

    let mut sum = 0u32;
    for (i, &ch) in digits[..7].iter().enumerate() {
        let d = ch.to_digit(10)?;
        sum += d * (8 - i as u32);
    }

    let check = digits[7];
    let expected_remainder = (11 - (sum % 11)) % 11;
    let check_val = if check == 'X' || check == 'x' {
        10
    } else {
        check.to_digit(10)?
    };

    if check_val == expected_remainder {
        let normalized: String = digits.iter().collect();
        Some(format!("{}-{}", &normalized[..4], &normalized[4..]))
    } else {
        None
    }
}

/// Validates and normalizes an ISBN (strips hyphens, validates check digit).
///
/// Handles both ISBN-10 (mod 11) and ISBN-13 (mod 10).
/// Returns `Some(normalized)` if valid, `None` if invalid.
pub fn validate_isbn(isbn: &str) -> Option<String> {
    let digits: Vec<char> = isbn.chars().filter(|c| c.is_ascii_digit() || *c == 'X' || *c == 'x').collect();

    match digits.len() {
        10 => validate_isbn10(&digits),
        13 => validate_isbn13(&digits),
        _ => None,
    }
}

fn validate_isbn10(digits: &[char]) -> Option<String> {
    let mut sum = 0u32;
    for (i, &ch) in digits[..9].iter().enumerate() {
        let d = ch.to_digit(10)?;
        sum += d * (10 - i as u32);
    }

    let check = digits[9];
    let expected = (11 - (sum % 11)) % 11;
    let check_val = if check == 'X' || check == 'x' {
        10
    } else {
        check.to_digit(10)?
    };

    if check_val == expected {
        Some(digits.iter().collect())
    } else {
        None
    }
}

fn validate_isbn13(digits: &[char]) -> Option<String> {
    let mut sum = 0u32;
    for (i, &ch) in digits[..12].iter().enumerate() {
        let d = ch.to_digit(10)?;
        let weight = if i % 2 == 0 { 1 } else { 3 };
        sum += d * weight;
    }

    let expected = (10 - (sum % 10)) % 10;
    let check_val = digits[12].to_digit(10)?;

    if check_val == expected {
        Some(digits.iter().collect())
    } else {
        None
    }
}

/// Extracts a 4-digit year from a string.
///
/// `"2023a"` -> `"2023"`, `"(2023)"` -> `"2023"`, `"circa 2020"` -> `"2020"`.
pub fn normalize_year(year: &str) -> Option<String> {
    let s = year.trim();
    for i in 0..s.len().saturating_sub(3) {
        let slice = &s[i..i + 4];
        if slice.chars().all(|c| c.is_ascii_digit()) {
            let n: u32 = slice.parse().ok()?;
            if (1000..=2100).contains(&n) {
                return Some(slice.to_string());
            }
        }
    }
    None
}

/// Normalizes a title: detects ALL CAPS and converts to title case.
///
/// Words already protected with `{...}` are preserved. Common short words
/// (articles, prepositions) are lowercased except at the start.
pub fn normalize_title(title: &str) -> String {
    let trimmed = title.trim();
    if !is_all_caps(trimmed) {
        return trimmed.to_string();
    }
    to_title_case(trimmed)
}

fn is_all_caps(s: &str) -> bool {
    let letters: Vec<char> = s.chars().filter(|c| c.is_alphabetic()).collect();
    if letters.is_empty() {
        return false;
    }
    let upper_count = letters.iter().filter(|c| c.is_uppercase()).count();
    upper_count as f64 / letters.len() as f64 > 0.8
}

const LOWERCASE_WORDS: &[&str] = &[
    "a", "an", "the", "and", "but", "or", "nor", "for", "yet", "so", "in",
    "on", "at", "to", "by", "of", "up", "as", "is", "if", "it", "with",
    "from", "into", "than", "that", "this", "over", "upon",
];

fn to_title_case(s: &str) -> String {
    let words: Vec<&str> = s.split_whitespace().collect();
    let mut result = Vec::new();

    for (i, word) in words.iter().enumerate() {
        if word.starts_with('{') && word.ends_with('}') {
            result.push(word.to_string());
            continue;
        }

        let lower = word.to_lowercase();
        if i > 0 && LOWERCASE_WORDS.contains(&lower.as_str()) {
            result.push(lower);
        } else {
            let mut chars = lower.chars();
            match chars.next() {
                Some(c) => {
                    let mut capitalized = c.to_uppercase().to_string();
                    capitalized.extend(chars);
                    result.push(capitalized);
                }
                None => result.push(String::new()),
            }
        }
    }

    result.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doi_strip_https() {
        assert_eq!(normalize_doi("https://doi.org/10.1000/xyz"), "10.1000/xyz");
    }

    #[test]
    fn doi_strip_http() {
        assert_eq!(normalize_doi("http://doi.org/10.1000/xyz"), "10.1000/xyz");
    }

    #[test]
    fn doi_strip_dx() {
        assert_eq!(
            normalize_doi("https://dx.doi.org/10.1000/xyz"),
            "10.1000/xyz"
        );
    }

    #[test]
    fn doi_strip_prefix_text() {
        assert_eq!(normalize_doi("doi:10.1000/xyz"), "10.1000/xyz");
        assert_eq!(normalize_doi("DOI:10.1000/xyz"), "10.1000/xyz");
    }

    #[test]
    fn doi_already_clean() {
        assert_eq!(normalize_doi("10.1000/xyz"), "10.1000/xyz");
    }

    #[test]
    fn doi_whitespace() {
        assert_eq!(normalize_doi("  10.1000/xyz  "), "10.1000/xyz");
    }

    #[test]
    fn pages_single_dash() {
        assert_eq!(normalize_pages("15-20"), "15--20");
    }

    #[test]
    fn pages_en_dash() {
        assert_eq!(normalize_pages("15–20"), "15--20");
    }

    #[test]
    fn pages_em_dash() {
        assert_eq!(normalize_pages("15—20"), "15--20");
    }

    #[test]
    fn pages_spaced_en_dash() {
        assert_eq!(normalize_pages("15 – 20"), "15--20");
    }

    #[test]
    fn pages_already_correct() {
        assert_eq!(normalize_pages("15--20"), "15--20");
    }

    #[test]
    fn issn_valid() {
        assert_eq!(validate_issn("0317-8471"), Some("0317-8471".into()));
    }

    #[test]
    fn issn_valid_no_hyphen() {
        assert_eq!(validate_issn("03178471"), Some("0317-8471".into()));
    }

    #[test]
    fn issn_with_x() {
        assert_eq!(validate_issn("0378-5955"), Some("0378-5955".into()));
    }

    #[test]
    fn issn_invalid() {
        assert_eq!(validate_issn("1234-5678"), None);
    }

    #[test]
    fn isbn13_valid() {
        assert_eq!(
            validate_isbn("978-0-201-63361-0"),
            Some("9780201633610".into())
        );
    }

    #[test]
    fn isbn10_valid() {
        assert_eq!(validate_isbn("0-201-63361-2"), Some("0201633612".into()));
    }

    #[test]
    fn isbn_invalid() {
        assert_eq!(validate_isbn("978-0-201-63361-9"), None);
    }

    #[test]
    fn year_simple() {
        assert_eq!(normalize_year("2023"), Some("2023".into()));
    }

    #[test]
    fn year_with_suffix() {
        assert_eq!(normalize_year("2023a"), Some("2023".into()));
    }

    #[test]
    fn year_in_parens() {
        assert_eq!(normalize_year("(2023)"), Some("2023".into()));
    }

    #[test]
    fn year_with_text() {
        assert_eq!(normalize_year("circa 2020"), Some("2020".into()));
    }

    #[test]
    fn year_invalid() {
        assert_eq!(normalize_year("no year"), None);
    }

    #[test]
    fn title_all_caps() {
        assert_eq!(
            normalize_title("A NOVEL APPROACH TO MACHINE LEARNING"),
            "A Novel Approach to Machine Learning"
        );
    }

    #[test]
    fn title_normal_case_unchanged() {
        let input = "A Novel Approach to Machine Learning";
        assert_eq!(normalize_title(input), input);
    }

    #[test]
    fn title_with_braces_preserved() {
        assert_eq!(
            normalize_title("ANALYSIS OF {DNA} SEQUENCES"),
            "Analysis of {DNA} Sequences"
        );
    }
}
