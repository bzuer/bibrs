use bibrs::parser::Parser;
use bibrs::serializer::{serialize, SerializeConfig};

fn roundtrip(input: &str) {
    let r1 = Parser::parse(input);
    let serialized = serialize(&r1.bibliography, &SerializeConfig::default());
    let r2 = Parser::parse(&serialized);

    assert_eq!(r1.bibliography.entries.len(), r2.bibliography.entries.len());
    for (a, b) in r1
        .bibliography
        .entries
        .iter()
        .zip(r2.bibliography.entries.iter())
    {
        assert_eq!(a.cite_key, b.cite_key);
        assert_eq!(a.entry_type, b.entry_type);
        assert_eq!(a.fields.len(), b.fields.len());
    }
}

#[test]
fn roundtrip_simple() {
    roundtrip(
        r#"
@article{silva2023,
  author = {Silva, Jo\~ao and Santos, Maria},
  title = {T\'itulo do Artigo},
  journal = {Revista Brasileira},
  year = {2023},
  volume = {10},
  pages = {1--20},
}
"#,
    );
}

#[test]
fn roundtrip_string_macros() {
    roundtrip(
        r#"
@string{jbr = "Jornal Brasileiro"}

@article{key1,
  author = {Autor},
  journal = jbr,
  year = 2024,
}
"#,
    );
}

#[test]
fn roundtrip_concat() {
    roundtrip(
        r#"
@string{first = "Jo\~ao"}

@book{key2,
  author = first # " Silva",
  title = {Livro},
  year = {2020},
}
"#,
    );
}

#[test]
fn roundtrip_empty_bibliography() {
    let bib = bibrs::model::Bibliography::default();
    assert!(bib.entries.is_empty());
    assert!(bib.strings.is_empty());
    assert!(bib.preambles.is_empty());
    assert!(bib.trailing_content.is_empty());
}

#[test]
fn roundtrip_multiple_entries() {
    roundtrip(
        r#"
@article{key1,
  author = {Author One},
  title = {First Article},
  year = {2020},
}

@book{key2,
  author = {Author Two},
  title = {A Book},
  publisher = {Publisher},
  year = {2021},
}

@inproceedings{key3,
  author = {Author Three},
  title = {Conference Paper},
  booktitle = {Proceedings},
  year = {2022},
}
"#,
    );
}

#[test]
fn roundtrip_preamble_and_strings() {
    let input = r#"
@preamble{"\newcommand{\noopsort}[1]{}"}

@string{ieee = "IEEE"}

@article{test,
  author = {Test},
  journal = ieee,
  year = {2023},
}
"#;
    let r1 = Parser::parse(input);
    assert_eq!(r1.bibliography.preambles.len(), 1);
    assert_eq!(r1.bibliography.strings.len(), 1);
    assert_eq!(r1.bibliography.entries.len(), 1);

    let serialized = serialize(&r1.bibliography, &SerializeConfig::default());
    let r2 = Parser::parse(&serialized);
    assert_eq!(r2.bibliography.preambles.len(), 1);
    assert_eq!(r2.bibliography.strings.len(), 1);
    assert_eq!(r2.bibliography.entries.len(), 1);
}
