use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// BibTeX entry type classification.
///
/// Named variants for BibTeX + BibLaTeX spec types.
/// `Other(String)` never discards unknown types.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EntryType {
    Article,
    Book,
    Booklet,
    InBook,
    InCollection,
    InProceedings,
    Manual,
    MastersThesis,
    PhdThesis,
    Misc,
    Proceedings,
    TechReport,
    Unpublished,
    Online,
    Report,
    Thesis,
    Dataset,
    Software,
    Other(String),
}

impl EntryType {
    /// Case-insensitive parse. "ARTICLE" -> Article, "" -> Other("").
    pub fn parse(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "article" => Self::Article,
            "book" => Self::Book,
            "booklet" => Self::Booklet,
            "inbook" => Self::InBook,
            "incollection" => Self::InCollection,
            "inproceedings" | "conference" => Self::InProceedings,
            "manual" => Self::Manual,
            "mastersthesis" => Self::MastersThesis,
            "phdthesis" => Self::PhdThesis,
            "misc" => Self::Misc,
            "proceedings" => Self::Proceedings,
            "techreport" => Self::TechReport,
            "unpublished" => Self::Unpublished,
            "online" => Self::Online,
            "report" => Self::Report,
            "thesis" => Self::Thesis,
            "dataset" => Self::Dataset,
            "software" => Self::Software,
            other => Self::Other(other.to_string()),
        }
    }

    /// Canonical representation for serialization.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Article => "article",
            Self::Book => "book",
            Self::Booklet => "booklet",
            Self::InBook => "inbook",
            Self::InCollection => "incollection",
            Self::InProceedings => "inproceedings",
            Self::Manual => "manual",
            Self::MastersThesis => "mastersthesis",
            Self::PhdThesis => "phdthesis",
            Self::Misc => "misc",
            Self::Proceedings => "proceedings",
            Self::TechReport => "techreport",
            Self::Unpublished => "unpublished",
            Self::Online => "online",
            Self::Report => "report",
            Self::Thesis => "thesis",
            Self::Dataset => "dataset",
            Self::Software => "software",
            Self::Other(s) => s.as_str(),
        }
    }
}

/// BibTeX field value.
///
/// Preserves original structure for faithful roundtrip.
/// `Literal` = content between { } or " ".
/// `Integer` = unquoted number.
/// `StringRef` = reference to @string macro.
/// `Concat` = parts joined by #.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FieldValue {
    Literal(String),
    Integer(i64),
    StringRef(String),
    Concat(Vec<FieldValue>),
}

impl FieldValue {
    /// Resolves to final string, expanding @string references.
    pub fn resolve(&self, strings: &IndexMap<String, String>) -> String {
        match self {
            Self::Literal(s) => s.clone(),
            Self::Integer(n) => n.to_string(),
            Self::StringRef(key) => strings
                .get(key)
                .cloned()
                .unwrap_or_else(|| format!("{{{key}}}")),
            Self::Concat(parts) => parts.iter().map(|p| p.resolve(strings)).collect(),
        }
    }

    /// Faithful BibTeX representation (for roundtrip serialization).
    pub fn to_bibtex(&self) -> String {
        match self {
            Self::Literal(s) => format!("{{{s}}}"),
            Self::Integer(n) => n.to_string(),
            Self::StringRef(key) => key.clone(),
            Self::Concat(parts) => parts
                .iter()
                .map(|p| p.to_bibtex())
                .collect::<Vec<_>>()
                .join(" # "),
        }
    }
}

/// A single BibTeX entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    pub entry_type: EntryType,
    pub cite_key: String,
    /// Fields in original order (IndexMap).
    pub fields: IndexMap<String, FieldValue>,
    /// Comments/blank lines preceding this entry. Preserved for roundtrip.
    pub leading_comments: Vec<String>,
}

impl Entry {
    /// Resolved value of a field, or None.
    pub fn get_resolved(
        &self,
        field: &str,
        strings: &IndexMap<String, String>,
    ) -> Option<String> {
        self.fields.get(field).map(|v| v.resolve(strings))
    }

    /// Shortcut for simple literal field.
    pub fn get_str(&self, field: &str) -> Option<&str> {
        match self.fields.get(field) {
            Some(FieldValue::Literal(s)) => Some(s.as_str()),
            _ => None,
        }
    }
}

/// Complete content of a .bib file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bibliography {
    pub preambles: Vec<String>,
    pub strings: IndexMap<String, String>,
    pub entries: Vec<Entry>,
    pub trailing_content: String,
}

impl Bibliography {
    /// Creates an empty bibliography.
    pub fn new() -> Self {
        Self {
            preambles: Vec::new(),
            strings: IndexMap::new(),
            entries: Vec::new(),
            trailing_content: String::new(),
        }
    }

    /// Finds an entry by cite key.
    pub fn find_by_key(&self, key: &str) -> Option<&Entry> {
        self.entries.iter().find(|e| e.cite_key == key)
    }

    /// Finds an entry by cite key (mutable).
    pub fn find_by_key_mut(&mut self, key: &str) -> Option<&mut Entry> {
        self.entries.iter_mut().find(|e| e.cite_key == key)
    }

    /// Count entries by type.
    pub fn count_by_type(&self) -> IndexMap<&str, usize> {
        let mut counts = IndexMap::new();
        for e in &self.entries {
            *counts.entry(e.entry_type.as_str()).or_insert(0) += 1;
        }
        counts
    }
}

impl Default for Bibliography {
    fn default() -> Self {
        Self::new()
    }
}
