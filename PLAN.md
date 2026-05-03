# Project Plan

## Phase 1 — CLI

Core engine and a complete command-line tool. Every feature in this phase has corresponding unit and functional tests.

### 1a. Full-deal DDS evaluation

- [ ] Set up Rust project structure (`Cargo.toml`, `src/lib.rs`, `src/cli/`, `build.rs`).
- [ ] Add `dds` C library as a git submodule under `engine/`.
- [ ] Configure `bindgen` in `build.rs` to generate FFI bindings for `dds.h`.
- [ ] Build a safe Rust wrapper around `SolveBoard` in `src/dds/`.
- [ ] Implement `Deal` and `Hand` types with PBN round-trip parsing and generation.
- [ ] Implement the 20-result tricks matrix computation.
- [ ] Implement Par calculation from the tricks matrix, given vulnerability.
- [ ] CLI `solve` sub-command: accept PBN or JSON on `stdin`, output tricks matrix + Par (table or JSON).

### 1b. Mid-hand analysis

- [ ] Implement PBN Play Trace parser.
- [ ] Implement play-trace legality validation.
- [ ] Implement residual state derivation (remaining cards, current leader, current trick state).
- [ ] Feed residual state to `SolveBoard` and return continuation analysis.
- [ ] CLI: extend `solve` with `--play` flag for mid-hand continuation.

## Phase 2 — API Service

Wrap the core library in a REST API, sharing the same `lib.rs`. Tests at the HTTP layer (integration tests against a running server).

- [ ] Set up `axum` server with `POST /api/solve` and `POST /api/analyze` endpoints.
- [ ] Request/response types shared between CLI and server.
- [ ] Decide and implement dev-mode front-end serving strategy (hot-reload vs. embedded).

## Phase 3 — Web Frontend

GUI layer, developed and tested independently from the backend. Communicates with the API service via HTTP.

- [ ] Scaffold React project with Vite in `web/`.
- [ ] Deal input component (manual card selection + PBN paste).
- [ ] Tricks matrix table view with color-coded optimal contracts.
- [ ] Par result card.
- [ ] Mid-hand continuation viewer with card-by-card replay and suggested plays.

## Phase 4 — Polish

- [ ] More features: - single-dummy Monte Carlo simulation, batch analysis (`bridge analyze`), etc. may or may not included in version 1.0.
- [ ] `rust-embed` the React build output into the server binary for single-file deployment.
- [ ] Cross-compilation setup (`x86_64-linux`, `aarch64-linux`, `x86_64-macos`).
- [ ] CI pipeline (build + test + lint).
- [ ] README and usage documentation.
- [ ] v1.0 release.
