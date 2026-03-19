use bibrs::model::Bibliography;

#[test]
fn empty_bibliography() {
    let bib = Bibliography::default();
    assert!(bib.entries.is_empty());
    assert!(bib.strings.is_empty());
    assert!(bib.preambles.is_empty());
    assert!(bib.trailing_content.is_empty());
}
