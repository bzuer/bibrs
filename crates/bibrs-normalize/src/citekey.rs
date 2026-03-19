use crate::names;
use unicode_normalization::UnicodeNormalization;
use std::collections::HashSet;

/// Generates a cite key from an entry's fields.
///
/// Default pattern: `{auth}{year}{shorttitle}`.
/// - `{auth}`: primary author surname, lowercase, accents stripped.
/// - `{year}`: year field value.
/// - `{shorttitle}`: first significant title word (>3 chars), lowercase.
pub fn generate_cite_key(
    author: Option<&str>,
    year: Option<&str>,
    title: Option<&str>,
    pattern: &str,
    lowercase: bool,
) -> String {
    let auth = author
        .map(|a| {
            let authors = names::parse_authors(a);
            authors
                .first()
                .map(|n| strip_accents(&n.last))
                .unwrap_or_default()
        })
        .unwrap_or_default();

    let yr = year
        .and_then(crate::fields::normalize_year)
        .unwrap_or_default();

    let short_title = title
        .map(first_significant_word)
        .unwrap_or_default();

    let mut key = pattern.to_string();
    key = key.replace("{auth}", &auth);
    key = key.replace("{year}", &yr);
    key = key.replace("{shorttitle}", &short_title);

    if lowercase {
        key = key.to_lowercase();
    }

    key.chars()
        .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
        .collect()
}

/// Generates unique cite keys for a bibliography, appending suffixes on collision.
pub fn generate_unique_keys(
    keys: Vec<String>,
    suffix_mode: &str,
) -> Vec<String> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut result = Vec::with_capacity(keys.len());

    for key in keys {
        if !seen.contains(&key) {
            seen.insert(key.clone());
            result.push(key);
        } else {
            let mut suffix_idx = 0u32;
            loop {
                let suffixed = match suffix_mode {
                    "numeric" => format!("{}{}", key, suffix_idx + 1),
                    _ => format!(
                        "{}{}",
                        key,
                        (b'a' + (suffix_idx as u8 % 26)) as char
                    ),
                };
                if !seen.contains(&suffixed) {
                    seen.insert(suffixed.clone());
                    result.push(suffixed);
                    break;
                }
                suffix_idx += 1;
            }
        }
    }

    result
}

fn strip_accents(s: &str) -> String {
    s.nfd()
        .filter(|c| !unicode_normalization::char::is_combining_mark(*c))
        .collect()
}

const INSIGNIFICANT_WORDS: &[&str] = &[
    "a", "an", "the", "and", "or", "but", "in", "on", "at", "to", "for",
    "of", "with", "by", "from", "as",
];

fn first_significant_word(title: &str) -> String {
    title
        .split_whitespace()
        .find(|w| {
            let lower = w.to_lowercase();
            let clean: String = lower.chars().filter(|c| c.is_alphabetic()).collect();
            clean.len() > 3 && !INSIGNIFICANT_WORDS.contains(&clean.as_str())
        })
        .map(|w| {
            w.chars()
                .filter(|c| c.is_alphabetic())
                .collect::<String>()
                .to_lowercase()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_cite_key() {
        let key = generate_cite_key(
            Some("Silva, João"),
            Some("2023"),
            Some("A Novel Approach to Machine Learning"),
            "{auth}{year}{shorttitle}",
            true,
        );
        assert_eq!(key, "silva2023novel");
    }

    #[test]
    fn cite_key_with_accent() {
        let key = generate_cite_key(
            Some("Müller, Hans"),
            Some("2020"),
            Some("Deep Learning for NLP"),
            "{auth}{year}{shorttitle}",
            true,
        );
        assert_eq!(key, "muller2020deep");
    }

    #[test]
    fn cite_key_missing_fields() {
        let key = generate_cite_key(None, None, None, "{auth}{year}{shorttitle}", true);
        assert_eq!(key, "");
    }

    #[test]
    fn cite_key_von_particle() {
        let key = generate_cite_key(
            Some("von Neumann, John"),
            Some("1945"),
            Some("First Draft of a Report"),
            "{auth}{year}{shorttitle}",
            true,
        );
        assert_eq!(key, "neumann1945first");
    }

    #[test]
    fn unique_keys_alpha() {
        let keys = vec![
            "silva2023".into(),
            "silva2023".into(),
            "silva2023".into(),
        ];
        let result = generate_unique_keys(keys, "alpha");
        assert_eq!(result, vec!["silva2023", "silva2023a", "silva2023b"]);
    }

    #[test]
    fn unique_keys_numeric() {
        let keys = vec!["key".into(), "key".into()];
        let result = generate_unique_keys(keys, "numeric");
        assert_eq!(result, vec!["key", "key1"]);
    }
}
