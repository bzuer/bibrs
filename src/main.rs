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
    /// Verify .bib file and report errors.
    Check { file: PathBuf },
    /// Reformat .bib file.
    Format {
        file: PathBuf,
        /// Overwrite original file (default: print to stdout).
        #[arg(long)]
        in_place: bool,
    },
    /// File statistics.
    Stats { file: PathBuf },
    /// Normalize entries (authors, fields, cite keys).
    Normalize { file: PathBuf },
    /// Search external APIs for bibliographic data.
    Search {
        /// Source to query (crossref, openalex, google_books, openlibrary).
        #[arg(long)]
        source: String,
        /// DOI to look up.
        #[arg(long)]
        doi: Option<String>,
        /// Free-text query.
        #[arg(long)]
        query: Option<String>,
        /// Disable cache for this request.
        #[arg(long)]
        no_cache: bool,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_env("BIBRS_LOG")
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();

    match cli.command {
        Command::Check { file } => cmd_check(&file)?,
        Command::Format { file, in_place } => cmd_format(&file, in_place)?,
        Command::Stats { file } => cmd_stats(&file)?,
        Command::Normalize { file } => cmd_normalize(&file)?,
        Command::Search {
            source,
            doi,
            query,
            no_cache,
        } => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(cmd_search(&source, doi, query, no_cache))?;
        }
    }

    Ok(())
}

fn cmd_check(file: &PathBuf) -> Result<(), bibrs_core::error::BibrsError> {
    let bytes = std::fs::read(file)?;
    let enc = bibrs_core::encoding::detect_and_convert(&bytes);

    if enc.original != bibrs_core::encoding::DetectedEncoding::Utf8 {
        eprintln!(
            "encoding: {:?}{}",
            enc.original,
            if enc.had_bom { " (with BOM)" } else { "" }
        );
    }
    if !enc.lossy.is_empty() {
        eprintln!("{} lossy conversions", enc.lossy.len());
    }

    let result = bibrs_core::parser::Parser::parse(&enc.content);

    eprintln!("{} entries", result.bibliography.entries.len());
    eprintln!("{} @string macros", result.bibliography.strings.len());

    if result.errors.is_empty() {
        eprintln!("no errors");
    } else {
        eprintln!("{} errors:", result.errors.len());
        for e in &result.errors {
            eprintln!("  {}", e);
        }
    }

    Ok(())
}

fn cmd_format(file: &PathBuf, in_place: bool) -> Result<(), bibrs_core::error::BibrsError> {
    let bytes = std::fs::read(file)?;
    let enc = bibrs_core::encoding::detect_and_convert(&bytes);
    let result = bibrs_core::parser::Parser::parse(&enc.content);

    let config = bibrs_core::serializer::SerializeConfig::default();
    let output = bibrs_core::serializer::serialize(&result.bibliography, &config);

    if in_place {
        std::fs::write(file, &output)?;
        eprintln!("formatted: {}", file.display());
    } else {
        print!("{output}");
    }

    Ok(())
}

fn cmd_stats(file: &PathBuf) -> Result<(), bibrs_core::error::BibrsError> {
    let bytes = std::fs::read(file)?;
    let enc = bibrs_core::encoding::detect_and_convert(&bytes);
    let result = bibrs_core::parser::Parser::parse(&enc.content);
    let bib = &result.bibliography;

    eprintln!("entries: {}", bib.entries.len());
    eprintln!("@string: {}", bib.strings.len());
    eprintln!("@preamble: {}", bib.preambles.len());
    eprintln!("parse errors: {}", result.errors.len());

    let counts = bib.count_by_type();
    for (t, n) in &counts {
        eprintln!("  @{}: {}", t, n);
    }

    Ok(())
}

fn cmd_normalize(file: &PathBuf) -> Result<(), bibrs_core::error::BibrsError> {
    let config = bibrs_core::config::Config::load();

    let bytes = std::fs::read(file)?;
    let enc = bibrs_core::encoding::detect_and_convert(&bytes);
    let result = bibrs_core::parser::Parser::parse(&enc.content);
    let mut bib = result.bibliography;

    for entry in &mut bib.entries {
        if let Some(bibrs_core::model::FieldValue::Literal(doi)) =
            entry.fields.get("doi").cloned()
        {
            let normalized = bibrs_normalize::fields::normalize_doi(&doi);
            entry.fields.insert(
                "doi".to_string(),
                bibrs_core::model::FieldValue::Literal(normalized),
            );
        }

        if let Some(bibrs_core::model::FieldValue::Literal(pages)) =
            entry.fields.get("pages").cloned()
        {
            let normalized = bibrs_normalize::fields::normalize_pages(&pages);
            entry.fields.insert(
                "pages".to_string(),
                bibrs_core::model::FieldValue::Literal(normalized),
            );
        }

        if let Some(bibrs_core::model::FieldValue::Literal(year)) =
            entry.fields.get("year").cloned()
        {
            if let Some(normalized) = bibrs_normalize::fields::normalize_year(&year) {
                entry.fields.insert(
                    "year".to_string(),
                    bibrs_core::model::FieldValue::Literal(normalized),
                );
            }
        }

        if config.normalize.protect_acronyms {
            if let Some(bibrs_core::model::FieldValue::Literal(title)) =
                entry.fields.get("title").cloned()
            {
                let normalized = bibrs_normalize::fields::normalize_title(&title);
                entry.fields.insert(
                    "title".to_string(),
                    bibrs_core::model::FieldValue::Literal(normalized),
                );
            }
        }
    }

    let duplicates = bibrs_normalize::dedup::find_duplicates(&bib, config.dedup.fuzzy_threshold);

    let ser_config = bibrs_core::serializer::SerializeConfig::default();
    let output = bibrs_core::serializer::serialize(&bib, &ser_config);
    print!("{output}");

    if !duplicates.is_empty() {
        eprintln!("\n{} potential duplicate group(s):", duplicates.len());
        for group in &duplicates {
            let keys: Vec<&str> = group
                .indices
                .iter()
                .filter_map(|&i| bib.entries.get(i).map(|e| e.cite_key.as_str()))
                .collect();
            eprintln!(
                "  [{:.0}%] {} — {}",
                group.confidence * 100.0,
                keys.join(", "),
                group.reason
            );
        }
    }

    Ok(())
}

async fn cmd_search(
    source: &str,
    doi: Option<String>,
    query: Option<String>,
    _no_cache: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = bibrs_core::config::Config::load();

    use bibrs_sources::source::{BibSource, SearchQuery};

    let search_query = SearchQuery {
        doi: doi.clone(),
        query: query.clone(),
        title: query,
        max_results: 5,
        ..Default::default()
    };

    let results = match source {
        "crossref" => {
            let client = bibrs_sources::crossref::CrossRef::new(&config.sources.mailto);
            client.search(&search_query).await?
        }
        "openalex" => {
            let client = bibrs_sources::openalex::OpenAlex::new(&config.sources.mailto);
            client.search(&search_query).await?
        }
        "google_books" => {
            let client = bibrs_sources::google_books::GoogleBooks::new();
            client.search(&search_query).await?
        }
        "openlibrary" => {
            let client = bibrs_sources::openlibrary::OpenLibrary::new();
            client.search(&search_query).await?
        }
        other => {
            eprintln!("unknown source: {}", other);
            return Ok(());
        }
    };

    if results.is_empty() {
        eprintln!("no results found");
        return Ok(());
    }

    eprintln!("{} result(s) from {}", results.len(), source);

    let ser_config = bibrs_core::serializer::SerializeConfig::default();
    let mut bib = bibrs_core::model::Bibliography::new();
    for r in results {
        bib.entries.push(r.entry);
    }
    print!("{}", bibrs_core::serializer::serialize(&bib, &ser_config));

    Ok(())
}
