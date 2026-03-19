use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// BibTeX entry type classification.
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

/// Value of a BibTeX field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FieldValue {
    Literal(String),
    Integer(i64),
    StringRef(String),
    Concat(Vec<FieldValue>),
}

/// A single BibTeX entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    pub entry_type: EntryType,
    pub cite_key: String,
    pub fields: IndexMap<String, FieldValue>,
    pub leading_comments: Vec<String>,
}

/// A complete BibTeX bibliography.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Bibliography {
    pub preambles: Vec<String>,
    pub strings: IndexMap<String, String>,
    pub entries: Vec<Entry>,
    pub trailing_content: String,
}
