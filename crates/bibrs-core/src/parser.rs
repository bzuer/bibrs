/// BibTeX parser built on nom combinators.
use nom::{
    bytes::complete::take_while1,
    character::complete::{char, digit1, multispace0},
    combinator::{map, map_res, opt},
    branch::alt,
    IResult, Parser as NomParser,
};

use crate::error::*;
use crate::model::*;
use indexmap::IndexMap;

fn ws(input: &str) -> IResult<&str, ()> {
    let (input, _) = multispace0(input)?;
    if input.starts_with('%') {
        let end = input.find('\n').unwrap_or(input.len());
        let (input, _) = multispace0(&input[end..])?;
        ws(input)
    } else {
        Ok((input, ()))
    }
}

fn identifier(input: &str) -> IResult<&str, &str> {
    take_while1(|c: char| c.is_alphanumeric() || "_-.:".contains(c)).parse(input)
}

fn number(input: &str) -> IResult<&str, i64> {
    map_res(digit1, |s: &str| s.parse::<i64>()).parse(input)
}

fn braced_content(input: &str) -> IResult<&str, String> {
    let (input, _) = char('{').parse(input)?;
    let mut depth = 1u32;
    let mut pos = 0;

    let bytes = input.as_bytes();
    while pos < bytes.len() && depth > 0 {
        match bytes[pos] {
            b'{' => depth += 1,
            b'}' => depth -= 1,
            b'\\' if pos + 1 < bytes.len() => pos += 1,
            b'@' if depth == 1 && pos > 0 && bytes[pos - 1] == b'\n' => {
                break;
            }
            _ => {}
        }
        if depth > 0 {
            pos += 1;
        }
    }

    if depth > 0 {
        Err(nom::Err::Error(nom::error::Error::new(
            &input[pos..],
            nom::error::ErrorKind::Tag,
        )))
    } else {
        let content = input[..pos].to_string();
        Ok((&input[pos + 1..], content))
    }
}

fn quoted_content(input: &str) -> IResult<&str, String> {
    let (input, _) = char('"').parse(input)?;
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
        Ok((&input[pos + 1..], content))
    } else {
        Ok((&input[pos..], content))
    }
}

fn single_value(input: &str) -> IResult<&str, FieldValue> {
    let (input, _) = ws(input)?;
    alt((
        map(braced_content, FieldValue::Literal),
        map(quoted_content, FieldValue::Literal),
        map(number, FieldValue::Integer),
        map(identifier, |s: &str| FieldValue::StringRef(s.to_string())),
    ))
    .parse(input)
}

fn field_value(input: &str) -> IResult<&str, FieldValue> {
    let (input, first) = single_value(input)?;
    let (input, _) = ws(input)?;

    let mut parts = vec![first];
    let mut remaining = input;

    loop {
        let (inp, _) = ws(remaining)?;
        if inp.starts_with('#') {
            let (inp, _) = char('#').parse(inp)?;
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

fn field(input: &str) -> IResult<&str, (String, FieldValue)> {
    let (input, _) = ws(input)?;
    let (input, name) = identifier(input)?;
    let (input, _) = ws(input)?;
    let (input, _) = char('=').parse(input)?;
    let (input, _) = ws(input)?;
    let (input, val) = field_value(input)?;
    Ok((input, (name.to_ascii_lowercase(), val)))
}

fn field_list(input: &str) -> IResult<&str, IndexMap<String, FieldValue>> {
    let mut fields = IndexMap::new();
    let mut remaining = input;

    loop {
        let (inp, _) = ws(remaining)?;
        match field(inp) {
            Ok((inp, (name, val))) => {
                fields.insert(name, val);
                let (inp, _) = ws(inp)?;
                if let Some(stripped) = inp.strip_prefix(',') {
                    remaining = stripped;
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

fn delimited_body(input: &str) -> IResult<&str, &str> {
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
    let (input, _) = char(open_char).parse(input)?;

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
    let rest = if pos < bytes.len() {
        &input[pos + 1..]
    } else {
        &input[pos..]
    };
    Ok((rest, body))
}

fn parse_entry<'a>(entry_type: &str, input: &'a str) -> IResult<&'a str, Entry> {
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
    let (input, _) = char(open_char).parse(input)?;
    let (input, _) = ws(input)?;

    let (input, cite_key) = take_while1(|c: char| {
        c.is_alphanumeric() || "_-.:/@+".contains(c)
    })
    .parse(input)?;
    let (input, _) = ws(input)?;
    let (input, _) = opt(char(',')).parse(input)?;

    let (input, fields) = field_list(input)?;
    let (input, _) = ws(input)?;

    let input = if input.starts_with(close_char) {
        &input[close_char.len_utf8()..]
    } else {
        return Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Tag,
        )));
    };

    Ok((
        input,
        Entry {
            entry_type: EntryType::parse(entry_type),
            cite_key: cite_key.to_string(),
            fields,
            leading_comments: Vec::new(),
        },
    ))
}

fn parse_string_def(input: &str) -> IResult<&str, (String, String)> {
    let (rest, body) = delimited_body(input)?;
    let (_, _) = ws(body)?;
    match field(body) {
        Ok((_, (key, val))) => {
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

/// Main BibTeX parser. Operates with tolerance: errors are collected, not propagated.
pub struct Parser;

impl Parser {
    /// Parses a BibTeX input string into a `ParseResult`.
    ///
    /// The parser never aborts. Syntax errors are collected in the result's
    /// `errors` field while parsing continues to extract maximum data.
    pub fn parse(input: &str) -> ParseResult {
        let mut bib = Bibliography::new();
        let mut errors = Vec::new();
        let mut remaining = input;

        while !remaining.is_empty() {
            match remaining.find('@') {
                Some(pos) => {
                    remaining = &remaining[pos..];
                }
                None => break,
            }

            remaining = &remaining[1..];

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
                "string" => match parse_string_def(remaining) {
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
                },
                "preamble" => match parse_preamble(remaining) {
                    Ok((inp, content)) => {
                        bib.preambles.push(content);
                        remaining = inp;
                    }
                    Err(_) => {
                        Self::skip_to_next_at(&mut remaining);
                    }
                },
                "comment" => match parse_comment_entry(remaining) {
                    Ok((inp, _)) => remaining = inp,
                    Err(_) => Self::skip_to_next_at(&mut remaining),
                },
                _ => match parse_entry(type_str, remaining) {
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
                },
            }
        }

        ParseResult {
            bibliography: bib,
            errors,
        }
    }

    fn skip_to_next_at(remaining: &mut &str) {
        match remaining.find('@') {
            Some(pos) => *remaining = &remaining[pos..],
            None => *remaining = "",
        }
    }

    fn span_at(original: &str, current: &str) -> Span {
        let offset = original.len() - current.len();
        let consumed = &original[..offset];
        let line = consumed.matches('\n').count() + 1;
        let col = consumed
            .rfind('\n')
            .map(|pos| offset - pos)
            .unwrap_or(offset + 1);
        Span { offset, line, col }
    }
}
