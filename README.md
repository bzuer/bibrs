# bibrs

A BibTeX bibliography manager written in Rust. Reads `.bib` files, reports problems, normalizes data, queries external APIs, and serves a local web interface. The `.bib` file remains the native format — no intermediate database, no lock-in.

## Build

```bash
cargo build
```

## Test

```bash
cargo test
```

## Lint

```bash
cargo clippy
cargo fmt --check
```

## Usage

```bash
bibrs check <file.bib>
bibrs format <file.bib>
bibrs format --in-place <file.bib>
bibrs stats <file.bib>
```

## Status

Foundation layer in progress: parser, model, serializer, encoding detection, CLI.
