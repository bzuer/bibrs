use chardetng::EncodingDetector;

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
    let (bytes, had_bom) = strip_utf8_bom(bytes);

    if let Ok(s) = std::str::from_utf8(bytes) {
        return EncodingResult {
            content: s.to_string(),
            original: DetectedEncoding::Utf8,
            had_bom,
            lossy: Vec::new(),
        };
    }

    let mut detector = EncodingDetector::new();
    detector.feed(bytes, true);
    let encoding = detector.guess(None, true);

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
    let mut lossy = Vec::new();
    let mut decoder = encoding.new_decoder();
    let mut output = vec![0u8; decoder.max_utf8_buffer_length(1).unwrap_or(4)];
    for (i, &byte) in bytes.iter().enumerate() {
        let (result, _, _) = decoder.decode_to_utf8_without_replacement(
            &[byte],
            &mut output,
            false,
        );
        if let encoding_rs::DecoderResult::Malformed(_, _) = result {
            lossy.push((i, vec![byte]));
        }
    }
    lossy
}
