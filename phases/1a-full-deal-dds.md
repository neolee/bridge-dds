# `Phase 1a` - Full-Deal `DDS` Evaluation

## Goal

Build a `CLI` tool that accepts a `PBN` record, extracts the `Deal`, `Dealer`, and `Vulnerable` tags, calls `DDS` to compute the full `4x5` double-dummy tricks matrix, derives the `DealerPar` result, and prints both to `stdout`. The `CLI` is a thin wrapper around a shared `Rust` library crate.

## Reference

- `DDS` API: `engine/dds/include/dll.h`.
- `DDS` documentation: `engine/dds/doc/dll-description.md`.
- `PBN` specification: <https://www.tistis.nl/pbn/pbn_v21.txt>.
- `DDS` functions for this phase: `CalcDDtablePBN`, `DealerPar`, `SetMaxThreads`, and `ErrorMessage`.
- `DDS` macOS static build file: `engine/dds/src/Makefiles/Makefile_Mac_clang_static`.

## Inputs

The primary input is a `PBN` record. For this phase, the record must contain these tags:

```pbn
[Dealer "N"]
[Vulnerable "None"]
[Deal "N:QJ6.K652.J85.T98 873.J97.AT764.Q4 K5.T83.KQ9.A7652 AT942.AQ4.32.KJ3"]
```

`Dealer` is the game's dealer and is passed to `DealerPar`. `Vulnerable` is mapped to the `DDS` vulnerability code. `Deal` is passed to `CalcDDtablePBN` as a `ddTableDealPBN.cards` string.

The `<first>` direction in the `Deal` tag identifies the first hand listed. It must be preserved when serializing the `Deal` tag value for `DDS`, but it must not be treated as the dealer.

## Tasks

### 1. Project Skeleton

Create a single `Rust` crate with a library target and one binary target. The project uses hand-written `FFI`; it does not use `bindgen`.

`Cargo.toml`:

```toml
[package]
name = "bridge-dds"
version = "0.1.0"
edition = "2021"

[lib]
path = "src/lib.rs"

[[bin]]
name = "bridge"
path = "src/cli/main.rs"

[dependencies]
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
```

`src/lib.rs`:

```rust
pub mod core;
pub mod dds;
```

`src/cli/main.rs` starts as a minimal `clap` app with `--version`, then grows into `bridge solve`.

### 2. `DDS` Static Library

`DDS` is already present at `engine/dds/` as a git submodule.

Key facts from `engine/dds/include/dll.h`:

- `ddTableDealPBN` contains one `char cards[80]` buffer.
- `CalcDDtablePBN` takes `ddTableDealPBN` by value and writes `ddTableResults`.
- `ddTableResults.resTable` is indexed as `[strain][declarer]`.
- `parResultsDealer` contains `number`, `score`, and `contracts[10][10]`.
- `DealerPar` takes `ddTableResults *`, `parResultsDealer *`, `dealer`, and `vulnerable`.
- `SetMaxThreads` and `ErrorMessage` both return `void`.

The root `Makefile` owns the stable library location:

```make
.PHONY: build-dds build-cli test

build-dds:
	$(MAKE) -C engine/dds/src -f Makefiles/Makefile_Mac_clang_static macos
	mkdir -p engine/dds/lib
	cp engine/dds/src/libdds.a engine/dds/lib/libdds.a

build-cli: build-dds
	cargo build --release

test: build-dds
	cargo test
```

`build.rs` links against `engine/dds/lib`:

```rust
fn main() {
    println!("cargo:rustc-link-search=native=engine/dds/lib");
    println!("cargo:rustc-link-lib=static=dds");
    println!("cargo:rustc-link-lib=dylib=c++");
}
```

### 3. `FFI` Layer

All raw `DDS` declarations live in `src/dds/ffi.rs`. Only `src/dds/solver.rs` may call this module.

```rust
use std::ffi::c_char;
use std::os::raw::c_int;

pub const RETURN_NO_FAULT: c_int = 1;

#[repr(C)]
pub struct ddTableDealPBN {
    pub cards: [c_char; 80],
}

#[repr(C)]
pub struct ddTableResults {
    pub resTable: [[c_int; 4]; 5],
}

#[repr(C)]
pub struct parResultsDealer {
    pub number: c_int,
    pub score: c_int,
    pub contracts: [[c_char; 10]; 10],
}

extern "C" {
    pub fn SetMaxThreads(userThreads: c_int);

    pub fn CalcDDtablePBN(
        tableDealPBN: ddTableDealPBN,
        tablep: *mut ddTableResults,
    ) -> c_int;

    pub fn DealerPar(
        tablep: *mut ddTableResults,
        presp: *mut parResultsDealer,
        dealer: c_int,
        vulnerable: c_int,
    ) -> c_int;

    pub fn ErrorMessage(code: c_int, line: *mut c_char);
}
```

`SolveBoardPBN` and `AnalysePlayPBN` are intentionally left for `Phase 1b`.

### 4. Domain Types

Domain types do not depend on `FFI`.

`src/core/deal.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Suit {
    Spades,
    Hearts,
    Diamonds,
    Clubs,
}

impl Suit {
    pub fn all() -> [Suit; 4];
    pub fn as_char(self) -> char;
    pub fn from_char(c: char) -> Option<Suit>;
    pub fn dds_index(self) -> usize;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Rank {
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
    Ten,
    Jack,
    Queen,
    King,
    Ace,
}

impl Rank {
    pub fn all() -> [Rank; 13];
    pub fn as_char(self) -> char;
    pub fn from_char(c: char) -> Option<Rank>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Card {
    pub suit: Suit,
    pub rank: Rank,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    North,
    East,
    South,
    West,
}

impl Direction {
    pub fn partner(self) -> Direction;
    pub fn next(self) -> Direction;
    pub fn dds_index(self) -> usize;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Strain {
    Spades,
    Hearts,
    Diamonds,
    Clubs,
    NoTrump,
}

impl Strain {
    pub fn all() -> [Strain; 5];
    pub fn dds_index(self) -> usize;
    pub fn as_label(self) -> &'static str;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hand(u64);

impl Hand {
    pub fn empty() -> Self;
    pub fn from_cards(cards: &[Card]) -> Result<Self, Error>;
    pub fn cards(&self) -> impl Iterator<Item = Card>;
    pub fn contains(&self, card: Card) -> bool;
    pub fn len(&self) -> usize;
    pub fn remove(&self, card: Card) -> Self;
    pub fn add(&self, card: Card) -> Result<Self, Error>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Deal {
    pub first: Direction,
    pub hands: [Hand; 4],
}
```

`Deal.first` is the `<first>` value from the `Deal` tag. The `hands` array is stored in `N`, `E`, `S`, `W` order regardless of the order used in the input tag.

`src/core/board.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Board {
    pub deal: Deal,
    pub dealer: Direction,
    pub vulnerability: Vulnerability,
}
```

`Board.dealer` comes from the `Dealer` tag, not from `Deal.first`.

### 5. `PBN` Parsing

`src/core/pbn.rs` is the single source of truth for `PBN` parsing and serialization.

```rust
pub fn parse_record(input: &str) -> Result<Board, Error>;
pub fn parse_deal_tag(value: &str) -> Result<Deal, Error>;
pub fn deal_to_pbn_tag_value(deal: &Deal) -> String;
pub fn parse_dealer_tag(value: &str) -> Result<Direction, Error>;
pub fn parse_vulnerable_tag(value: &str) -> Result<Vulnerability, Error>;
```

Supported `Vulnerable` values:

- `None`, `Love`, and `-` map to `Vulnerability::None`.
- `NS` maps to `Vulnerability::NS`.
- `EW` maps to `Vulnerability::EW`.
- `All` and `Both` map to `Vulnerability::Both`.

`Phase 1a` requires all four hands in the `Deal` tag. Partial deals using `-` are rejected with a clear parse error.

### 6. Tricks Matrix

`src/core/tricks.rs` preserves the full `DDS` result.

```rust
#[derive(Debug, Clone, Serialize)]
pub struct TricksMatrix {
    data: [[u8; 5]; 4],
}

impl TricksMatrix {
    pub fn from_dds(raw: &[[i32; 4]; 5]) -> Result<Self, Error>;
    pub fn get(&self, declarer: Direction, strain: Strain) -> u8;
    pub(crate) fn to_dds(&self) -> [[i32; 4]; 5];
}
```

The public layout is `[declarer][strain]`, which is natural for text output. Conversion to and from `DDS` preserves the native `[strain][declarer]` layout.

The default text output prints:

```text
       S  H  D  C NT
N      5  6  5  7  6
E      8  6  7  5  6
S      5  6  5  7  6
W      8  6  7  5  6
Par: -110; 2S-EW
```

### 7. Par Types

`src/core/par.rs` defines the Rust-side par result.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vulnerability {
    None = 0,
    Both = 1,
    NS = 2,
    EW = 3,
}

impl Vulnerability {
    pub fn dds_code(self) -> i32;
}

#[derive(Debug, Clone, Serialize)]
pub struct ParResult {
    pub score: i32,
    pub contracts: Vec<String>,
}
```

`ParResult.score` is from the `NS` perspective, matching `DealerPar`.

### 8. Safe `DDS` Wrapper

`src/dds/solver.rs` owns all unsafe calls.

```rust
pub struct DdsSolver;

impl DdsSolver {
    pub fn init();
    pub fn solve_table(deal: &Deal) -> Result<TricksMatrix, Error>;
    pub fn compute_par(
        table: &TricksMatrix,
        dealer: Direction,
        vulnerability: Vulnerability,
    ) -> Result<ParResult, Error>;
    pub fn error_message(code: i32) -> String;
}
```

`solve_table` serializes `Deal` back to a `PBN` deal-tag value including `<first>:` and copies it into `ddTableDealPBN.cards`. It must reject strings that do not fit into the `80` byte `DDS` buffer.

### 9. `CLI`

`src/cli/main.rs` exposes `bridge solve`.

```rust
use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(name = "bridge", version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Solve {
        #[arg(default_value = "-")]
        input: String,

        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },
}

#[derive(Clone, Copy, ValueEnum)]
enum OutputFormat {
    Text,
    Json,
}
```

Execution flow:

1. Read a `PBN` record from `stdin`, a file path, or a direct string argument.
2. Parse `Deal`, `Dealer`, and `Vulnerable`.
3. Call `DdsSolver::solve_table`.
4. Call `DdsSolver::compute_par`.
5. Print text or `JSON`.

The `CLI` does not expose `--dealer` or `--vul` in `Phase 1a`.

### 10. Error Handling

`src/core/error.rs`:

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("PBN parse error: {0}")]
    PbnParse(String),

    #[error("missing required PBN tag: {0}")]
    MissingPbnTag(&'static str),

    #[error("DDS error: {0}")]
    Dds(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
```

The `CLI` prints errors to `stderr` and exits non-zero.

## Verification

### Automated Checks

- `cargo test` for `PBN` tag parsing:
  - Parses `Deal`, `Dealer`, and `Vulnerable`.
  - Accepts valid `Vulnerable` aliases.
  - Rejects missing `Dealer`.
  - Rejects missing `Vulnerable`.
  - Rejects partial `Deal` tags with `-`.
  - Preserves `Deal.first` without treating it as `Board.dealer`.
- `cargo test` for domain types:
  - `Hand::from_cards` and `Hand::cards` round-trip.
  - Duplicate cards are rejected.
  - Each full parsed board has `52` unique cards.
- `cargo test` for `DDS` integration:
  - Build `engine/dds/lib/libdds.a`.
  - Use a known `DDS` example deal and assert all `20` table values.
  - Use a known `DDS` example table and assert `DealerPar` score and contracts.
  - Verify `ErrorMessage(RETURN_NO_FAULT)` returns a non-empty message.

### Manual Checks

- `bridge solve examples/board.pbn` prints a `4x5` tricks matrix and par line.
- `bridge solve - < examples/board.pbn` behaves the same.
- `bridge solve examples/board.pbn --format json` prints valid `JSON`.
- A record without `Dealer` fails clearly.
- A record without `Vulnerable` fails clearly.
- A record whose `Deal` `<first>` differs from `Dealer` is accepted and computes par using `Dealer`.
