use bibrs::encoding::{detect_and_convert, DetectedEncoding};

#[test]
fn utf8_passthrough() {
    let input = "Hello, world! São Paulo café";
    let result = detect_and_convert(input.as_bytes());
    assert_eq!(result.original, DetectedEncoding::Utf8);
    assert!(!result.had_bom);
    assert!(result.lossy.is_empty());
    assert_eq!(result.content, input);
}

#[test]
fn utf8_with_bom() {
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(b"Hello BOM");
    let result = detect_and_convert(&bytes);
    assert_eq!(result.original, DetectedEncoding::Utf8);
    assert!(result.had_bom);
    assert_eq!(result.content, "Hello BOM");
}

#[test]
fn latin1_detection() {
    let bytes: Vec<u8> = vec![
        b'c', b'a', b'f', 0xe9, b' ', b'n', b'a', b'i', b'v', b'e',
    ];
    let result = detect_and_convert(&bytes);
    assert_ne!(result.original, DetectedEncoding::Utf8);
    assert!(result.content.contains("caf"));
}

#[test]
fn empty_input() {
    let result = detect_and_convert(b"");
    assert_eq!(result.original, DetectedEncoding::Utf8);
    assert!(result.content.is_empty());
    assert!(!result.had_bom);
}

#[test]
fn pure_ascii() {
    let result = detect_and_convert(b"@article{key, author = {Smith}}");
    assert_eq!(result.original, DetectedEncoding::Utf8);
    assert!(result.lossy.is_empty());
}
