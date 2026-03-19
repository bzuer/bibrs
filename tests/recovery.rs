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
    assert!(!result.errors.is_empty());
    assert!(result
        .bibliography
        .entries
        .iter()
        .any(|e| e.cite_key == "key2"));
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

#[test]
fn junk_between_entries() {
    let input = r#"
Some junk text here
@article{key1, author = {A}, year = {2020}}
More junk
@book{key2, title = {B}, year = {2021}}
"#;
    let result = Parser::parse(input);
    assert_eq!(result.bibliography.entries.len(), 2);
    assert_eq!(result.bibliography.entries[0].cite_key, "key1");
    assert_eq!(result.bibliography.entries[1].cite_key, "key2");
}

#[test]
fn parenthesis_delimiters() {
    let input = "@article(key1, author = {Test}, year = {2024})";
    let result = Parser::parse(input);
    assert_eq!(result.bibliography.entries.len(), 1);
    assert_eq!(result.bibliography.entries[0].cite_key, "key1");
}

#[test]
fn comment_entry_ignored() {
    let input = r#"
@comment{this is a comment entry}
@article{key1, author = {A}}
"#;
    let result = Parser::parse(input);
    assert_eq!(result.bibliography.entries.len(), 1);
}

#[test]
fn integer_field_value() {
    let input = "@article{key1, year = 2024, volume = 10}";
    let result = Parser::parse(input);
    assert_eq!(result.bibliography.entries.len(), 1);
    let entry = &result.bibliography.entries[0];
    assert_eq!(
        entry.fields.get("year"),
        Some(&bibrs::model::FieldValue::Integer(2024))
    );
    assert_eq!(
        entry.fields.get("volume"),
        Some(&bibrs::model::FieldValue::Integer(10))
    );
}

#[test]
fn concatenated_values() {
    let input = r#"
@string{firstname = "John"}
@article{key1, author = firstname # " Doe"}
"#;
    let result = Parser::parse(input);
    assert_eq!(result.bibliography.entries.len(), 1);
    let entry = &result.bibliography.entries[0];
    match entry.fields.get("author") {
        Some(bibrs::model::FieldValue::Concat(parts)) => {
            assert_eq!(parts.len(), 2);
        }
        other => panic!("expected Concat, got {:?}", other),
    }
}
