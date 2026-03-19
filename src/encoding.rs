/// Detected encoding of the input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DetectedEncoding {
    Utf8,
    Latin1,
    Windows1252,
    Other(String),
}

/// Result of encoding detection and conversion.
pub struct EncodingResult {
    pub content: String,
    pub original: DetectedEncoding,
    pub had_bom: bool,
    pub lossy: Vec<(usize, Vec<u8>)>,
}
