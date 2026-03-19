# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**bibrs** is a BibTeX bibliography manager written in Rust. It reads `.bib` files, reports problems, normalizes data, queries external APIs, and will serve a local web GUI. The `.bib` file is the native format — no intermediate database.

The project is also a Rust learning vehicle, structured so each layer exercises progressively deeper Rust concepts (ownership, composition, traits, async, systems architecture).

The full project plan lives in `README.md`. All design decisions, API specs, and acceptance criteria are defined there. Read it before making architectural choices.

## Current State

- **Foundation:** Complete. Parser, model, serializer, encoding, CLI (`check|format|stats`).
- **Structure:** Complete. Workspace with 3 crates, normalization, external APIs, cache, config, CLI (`normalize|search`).
- **Surface:** Not started. `axum` HTTP server, REST API, vanilla JS frontend.

100 tests passing, zero clippy warnings, cargo doc clean. Tested against a real 5544-entry `.bib` file.

## Three-Layer Architecture

Each layer must be complete before starting the next. No exceptions.

1. **Foundation:** Parser (`nom`), model, serializer, encoding detection, CLI. No network, no async. Output: `bibrs check|format|stats`.
2. **Structure:** Workspace split into crates. Author/field normalization, dedup, cite key generation, external API clients (CrossRef, OpenAlex, Google Books, OpenLibrary), disk cache, INI config, `tracing` logging. Output: `bibrs normalize|search`.
3. **Surface:** `axum` HTTP server, REST API, vanilla JS frontend (ES6 modules, no framework, no build step). Three-panel layout at `localhost:3000`.

### Implementation Order

```
Foundation  model → error → encoding → parser (nom) → serializer → CLI → tests → docs
Structure   extract workspace → config (INI) → normalize → sources (APIs + cache + logging) → mocks → extended CLI
Surface     server → endpoints → frontend (ES6 modules) → integrated flow
```

## Key Design Principles

- **The parser never aborts.** `ParseResult` always contains a `Bibliography` (possibly incomplete). Parse errors are collected in `errors: Vec<ParseError>`, never propagated as `Result::Err`. Only I/O and irrecoverable encoding failures are fatal (`BibrsError`).
- **Roundtrip fidelity.** `FieldValue` is an enum (Literal, Integer, StringRef, Concat) to preserve macros and concatenations. `IndexMap` preserves field order. `leading_comments` on entries preserves surrounding comments. Serialized output must survive parse->serialize->parse.
- **Unknown data is never discarded.** `EntryType::Other(String)` catches unknown entry types. Duplicate cite keys are kept (dedup is a separate concern).
- **Tolerant encoding.** Pipeline: BOM detection -> UTF-8 strict -> `chardetng` fallback -> `encoding_rs` conversion. Lossy positions are recorded, never cause abort.
- **Unicode-safe string handling.** Name splitting and text processing use `str::get()` or char-aware iteration to avoid panics on multi-byte UTF-8 sequences.

## Build & Test Commands

```bash
cargo build
cargo test --workspace               # all crates (100 tests)
cargo test --features integration    # real API calls
cargo clippy --workspace             # must pass with zero warnings
cargo doc --workspace --no-deps      # must build without errors
cargo run -- check <file.bib>
cargo run -- format <file.bib>
cargo run -- format --in-place <file.bib>
cargo run -- stats <file.bib>
cargo run -- normalize <file.bib>
cargo run -- search --source crossref --doi 10.xxx/yyy
cargo run -- search --source openalex --query "search terms"
```

## Project Layout

```
bibrs/
├── Cargo.toml                  # workspace root
├── src/main.rs                 # unified CLI (check, format, stats, normalize, search)
├── crates/
│   ├── bibrs-core/             # model, parser, serializer, encoding, error, config
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── model.rs        # EntryType, FieldValue, Entry, Bibliography
│   │   │   ├── error.rs        # ParseError, ParseResult, BibrsError
│   │   │   ├── encoding.rs     # detect_and_convert
│   │   │   ├── parser.rs       # nom combinators, tolerant parser
│   │   │   ├── serializer.rs   # SerializeConfig, serialize()
│   │   │   └── config.rs       # INI config (~/.config/bibrs/config.ini)
│   │   └── tests/
│   │       ├── roundtrip.rs
│   │       ├── recovery.rs
│   │       ├── encoding.rs
│   │       ├── real_file.rs    # integration tests with 5544-entry file
│   │       └── fixtures/
│   ├── bibrs-normalize/        # names, fields, dedup, cite keys
│   │   ├── src/
│   │   │   ├── names.rs        # PersonName, parse_authors()
│   │   │   ├── fields.rs       # DOI, pages, ISSN, ISBN, year, title
│   │   │   ├── dedup.rs        # DuplicateGroup, inverted-index fuzzy match
│   │   │   └── citekey.rs      # generate_cite_key(), collision suffixes
│   │   └── tests/
│   │       └── real_normalize.rs
│   └── bibrs-sources/          # external API clients + cache
│       ├── src/
│       │   ├── source.rs       # BibSource trait, SearchQuery, SearchResult
│       │   ├── crossref.rs     # CrossRef API
│       │   ├── openalex.rs     # OpenAlex API
│       │   ├── google_books.rs # Google Books API
│       │   ├── openlibrary.rs  # OpenLibrary API
│       │   └── cache.rs        # disk cache (~/.cache/bibrs/), SHA-256, TTL
│       └── tests/
│           ├── api_mock.rs     # wiremock tests
│           └── fixtures/api/
└── tests/
    └── bib.bib                 # real-world test fixture (5544 entries)
```

## Dependencies

### bibrs-core
`nom` 8, `indexmap` 2, `serde` 1, `thiserror` 2, `encoding_rs` 0.8, `chardetng` 0.1, `unicode-normalization` 0.1, `rust-ini` 0.21, `dirs` 6.

### bibrs-normalize
`bibrs-core`, `unicode-normalization` 0.1, `indexmap` 2.

### bibrs-sources
`bibrs-core`, `reqwest` 0.12 (json), `tokio` 1 (full), `serde` 1, `serde_json` 1, `tracing` 0.1, `dirs` 6, `sha2` 0.10, `indexmap` 2. Dev: `wiremock` 0.6.

### Root binary
`bibrs-core`, `bibrs-normalize`, `bibrs-sources`, `clap` 4 (derive), `tokio` 1 (full), `tracing-subscriber` 0.3 (env-filter).

## Configuration

INI format at `~/.config/bibrs/config.ini`. File is optional — all fields have defaults. Only an email is needed in `[sources]` for the CrossRef polite pool; none of the APIs require keys at test volumes.

## Code Style Rules

- **English everywhere.** All code, metadata, documentation, UI text, component names, variables, tests, commits, and PRs in technical English.
- **Comments policy.**
  - **Allowed:** `///` doc comments on public items (structs, enums, traits, functions, modules). These are structural documentation and are required — `cargo doc` must build without errors.
  - **Forbidden:** `//` inline comments, `/* */` block comments, explanatory annotations, TODO/FIXME markers, commented-out code. When modifying files, remove any such comments in the edited sections.
- **Conventional Commits.** `feat:`, `fix:`, `chore:`, etc. Short and imperative.
- **PRs.** English description, linked issues, screenshots for visual changes, tests and docs updated.
- **Commit at the end of every session or after significant changes.**
