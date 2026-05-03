# Vision & Architecture

## Purpose

A tool for bridge enthusiasts to determine the optimal contract and line of play for a given deal under double-dummy conditions. The core solves the question: *given all four hands are known, what is the best result both sides can achieve with perfect play and perfect defense?*

## Scope

### Core features

1. **Double-dummy solving.** Given a deal, output the maximum number of tricks each declarer can win in each strain (`S`, `H`, `D`, `C`, `NT`). This yields a 20-entry tricks matrix (5 strains x 4 declarers, though NS and EW positions are symmetric).
2. **Par calculation.** From the 20 tricks results, compute the theoretical par contract and score, accounting for vulnerability, doubles, and game/slam bonuses.
3. **Mid-hand analysis.** Given a partial play trace (the first `k` tricks already played), compute the remaining optimal line of play from the current position. This includes validating the legality of the play trace and deriving the residual state.
4. **Batch analysis.** Process multiple deals from a `PBN` file and produce aggregate statistics.

### Non-features (deliberate exclusions)

- Bidding system comparison.
- Interactive TUI; all CLI output is plain text, suitable for piping.
- Online multiplayer or user accounts.

## Engine

We use Bo Haglund's [dds](https://github.com/dds-bridge/dds) C library (v2.9.0, Apache 2.0). It provides `CalcDDtablePBN` for full-deal solving, `DealerPar` for par calculation, and `SolveBoardPBN` for mid-hand analysis. All DDS communication uses PBN string format (`dealPBN` / `ddTableDealPBN`), avoiding the binary `deal.remainCards` format entirely. The library has been battle-tested for decades and is used by BBO and Bridge Solver Online.

We will **not** rewrite the DDS engine. We bind to the C library via hand-written Rust FFI (no `bindgen`).

## Architecture

- **Single language:** Rust for both CLI and Web Server.
- **Single crate, two binary targets:** `bridge` (CLI) and `bridge-server` (Web API + embedded front-end). Both consume a shared `lib.rs` that exposes the core domain logic.
- **Core library (`lib.rs`)** contains: hand-written FFI bindings to `dds` (verified against `dll.h`), safe Rust wrappers around `CalcDDtablePBN` / `DealerPar` / `SolveBoardPBN`, PBN parsing/writing, Play Trace parsing, tricks matrix computation, and Par result type.
- **CLI** accepts PBN or JSON on `stdin` (or as a positional argument), produces a tricks matrix and Par result on stdout. `--format json` for machine consumption; default is a human-readable table.
- **Web Server** wraps the same library functions in REST endpoints (`POST /api/solve`, `POST /api/analyze`). The React SPA front-end is embedded in the server binary via `rust-embed` so that deployment requires only a single file.

## Distribution

Both binaries compile to standalone executables with zero runtime dependencies. Deployment is:

```
scp bridge bridge-server user@host:~
ssh user@host './bridge-server &'
```

No Docker, no Python runtime, no `node_modules`. The React static build is compiled into the server binary.

## CLI Design Principles

- Text in, text out. No TUI, no interactive prompts.
- Default output is human-readable and well-formatted.
- `--format json` provides machine-parseable output for scripting or the Web front-end.
- The CLI is a thin wrapper around the core library — it is an API client that happens to run in the terminal.

## Key External Dependencies

| Dependency | Role |
|---|---|
| `dds` (C library) | Double-dummy solving and par calculation engine |
| `clap` | CLI argument parsing |
| `serde` / `serde_json` | JSON serialization |
| `axum` | HTTP server (Phase 2) |
| `rust-embed` | Embed static front-end assets (Phase 2) |
| React + Vite | Frontend UI (Phase 3) |

DDS is compiled separately via its own platform Makefile (e.g. `Makefile_Mac_clang_static`). The project root `Makefile` orchestrates DDS compilation before `cargo build`.

## Input / Output Formats

- **PBN** is the canonical interchange format, both for import and export.
- **Play Trace** follows the PBN play record specification.
- Internally, hands are represented as 52-bit masks (a Rust `Hand` newtype wrapping `u64`). This bit layout is our own convention; it is not aligned with DDS's binary `remainCards` format, which is never used directly. All DDS communication uses PBN strings.

## Design Decisions

### DDS `SolveBoard` return-value semantics

The exact semantics (remaining tricks vs. total tricks including already-won) will be determined by reading the DDS source code and documentation once the library is cloned into `engine/`. No guesswork; a unit test will pin the observed behavior.

### Frontend serving: dev vs. production

- **Development / testing:** The server proxies unmatched routes to the Vite dev server (`localhost:5173`), giving the front-end hot-module replacement during development.
- **Production:** The React build output is embedded into the server binary via `rust-embed`.
- The switch is driven by `#[cfg(debug_assertions)]` — debug builds proxy, release builds embed.

### Play-trace validation

User-supplied play traces validation is designed as a standalone, swappable module. For v1.0:

- **Hard errors:** Card not held by the claimed player; same card played twice.
- **Warnings / best-effort:** Follow-suit violations and trick-winner miscalculations produce a warning but do not block analysis.
- The module boundary allows swapping in a stricter rule engine later without touching the solver.
