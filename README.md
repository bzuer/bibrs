# bibrs — Initial Planning

## Initial Scope

**bibrs** functions as a bibliographic manager for BibTeX files, built in Rust.

The problem to be solved: maintaining a clean, consistent, and usable bibliographic database is a repetitive and error-prone task. 

Existing tools such as JabRef suffer from chronic issues—corrupted encoding, excessive memory consumption, crashes—which transform the workflow into a sequence of frustrations. 

Bibliographic APIs (CrossRef, OpenAlex, Google Books, OpenLibrary) exist and are rich, but the path between finding a reference and having it cleanly standardized in the `.bib` file involves excessive manual steps.

The objective: a lightweight tool, accessed via a browser on a local port, unifying search, import, normalization, and maintenance of bibliographic references in a single, frictionless flow. 

The `.bib` file remains the native format—no mandatory intermediary database, no lock-in. Real files with issues (broken encoding, malformed fields, accumulated inconsistencies) are read, problems are reported, and correction mechanisms are provided.

A Rust binary is expected to open a `.bib` file, expose a local web interface for navigation, search, API imports, name and field normalization, duplicate detection, and saving—executing rapidly, with low resource consumption, and tolerance for imperfect data. 

The project is structured in layers to progressively exercise ownership, function composition, traits, async, and system architecture specific to the Rust language.

---

## Conceptualization

Three layers. Each must be complete, tested, and functional before the next is initiated.

```
Surface      Web GUI, integrated flow, user experience
Structure    External APIs, normalization, search, dedup
Foundation   Parser, model, serializer, encoding, CLI
```

---

# FOUNDATION

Objective: read any `.bib` file, report problems, rewrite cleanly. No network, no async, no heavy dependencies. Final product: a Rust library and CLI functioning as a `.bib` linter/formatter.

## File Structure

```
bibrs/
├── Cargo.toml
├── src/
│   ├── main.rs          # CLI: bibrs check|format|stats <file.bib>
│   ├── lib.rs           # public re-exports
│   ├── model.rs         # fundamental types
│   ├── encoding.rs      # charset detection and conversion
│   ├── parser.rs        # nom combinators: &str → Bibliography (tolerant)
│   ├── serializer.rs    # Bibliography → String
│   └── error.rs         # error hierarchy
└── tests/
    ├── roundtrip.rs     # parse → serialize → parse → assert_eq
    ├── recovery.rs      # malformed files
    ├── encoding.rs      # Latin-1, Windows-1252, BOM, mixed
    └── fixtures/        # real .bib files
```

When complexity justifies it, `parser.rs` migrates to `parser/mod.rs`, `parser/combinators.rs`, `parser/recovery.rs`. Not before.

## Cargo.toml

```toml
[package]
name = "bibrs"
version = "0.1.0"
edition = "2021"

[dependencies]
indexmap = { version = "2", features = ["serde"] }
serde = { version = "1", features = ["derive"] }
thiserror = "2"
encoding_rs = "0.8"
chardetng = "0.1"
unicode-normalization = "0.1"
clap = { version = "4", features = ["derive"] }
nom = "8"
```

Zero async dependencies. Zero network operations. The resulting binary is small and fast.

## model.rs — fundamental types

```rust
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Entry type.
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
    // BibLaTeX
    Online,
    Report,
    Thesis,
    Dataset,
    Software,
    // Fallback
    Other(String),
}

impl EntryType {
    /// Case-insensitive parse. "ARTICLE" → Article, "" → Other("").
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
/// `StringRef` = reference to @string{...}.
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

/// A complete BibTeX entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    pub entry_type: EntryType,
    pub cite_key: String,
    /// Fields in original order (IndexMap).
    pub fields: IndexMap<String, FieldValue>,
    /// Comments/blank lines preceding this entry.
    /// Preserved for roundtrip.
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
    pub fn new() -> Self {
        Self {
            preambles: Vec::new(),
            strings: IndexMap::new(),
            entries: Vec::new(),
            trailing_content: String::new(),
        }
    }

    pub fn find_by_key(&self, key: &str) -> Option<&Entry> {
        self.entries.iter().find(|e| e.cite_key == key)
    }

    pub fn find_by_key_mut(&mut self, key: &str) -> Option<&mut Entry> {
        self.entries.iter_mut().find(|e| e.cite_key == key)
    }

    /// Count by entry type.
    pub fn count_by_type(&self) -> IndexMap<&str, usize> {
        let mut counts = IndexMap::new();
        for e in &self.entries {
            *counts.entry(e.entry_type.as_str()).or_insert(0) += 1;
        }
        counts
    }
}
```

### Design Decisions

| Decision | Reason |
|---|---|
| `FieldValue` enum, not `String` | Preserves macros, concatenations, integers. Without this, roundtrip fails. |
| `IndexMap`, not `HashMap` | Preserves field and `@string` order. Diff-friendly. |
| `leading_comments` in `Entry` | Comments coupled to entries are not lost in roundtrip. |
| `Other(String)` in `EntryType` | Unknown types are never discarded. |
| `find_by_key` is O(n) | Acceptable up to ~10k entries. Auxiliary index postponed until measurable need arises. |

## error.rs — error hierarchy

```rust
use thiserror::Error;

/// Position in the source file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub offset: usize,
    pub line: usize,
    pub col: usize,
}

impl std::fmt::Display for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.line, self.col)
    }
}

/// Parse error kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseErrorKind {
    // Syntax
    UnterminatedString,
    UnterminatedBrace,
    InvalidCharacter(char),
    ExpectedCiteKey,
    ExpectedField,
    ExpectedEquals,
    ExpectedValue,
    MissingClosingDelimiter,
    DuplicateCiteKey(String),

    // Encoding
    InvalidUtf8,
    MixedEncoding,

    // Recovery
    SkippedContent(String),
}

/// Non-fatal error encountered during parsing.
/// The parser continues after registering the error.
#[derive(Debug, Clone)]
pub struct ParseError {
    pub kind: ParseErrorKind,
    pub span: Span,
    pub message: String,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.span, self.message)
    }
}

/// Parse result: extracted data + encountered errors.
/// The parser NEVER aborts entirely — always returns maximum extractable data.
#[derive(Debug, Clone)]
pub struct ParseResult {
    pub bibliography: crate::model::Bibliography,
    pub errors: Vec<ParseError>,
}

/// Fatal errors (I/O, unrecoverable encoding).
#[derive(Debug, Error)]
pub enum BibrsError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("unrecoverable encoding: {0}")]
    Encoding(String),
}
```

### Core Principle: The parser never aborts

`ParseResult` always contains a `Bibliography`—possibly empty, possibly incomplete, but never `Err`. Syntactic errors are registered in `errors` and parsing continues. This represents the fundamental difference relative to `typst/biblatex`, whose `Bibliography::parse()` returns `Result` and fails on the first severe error.

The separation between `ParseError` (non-fatal, collectible) and `BibrsError` (fatal: inaccessible disk, entirely illegible encoding) is intentional. Only I/O and unrecoverable encoding are fatal.

## encoding.rs — detection and conversion

```rust
use encoding_rs::*;
use chardetng::EncodingDetector;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DetectedEncoding {
    Utf8,
    Latin1,
    Windows1252,
    Other(String),
}

/// Detection + conversion result.
pub struct EncodingResult {
    /// Content converted to UTF-8.
    pub content: String,
    /// Original detected encoding.
    pub original: DetectedEncoding,
    /// Presence of BOM.
    pub had_bom: bool,
    /// Bytes that did not convert cleanly (position, original bytes).
    pub lossy: Vec<(usize, Vec<u8>)>,
}

/// Detects encoding and converts to UTF-8.
///
/// Pipeline:
/// 1. Check BOM (UTF-8, UTF-16).
/// 2. Attempt strict UTF-8 decode.
/// 3. If failed, use chardetng to detect probable encoding.
/// 4. Convert with encoding_rs.
/// 5. Register lossy conversions without aborting.
pub fn detect_and_convert(bytes: &[u8]) -> EncodingResult {
    // 1. BOM check
    let (bytes, had_bom) = strip_utf8_bom(bytes);

    // 2. Strict UTF-8
    if let Ok(s) = std::str::from_utf8(bytes) {
        return EncodingResult {
            content: s.to_string(),
            original: DetectedEncoding::Utf8,
            had_bom,
            lossy: Vec::new(),
        };
    }

    // 3. Detection
    let mut detector = EncodingDetector::new();
    detector.feed(bytes, true);
    let encoding = detector.guess(None, true);

    // 4. Conversion
    let (cow, _actual_encoding, had_errors) = encoding.decode(bytes);

    let lossy = if had_errors {
        find_lossy_positions(bytes, encoding)
    } else {
        Vec::new()
    };

    let detected = match encoding.name() {
        "windows-1252" => DetectedEncoding::Windows1252,
        "ISO-8859-1" => DetectedEncoding::Latin1,
        "UTF-8" => DetectedEncoding::Utf8,
        other => DetectedEncoding::Other(other.to_string()),
    };

    EncodingResult {
        content: cow.into_owned(),
        original: detected,
        had_bom,
        lossy,
    }
}

fn strip_utf8_bom(bytes: &[u8]) -> (&[u8], bool) {
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        (&bytes[3..], true)
    } else {
        (bytes, false)
    }
}

fn find_lossy_positions(
    bytes: &[u8],
    encoding: &'static encoding_rs::Encoding,
) -> Vec<(usize, Vec<u8>)> {
    // Implementation: iterate byte by byte, attempt decoding,
    // register positions where substitution occurred.
    // Implementation details depend on the specific encoding.
    let _ = (bytes, encoding);
    Vec::new() // stub — implement per encoding
}
```

## parser.rs — nom combinators: &str → Bibliography

The parser utilizes `nom` to operate directly on `&str`. Without a separate tokenization phase—`nom` combines lexing and parsing in a single pass, composing small functions (combinators) into larger parsers.

### BibTeX grammar in combinators

```
file         = (junk | item)*
item         = '@' (string_def | preamble | comment | entry)
entry        = type delim_open cite_key ',' fields delim_close
fields       = (field (',' field)*)? ','?
field        = identifier '=' value
value        = single_value ('#' single_value)*
single_value = braced_content | quoted_content | number | identifier
string_def   = 'string' delim_open identifier '=' value delim_close
preamble     = 'preamble' delim_open braced_content delim_close
comment      = 'comment' delim_open ... delim_close
junk         = [^@]+
```

Each grammar rule is a `nom` function receiving `&str` and returning `IResult<&str, T>`.

### Code

```rust
use nom::{
    branch::alt,
    bytes::complete::{tag, take_until, take_while, take_while1},
    character::complete::{char, digit1, multispace0, none_of, satisfy},
    combinator::{map, map_res, opt, recognize, value},
    multi::{many0, separated_list0},
    sequence::{delimited, preceded, separated_pair, terminated, tuple},
    IResult,
};

use crate::error::*;
use crate::model::*;
use indexmap::IndexMap;

// ---------------------------------------------------------------------------
// Primitives
// ---------------------------------------------------------------------------

/// Consumes whitespace (including % comments until end of line).
fn ws(input: &str) -> IResult<&str, ()> {
    let (input, _) = multispace0(input)?;
    // % comments until EOL
    if input.starts_with('%') {
        let end = input.find('\n').unwrap_or(input.len());
        let (input, _) = multispace0(&input[end..])?;
        ws(input) // recursive for multiple comment lines
    } else {
        Ok((input, ()))
    }
}

/// Identifier: letters, digits, _, -, ., :
fn identifier(input: &str) -> IResult<&str, &str> {
    take_while1(|c: char| c.is_alphanumeric() || "_-.:".contains(c))(input)
}

/// Integer number.
fn number(input: &str) -> IResult<&str, i64> {
    map_res(digit1, |s: &str| s.parse::<i64>())(input)
}

// ---------------------------------------------------------------------------
// Braced and quoted content
// ---------------------------------------------------------------------------

/// Content between { }, respecting nesting.
/// Returns internal content without external braces.
fn braced_content(input: &str) -> IResult<&str, String> {
    let (input, _) = char('{')(input)?;
    let mut depth = 1u32;
    let mut pos = 0;

    let bytes = input.as_bytes();
    while pos < bytes.len() && depth > 0 {
        match bytes[pos] {
            b'{' => depth += 1,
            b'}' => depth -= 1,
            b'\\' if pos + 1 < bytes.len() => pos += 1, // skip escaped char
            _ => {}
        }
        if depth > 0 {
            pos += 1;
        }
    }

    if depth > 0 {
        // Unclosed brace — return available data (tolerance)
        let content = input[..pos].to_string();
        Ok((&input[pos..], content))
    } else {
        let content = input[..pos].to_string();
        Ok((&input[pos + 1..], content)) // +1 to consume final }
    }
}

/// Content between " ", respecting internal braces.
fn quoted_content(input: &str) -> IResult<&str, String> {
    let (input, _) = char('"')(input)?;
    let mut depth = 0u32;
    let mut pos = 0;

    let bytes = input.as_bytes();
    while pos < bytes.len() {
        match bytes[pos] {
            b'{' => depth += 1,
            b'}' if depth > 0 => depth -= 1,
            b'"' if depth == 0 => break,
            b'\\' if pos + 1 < bytes.len() => pos += 1,
            _ => {}
        }
        pos += 1;
    }

    let content = input[..pos].to_string();
    if pos < bytes.len() {
        Ok((&input[pos + 1..], content)) // +1 to consume final "
    } else {
        // Unterminated quoted string — tolerance
        Ok((&input[pos..], content))
    }
}

// ---------------------------------------------------------------------------
// Field values
// ---------------------------------------------------------------------------

/// Atomic value: braced, quoted, number, or reference to @string.
fn single_value(input: &str) -> IResult<&str, FieldValue> {
    let (input, _) = ws(input)?;
    alt((
        map(braced_content, FieldValue::Literal),
        map(quoted_content, FieldValue::Literal),
        map(number, FieldValue::Integer),
        map(identifier, |s: &str| FieldValue::StringRef(s.to_string())),
    ))(input)
}

/// Value possibly concatenated with #.
fn field_value(input: &str) -> IResult<&str, FieldValue> {
    let (input, first) = single_value(input)?;
    let (input, _) = ws(input)?;

    // Attempt to read concatenations
    let mut parts = vec![first];
    let mut remaining = input;

    loop {
        let (inp, _) = ws(remaining)?;
        if inp.starts_with('#') {
            let (inp, _) = char('#')(inp)?;
            let (inp, _) = ws(inp)?;
            match single_value(inp) {
                Ok((inp, val)) => {
                    parts.push(val);
                    remaining = inp;
                }
                Err(_) => break,
            }
        } else {
            break;
        }
    }

    if parts.len() == 1 {
        Ok((remaining, parts.into_iter().next().unwrap()))
    } else {
        Ok((remaining, FieldValue::Concat(parts)))
    }
}

// ---------------------------------------------------------------------------
// Field: name = value
// ---------------------------------------------------------------------------

fn field(input: &str) -> IResult<&str, (String, FieldValue)> {
    let (input, _) = ws(input)?;
    let (input, name) = identifier(input)?;
    let (input, _) = ws(input)?;
    let (input, _) = char('=')(input)?;
    let (input, _) = ws(input)?;
    let (input, val) = field_value(input)?;
    Ok((input, (name.to_ascii_lowercase(), val)))
}

/// Comma-separated field list, with optional trailing comma.
fn field_list(input: &str) -> IResult<&str, IndexMap<String, FieldValue>> {
    let mut fields = IndexMap::new();
    let mut remaining = input;

    loop {
        let (inp, _) = ws(remaining)?;
        // Attempt to parse a field
        match field(inp) {
            Ok((inp, (name, val))) => {
                fields.insert(name, val);
                let (inp, _) = ws(inp)?;
                // Optional comma
                if inp.starts_with(',') {
                    remaining = &inp[1..];
                } else {
                    remaining = inp;
                    break;
                }
            }
            Err(_) => {
                remaining = inp;
                break;
            }
        }
    }

    Ok((remaining, fields))
}

// ---------------------------------------------------------------------------
// Entries and special definitions
// ---------------------------------------------------------------------------

/// Delimiters: { } or ( )
fn delimited_body<'a>(input: &'a str) -> IResult<&'a str, &'a str> {
    let (input, _) = ws(input)?;
    let (open_char, close_char) = if input.starts_with('{') {
        ('{', '}')
    } else if input.starts_with('(') {
        ('(', ')')
    } else {
        return Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Tag,
        )));
    };
    let (input, _) = char(open_char)(input)?;

    // Find corresponding closing delimiter (with depth)
    let mut depth = 1u32;
    let mut pos = 0;
    let bytes = input.as_bytes();
    while pos < bytes.len() && depth > 0 {
        match bytes[pos] as char {
            c if c == open_char => depth += 1,
            c if c == close_char => depth -= 1,
            _ => {}
        }
        if depth > 0 {
            pos += 1;
        }
    }

    let body = &input[..pos];
    let rest = if pos < bytes.len() { &input[pos + 1..] } else { &input[pos..] };
    Ok((rest, body))
}

fn parse_entry(entry_type: &str, input: &str) -> IResult<&str, Entry> {
    let (input, _) = ws(input)?;
    let (open_char, close_char) = if input.starts_with('{') {
        ('{', '}')
    } else if input.starts_with('(') {
        ('(', ')')
    } else {
        return Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Tag,
        )));
    };
    let (input, _) = char(open_char)(input)?;
    let (input, _) = ws(input)?;

    // Cite key: identifier (may contain /)
    let (input, cite_key) = take_while1(|c: char| {
        c.is_alphanumeric() || "_-.:/@+".contains(c)
    })(input)?;
    let (input, _) = ws(input)?;
    let (input, _) = opt(char(','))(input)?;

    // Fields
    let (input, fields) = field_list(input)?;
    let (input, _) = ws(input)?;

    // Closing delimiter (tolerant — may be absent)
    let input = if input.starts_with(close_char) {
        &input[close_char.len_utf8()..]
    } else {
        input
    };

    Ok((input, Entry {
        entry_type: EntryType::parse(entry_type),
        cite_key: cite_key.to_string(),
        fields,
        leading_comments: Vec::new(),
    }))
}

fn parse_string_def(input: &str) -> IResult<&str, (String, String)> {
    let (rest, body) = delimited_body(input)?;
    // Inside body: key = value
    let (_, _) = ws(body)?;
    match field(body) {
        Ok((_, (key, val))) => {
            // Resolves to simple string (strings cannot reference others at parse time)
            let resolved = match val {
                FieldValue::Literal(s) => s,
                FieldValue::Integer(n) => n.to_string(),
                other => other.to_bibtex(),
            };
            Ok((rest, (key, resolved)))
        }
        Err(e) => Err(e),
    }
}

fn parse_preamble(input: &str) -> IResult<&str, String> {
    let (rest, body) = delimited_body(input)?;
    Ok((rest, body.to_string()))
}

fn parse_comment_entry(input: &str) -> IResult<&str, ()> {
    let (rest, _) = delimited_body(input)?;
    Ok((rest, ()))
}

// ---------------------------------------------------------------------------
// Top-level: complete file
// ---------------------------------------------------------------------------

/// Main parser. Operates with tolerance: errors are collected, not propagated.
pub struct Parser;

impl Parser {
    pub fn parse(input: &str) -> ParseResult {
        let mut bib = Bibliography::new();
        let mut errors = Vec::new();
        let mut remaining = input;

        while !remaining.is_empty() {
            // Consume whitespace and junk until next @
            match remaining.find('@') {
                Some(pos) => {
                    remaining = &remaining[pos..];
                }
                None => break, // no remaining @ — end
            }

            // Consume @
            remaining = &remaining[1..];

            // Read type
            let (inp, _) = match ws(remaining) {
                Ok(r) => r,
                Err(_) => break,
            };

            let type_str = match identifier(inp) {
                Ok((inp_after, t)) => {
                    remaining = inp_after;
                    t
                }
                Err(_) => {
                    errors.push(ParseError {
                        kind: ParseErrorKind::ExpectedField,
                        span: Self::span_at(input, remaining),
                        message: "expected entry type after @".into(),
                    });
                    remaining = inp;
                    continue;
                }
            };

            match type_str.to_ascii_lowercase().as_str() {
                "string" => {
                    match parse_string_def(remaining) {
                        Ok((inp, (key, val))) => {
                            bib.strings.insert(key, val);
                            remaining = inp;
                        }
                        Err(_) => {
                            errors.push(ParseError {
                                kind: ParseErrorKind::ExpectedValue,
                                span: Self::span_at(input, remaining),
                                message: "malformed @string".into(),
                            });
                            Self::skip_to_next_at(&mut remaining);
                        }
                    }
                }
                "preamble" => {
                    match parse_preamble(remaining) {
                        Ok((inp, content)) => {
                            bib.preambles.push(content);
                            remaining = inp;
                        }
                        Err(_) => {
                            Self::skip_to_next_at(&mut remaining);
                        }
                    }
                }
                "comment" => {
                    match parse_comment_entry(remaining) {
                        Ok((inp, _)) => remaining = inp,
                        Err(_) => Self::skip_to_next_at(&mut remaining),
                    }
                }
                _ => {
                    match parse_entry(type_str, remaining) {
                        Ok((inp, entry)) => {
                            bib.entries.push(entry);
                            remaining = inp;
                        }
                        Err(_) => {
                            errors.push(ParseError {
                                kind: ParseErrorKind::MissingClosingDelimiter,
                                span: Self::span_at(input, remaining),
                                message: format!("malformed @{} entry", type_str),
                            });
                            Self::skip_to_next_at(&mut remaining);
                        }
                    }
                }
            }
        }

        ParseResult {
            bibliography: bib,
            errors,
        }
    }

    /// Advances to next @ (top-level recovery).
    fn skip_to_next_at(remaining: &mut &str) {
        match remaining[1..].find('@') {
            Some(pos) => *remaining = &remaining[pos + 1..],
            None => *remaining = "",
        }
    }

    /// Calculates Span from current position in original input.
    fn span_at(original: &str, current: &str) -> Span {
        let offset = original.len() - current.len();
        let consumed = &original[..offset];
        let line = consumed.matches('\n').count() + 1;
        let col = consumed.rfind('\n')
            .map(|pos| offset - pos)
            .unwrap_or(offset + 1);
        Span { offset, line, col }
    }
}
```

### Rust paradigms applied via nom

1. **Function composition.** Each combinator (`braced_content`, `field_value`, `field`, `field_list`) is a pure function receiving `&str` and returning `IResult`. Grammar is constructed via composition—the entry parser utilizes the field parser, which utilizes the value parser. This pattern is central to idiomatic Rust.
2. **Zero-copy by default.** Combinators such as `identifier` return `&str`—references to the original input, sans allocation. `String` is instantiated only when required (braced/quoted content requiring ownership in the model). The transition between `&str` and `String` serves as a natural exercise in ownership mechanics.
3. **Closures as parsers.** `alt`, `map`, `delimited`, `separated_list0` accept closures or functions as arguments—direct application of `Fn`/`FnMut` traits and type inference.
4. **Idiomatic error handling.** `IResult<I, O>` maps to `Result<(I, O), Err<E>>`—structurally identical to Rust's `Result`, implementing the pattern of returning residual input alongside output. `nom::Err::Error` vs `nom::Err::Failure` maps directly to recoverable vs. unrecoverable errors.
5. **Recovery atop combinators.** The top-level loop within `Parser::parse` operates explicitly: parsing is attempted; upon failure, the pointer advances to the subsequent `@`. Manual recovery control is combined with `nom` combinators for internal grammar.

### Differences compared to the previous manual parser

| Aspect | Manual recursive descent | nom combinators |
|---|---|---|
| Tokenization | Separate lexer (`lexer.rs`) | Integrated into combinators |
| Lines of code | ~400 (lexer) + ~400 (parser) | ~300 (parser.rs) |
| Braced content | Manual `read_braced_content` | `braced_content` combinator |
| `#` Concatenation | Manual loop in `parse_value` | Explicit loop in `field_value` |
| Recovery | `recover_to_*` methods | `skip_to_next_at` + fallthrough |
| Rust Learning | Generic (loops, match, state) | Idiomatic (composition, traits, lifetimes, zero-copy) |

## serializer.rs — Bibliography → String

```rust
use crate::model::*;
use indexmap::IndexMap;

/// Serialization configuration.
pub struct SerializeConfig {
    /// Field indentation within entry.
    pub indent: String,
    /// Separator between field name and value. Typically " = ".
    pub field_separator: String,
    /// Align field '=' signs (name padding).
    pub align_equals: bool,
    /// Comma following final field.
    pub trailing_comma: bool,
    /// Blank lines between entries.
    pub entry_separator: usize,
    /// Preferred field order. Absent fields append in original order.
    pub field_order: Vec<String>,
}

impl Default for SerializeConfig {
    fn default() -> Self {
        Self {
            indent: "  ".into(),
            field_separator: " = ".into(),
            align_equals: false,
            trailing_comma: true,
            entry_separator: 1,
            field_order: vec![
                "author".into(), "title".into(), "year".into(), "date".into(),
                "journal".into(), "journaltitle".into(), "booktitle".into(),
                "volume".into(), "number".into(), "pages".into(),
                "publisher".into(), "doi".into(), "url".into(), "isbn".into(),
                "issn".into(), "abstract".into(), "keywords".into(), "file".into(),
            ],
        }
    }
}

pub fn serialize(bib: &Bibliography, config: &SerializeConfig) -> String {
    let mut out = String::new();

    // @string macros
    for (key, value) in &bib.strings {
        out.push_str(&format!("@string{{{key}{sep}{{{value}}}}}\n",
            sep = config.field_separator));
    }
    if !bib.strings.is_empty() {
        out.push('\n');
    }

    // @preamble
    for p in &bib.preambles {
        out.push_str(&format!("@preamble{{{{{p}}}}}\n"));
    }
    if !bib.preambles.is_empty() {
        out.push('\n');
    }

    // Entries
    let sep = "\n".repeat(config.entry_separator + 1);
    let mut first = true;

    for entry in &bib.entries {
        if !first {
            out.push_str(&sep);
        }
        first = false;

        // Preceding comments
        for c in &entry.leading_comments {
            out.push_str(c);
            out.push('\n');
        }

        serialize_entry(entry, config, &mut out);
    }

    // Trailing
    if !bib.trailing_content.is_empty() {
        out.push('\n');
        out.push_str(&bib.trailing_content);
    }

    out
}

fn serialize_entry(entry: &Entry, config: &SerializeConfig, out: &mut String) {
    out.push_str(&format!(
        "@{}{{{},\n",
        entry.entry_type.as_str(),
        entry.cite_key
    ));

    let ordered_fields = order_fields(&entry.fields, &config.field_order);
    let max_name_len = if config.align_equals {
        ordered_fields.iter().map(|(k, _)| k.len()).max().unwrap_or(0)
    } else {
        0
    };

    let total = ordered_fields.len();
    for (i, (name, value)) in ordered_fields.iter().enumerate() {
        let padded_name = if config.align_equals {
            format!("{:width$}", name, width = max_name_len)
        } else {
            name.to_string()
        };

        let comma = if i < total - 1 || config.trailing_comma { "," } else { "" };

        out.push_str(&format!(
            "{indent}{name}{sep}{value}{comma}\n",
            indent = config.indent,
            name = padded_name,
            sep = config.field_separator,
            value = value.to_bibtex(),
        ));
    }

    out.push_str("}\n");
}

fn order_fields<'a>(
    fields: &'a IndexMap<String, FieldValue>,
    order: &[String],
) -> Vec<(&'a str, &'a FieldValue)> {
    let mut ordered: Vec<(&str, &FieldValue)> = Vec::new();
    let mut seen = std::collections::HashSet::new();

    // Fields in preferred order
    for key in order {
        if let Some(value) = fields.get(key.as_str()) {
            ordered.push((key.as_str(), value));
            seen.insert(key.as_str());
        }
    }

    // Remaining fields in original order
    for (key, value) in fields {
        if !seen.contains(key.as_str()) {
            ordered.push((key.as_str(), value));
        }
    }

    ordered
}
```

## main.rs — CLI

```rust
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "bibrs", version, about = "BibTeX toolkit")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Verify .bib file and report errors.
    Check {
        file: PathBuf,
    },
    /// Reformat .bib file.
    Format {
        file: PathBuf,
        /// Overwrite original file (default: print to stdout).
        #[arg(long)]
        in_place: bool,
    },
    /// File statistics.
    Stats {
        file: PathBuf,
    },
}

fn main() -> Result<(), bibrs::error::BibrsError> {
    let cli = Cli::parse();

    match cli.command {
        Command::Check { file } => cmd_check(&file),
        Command::Format { file, in_place } => cmd_format(&file, in_place),
        Command::Stats { file } => cmd_stats(&file),
    }
}

fn cmd_check(file: &PathBuf) -> Result<(), bibrs::error::BibrsError> {
    let bytes = std::fs::read(file)?;
    let enc = bibrs::encoding::detect_and_convert(&bytes);

    if enc.original != bibrs::encoding::DetectedEncoding::Utf8 {
        eprintln!(
            "encoding: {:?}{}",
            enc.original,
            if enc.had_bom { " (with BOM)" } else { "" }
        );
    }
    if !enc.lossy.is_empty() {
        eprintln!("{} lossy conversions", enc.lossy.len());
    }

    let result = bibrs::parser::Parser::parse(&enc.content);

    eprintln!("{} entries", result.bibliography.entries.len());
    eprintln!("{} @string macros", result.bibliography.strings.len());

    if result.errors.is_empty() {
        eprintln!("no errors");
    } else {
        eprintln!("{} errors:", result.errors.len());
        for e in &result.errors {
            eprintln!("  {}", e);
        }
    }

    Ok(())
}

fn cmd_format(file: &PathBuf, in_place: bool) -> Result<(), bibrs::error::BibrsError> {
    let bytes = std::fs::read(file)?;
    let enc = bibrs::encoding::detect_and_convert(&bytes);
    let result = bibrs::parser::Parser::parse(&enc.content);

    let config = bibrs::serializer::SerializeConfig::default();
    let output = bibrs::serializer::serialize(&result.bibliography, &config);

    if in_place {
        std::fs::write(file, &output)?;
        eprintln!("formatted: {}", file.display());
    } else {
        print!("{output}");
    }

    Ok(())
}

fn cmd_stats(file: &PathBuf) -> Result<(), bibrs::error::BibrsError> {
    let bytes = std::fs::read(file)?;
    let enc = bibrs::encoding::detect_and_convert(&bytes);
    let result = bibrs::parser::Parser::parse(&enc.content);
    let bib = &result.bibliography;

    eprintln!("entries: {}", bib.entries.len());
    eprintln!("@string: {}", bib.strings.len());
    eprintln!("@preamble: {}", bib.preambles.len());
    eprintln!("parse errors: {}", result.errors.len());

    let counts = bib.count_by_type();
    for (t, n) in &counts {
        eprintln!("  @{}: {}", t, n);
    }

    Ok(())
}
```

## lib.rs — re-exports

```rust
pub mod model;
pub mod error;
pub mod encoding;
pub mod parser;
pub mod serializer;
```

## Tests — fixtures and roundtrip

The `tests/fixtures/` directory must contain actual `.bib` files. Sources:

* JabRef exports featuring known defects.
* `.bib` files extracted from public article repositories.
* Synthesized files encompassing edge cases: numerical cite keys, entries devoid of fields, deeply nested braces, `@string` entities referencing other `@string` entities, mixed encoding environments.

```rust
// tests/roundtrip.rs

use bibrs::parser::Parser;
use bibrs::serializer::{serialize, SerializeConfig};

fn roundtrip(input: &str) {
    let r1 = Parser::parse(input);
    let serialized = serialize(&r1.bibliography, &SerializeConfig::default());
    let r2 = Parser::parse(&serialized);

    assert_eq!(r1.bibliography.entries.len(), r2.bibliography.entries.len());
    for (a, b) in r1.bibliography.entries.iter().zip(r2.bibliography.entries.iter()) {
        assert_eq!(a.cite_key, b.cite_key);
        assert_eq!(a.entry_type, b.entry_type);
        assert_eq!(a.fields.len(), b.fields.len());
    }
}

#[test]
fn roundtrip_simple() {
    roundtrip(r#"
@article{silva2023,
  author = {Silva, João and Santos, Maria},
  title = {Título do Artigo},
  journal = {Revista Brasileira},
  year = {2023},
  volume = {10},
  pages = {1--20},
}
"#);
}

#[test]
fn roundtrip_string_macros() {
    roundtrip(r#"
@string{jbr = "Jornal Brasileiro"}

@article{key1,
  author = {Autor},
  journal = jbr,
  year = 2024,
}
"#);
}

#[test]
fn roundtrip_concat() {
    roundtrip(r#"
@string{first = "João"}

@book{key2,
  author = first # " Silva",
  title = {Livro},
  year = {2020},
}
"#);
}
```

```rust
// tests/recovery.rs

use bibrs::parser::Parser;

#[test]
fn missing_closing_brace() {
    let input = r#"
@article{key1,
  author = {Incomplete,
  title = {Complete},

@article{key2,
  author = {Other},
  year = {2024},
}
"#;
    let result = Parser::parse(input);
    // Must recover and capture key2 even with malformed key1
    assert!(!result.errors.is_empty());
    assert!(result.bibliography.entries.iter().any(|e| e.cite_key == "key2"));
}

#[test]
fn missing_cite_key() {
    let input = "@article{, author = {Nobody}}";
    let result = Parser::parse(input);
    assert!(!result.errors.is_empty());
}

#[test]
fn duplicate_cite_keys() {
    let input = r#"
@article{dup, author = {A}}
@article{dup, author = {B}}
"#;
    let result = Parser::parse(input);
    // Both must be preserved — dedup is delegated to another layer
    assert_eq!(result.bibliography.entries.len(), 2);
}

#[test]
fn empty_file() {
    let result = Parser::parse("");
    assert!(result.bibliography.entries.is_empty());
    assert!(result.errors.is_empty());
}

#[test]
fn only_comments() {
    let result = Parser::parse("% this is a comment\n% another one\n");
    assert!(result.bibliography.entries.is_empty());
}
```

## Foundation Completion Criteria

1.  `cargo test` passes with zero failures.
2.  `cargo clippy` emits zero warnings.
3.  `cargo doc --no-deps` generates documentation without errors. All public items implement `///` doc comments.
4.  `bibrs check <file.bib>` processes a minimum of 3 real `.bib` files (>100 entries each) without panicking.
5.  `bibrs format <file.bib>` outputs valid data sustaining the roundtrip constraint.
6.  Files containing Latin-1 and Windows-1252 encoding are detected and converted.
7.  Malformed files (unclosed braces, valueless fields, junk data between entries) are processed; errors are reported, and maximum data is extracted.

Implementation proceeds to the Structure layer strictly following satisfaction of the above criteria.

---

# STRUCTURE

Implementation must not begin until the Foundation is complete. The following is a specification, not final code.

## Scope

* Author name normalization.
* Field normalization (DOI, pages, ISSN, ISBN, year, title).
* Duplicate detection.
* Cite key generation.
* External API integration (CrossRef, OpenAlex, Google Books, OpenLibrary).
* API response disk caching.
* User configuration file parsing (TOML).
* Structured logging (`tracing`).

## Crate division criteria

Upon initiating the Structure layer, the project topology alters. External APIs necessitate `reqwest`, `tokio`, `async-trait`, `futures`—heavy dependencies that must not pollute the core. At this juncture, extraction is mandated:

```
bibrs/
├── Cargo.toml              # workspace
├── crates/
│   ├── bibrs-core/         # current unified crate (model, parser, serializer, encoding)
│   ├── bibrs-normalize/    # names, fields, dedup, cite keys
│   └── bibrs-sources/      # BibSource trait + implementations per API
├── src/
│   └── main.rs             # unified CLI (depends on all crates)
└── frontend/
```

Extraction is mechanical: migrate `src/*.rs` to `crates/bibrs-core/src/`, correct `use` declarations, verify `cargo test --workspace` continuity. Logic remains unaltered during this phase.

## Normalization — functional specification

### Author names

Input: `author` or `editor` field as a BibTeX string.
Output: `Vec<PersonName>` containing decomposed segments.

```
"Last, First"                    → { first: "First", last: "Last" }
"First Last"                     → { first: "First", last: "Last" }
"First Middle Last"              → { first: "First Middle", last: "Last" }
"Last, Jr., First"               → { first: "First", last: "Last", jr: "Jr." }
"{Institutional Name}"           → { last: "Institutional Name", is_institutional: true }
"van der Berg, Jan"              → { first: "Jan", von: "van der", last: "Berg" }
"João da Silva and Maria Santos" → 2 PersonName
```

Particle rule (`von`): lowercase-initiated words situated between first and last components. Direct reference: `typst/biblatex` algorithm in `src/types/person.rs` and classic BibTeX algorithm (Oren Patashnik, "Name Parsing" section in original documentation).

### Fields

| Field | Transformation |
|---|---|
| `doi` | Excise prefixes (`https://doi.org/`, `doi:`, whitespace). Result: `10.xxxx/yyyy`. |
| `pages` | Normalize hyphens: `15-20` → `15--20`, `15 – 20` → `15--20`. |
| `issn` | Excise hyphens, validate check digit (mod 11). |
| `isbn` | Excise hyphens, validate ISBN-10 (mod 11) and ISBN-13 (mod 10). |
| `year` | Extract 4 digits: `"2023a"` → `"2023"`, `"(2023)"` → `"2023"`. |
| `title` | Detect ALL CAPS → convert to title case. Protect acronyms utilizing `{...}`. |

### Duplicates

Two-layer strategy deployed:

1.  **Exact DOI match.** O(n) utilizing HashSet. Resolves primary load.
2.  **Fuzzy match via normalized title.** Lowercase cast, punctuation excision, stopword excision, comparison via Jaro-Winkler or Jaccard coefficient. Configurable threshold (default: 0.90).

Output: `Vec<DuplicateGroup>` containing indices, confidence level, and rationale.

### Cite keys

Configurable pattern: `{auth}{year}{shorttitle}`.
* `{auth}`: primary author surname, lowercase, accents excised.
* `{year}`: year field.
* `{shorttitle}`: initial significant title word (>3 characters), lowercase.

Dedup suffix: `a`, `b`, `c`... appended during key collision events.

## External APIs — functional specification

### Common Trait

```rust
#[async_trait]
pub trait BibSource: Send + Sync {
    fn id(&self) -> &'static str;
    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, SourceError>;
    async fn fetch_by_id(&self, id: &str) -> Result<Option<SearchResult>, SourceError>;
}
```

### API Mapping

**CrossRef** (`https://api.crossref.org/works`)
* Query targets: search string, DOI, author.
* `type` → `EntryType`: `"journal-article"` → Article, `"book"` → Book, etc.
* `author[].given` + `author[].family` → BibTeX `author` field.
* `container-title[0]` → `journal`.
* `published.date-parts[0][0]` → `year`.
* Header: `mailto` required for polite pool assignment.

**OpenAlex** (`https://api.openalex.org/works`)
* Query targets: title, DOI, ORCID.
* `authorships[].author.display_name` → `author` field.
* `primary_location.source.display_name` → `journal`.
* `publication_year` → `year`.
* Filters: `filter=doi:`, `filter=title.search:`.

**Google Books** (`https://www.googleapis.com/books/v1/volumes`)
* Query targets: title, author, ISBN.
* `volumeInfo.authors[]` → `author` field.
* `volumeInfo.publisher` → `publisher`.
* `volumeInfo.industryIdentifiers[]` → `isbn`.
* Entry type mapping: rigidly mapped to `Book` or `InBook`.

**OpenLibrary** (`https://openlibrary.org/search.json`, `https://openlibrary.org/isbn/`)
* Query targets: title, author, ISBN.
* `author_name[]` → `author` field.
* `publisher[]` → `publisher`.
* `isbn[]` → `isbn`.
* Function: supplements Google Books output for uncommon editions.

### Rate limiting

| API | Limit | Strategy |
|---|---|---|
| CrossRef | 50 req/s (polite) | `mailto` header + programmatic sleep if required |
| OpenAlex | 10 req/s (unauthenticated) | Inter-request sleep execution |
| Google Books | ~1000 req/day | Aggressive caching implementation |
| OpenLibrary | No formal threshold | 1 req/s throttling |

### Response caching

Cache directory target: `~/.cache/bibrs/` (leveraging `dirs` crate). Hierarchy:

```
~/.cache/bibrs/
├── crossref/
│   ├── doi/                  # single JSON file per DOI: 10.1000_xyz.json
│   └── search/               # SHA-256 query hash functions as filename
├── openalex/
│   ├── doi/
│   └── search/
├── google_books/
│   └── isbn/
└── openlibrary/
    └── isbn/
```

File contents consist of raw JSON response data coupled with a timestamp. TTL parameters are configurable (default specification: 7 days for search queries, 30 days for DOI/ISBN fetches). Cache utilization is non-mandatory—controlled via CLI `--no-cache` flag or `cache.enabled` configuration parameter.

### User Configuration

Target file: `~/.config/bibrs/config.toml` (leveraging `dirs` crate). Load sequence executes at initialization, overridden by CLI flag inputs. Default specifications must be robust—file existence is non-mandatory.

```toml
[serialize]
indent = "  "
align_equals = true
trailing_comma = true
field_order = ["author", "title", "year", "journal", "volume", "pages", "doi"]

[normalize]
name_format = "last_comma_first"    # last_comma_first | first_last | abbreviated
protect_acronyms = true
doi_strip_prefix = true

[citekey]
pattern = "{auth}{year}{shorttitle}"
lowercase = true
dedup_suffix = "alpha"              # alpha | numeric

[dedup]
fuzzy_threshold = 0.90

[sources]
mailto = ""                          # email parameter for CrossRef polite pool
default_sources = ["crossref", "openalex"]

[cache]
enabled = true
ttl_search_days = 7
ttl_id_days = 30
```

Implementation design: `Config` struct implementing `#[derive(Deserialize)]`, populated via default specifications via `Default` implementation, merged with TOML data via `toml` crate parsing.

### Logging

Implementation relies on `tracing` + `tracing-subscriber` equipped with level filtering controlled by `BIBRS_LOG` environment variable (default specification: `warn`).

```
BIBRS_LOG=info bibrs search --source crossref --doi 10.1000/xyz
```

Structured logging targets: HTTP execution (URL, status code, latency), normalization failures, duplicate event detection, cache hit/miss rates. Logging operations remain absent from the Foundation layer—`eprintln!` calls within the CLI module satisfy requirements at that tier.

### HTTP mock testing

Unit testing targeting `bibrs-sources` employs `wiremock` to replicate API outputs. JSON fixtures containing unadulterated responses are housed in `tests/fixtures/api/`. Integration testing (dependent on network connectivity) is isolated via a feature flag:

```toml
[features]
integration = []
```

```
cargo test                                    # strict mock execution
cargo test --features integration             # executes network calls (adhering to rate limits)
```

### Additional dependencies (exclusive to bibrs-sources)

```toml
reqwest = { version = "0.12", features = ["json"] }
tokio = { version = "1", features = ["full"] }
async-trait = "0.1"
futures = "0.3"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
toml = "0.8"
dirs = "6"
sha2 = "0.10"                   # hash generation for query caching
wiremock = "0.6"                # dev-dependency handling HTTP mocks
```

## Structure Completion Criteria

1.  `cargo test --workspace` evaluates successfully (mock execution, strictly localized).
2.  `cargo doc --workspace --no-deps` evaluates without error outputs. Public items contain documentation comments.
3.  Name normalization executes correctly against a corpus containing ≥100 factual names derived from public `.bib` repositories. Corpus must include: particles (`von`, `de`, `van der`), suffixes (`Jr.`, `III`), institutional assignments, and integrated `First Last`/`Last, First` structures.
4.  Field normalization executes correctly targeting DOI, pages, ISSN, ISBN, year properties.
5.  Duplicate detection algorithms locate specified pairings within test files.
6.  API implementations (minimum: CrossRef and OpenAlex) yield parseable payload data importing cleanly as `Entry` structures.
7.  Disk caching mechanism functions: identical secondary query occurrences circumvent HTTP execution protocols.
8.  `~/.config/bibrs/config.toml` is parsed and parameters applied. Default parameters function seamlessly without file presence.
9.  `BIBRS_LOG=debug` generates structured execution logs detailing HTTP operations.
10. CLI functionality extends to include: `bibrs normalize <file>`, `bibrs search --source crossref --doi 10.xxx/yyy`.

---

# SURFACE

Implementation must not begin until the Structure is complete.

## Scope

* Local HTTP server initialization (`axum`).
* Internal REST API architecture.
* Web frontend execution (HTML/CSS/JS).
* Integrated operational sequence: access → traverse → query → import → normalize → save.
* Associated file parameter management (`file` field integration, PDF linkage).

## Server crate creation criteria

Upon initiating the Surface layer, workspace expansion is mandated:

```
crates/
└── bibrs-server/       # dependency parameters: bibrs-core, bibrs-normalize, bibrs-sources
```

## Server — specification

Stack dependencies: `axum` + `tokio` + `tower-http` (ServeDir, CORS implementation).

State parameters: `Arc<RwLock<AppState>>` housing memory-resident `Bibliography`, target file path, configured API sources, mutation dirty flag.

Frontend deployment utilizes static file service mapped to `/frontend/`.

### Endpoints

```
GET    /api/entries              list execution (pagination, filter parameters)
GET    /api/entries/:key         detail retrieval
POST   /api/entries              addition protocol
PUT    /api/entries/:key         update protocol
DELETE /api/entries/:key         removal protocol

POST   /api/import               .bib ingest (upload)
GET    /api/export               .bib output (download)

POST   /api/search               external API query
POST   /api/search/:source       specific external API query

POST   /api/normalize/entry/:key single entry normalization
POST   /api/normalize/all        global normalization
GET    /api/duplicates           duplicate detection protocol

POST   /api/file/save            disk write execution
POST   /api/file/open            disk read execution

GET    /api/stats                statistical retrieval
```

### Frontend

Core constraints: strict HTML + CSS + JS implementation. Exclude frameworks. Exclude build processes. ES6 modules (`import`/`export`) are mandated from initialization to enforce structural coherence:

```
frontend/
├── index.html
├── css/
│   └── main.css
└── js/
    ├── app.js           # initialization protocols, state routing
    ├── api.js           # fetch execution wrapper handling /api/*
    ├── entries.js       # list rendering and filter execution
    ├── editor.js        # entry modification handling
    └── search.js        # external API query management
```

Layout parameters:
* Left-side panel: entry list rendering (filter capable, sort capable).
* Right-side panel: detail visualization/editor for active entry.
* Lower panel: external API query interface, result rendering paired with import execution button.
* Status bar: active entry quantification, error rendering, duplicate quantification, mutation dirty flag status.

Framework integration (Alpine.js or htmx) is restricted until JS complexity exceeds ~2000 lines and structural degradation is observed. Implementation prior to this threshold is prohibited.

## Surface Completion Criteria

1.  `cargo test --workspace` evaluates successfully.
2.  `cargo doc --workspace --no-deps` evaluates without error outputs.
3.  `http://localhost:3000` accesses the active interface.
4.  Operational sequences function: `.bib` ingest, entry traversal, field modification, disk write execution.
5.  Query execution (title/DOI) via CrossRef or OpenAlex functions; corresponding output ingest executes flawlessly.
6.  Single-entry and global normalization function correctly, paired with visual feedback triggers.
7.  Duplicate events are correctly detected and rendered.
8.  `file` property renders functioning link pointing to mapped PDF.
9.  Autosave mechanism or unsaved mutation indicator functions correctly.
10. User configuration (`config.toml`) parameters map correctly into the active interface (field sequence parameters, name format parameters, etc.).

---

# Workflow Sequence

```
FOUNDATION   model → error → encoding → parser (nom) → serializer → CLI → tests → doc
             Output constraint: bibrs check|format|stats functions flawlessly.

STRUCTURE    workspace extraction → config (TOML) → normalize (names, fields, dedup, keys) → sources (APIs + cache + logging) → mocks → CLI extension
             Output constraint: bibrs normalize|search functions flawlessly. Cache, configuration, and logging systems operational.

SURFACE      server → endpoints → frontend (ES6 modules) → integrated sequence
             Output constraint: Local web GUI functions flawlessly.
```

Each layer is closed before opening the next. No exceptions.
