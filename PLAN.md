# Project Plan

## Phase 1 - `CLI`

`Phase 1` delivers the shared engine and a complete command-line tool. Every feature in this phase has focused unit tests and functional tests.

### `1a` - Full-Deal `DDS` Evaluation

- [ ] Set up the `Rust` project structure: `Cargo.toml`, `src/lib.rs`, `src/cli/`, and `build.rs`.
- [ ] Keep `dds` as a git submodule under `engine/dds`.
- [ ] Build `DDS` with its platform `Makefile`, then copy the static library to `engine/dds/lib/libdds.a`.
- [ ] Add hand-written `FFI` declarations in `src/dds/ffi.rs`, verified against `engine/dds/include/dll.h`.
- [ ] Build a safe wrapper around `CalcDDtablePBN`, `DealerPar`, `SetMaxThreads`, and `ErrorMessage` in `src/dds/`.
- [ ] Implement `PBN` record parsing for `Deal`, `Dealer`, and `Vulnerable`.
- [ ] Implement `Deal`, `Hand`, `Direction`, `Strain`, `Suit`, `Rank`, and `Card` domain types.
- [ ] Implement the full `4x5` tricks matrix, preserving all `20` declarer-and-strain results.
- [ ] Implement `DealerPar` calculation from the tricks matrix using parsed `Dealer` and `Vulnerable` tags.
- [ ] Add the `bridge solve` command: accept a `PBN` record from a file, argument, or `stdin`; output the tricks matrix and par result as text or `JSON`.

### `1b` - Mid-Hand Analysis

- [ ] Implement `PBN` `Play` tag parsing.
- [ ] Implement play-trace legality validation.
- [ ] Use `AnalysePlayPBN` to evaluate the supplied play trace.
- [ ] Derive the residual state: remaining cards, leader, and current trick.
- [ ] Use `SolveBoardPBN` to return continuation analysis from the residual state.
- [ ] Extend `bridge solve` with play-trace analysis output when a `Play` tag is present.

## Phase 2 - `REST` API Service

`Phase 2` wraps the shared library in a `REST` API. The request model should remain compatible with the `CLI` by accepting `PBN` as the primary input.

- [ ] Set up an `axum` server with `POST /api/solve` and `POST /api/analyze`.
- [ ] Share request and response types between the `CLI` and server where practical.
- [ ] Define error responses for invalid `PBN`, missing `Dealer`, missing `Vulnerable`, and `DDS` failures.
- [ ] Decide and implement the development frontend-serving strategy.

## Phase 3 - `Web` Frontend

`Phase 3` adds a graphical interface that communicates with the `REST` API.

- [ ] Scaffold `React` with `Vite` in `web/`.
- [ ] Add board input through `PBN` paste and manual card selection.
- [ ] Render the full `4x5` tricks matrix.
- [ ] Render the `DealerPar` result.
- [ ] Add a play-trace viewer with replay and suggested continuations.

## Phase 4 - Polish

- [ ] Add batch analysis for multi-board `PBN` files.
- [ ] Embed the `React` build output into `bridge-server` with `rust-embed`.
- [ ] Add cross-compilation targets for `x86_64-linux`, `aarch64-linux`, and `x86_64-macos`.
- [ ] Add `CI` for build, tests, formatting, linting, and basic `CLI` fixtures.
- [ ] Finalize `README.md` and usage documentation.
- [ ] Prepare a `v1.0` release.
