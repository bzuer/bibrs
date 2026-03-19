# bibrs — Planejamento Inicial



## Pretenção inicial

**bibrs** pretende ser um gerenciador bibliográfico para arquivos BibTeX, construído em Rust.

O problema que deve resolver: manter uma base bibliográfica limpa, consistente e utilizável é trabalho repetitivo e propenso a erros. Ferramentas existentes como o JabRef sofrem de problemas crônicos — encoding corrompido, consumo excessivo de memória, travamentos — que transformam o que deveria ser um fluxo contínuo em uma sequência de frustrações. APIs bibliográficas (CrossRef, OpenAlex, Google Books, OpenLibrary) existem e são ricas, mas o caminho entre "encontrar uma referência" e "tê-la limpa e padronizada no `.bib`" envolve etapas manuais demais.

O que se pretende: uma ferramenta leve, acessada via navegador em porta local, que unifica busca, importação, normalização e manutenção de referências bibliográficas num fluxo único e sem atrito. O arquivo `.bib` permanece como formato nativo — sem banco de dados intermediário obrigatório, sem lock-in. A ferramenta lê arquivos reais com problemas (encoding quebrado, campos malformados, inconsistências acumuladas ao longo de anos), reporta o que encontra, e oferece mecanismos para corrigir.

O resultado concreto é duplo. O primeiro é a ferramenta em si: um binário Rust que abre um `.bib`, expõe uma interface web local para navegar, buscar, importar de APIs, normalizar nomes e campos, detectar duplicatas, e salvar — tudo rápido, com baixo consumo de recursos, tolerante a dados imperfeitos. O segundo resultado é pedagógico: o projeto é veículo de aprendizado de Rust, estruturado em três camadas (parser com `nom`, normalização e APIs, servidor e GUI) que exercitam progressivamente ownership, composição de funções, traits, async, e arquitetura de sistemas.

----

## Conceituação

Três camadas. Cada uma completa, testada e funcional antes de iniciar a próxima.

```
Superfície    GUI web, fluxo integrado, experiência de uso
Estrutura     APIs externas, normalização, busca, dedup
Alicerce      Parser, modelo, serializer, encoding, CLI
```

---

# ALICERCE

Objetivo: ler qualquer `.bib`, reportar problemas, reescrever limpo. Sem rede, sem async, sem dependências pesadas. Produto final: uma biblioteca Rust + CLI que funciona como linter/formatter de `.bib`.

## Estrutura de arquivos

```
bibrs/
├── Cargo.toml
├── src/
│   ├── main.rs          # CLI: bibrs check|format|stats <file.bib>
│   ├── lib.rs           # re-exports públicos
│   ├── model.rs         # tipos fundamentais
│   ├── encoding.rs      # detecção e conversão de charset
│   ├── parser.rs        # nom combinators: &str → Bibliography (tolerante)
│   ├── serializer.rs    # Bibliography → String
│   └── error.rs         # hierarquia de erros
└── tests/
    ├── roundtrip.rs     # parse → serialize → parse → assert_eq
    ├── recovery.rs      # arquivos malformados
    ├── encoding.rs      # Latin-1, Windows-1252, BOM, misto
    └── fixtures/        # .bib reais
```

Quando a complexidade justificar, `parser.rs` migra para `parser/mod.rs`, `parser/combinators.rs`, `parser/recovery.rs`. Não antes.

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

Zero dependências async. Zero rede. O binário resultante é pequeno e rápido.

## model.rs — tipos fundamentais

```rust
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Tipo da entry.
/// Variantes nomeadas para os tipos da spec BibTeX + BibLaTeX.
/// `Other(String)` nunca descarta tipos desconhecidos.
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
    /// Parse case-insensitive. "ARTICLE" → Article, "xyzzy" → Other("xyzzy").
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

    /// Representação canônica para serialização.
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

/// Valor de um campo BibTeX.
///
/// Preserva a estrutura original para roundtrip fiel.
/// `Literal` = conteúdo entre { } ou " ".
/// `Integer` = número sem delimitador.
/// `StringRef` = referência a @string{...}.
/// `Concat` = partes unidas por #.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FieldValue {
    Literal(String),
    Integer(i64),
    StringRef(String),
    Concat(Vec<FieldValue>),
}

impl FieldValue {
    /// Resolve para string final, expandindo referências a @string.
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

    /// Representação BibTeX fiel (para serialização roundtrip).
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

/// Uma entry BibTeX completa.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    pub entry_type: EntryType,
    pub cite_key: String,
    /// Campos na ordem original (IndexMap).
    pub fields: IndexMap<String, FieldValue>,
    /// Comentários/linhas em branco antes desta entry.
    /// Preservados para roundtrip.
    pub leading_comments: Vec<String>,
}

impl Entry {
    /// Valor resolvido de um campo, ou None.
    pub fn get_resolved(
        &self,
        field: &str,
        strings: &IndexMap<String, String>,
    ) -> Option<String> {
        self.fields.get(field).map(|v| v.resolve(strings))
    }

    /// Atalho para campo literal simples.
    pub fn get_str(&self, field: &str) -> Option<&str> {
        match self.fields.get(field) {
            Some(FieldValue::Literal(s)) => Some(s.as_str()),
            _ => None,
        }
    }
}

/// Conteúdo completo de um arquivo .bib.
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

    /// Contagem por tipo de entry.
    pub fn count_by_type(&self) -> IndexMap<&str, usize> {
        let mut counts = IndexMap::new();
        for e in &self.entries {
            *counts.entry(e.entry_type.as_str()).or_insert(0) += 1;
        }
        counts
    }
}
```

### Decisões de design

| Decisão | Razão |
|---|---|
| `FieldValue` enum, não `String` | Preserva macros, concatenações, inteiros. Sem isso, roundtrip falha. |
| `IndexMap`, não `HashMap` | Preserva ordem de campos e `@string`. Diff-friendly. |
| `leading_comments` em `Entry` | Comentários acoplados a entries não são perdidos no roundtrip. |
| `Other(String)` em `EntryType` | Tipos desconhecidos nunca são descartados. |
| `find_by_key` é O(n) | Aceitável até ~10k entries. Índice auxiliar adiado para quando houver necessidade mensurável. |

## error.rs — hierarquia de erros

```rust
use thiserror::Error;

/// Posição no arquivo fonte.
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

/// Classe do erro de parse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseErrorKind {
    // Sintaxe
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

/// Erro não-fatal encontrado durante o parse.
/// O parser continua após registrar o erro.
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

/// Resultado do parse: dados extraídos + erros encontrados.
/// O parser NUNCA falha totalmente — sempre retorna o máximo que conseguiu.
#[derive(Debug, Clone)]
pub struct ParseResult {
    pub bibliography: crate::model::Bibliography,
    pub errors: Vec<ParseError>,
}

/// Erros fatais (I/O, encoding irrecuperável).
#[derive(Debug, Error)]
pub enum BibrsError {
    #[error("erro de I/O: {0}")]
    Io(#[from] std::io::Error),

    #[error("encoding irrecuperável: {0}")]
    Encoding(String),
}
```

### Princípio central: o parser nunca aborta

`ParseResult` sempre contém uma `Bibliography` — possivelmente vazia, possivelmente incompleta, mas nunca `Err`. Erros sintáticos são registrados em `errors` e o parse continua. Isso é a diferença fundamental em relação ao `typst/biblatex`, cujo `Bibliography::parse()` retorna `Result` e falha no primeiro erro grave.

A separação entre `ParseError` (não-fatal, colecionável) e `BibrsError` (fatal: disco inacessível, encoding completamente ilegível) é intencional. Apenas I/O e encoding irrecuperável são fatais.

## encoding.rs — detecção e conversão

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

/// Resultado da detecção + conversão.
pub struct EncodingResult {
    /// Conteúdo convertido para UTF-8.
    pub content: String,
    /// Encoding original detectado.
    pub original: DetectedEncoding,
    /// Presença de BOM.
    pub had_bom: bool,
    /// Bytes que não converteram de forma limpa (posição, bytes originais).
    pub lossy: Vec<(usize, Vec<u8>)>,
}

/// Detecta encoding e converte para UTF-8.
///
/// Pipeline:
/// 1. Verificar BOM (UTF-8, UTF-16).
/// 2. Tentar decode como UTF-8 estrito.
/// 3. Se falhar, usar chardetng para detectar encoding provável.
/// 4. Converter com encoding_rs.
/// 5. Registrar conversões lossy sem abortar.
pub fn detect_and_convert(bytes: &[u8]) -> EncodingResult {
    // 1. BOM check
    let (bytes, had_bom) = strip_utf8_bom(bytes);

    // 2. UTF-8 estrito
    if let Ok(s) = std::str::from_utf8(bytes) {
        return EncodingResult {
            content: s.to_string(),
            original: DetectedEncoding::Utf8,
            had_bom,
            lossy: Vec::new(),
        };
    }

    // 3. Detecção
    let mut detector = EncodingDetector::new();
    detector.feed(bytes, true);
    let encoding = detector.guess(None, true);

    // 4. Conversão
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
    // Implementação: iterar byte a byte, tentar decodificar,
    // registrar posições onde houve substituição.
    // Detalhes da implementação dependem do encoding específico.
    let _ = (bytes, encoding);
    Vec::new() // stub — implementar por encoding
}
```

## parser.rs — nom combinators: &str → Bibliography

O parser usa `nom` para operar diretamente sobre `&str`. Sem fase de tokenização separada — `nom` combina lexing e parsing numa única passagem, compondo funções pequenas (combinators) em parsers maiores.

### Gramática BibTeX em combinators

```
arquivo      = (junk | item)*
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

Cada regra da gramática é uma função `nom` que recebe `&str` e retorna `IResult<&str, T>`.

### Código

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
// Primitivas
// ---------------------------------------------------------------------------

/// Consome whitespace (incluindo comentários % até fim de linha).
fn ws(input: &str) -> IResult<&str, ()> {
    let (input, _) = multispace0(input)?;
    // Comentários % até EOL
    if input.starts_with('%') {
        let end = input.find('\n').unwrap_or(input.len());
        let (input, _) = multispace0(&input[end..])?;
        ws(input) // recursivo para múltiplas linhas de comentário
    } else {
        Ok((input, ()))
    }
}

/// Identifier: letras, dígitos, _, -, ., :
fn identifier(input: &str) -> IResult<&str, &str> {
    take_while1(|c: char| c.is_alphanumeric() || "_-.:".contains(c))(input)
}

/// Número inteiro.
fn number(input: &str) -> IResult<&str, i64> {
    map_res(digit1, |s: &str| s.parse::<i64>())(input)
}

// ---------------------------------------------------------------------------
// Conteúdo braced e quoted
// ---------------------------------------------------------------------------

/// Conteúdo entre { }, respeitando aninhamento.
/// Retorna o conteúdo interno sem as braces externas.
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
        // Brace não fechada — retornar o que tem (tolerância)
        let content = input[..pos].to_string();
        Ok((&input[pos..], content))
    } else {
        let content = input[..pos].to_string();
        Ok((&input[pos + 1..], content)) // +1 para consumir o } final
    }
}

/// Conteúdo entre " ", respeitando braces internas.
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
        Ok((&input[pos + 1..], content)) // +1 para consumir o " final
    } else {
        // Quoted string não terminada — tolerância
        Ok((&input[pos..], content))
    }
}

// ---------------------------------------------------------------------------
// Valores de campos
// ---------------------------------------------------------------------------

/// Um valor atômico: braced, quoted, número ou referência a @string.
fn single_value(input: &str) -> IResult<&str, FieldValue> {
    let (input, _) = ws(input)?;
    alt((
        map(braced_content, FieldValue::Literal),
        map(quoted_content, FieldValue::Literal),
        map(number, FieldValue::Integer),
        map(identifier, |s: &str| FieldValue::StringRef(s.to_string())),
    ))(input)
}

/// Valor possivelmente concatenado com #.
fn field_value(input: &str) -> IResult<&str, FieldValue> {
    let (input, first) = single_value(input)?;
    let (input, _) = ws(input)?;

    // Tentar ler concatenações
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
// Campo: nome = valor
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

/// Lista de campos separados por vírgula, com vírgula trailing opcional.
fn field_list(input: &str) -> IResult<&str, IndexMap<String, FieldValue>> {
    let mut fields = IndexMap::new();
    let mut remaining = input;

    loop {
        let (inp, _) = ws(remaining)?;
        // Tentar parsear um campo
        match field(inp) {
            Ok((inp, (name, val))) => {
                fields.insert(name, val);
                let (inp, _) = ws(inp)?;
                // Vírgula opcional
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
// Entries e definições especiais
// ---------------------------------------------------------------------------

/// Delimitadores: { } ou ( )
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

    // Encontrar o delimitador de fechamento correspondente (com profundidade)
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

    // Cite key: identifier (pode conter /)
    let (input, cite_key) = take_while1(|c: char| {
        c.is_alphanumeric() || "_-.:/@+".contains(c)
    })(input)?;
    let (input, _) = ws(input)?;
    let (input, _) = opt(char(','))(input)?;

    // Campos
    let (input, fields) = field_list(input)?;
    let (input, _) = ws(input)?;

    // Delimitador de fechamento (tolerante — pode estar ausente)
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
    // Dentro do body: key = value
    let (_, _) = ws(body)?;
    match field(body) {
        Ok((_, (key, val))) => {
            // Resolve para string simples (strings não podem referenciar outras no momento do parse)
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
// Top-level: arquivo completo
// ---------------------------------------------------------------------------

/// Parser principal. Opera com tolerância: erros são coletados, não propagados.
pub struct Parser;

impl Parser {
    pub fn parse(input: &str) -> ParseResult {
        let mut bib = Bibliography::new();
        let mut errors = Vec::new();
        let mut remaining = input;

        while !remaining.is_empty() {
            // Consumir whitespace e junk até o próximo @
            match remaining.find('@') {
                Some(pos) => {
                    remaining = &remaining[pos..];
                }
                None => break, // nenhum @ restante — fim
            }

            // Consumir @
            remaining = &remaining[1..];

            // Ler tipo
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
                        message: "esperado tipo de entry após @".into(),
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
                                message: "@string malformado".into(),
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
                                message: format!("entry @{} malformada", type_str),
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

    /// Avança até o próximo @ (recovery de nível top).
    fn skip_to_next_at(remaining: &mut &str) {
        match remaining[1..].find('@') {
            Some(pos) => *remaining = &remaining[pos + 1..],
            None => *remaining = "",
        }
    }

    /// Calcula Span a partir da posição atual no input original.
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

### O que nom ensina aqui

1. **Composição de funções.** Cada combinator (`braced_content`, `field_value`, `field`, `field_list`) é uma função pura que recebe `&str` e retorna `IResult`. A gramática é construída por composição — o parser de entry *usa* o parser de field que *usa* o parser de value. Esse padrão (funções pequenas compostas em funções maiores) é central em Rust idiomático.

2. **Zero-copy por default.** Combinators como `identifier` retornam `&str` — referências ao input original, sem alocação. `String` é criada apenas quando necessário (conteúdo braced/quoted que precisa ser owned no modelo). A tensão entre `&str` e `String` é o exercício natural de ownership.

3. **Closures como parsers.** `alt`, `map`, `delimited`, `separated_list0` recebem closures ou funções como argumentos — exercício direto de traits `Fn`/`FnMut` e inferência de tipos.

4. **Error handling idiomático.** `IResult<I, O>` é `Result<(I, O), Err<E>>` — mesma estrutura de `Result` do Rust, com o padrão de retornar o input restante junto com o resultado. `nom::Err::Error` vs `nom::Err::Failure` mapeia para erros recuperáveis vs. irrecuperáveis.

5. **Recovery sobre combinators.** O loop top-level no `Parser::parse` é explícito: tenta parsear, se falha, avança até o próximo `@`. Combina o controle manual de recovery com os combinators de `nom` para a gramática interna. Essa fronteira entre "nom cuida da gramática, código manual cuida da tolerância" é uma decisão de design formativa.

### Diferença em relação ao parser artesanal anterior

| Aspecto | Recursive descent manual | nom combinators |
|---|---|---|
| Tokenização | Lexer separado (`lexer.rs`) | Integrada nos combinators |
| Linhas de código | ~400 (lexer) + ~400 (parser) | ~300 (parser.rs) |
| Braced content | `read_braced_content` manual | `braced_content` combinator |
| Concatenação `#` | Loop manual em `parse_value` | Loop explícito em `field_value` |
| Recovery | Métodos `recover_to_*` | `skip_to_next_at` + fallthrough |
| Aprendizado Rust | Genérico (loops, match, state) | Idiomático (composição, traits, lifetimes, zero-copy) |

## serializer.rs — Bibliography → String

```rust
use crate::model::*;
use indexmap::IndexMap;

/// Configuração de serialização.
pub struct SerializeConfig {
    /// Indentação de campos dentro da entry.
    pub indent: String,
    /// Separador entre nome do campo e valor. Tipicamente " = ".
    pub field_separator: String,
    /// Alinhar o '=' dos campos (padding do nome).
    pub align_equals: bool,
    /// Vírgula após o último campo.
    pub trailing_comma: bool,
    /// Linhas em branco entre entries.
    pub entry_separator: usize,
    /// Ordem preferida de campos. Campos ausentes da lista vêm depois, na ordem original.
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

        // Comentários precedentes
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

    // Campos na ordem preferida
    for key in order {
        if let Some(value) = fields.get(key.as_str()) {
            ordered.push((key.as_str(), value));
            seen.insert(key.as_str());
        }
    }

    // Campos restantes na ordem original
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
    /// Verificar arquivo .bib e reportar erros.
    Check {
        file: PathBuf,
    },
    /// Reformatar arquivo .bib.
    Format {
        file: PathBuf,
        /// Sobrescrever o arquivo original (default: imprimir em stdout).
        #[arg(long)]
        in_place: bool,
    },
    /// Estatísticas do arquivo.
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
            if enc.had_bom { " (com BOM)" } else { "" }
        );
    }
    if !enc.lossy.is_empty() {
        eprintln!("{} conversões lossy", enc.lossy.len());
    }

    let result = bibrs::parser::Parser::parse(&enc.content);

    eprintln!("{} entries", result.bibliography.entries.len());
    eprintln!("{} @string macros", result.bibliography.strings.len());

    if result.errors.is_empty() {
        eprintln!("sem erros");
    } else {
        eprintln!("{} erros:", result.errors.len());
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
        eprintln!("formatado: {}", file.display());
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
    eprintln!("erros de parse: {}", result.errors.len());

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

## Testes — fixtures e roundtrip

Diretório `tests/fixtures/` deve conter `.bib` reais. Fontes:

- Exportação do JabRef com problemas conhecidos.
- Arquivos `.bib` de repositórios públicos de artigos.
- Arquivos sintetizados com edge cases: cite key numérica, entry sem campos, braces aninhados em profundidade, `@string` referenciando outro `@string`, encoding misto.

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
  author = {Incompleto,
  title = {Completo},

@article{key2,
  author = {Outro},
  year = {2024},
}
"#;
    let result = Parser::parse(input);
    // Deve recuperar e capturar key2 mesmo com key1 malformado
    assert!(!result.errors.is_empty());
    assert!(result.bibliography.entries.iter().any(|e| e.cite_key == "key2"));
}

#[test]
fn missing_cite_key() {
    let input = "@article{, author = {Ninguém}}";
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
    // Ambas devem ser preservadas — dedup é responsabilidade de outra camada
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
    let result = Parser::parse("% isto é um comentário\n% outro\n");
    assert!(result.bibliography.entries.is_empty());
}
```

## Critérios de conclusão do Alicerce

1. `cargo test` passa com zero falhas.
2. `cargo clippy` sem warnings.
3. `cargo doc --no-deps` gera documentação sem erros. Todos os items públicos têm `///` doc comments.
4. `bibrs check <arquivo.bib>` processa pelo menos 3 arquivos `.bib` reais (>100 entries cada) sem panic.
5. `bibrs format <arquivo.bib>` produz output válido que sobrevive ao roundtrip.
6. Arquivos com encoding Latin-1 e Windows-1252 são detectados e convertidos.
7. Arquivos malformados (brace não fechada, campo sem valor, lixo entre entries) são processados com erros reportados e máximo de dados extraído.

Após isso — e somente após — inicia-se a Estrutura.

---

# ESTRUTURA

Não implementar até o Alicerce estar completo. O que segue é especificação, não código final.

## Escopo

- Normalização de nomes de autores.
- Normalização de campos (DOI, pages, ISSN, ISBN, year, title).
- Detecção de duplicatas.
- Geração de cite keys.
- Integração com APIs externas (CrossRef, OpenAlex, Google Books, OpenLibrary).
- Cache de respostas de APIs em disco.
- Arquivo de configuração do usuário (TOML).
- Logging estruturado (`tracing`).

## Quando dividir em crates

Ao iniciar a Estrutura, o projeto muda de topologia. APIs externas puxam `reqwest`, `tokio`, `async-trait`, `futures` — dependências pesadas que não devem contaminar o core. Neste momento, extrair:

```
bibrs/
├── Cargo.toml              # workspace
├── crates/
│   ├── bibrs-core/         # o que hoje é o crate único (model, parser, serializer, encoding)
│   ├── bibrs-normalize/    # nomes, campos, dedup, cite keys
│   └── bibrs-sources/      # trait BibSource + implementações por API
├── src/
│   └── main.rs             # CLI unificada (depende de todos os crates)
└── frontend/
```

A extração é mecânica: mover `src/*.rs` para `crates/bibrs-core/src/`, ajustar `use`, verificar que `cargo test --workspace` continua passando. Não alterar lógica neste passo.

## Normalização — especificação funcional

### Nomes de autores

Entrada: campo `author` ou `editor` como string BibTeX.
Saída: `Vec<PersonName>` com partes decompostas.

```
"Last, First"                    → { first: "First", last: "Last" }
"First Last"                     → { first: "First", last: "Last" }
"First Middle Last"              → { first: "First Middle", last: "Last" }
"Last, Jr., First"               → { first: "First", last: "Last", jr: "Jr." }
"{Institutional Name}"           → { last: "Institutional Name", is_institutional: true }
"van der Berg, Jan"              → { first: "Jan", von: "van der", last: "Berg" }
"João da Silva and Maria Santos" → 2 PersonName
```

Regra de partículas (`von`): palavras que iniciam com minúscula entre o first e o last. Referência direta: algoritmo do `typst/biblatex` em `src/types/person.rs` e o algoritmo clássico do BibTeX (Oren Patashnik, seção "Name Parsing" da documentação original).

### Campos

| Campo | Transformação |
|---|---|
| `doi` | Remover prefixos (`https://doi.org/`, `doi:`, espaços). Resultado: `10.xxxx/yyyy`. |
| `pages` | Normalizar hífens: `15-20` → `15--20`, `15 – 20` → `15--20`. |
| `issn` | Remover hífens, validar dígito verificador (mod 11). |
| `isbn` | Remover hífens, validar ISBN-10 (mod 11) e ISBN-13 (mod 10). |
| `year` | Extrair 4 dígitos: `"2023a"` → `"2023"`, `"(2023)"` → `"2023"`. |
| `title` | Detectar ALL CAPS → converter para title case. Proteger acrônimos com `{...}`. |

### Duplicatas

Estratégia em duas camadas:

1. **Match exato por DOI.** O(n) com HashSet. Resolve a maioria.
2. **Match fuzzy por título normalizado.** Lowercase, remover pontuação, remover stopwords, comparar com Jaro-Winkler ou coeficiente de Jaccard. Threshold configurável (default: 0.90).

Resultado: `Vec<DuplicateGroup>` com índices, confiança e razão.

### Cite keys

Padrão configurável: `{auth}{year}{shorttitle}`.
- `{auth}`: sobrenome do primeiro autor, lowercase, sem acentos.
- `{year}`: campo year.
- `{shorttitle}`: primeira palavra significativa do título (>3 letras), lowercase.

Sufixo de dedup: `a`, `b`, `c`... quando keys colidem.

## APIs externas — especificação funcional

### Trait comum

```rust
#[async_trait]
pub trait BibSource: Send + Sync {
    fn id(&self) -> &'static str;
    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, SourceError>;
    async fn fetch_by_id(&self, id: &str) -> Result<Option<SearchResult>, SourceError>;
}
```

### Mapeamento por API

**CrossRef** (`https://api.crossref.org/works`)
- Busca por query, DOI, autor.
- `type` → `EntryType`: `"journal-article"` → Article, `"book"` → Book, etc.
- `author[].given` + `author[].family` → campo `author` BibTeX.
- `container-title[0]` → `journal`.
- `published.date-parts[0][0]` → `year`.
- Header `mailto` para polite pool.

**OpenAlex** (`https://api.openalex.org/works`)
- Busca por título, DOI, ORCID.
- `authorships[].author.display_name` → campo `author`.
- `primary_location.source.display_name` → `journal`.
- `publication_year` → `year`.
- Filtros: `filter=doi:`, `filter=title.search:`.

**Google Books** (`https://www.googleapis.com/books/v1/volumes`)
- Busca por título, autor, ISBN.
- `volumeInfo.authors[]` → campo `author`.
- `volumeInfo.publisher` → `publisher`.
- `volumeInfo.industryIdentifiers[]` → `isbn`.
- Entry type: sempre `Book` ou `InBook`.

**OpenLibrary** (`https://openlibrary.org/search.json`, `https://openlibrary.org/isbn/`)
- Busca por título, autor, ISBN.
- `author_name[]` → campo `author`.
- `publisher[]` → `publisher`.
- `isbn[]` → `isbn`.
- Complementa Google Books para edições menos comuns.

### Rate limiting

| API | Limite | Estratégia |
|---|---|---|
| CrossRef | 50 req/s (polite) | Header `mailto` + sleep se necessário |
| OpenAlex | 10 req/s (sem key) | Sleep entre requests |
| Google Books | ~1000 req/dia | Cache agressivo |
| OpenLibrary | Sem limite formal | Throttle 1 req/s por cortesia |

### Cache de respostas

Diretório de cache em `~/.cache/bibrs/` (via crate `dirs`). Estrutura:

```
~/.cache/bibrs/
├── crossref/
│   ├── doi/                  # um arquivo JSON por DOI: 10.1000_xyz.json
│   └── search/               # hash SHA-256 da query como nome do arquivo
├── openalex/
│   ├── doi/
│   └── search/
├── google_books/
│   └── isbn/
└── openlibrary/
    └── isbn/
```

Cada arquivo contém o JSON bruto da resposta + timestamp. TTL configurável (default: 7 dias para buscas, 30 dias para fetch por DOI/ISBN). Cache é opcional — flag `--no-cache` na CLI, campo `cache.enabled` no config.

### Configuração do usuário

Arquivo `~/.config/bibrs/config.toml` (via crate `dirs`). Carregado na inicialização, sobrescrito por flags da CLI. Defaults razoáveis para tudo — o arquivo é opcional.

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
mailto = ""                          # email para polite pool do CrossRef
default_sources = ["crossref", "openalex"]

[cache]
enabled = true
ttl_search_days = 7
ttl_id_days = 30
```

Implementação: struct `Config` com `#[derive(Deserialize)]`, preenchido com defaults via `Default` impl, merge com TOML via `toml` crate.

### Logging

`tracing` + `tracing-subscriber` com filtro por nível via variável de ambiente `BIBRS_LOG` (padrão: `warn`).

```
BIBRS_LOG=info bibrs search --source crossref --doi 10.1000/xyz
```

Logs estruturados para: chamadas HTTP (URL, status, tempo), erros de normalização, duplicatas encontradas, cache hits/misses. Sem logging no Alicerce — `eprintln!` na CLI é suficiente naquela camada.

### Testes com mocks HTTP

Testes unitários de `bibrs-sources` usam `wiremock` para simular respostas das APIs. Fixtures JSON com respostas reais capturadas em `tests/fixtures/api/`. Testes de integração (rede real) separados via feature flag:

```toml
[features]
integration = []
```

```
cargo test                                    # apenas mocks
cargo test --features integration             # inclui chamadas reais (respeita rate limiting)
```

### Dependências adicionais (apenas para bibrs-sources)

```toml
reqwest = { version = "0.12", features = ["json"] }
tokio = { version = "1", features = ["full"] }
async-trait = "0.1"
futures = "0.3"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
toml = "0.8"
dirs = "6"
sha2 = "0.10"                   # hash de queries para cache
wiremock = "0.6"                # dev-dependency para mocks HTTP
```

## Critérios de conclusão da Estrutura

1. `cargo test --workspace` passa (mocks, sem rede).
2. `cargo doc --workspace --no-deps` sem erros. Items públicos documentados.
3. Normalização de nomes correta para corpus de ≥100 nomes reais, extraídos de `.bib` públicos. Corpus inclui: partículas (`von`, `de`, `van der`), sufixos (`Jr.`, `III`), nomes institucionais, formatos `First Last` e `Last, First` misturados.
4. Normalização de campos funcional para DOI, pages, ISSN, ISBN, year.
5. Detecção de duplicatas encontra pares conhecidos em arquivo de teste.
6. Pelo menos CrossRef e OpenAlex retornam resultados parseáveis e importáveis como `Entry`.
7. Cache em disco funcional: segunda busca idêntica não faz requisição HTTP.
8. `~/.config/bibrs/config.toml` lido e aplicado. Defaults funcionam sem arquivo.
9. `BIBRS_LOG=debug` produz logs estruturados das chamadas HTTP.
10. CLI estendida: `bibrs normalize <file>`, `bibrs search --source crossref --doi 10.xxx/yyy`.

---

# SUPERFÍCIE

Não implementar até a Estrutura estar completa.

## Escopo

- Servidor HTTP local (`axum`).
- API REST interna.
- Frontend web (HTML/CSS/JS).
- Fluxo integrado: abrir → navegar → buscar → importar → normalizar → salvar.
- Gestão de arquivos associados (campo `file`, link para PDFs).

## Quando criar o crate do servidor

Ao iniciar a Superfície, adicionar ao workspace:

```
crates/
└── bibrs-server/       # depende de bibrs-core, bibrs-normalize, bibrs-sources
```

## Servidor — especificação

Stack: `axum` + `tokio` + `tower-http` (ServeDir, CORS).

Estado: `Arc<RwLock<AppState>>` com `Bibliography` em memória, caminho do arquivo, fontes configuradas, flag dirty.

Frontend servido como arquivos estáticos em `/frontend/`.

### Endpoints

```
GET    /api/entries                  listar (paginação, filtros)
GET    /api/entries/:key             detalhes
POST   /api/entries                  adicionar
PUT    /api/entries/:key             atualizar
DELETE /api/entries/:key             remover

POST   /api/import                   importar de .bib (upload)
GET    /api/export                   exportar .bib (download)

POST   /api/search                   buscar em APIs externas
POST   /api/search/:source           buscar em API específica

POST   /api/normalize/entry/:key     normalizar entry
POST   /api/normalize/all            normalizar tudo
GET    /api/duplicates               detectar duplicatas

POST   /api/file/save                salvar em disco
POST   /api/file/open                abrir de disco

GET    /api/stats                    estatísticas
```

### Frontend

HTML + CSS + JS puro. Sem framework. Sem build step. Módulos ES6 (`import`/`export`) desde o início para manter organização:

```
frontend/
├── index.html
├── css/
│   └── main.css
└── js/
    ├── app.js           # inicialização, roteamento de estado
    ├── api.js           # fetch wrapper para /api/*
    ├── entries.js       # lista e filtros
    ├── editor.js        # edição de entry
    └── search.js        # busca em APIs externas
```

Layout:
- Painel esquerdo: lista de entries (filtrável, ordenável).
- Painel direito: detalhe/editor da entry selecionada.
- Painel inferior: busca em APIs, resultados com botão "importar".
- Barra de status: contagem de entries, erros, duplicatas, flag dirty.

Se JS puro ultrapassar ~2000 linhas e a organização se degradar, migrar para Alpine.js ou htmx. Não antes.

## Critérios de conclusão da Superfície

1. `cargo test --workspace` passa.
2. `cargo doc --workspace --no-deps` sem erros.
3. `http://localhost:3000` abre a interface.
4. Carregar `.bib`, navegar entries, editar campos, salvar — funcional.
5. Buscar por título/DOI em CrossRef ou OpenAlex, importar resultado.
6. Normalizar entry individual ou todas, com feedback visual.
7. Detectar e exibir duplicatas.
8. Campo `file` com link para PDF associado.
9. Autosave ou indicador de alterações não salvas.
10. Configuração do usuário (`config.toml`) refletida na interface (ordem de campos, formato de nomes, etc.).

---

# Sequência de trabalho

```
ALICERCE   model → error → encoding → parser (nom) → serializer → CLI → testes → doc
           Resultado: bibrs check|format|stats funcional.

ESTRUTURA  extrair workspace → config (TOML) → normalize (nomes, campos, dedup, keys) → sources (APIs + cache + logging) → mocks → CLI estendida
           Resultado: bibrs normalize|search funcional. Cache, config e logging operacionais.

SUPERFÍCIE server → endpoints → frontend (ES6 modules) → fluxo integrado
           Resultado: GUI web local funcional.
```

Cada camada é fechada antes de abrir a próxima. Sem exceções.

