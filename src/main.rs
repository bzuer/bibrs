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
            if enc.had_bom { " (with BOM)" } else { "" }
        );
    }
    if !enc.lossy.is_empty() {
        eprintln!("{} lossy conversions", enc.lossy.len());
    }

    let result = bibrs::parser::Parser::parse(&enc.content);

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

fn cmd_format(file: &PathBuf, in_place: bool) -> Result<(), bibrs::error::BibrsError> {
    let bytes = std::fs::read(file)?;
    let enc = bibrs::encoding::detect_and_convert(&bytes);
    let result = bibrs::parser::Parser::parse(&enc.content);

    let config = bibrs::serializer::SerializeConfig::default();
    let output = bibrs::serializer::serialize(&result.bibliography, &config);

    if in_place {
        std::fs::write(file, &output)?;
        eprintln!("formatted: {}", file.display());
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
    eprintln!("parse errors: {}", result.errors.len());

    let counts = bib.count_by_type();
    for (t, n) in &counts {
        eprintln!("  @{}: {}", t, n);
    }

    Ok(())
}
