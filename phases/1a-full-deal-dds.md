# Phase 1a -- Full-deal DDS Evaluation

## Goal

A CLI tool that accepts a PBN deal string and a vulnerability, calls the DDS library to compute the double-dummy tricks for all 20 declarer/strain combinations, derives the par contract, and prints both to stdout. The tool is a thin wrapper around a shared Rust library crate.

## Reference

- DDS API: `engine/dds/include/dll.h` (the public C API header, verified after submodule clone).
- DDS documentation: `engine/dds/doc/dll-description.md`.
- Functions used in Phase 1a: `CalcDDtablePBN`, `DealerPar`, `SetMaxThreads`, `ErrorMessage`.
- PBN 2.1 spec: <http://www.tistis.nl/pbn/>.
- Build: `engine/dds/src/Makefiles/Makefile_Mac_clang_static` (macOS). Analogous Makefiles exist for Linux and Windows.

## Tasks

### 1. Project skeleton

Single Rust crate with a library target and one binary target. No `bindgen` dependency -- all FFI is hand-written, verified against `engine/dds/include/dll.h`.

**`Cargo.toml`:**
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

# No build-dependencies needed: DDS is built separately via its own Makefile.
```

**`src/lib.rs`:**
```rust
pub mod dds;
pub mod core;
```

**`src/cli/main.rs`:** minimal `clap` app, just `--version` for skeleton verification.

### 2. DDS C library (already cloned)

The library is at `engine/dds/` as a git submodule. Key findings from inspecting `dll.h` and documentation:

- **Par calculation:** DDS provides `DealerPar()` (returns structured `parResultsDealer`) and `Par()` (returns text). We wrap `DealerPar()` for its numeric score and structured contract list.
- **`ddTableDealPBN`:** Single `char cards[80]` buffer holding the **full PBN deal string** (four hands space-separated, each hand is four dot-separated suit strings). Not a 4-element array.
- **`CalcDDtablePBN`:** Takes `ddTableDealPBN` **by value** (not pointer), returns `ddTableResults`.
- **`SolveBoardPBN`:** Exists for Phase 1b. Takes `dealPBN` (with `char remainCards[80]` in PBN format) by value.
- **Build system:** Platform-specific Makefiles under `engine/dds/src/Makefiles/`. macOS: `Makefile_Mac_clang_static` produces `libdds.a`.
- **Vulnerability encoding:** 0=None, 1=Both, 2=NS, 3=EW.
- **Binary card encoding** (not needed since we use PBN functions only): `deal.remainCards[4][4]` uses per-suit `unsigned int`, bits 14-2 = Ace-Deuce, bits 0-1 always zero. `deal` struct is not declared in our FFI since we exclusively use `dealPBN` / `SolveBoardPBN`.

### 3. FFI layer (`src/dds/`)

**Decision:** All DDS FFI is hand-written in `src/dds/ffi.rs`. Declarations are verified against `engine/dds/include/dll.h`. The API surface is small and stable. The entire module is `unsafe`; only `src/dds/solver.rs` (safe wrapper) touches it.

**`src/dds/ffi.rs`:**

```rust
use std::ffi::c_char;
use std::os::raw::c_int;

// --- Structs for Phase 1a and 1b ---

/// PBN deal input for CalcDDtablePBN. Single 80-byte buffer containing
/// the full deal string: four hands separated by spaces, each hand is
/// four suit strings separated by dots, e.g. "W:T5.K4... K6.QJT9..."
#[repr(C)]
pub struct ddTableDealPBN {
    pub cards: [c_char; 80],
}

/// 20-result tricks table. resTable[strain 0=S..4=NT][declarer 0=N..3=W].
#[repr(C)]
pub struct ddTableResults {
    pub resTable: [[c_int; 4]; 5],
}

/// Structured par result from DealerPar().
#[repr(C)]
pub struct parResultsDealer {
    pub number: c_int,                  // count of par contracts (1-10)
    pub score: c_int,                   // score from NS perspective
    pub contracts: [[c_char; 10]; 10],  // e.g. "4S-NS", "4H-NS", "5C*-NS-2"
}

// --- Structs for Phase 1b (SolveBoardPBN) ---

#[repr(C)]
pub struct dealPBN {
    pub trump: c_int,              // 0=S, 1=H, 2=D, 3=C, 4=NT
    pub first: c_int,              // 0=N, 1=E, 2=S, 3=W; leader to this trick
    pub currentTrickSuit: [c_int; 3],
    pub currentTrickRank: [c_int; 3],
    pub remainCards: [c_char; 80], // PBN string of remaining cards
}

#[repr(C)]
pub struct futureTricks {
    pub nodes: c_int,
    pub cards: c_int,          // number of returned card alternatives
    pub suit: [c_int; 13],
    pub rank: [c_int; 13],
    pub equals: [c_int; 13],   // lower-ranked equivalent cards (binary encoding)
    pub score: [c_int; 13],    // tricks if this card is played (-1 = target not reachable)
}

// --- Functions ---

extern "C" {
    /// Auto-configure threads. Call once at startup. 0 = let DDS decide.
    pub fn SetMaxThreads(userThreads: c_int);

    /// Compute the 20-result double-dummy table for a fresh deal.
    /// tableDealPBN is passed by value (not pointer) per the DDS API.
    pub fn CalcDDtablePBN(
        tableDealPBN: ddTableDealPBN,
        tablep: *mut ddTableResults,
    ) -> c_int;

    /// Compute par score and contracts from a DD table, dealer-aware.
    /// dealer: 0=N, 1=E, 2=S, 3=W. vulnerable: 0=None, 1=Both, 2=NS, 3=EW.
    pub fn DealerPar(
        tablep: *mut ddTableResults,
        presp: *mut parResultsDealer,
        dealer: c_int,
        vulnerable: c_int,
    ) -> c_int;

    /// Solve a single position (fresh or mid-hand) with PBN input.
    /// target: -1 = find max tricks. solutions: 1 = one card, 2 = all best / 3 = all.
    /// mode: 0 = fast (skip single-card searches), 1 = always search.
    pub fn SolveBoardPBN(
        dlpbn: dealPBN,
        target: c_int,
        solutions: c_int,
        mode: c_int,
        futp: *mut futureTricks,
        thrId: c_int,
    ) -> c_int;

    /// Convert a DDS return code to a human-readable string.
    pub fn ErrorMessage(code: c_int, line: *mut c_char);
}
```

**`build.rs`:**

```rust
fn main() {
    // DDS is built separately via its own Makefile (see project root Makefile).
    // build.rs only declares the link dependency.
    println!("cargo:rustc-link-search=native=engine/dds/lib");
    println!("cargo:rustc-link-lib=static=dds");
    // Link the C++ standard library (required by DDS).
    println!("cargo:rustc-link-lib=dylib=c++");
}
```

The project root `Makefile` encapsulates DDS compilation:

```make
.PHONY: build-dds build-cli test

build-dds:
	cd engine/dds/src && \
	cp Makefiles/Makefile_Mac_clang_static Makefile && \
	make

build-cli: build-dds
	cargo build --release

test: build-dds
	cargo test
```

### 4. Domain types (`src/core/`)

Two-layer design: domain types never touch FFI. Conversion between layers happens in `src/dds/solver.rs`.

#### `src/core/deal.rs`

```rust
/// Bridge suit. DDS indices: S=0, H=1, D=2, C=3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Suit { Spades, Hearts, Diamonds, Clubs }

impl Suit {
    pub fn all() -> [Suit; 4];
    pub fn as_char(self) -> char;
    pub fn from_char(c: char) -> Option<Suit>;
    pub fn dds_index(self) -> usize;
}

/// Card rank. Ord: Two < Three < ... < Ace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Rank {
    Two, Three, Four, Five, Six, Seven, Eight, Nine, Ten, Jack, Queen, King, Ace,
}

impl Rank {
    pub fn all() -> [Rank; 13];
    pub fn as_char(self) -> char;  // 2-9, T, J, Q, K, A
    pub fn from_char(c: char) -> Option<Rank>;
}

/// A single playing card.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Card { pub suit: Suit, pub rank: Rank }

impl Card {
    pub fn new(suit: Suit, rank: Rank) -> Self;
}

/// Compass direction. DDS indices: N=0, E=1, S=2, W=3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction { North, East, South, West }

impl Direction {
    pub fn partner(self) -> Direction;
    pub fn next(self) -> Direction;
    pub fn dds_index(self) -> usize;
}

/// Denomination including No Trump. DDS indices: S=0, H=1, D=2, C=3, NT=4.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Strain { Spades, Hearts, Diamonds, Clubs, NoTrump }

impl Strain {
    pub fn dds_index(self) -> usize;
    pub fn as_char(self) -> char;
    pub fn from_char(c: char) -> Option<Strain>;
}
```

```rust
/// One player's hand.
///
/// Stored as a 52-bit mask purely for internal efficiency: bitwise operations
/// make `contains`, `remove`, and suit extraction cheap. This is NOT aligned
/// with DDS's binary `remainCards` format -- all DDS communication goes through
/// PBN strings, so the bit layout is our own convention and can change freely
/// without affecting any external interface. The public API only speaks `Card` values.
///
/// Bit layout:
///   bits  0-12  S-A, S-K, ..., S-2  (bit 0 = highest rank)
///   bits 13-25  H-A, H-K, ..., H-2
///   bits 26-38  D-A, D-K, ..., D-2
///   bits 39-51  C-A, C-K, ..., C-2
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hand(u64);

impl Hand {
    pub fn empty() -> Self;
    pub fn from_cards(cards: &[Card]) -> Self;
    pub fn cards(&self) -> impl Iterator<Item = Card>;
    pub fn contains(&self, card: Card) -> bool;
    pub fn len(&self) -> usize;
    pub fn remove(&self, card: Card) -> Self;
    pub fn add(&self, card: Card) -> Self;
}

/// Four hands plus dealer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Deal {
    pub dealer: Direction,
    pub hands: [Hand; 4],  // N, E, S, W order
}
```

The `Hand` newtype uses `u64` internally as an efficient Rust representation, but this is decoupled from DDS's binary `remainCards` format. All DDS communication uses PBN strings (`CalcDDtablePBN` and `SolveBoardPBN`), so the only conversion needed is `Hand` <-> PBN fragment via `hand_to_pbn()` in `core::pbn`.

#### `src/core/tricks.rs`

```rust
/// Double-dummy tricks matrix. data[strain.dds_index()][declarer.dds_index()].
#[derive(Debug, Clone, Serialize)]
pub struct TricksMatrix {
    data: [[u8; 4]; 5],
}

impl TricksMatrix {
    /// Create from the DDS ddTableResults raw array.
    pub fn from_dds(raw: &[[i32; 4]; 5]) -> Self;
    pub fn get(&self, strain: Strain, declarer: Direction) -> u8;
    pub fn best_for_side(&self, side: Side, strain: Strain) -> u8;
    /// Convert back to DDS ddTableResults layout. Needed for DealerPar() input.
    pub(crate) fn to_dds(&self) -> [[i32; 4]; 5];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side { NS, EW }
```

#### `src/core/par.rs`

DDS provides `DealerPar()` which handles all scoring logic (game/slam bonuses, doubles, sacrifices). We only define the Rust-side types.

```rust
/// Vulnerability encoding matches DDS: 0=None, 1=Both, 2=NS, 3=EW.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vulnerability { None = 0, Both = 1, NS = 2, EW = 3 }

impl Vulnerability {
    pub fn from_arg(s: &str) -> Result<Self, Error>;
    pub fn dds_code(self) -> i32;
}

/// Par contract list as returned by DDS's DealerPar().
/// Contracts are text strings like "4S-NS", "3N-EW", "5C*-NS-2".
#[derive(Debug, Clone, Serialize)]
pub struct ParResult {
    /// Numeric score from NS perspective: positive = NS gain, negative = EW gain.
    pub score: i32,
    /// List of par contract strings, one per alternative (e.g. ["4S-NS", "4H-NS"]).
    pub contracts: Vec<String>,
}
```

### 5. Safe DDS wrapper (`src/dds/solver.rs`)

A safe Rust wrapper that owns all FFI interaction. The rest of the codebase never touches `ffi` directly.

```rust
use crate::core::deal::{Deal, Direction};
use crate::core::tricks::TricksMatrix;
use crate::core::par::{ParResult, Vulnerability};

pub struct DdsSolver;

impl DdsSolver {
    /// Initialize DDS threading. Call once at startup.
    pub fn init() {
        unsafe { ffi::SetMaxThreads(0); }
    }

    /// Compute the full 20-result tricks matrix for a fresh deal.
    pub fn solve_table(deal: &Deal) -> Result<TricksMatrix, Error> {
        // 1. Serialize Deal to a single PBN string (four hands, space-separated).
        // 2. Copy into ffi::ddTableDealPBN.cards[0..79].
        // 3. Call unsafe { ffi::CalcDDtablePBN(...) }; check return code.
        // 4. Convert ffi::ddTableResults.resTable to TricksMatrix.
    }

    /// Compute par contract and score from a DD results table.
    pub fn compute_par(
        table: &TricksMatrix,
        dealer: Direction,
        vul: Vulnerability,
    ) -> Result<ParResult, Error> {
        // 1. Convert TricksMatrix back to ffi::ddTableResults.
        // 2. Call unsafe { ffi::DealerPar(...) }; check return code.
        // 3. Extract contracts from parResultsDealer.contracts (C strings).
        // 4. Return ParResult { score: pres.number? pres.score?, contracts }.
    }

    /// Convert a DDS return code to a string via ffi::ErrorMessage.
    pub fn error_message(code: i32) -> String;
}
```

### 6. PBN parser (`src/core/pbn.rs`)

```rust
/// Parse a PBN deal string into a Deal.
///
/// Input format:  "N:AKQJT98..8642 76543..JT97 Q8542..KJ8 .KQJ97632.AQT53"
///   - Optional dealer prefix (N:, E:, S:, W:), defaults to N if absent.
///   - Four hand strings separated by whitespace.
///   - Each hand: four suit substrings separated by `.`, in S-H-D-C order.
///   - Empty substring between dots means a void suit.
///   - Rank characters: 2-9, T, J, Q, K, A.
pub fn parse_deal(input: &str) -> Result<Deal, Error>;

/// Serialize a single hand to a PBN fragment (no dealer prefix).
/// Format: "S-cards.H-cards.D-cards.C-cards", e.g. "AKQJT98..8642".
pub fn hand_to_pbn(hand: &Hand) -> String;

/// Serialize a Deal to a full PBN deal string (four space-separated hands, no dealer prefix).
pub fn deal_to_dds_pbn(deal: &Deal) -> String;
```

`deal_to_dds_pbn()` is used by `DdsSolver::solve_table()` to populate `ddTableDealPBN.cards`. The PBN parser is the single source of truth for all hand-to-string and string-to-hand conversion.

### 7. CLI (`src/cli/main.rs`)

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "bridge", version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Evaluate a single deal
    Solve {
        /// PBN deal string, or "-" to read from stdin
        deal: Option<String>,

        /// Vulnerability: none, ns, ew, both
        #[arg(long, default_value = "none")]
        vul: String,

        /// Output format: text (default) or json
        #[arg(long, default_value = "text")]
        format: String,
    },
}
```

Execution flow:
1. Parse `--vul` into `Vulnerability` (error on unknown value).
2. Read deal string from positional arg or stdin.
3. `parse_deal` -> `DdsSolver::solve_table` -> `DdsSolver::compute_par`.
4. If `--format json`: serialize `{ "tricks": ..., "par": ... }` to stdout.
5. If `--format text` (default): minimal human-readable output:

```
     S  H  D  C  N
NS: 10  6  5  8  7
EW:  3  7  8  5  6
Par: NS 4S (420)
```

No Unicode box-drawing, no colors, no alignment beyond basic column spacing. We will refine the output format once we see real results.

### 8. Error handling (`src/core/error.rs`)

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("PBN parse error: {0}")]
    PbnParse(String),

    #[error("DDS error: {0}")]
    Dds(String),

    #[error("invalid vulnerability '{0}'; expected one of: none, ns, ew, both")]
    InvalidVulnerability(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
```

All library functions return `Result<T, Error>`. The CLI catches at `main`, prints to `stderr`, exits non-zero.

## Verification

### Automated (`cargo test`)

- `core::pbn` unit tests:
  - Round-trip: parse then serialize a PBN deal, compare.
  - Void suits: `"...AKQ... .AKQ.."` etc.
  - Missing dealer prefix defaults to N.
  - Invalid input (wrong number of hands, wrong suit count) returns an error.

- `core::deal` unit tests:
  - `Hand::from_cards` then `hand.cards()` round-trips correctly.
  - `Card` rank ordering: `Rank::Ace > Rank::King`.
  - `Suit::all()` and `Rank::all()` produce correct counts with 4 and 13 elements respectively.

- `dds` integration test (requires `libdds.a` built):
  - Feed a known deal, assert the tricks matrix matches a hand-computed reference.
  - Feed a known deal with known par result, assert `DealerPar` output matches expected contracts and score.
  - Verify `ErrorMessage()` returns a non-empty string for a known error code like `RETURN_NO_FAULT (1)`.

### Manual (developer performs)

- `bridge solve "N:AKQJT98..8642 76543..JT97 Q8542..KJ8 .KQJ97632.AQT53" --vul none` prints a tricks table and par line.
- `bridge solve --format json | jq` produces valid JSON with all 20 trick values and par contracts array.
- Pipe via `stdin`: `echo 'N:...' | bridge solve -` works identically.
- Invalid deal prints a clear error to `stderr` and exits non-zero.
