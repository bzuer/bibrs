use bibrs_core::model::Bibliography;
use std::collections::{HashMap, HashSet};

/// A group of duplicate entries.
#[derive(Debug, Clone)]
pub struct DuplicateGroup {
    /// Indices into the bibliography's entries vector.
    pub indices: Vec<usize>,
    /// Confidence level (0.0 to 1.0).
    pub confidence: f64,
    /// Reason for the match.
    pub reason: String,
}

/// Finds duplicate entries in a bibliography.
///
/// Two-layer strategy:
/// 1. Exact DOI match (confidence 1.0)
/// 2. Fuzzy normalized title match (configurable threshold)
pub fn find_duplicates(bib: &Bibliography, fuzzy_threshold: f64) -> Vec<DuplicateGroup> {
    let mut groups: Vec<DuplicateGroup> = Vec::new();
    let mut already_matched: HashSet<usize> = HashSet::new();

    find_doi_duplicates(bib, &mut groups, &mut already_matched);
    find_title_duplicates(bib, fuzzy_threshold, &mut groups, &mut already_matched);

    groups
}

fn find_doi_duplicates(
    bib: &Bibliography,
    groups: &mut Vec<DuplicateGroup>,
    already_matched: &mut HashSet<usize>,
) {
    let mut doi_map: HashMap<String, Vec<usize>> = HashMap::new();

    for (i, entry) in bib.entries.iter().enumerate() {
        if let Some(doi_val) = entry.get_str("doi") {
            let normalized = doi_val.trim().to_lowercase();
            if !normalized.is_empty() {
                doi_map.entry(normalized).or_default().push(i);
            }
        }
    }

    for (doi, indices) in doi_map {
        if indices.len() > 1 {
            for &idx in &indices {
                already_matched.insert(idx);
            }
            groups.push(DuplicateGroup {
                indices,
                confidence: 1.0,
                reason: format!("exact DOI match: {}", doi),
            });
        }
    }
}

fn find_title_duplicates(
    bib: &Bibliography,
    threshold: f64,
    groups: &mut Vec<DuplicateGroup>,
    already_matched: &mut HashSet<usize>,
) {
    let titles: Vec<(usize, HashSet<String>)> = bib
        .entries
        .iter()
        .enumerate()
        .filter(|(i, _)| !already_matched.contains(i))
        .filter_map(|(i, entry)| {
            entry.get_str("title").map(|t| {
                let normalized = normalize_for_comparison(t);
                let words: HashSet<String> =
                    normalized.split_whitespace().map(|w| w.to_string()).collect();
                (i, words)
            })
        })
        .filter(|(_, words)| !words.is_empty())
        .collect();

    let mut word_index: HashMap<String, Vec<usize>> = HashMap::new();
    for (pos, (_, words)) in titles.iter().enumerate() {
        for word in words {
            word_index.entry(word.clone()).or_default().push(pos);
        }
    }

    let mut candidate_pairs: HashSet<(usize, usize)> = HashSet::new();
    for positions in word_index.values() {
        if positions.len() > 100 {
            continue;
        }
        for i in 0..positions.len() {
            for j in (i + 1)..positions.len() {
                let a = positions[i].min(positions[j]);
                let b = positions[i].max(positions[j]);
                candidate_pairs.insert((a, b));
            }
        }
    }

    for (i, j) in candidate_pairs {
        let idx_i = titles[i].0;
        let idx_j = titles[j].0;
        if already_matched.contains(&idx_i) || already_matched.contains(&idx_j) {
            continue;
        }

        let set_a = &titles[i].1;
        let set_b = &titles[j].1;
        let intersection = set_a.intersection(set_b).count();
        let union = set_a.union(set_b).count();
        if union == 0 {
            continue;
        }
        let sim = intersection as f64 / union as f64;

        if sim >= threshold {
            already_matched.insert(idx_i);
            already_matched.insert(idx_j);
            groups.push(DuplicateGroup {
                indices: vec![idx_i, idx_j],
                confidence: sim,
                reason: "fuzzy title match".to_string(),
            });
        }
    }
}

fn normalize_for_comparison(title: &str) -> String {
    let lower = title.to_lowercase();
    lower
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .split_whitespace()
        .filter(|w| !is_stopword(w))
        .collect::<Vec<_>>()
        .join(" ")
}

const STOPWORDS: &[&str] = &[
    "a", "an", "the", "and", "or", "but", "in", "on", "at", "to", "for",
    "of", "with", "by", "from", "as", "is", "was", "are", "were", "be",
    "been", "being", "have", "has", "had", "do", "does", "did", "will",
    "would", "could", "should", "may", "might", "shall", "can", "this",
    "that", "these", "those", "it", "its",
];

fn is_stopword(word: &str) -> bool {
    STOPWORDS.contains(&word)
}

/// Jaccard similarity between two strings (word-level).
#[cfg(test)]
fn jaccard_similarity(a: &str, b: &str) -> f64 {
    let set_a: HashSet<&str> = a.split_whitespace().collect();
    let set_b: HashSet<&str> = b.split_whitespace().collect();

    if set_a.is_empty() && set_b.is_empty() {
        return 1.0;
    }

    let intersection = set_a.intersection(&set_b).count();
    let union = set_a.union(&set_b).count();

    if union == 0 {
        return 0.0;
    }

    intersection as f64 / union as f64
}

#[cfg(test)]
mod tests {
    use super::*;
    use bibrs_core::model::*;
    use indexmap::IndexMap;

    fn make_entry(key: &str, title: &str, doi: Option<&str>) -> Entry {
        let mut fields = IndexMap::new();
        fields.insert("title".to_string(), FieldValue::Literal(title.to_string()));
        if let Some(d) = doi {
            fields.insert("doi".to_string(), FieldValue::Literal(d.to_string()));
        }
        Entry {
            entry_type: EntryType::Article,
            cite_key: key.to_string(),
            fields,
            leading_comments: Vec::new(),
        }
    }

    #[test]
    fn doi_exact_duplicate() {
        let mut bib = Bibliography::new();
        bib.entries.push(make_entry("a", "Title A", Some("10.1000/xyz")));
        bib.entries.push(make_entry("b", "Title B", Some("10.1000/xyz")));
        bib.entries.push(make_entry("c", "Title C", Some("10.1000/abc")));

        let groups = find_duplicates(&bib, 0.90);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].indices.len(), 2);
        assert_eq!(groups[0].confidence, 1.0);
    }

    #[test]
    fn title_fuzzy_duplicate() {
        let mut bib = Bibliography::new();
        bib.entries.push(make_entry(
            "a",
            "A Novel Approach to Machine Learning",
            None,
        ));
        bib.entries.push(make_entry(
            "b",
            "A Novel Approach to Machine Learning Methods",
            None,
        ));
        bib.entries.push(make_entry("c", "Completely Different Topic", None));

        let groups = find_duplicates(&bib, 0.60);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].indices, vec![0, 1]);
    }

    #[test]
    fn no_duplicates() {
        let mut bib = Bibliography::new();
        bib.entries.push(make_entry("a", "Alpha", Some("10.1/a")));
        bib.entries.push(make_entry("b", "Beta", Some("10.1/b")));
        bib.entries.push(make_entry("c", "Gamma", Some("10.1/c")));

        let groups = find_duplicates(&bib, 0.90);
        assert!(groups.is_empty());
    }

    #[test]
    fn jaccard_identical() {
        assert_eq!(jaccard_similarity("hello world", "hello world"), 1.0);
    }

    #[test]
    fn jaccard_disjoint() {
        assert_eq!(jaccard_similarity("hello world", "foo bar"), 0.0);
    }
}
