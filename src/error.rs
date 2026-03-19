use thiserror::Error;

/// Position in the source input.
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

/// Classification of a non-fatal parse error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseErrorKind {
    UnterminatedString,
    UnterminatedBrace,
    InvalidCharacter(char),
    ExpectedCiteKey,
    ExpectedField,
    ExpectedEquals,
    ExpectedValue,
    MissingClosingDelimiter,
    DuplicateCiteKey(String),
    InvalidUtf8,
    MixedEncoding,
    SkippedContent(String),
}

/// Non-fatal error collected during parsing.
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

/// Result of parsing a BibTeX input.
#[derive(Debug, Clone)]
pub struct ParseResult {
    pub bibliography: crate::model::Bibliography,
    pub errors: Vec<ParseError>,
}

/// Fatal errors that prevent processing.
#[derive(Debug, Error)]
pub enum BibrsError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("irrecoverable encoding error: {0}")]
    Encoding(String),
}
