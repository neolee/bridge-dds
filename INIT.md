# Vision And Architecture

## Purpose

`bridge-dds` helps bridge players analyze a board under `double-dummy` conditions. Given all four hands, the tool answers what each declarer can make in each strain, what the par result is, and later what the optimal continuation is after a partial play trace.

## Scope

### Core Features

1. `Double-dummy` solving. Given a `PBN` record, output the maximum number of tricks for every declarer and strain. The primary result is a `4x5` matrix: declarers `N`, `E`, `S`, `W` by strains `S`, `H`, `D`, `C`, `NT`.
2. `Par` calculation. From the `20` double-dummy results, compute the theoretical par contract and score using `DDS` `DealerPar`, including vulnerability, doubles, game bonuses, slam bonuses, and sacrifices.
3. `Mid-hand` analysis. Given a `PBN` play trace, evaluate the play and compute the optimal continuation from the current position.
4. `Batch` analysis. Process multiple boards from a `PBN` file and produce aggregate statistics.

### Non-Features

- No bidding-system comparison.
- No interactive `TUI`; all `CLI` output is plain text and suitable for piping.
- No online multiplayer or user accounts.

## Engine

The project uses Bo Haglund's `DDS` `C` library, version `2.9.0`, licensed under `Apache-2.0`. The `Phase 1` functions are `CalcDDtablePBN`, `DealerPar`, `SetMaxThreads`, and `ErrorMessage`. `Phase 1b` may additionally use `AnalysePlayPBN` for play-trace evaluation and `SolveBoardPBN` for continuation analysis.

The project does not rewrite the `DDS` engine. It binds to the small required `C` API through hand-written `Rust` `FFI`, verified against `engine/dds/include/dll.h`. All solver inputs use `PBN` string formats such as `ddTableDealPBN` and `dealPBN`; the project does not use the binary `deal.remainCards` interface.

## Input Model

`PBN` is the canonical input format. The accepted subset is defined in `phases/pbn-input-contract.md`. The `CLI` accepts one `PBN` record on `stdin`, not separate command-line options for fields already present in `PBN`.

For `Phase 1a`, a valid input record must include:

- `Deal`: the card layout in `PBN` deal-tag format.
- `Dealer`: the dealer direction, used as the `dealer` argument to `DealerPar`.
- `Vulnerable`: the vulnerability, mapped to the `DDS` vulnerability encoding.

The `Deal` tag's `<first>` direction is only the first hand listed in the deal string. In `PBN` import format it is not necessarily the dealer. The implementation must not infer `Dealer` from `Deal`.

## Architecture

- `Rust` is used for the `CLI`, shared library, and later `Web` server.
- A single crate exposes shared domain logic from `src/lib.rs`.
- The `bridge` binary is the `CLI`.
- The later `bridge-server` binary will expose the same operations through `REST` endpoints.
- `DDS` build output is copied to `engine/dds/lib/libdds.a`, and `build.rs` links against that stable path.
- The `macOS` `DDS` build disables `DDS_THREADS_BOOST` and uses `DDS_THREADS_GCD` plus `DDS_THREADS_STL`.

The core library contains:

- Hand-written `FFI` bindings for `DDS`.
- Safe wrappers around `CalcDDtablePBN`, `DealerPar`, `AnalysePlayPBN`, and `SolveBoardPBN`.
- `PBN` record parsing for `Deal`, `Dealer`, `Vulnerable`, and later `Play`.
- Domain types for cards, hands, directions, strains, trick tables, and par results.
- Text and `JSON` response types shared by the `CLI` and later `REST` API where practical.

## Distribution

Both binaries should compile to standalone executables with no required runtime service. The `React` build for the later `Web` UI will be embedded into `bridge-server` via `rust-embed`.

## `CLI` Design Principles

- `PBN` in, text or `JSON` out.
- One board is read from `stdin`.
- No interactive prompts.
- The default output is readable plain text.
- `--format json` provides machine-readable output.
- The `CLI` is a thin wrapper around the shared library.

## Dependencies

- `dds`: `C` double-dummy solver and par engine.
- `clap`: `CLI` argument parsing.
- `serde` and `serde_json`: structured output.
- `thiserror`: library error types.
- `axum`: later `REST` server.
- `rust-embed`: later static frontend embedding.
- `React` and `Vite`: later frontend UI.

## Design Decisions

### `PBN` Tags

The project treats `Dealer` and `Vulnerable` as standard `PBN` data. The `CLI` should not add `--dealer` or `--vul` options unless a later compatibility mode explicitly needs them.

### `DDS` `SolveBoardPBN` Semantics

The exact result semantics for `SolveBoardPBN` in continuation analysis must be pinned by tests before `Phase 1b` is considered complete.

### Play-Trace Analysis

`AnalysePlayPBN` is the preferred interface for evaluating a supplied play trace because it returns values before and after played cards. `SolveBoardPBN` remains the interface for asking what to play from the derived current position.

### Frontend Serving

During development, `bridge-server` may proxy unmatched routes to the `Vite` dev server. In release builds, the `React` output is embedded with `rust-embed`.
