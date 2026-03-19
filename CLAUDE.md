# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**bibrs** is a BibTeX bibliography manager written in Rust. It reads `.bib` files, reports problems, normalizes data, queries external APIs, and serves a local web GUI. The `.bib` file is the native format — no intermediate database.

The project is also a Rust learning vehicle, structured so each layer exercises progressively deeper Rust concepts (ownership, composition, traits, async, systems architecture).

The full project plan lives in `projeto_bibrs.md`. All design decisions, API specs, and acceptance criteria are defined there. Read it before making architectural choices.

## Three-Layer Architecture

Each layer must be complete before starting the next. No exceptions.

1. **Foundation:** Parser (`nom`), model, serializer, encoding detection, CLI. No network, no async. Output: `bibrs check|format|stats`.
2. **Structure:** Workspace split into crates. Author/field normalization, dedup, cite key generation, external API clients (CrossRef, OpenAlex, Google Books, OpenLibrary), disk cache, TOML config, `tracing` logging. Output: `bibrs normalize|search`.
3. **Surface:** `axum` HTTP server, REST API, vanilla JS frontend (ES6 modules, no framework, no build step). Three-panel layout at `localhost:3000`.

### Implementation Order

```
Foundation  model → error → encoding → parser (nom) → serializer → CLI → tests → docs
Structure   extract workspace → config (TOML) → normalize → sources (APIs + cache + logging) → mocks → extended CLI
Surface     server → endpoints → frontend (ES6 modules) → integrated flow
```

## Key Design Principles

- **The parser never aborts.** `ParseResult` always contains a `Bibliography` (possibly incomplete). Parse errors are collected in `errors: Vec<ParseError>`, never propagated as `Result::Err`. Only I/O and irrecoverable encoding failures are fatal (`BibrsError`).
- **Roundtrip fidelity.** `FieldValue` is an enum (Literal, Integer, StringRef, Concat) to preserve macros and concatenations. `IndexMap` preserves field order. `leading_comments` on entries preserves surrounding comments. Serialized output must survive parse→serialize→parse.
- **Unknown data is never discarded.** `EntryType::Other(String)` catches unknown entry types. Duplicate cite keys are kept (dedup is a separate concern).
- **Tolerant encoding.** Pipeline: BOM detection → UTF-8 strict → `chardetng` fallback → `encoding_rs` conversion. Lossy positions are recorded, never cause abort.
- **Split only when justified.** `parser.rs` stays as a single file until complexity demands splitting into `parser/mod.rs`, `parser/combinators.rs`, `parser/recovery.rs`.

## Build & Test Commands

```bash
cargo build
cargo test
cargo test --features integration    # real API calls (Structure layer+)
cargo test --workspace               # all crates (after workspace split)
cargo clippy                         # must pass with zero warnings
cargo doc --no-deps                  # must build without errors
cargo run -- check <file.bib>
cargo run -- format <file.bib>
cargo run -- format --in-place <file.bib>
cargo run -- stats <file.bib>
cargo run -- normalize <file.bib>    # Structure layer+
cargo run -- search --source crossref --doi 10.xxx/yyy  # Structure layer+
```

## Dependencies (Foundation)

`nom` 8 (parser), `indexmap` 2 (ordered maps), `encoding_rs` + `chardetng` (encoding), `clap` 4 (CLI), `thiserror` 2 (errors), `serde` 1 (serialization), `unicode-normalization` (text). Zero async dependencies at this layer.

## Project Layout

Foundation (single crate):
```
bibrs/
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── lib.rs
│   ├── model.rs
│   ├── encoding.rs
│   ├── parser.rs
│   ├── serializer.rs
│   └── error.rs
└── tests/
    ├── roundtrip.rs
    ├── recovery.rs
    ├── encoding.rs
    └── fixtures/
```

After workspace split (Structure layer):
```
bibrs/
├── Cargo.toml              # workspace
├── crates/
│   ├── bibrs-core/         # model, parser, serializer, encoding, errors
│   ├── bibrs-normalize/    # names, fields, dedup, cite keys
│   ├── bibrs-sources/      # trait BibSource + API clients + cache
│   └── bibrs-server/       # axum server (Surface layer)
├── src/main.rs             # unified CLI
├── frontend/               # static HTML/CSS/JS (Surface layer)
└── tests/fixtures/
```

## Code Style Rules

- **English everywhere.** All code, metadata, documentation, UI text, component names, variables, tests, commits, and PRs in technical English.
- **No inline comments.** Do not add `//` comments or annotations in source code. When modifying files, remove any inline comments in the edited sections. Doc comments (`///`) on public items are required — they are API documentation, not inline comments.
- **Conventional Commits.** `feat:`, `fix:`, `chore:`, etc. Short and imperative.
- **PRs.** English description, linked issues, screenshots for visual changes, tests and docs updated.
- **Commit at the end of every session or after significant changes.**
