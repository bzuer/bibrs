use crate::model::*;
use indexmap::IndexMap;
use std::collections::HashSet;
use std::fmt::Write;

/// Configuration for BibTeX serialization.
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
                "author".into(),
                "title".into(),
                "year".into(),
                "date".into(),
                "journal".into(),
                "journaltitle".into(),
                "booktitle".into(),
                "volume".into(),
                "number".into(),
                "pages".into(),
                "publisher".into(),
                "doi".into(),
                "url".into(),
                "isbn".into(),
                "issn".into(),
                "abstract".into(),
                "keywords".into(),
                "file".into(),
            ],
        }
    }
}

/// Serializes a Bibliography back to BibTeX format.
pub fn serialize(bib: &Bibliography, config: &SerializeConfig) -> String {
    let mut out = String::new();

    for (key, value) in &bib.strings {
        let _ = writeln!(
            out,
            "@string{{{key}{sep}{{{value}}}}}",
            sep = config.field_separator
        );
    }
    if !bib.strings.is_empty() {
        out.push('\n');
    }

    for p in &bib.preambles {
        let _ = writeln!(out, "@preamble{{{{{p}}}}}");

    }
    if !bib.preambles.is_empty() {
        out.push('\n');
    }

    let sep = "\n".repeat(config.entry_separator + 1);
    let mut first = true;

    for entry in &bib.entries {
        if !first {
            out.push_str(&sep);
        }
        first = false;

        for c in &entry.leading_comments {
            out.push_str(c);
            out.push('\n');
        }

        serialize_entry(entry, config, &mut out);
    }

    if !bib.trailing_content.is_empty() {
        out.push('\n');
        out.push_str(&bib.trailing_content);
    }

    out
}

fn serialize_entry(entry: &Entry, config: &SerializeConfig, out: &mut String) {
    let _ = writeln!(
        out,
        "@{}{{{},",
        entry.entry_type.as_str(),
        entry.cite_key
    );

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

        let comma = if i < total - 1 || config.trailing_comma {
            ","
        } else {
            ""
        };

        let _ = writeln!(
            out,
            "{indent}{name}{sep}{value}{comma}",
            indent = config.indent,
            name = padded_name,
            sep = config.field_separator,
            value = value.to_bibtex(),
        );
    }

    out.push_str("}\n");
}

fn order_fields<'a>(
    fields: &'a IndexMap<String, FieldValue>,
    order: &[String],
) -> Vec<(&'a str, &'a FieldValue)> {
    let mut ordered: Vec<(&'a str, &'a FieldValue)> = Vec::new();
    let mut seen = HashSet::new();

    for key in order {
        if let Some((actual_key, value)) = fields.get_key_value(key.as_str()) {
            ordered.push((actual_key.as_str(), value));
            seen.insert(actual_key.as_str());
        }
    }

    for (key, value) in fields {
        if !seen.contains(key.as_str()) {
            ordered.push((key.as_str(), value));
        }
    }

    ordered
}
