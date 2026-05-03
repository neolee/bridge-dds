# Phase 1a -- Full-deal DDS Evaluation

## Goal

A CLI tool that accepts a PBN deal string and a vulnerability, calls the DDS library to compute the double-dummy tricks for all 20 declarer/strain combinations, derives the par contract, and prints both to stdout. The tool is a thin wrapper around a shared Rust library crate.

## Reference

- DDS API: `engine/dds/include/dds.h` (to be cloned as a submodule)
- DDS function used: `CalcDDtablePBN` (single call returns all 20 results for a fresh deal)
- PBN 2.1 spec: <http://www.tistis.nl/pbn/>

## Tasks

### 1. Project skeleton

Single Rust crate with a library target and one binary target. No `bindgen` dependency -- all FFI is hand-written.

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

[build-dependencies]
cc = "1"
```

**`src/lib.rs`:**
```rust
pub mod dds;
pub mod core;
```

**`src/cli/main.rs`:** minimal `clap` app, just `--version` for skeleton verification.

### 2. Clone DDS C library

```bash
git submodule add https://github.com/dds-bridge/dds engine/dds
```

Inspect the header and source tree to answer:

- Does `dds.h` provide a par calculation function (e.g. `CalcPar` or `Par`)? If yes, we will wrap it rather than implementing our own.
- Exact signatures of `CalcDDtablePBN` and `ddTableDealPBN` / `ddTableResults` struct layouts.
- Build system: CMake, bare Makefile, or trivial set of `.cpp` files.
- Any platform-specific compilation quirks.

### 3. FFI layer (`src/dds/`)

**Decision:** All DDS FFI is hand-written in `src/dds/ffi.rs`. The DDS API surface we need is small and stable (3 functions, 4 structs). Hand-written declarations are transparent, avoid a `bindgen` build dependency, and give us full control over type mapping. The entire module is `unsafe`; only `src/dds/solver.rs` (safe wrapper) touches it.

**`src/dds/ffi.rs`** -- direct C type and function declarations:

```rust
use std::ffi::c_char;
use std::os::raw::{c_int, c_uint};

// --- Structs ---

#[repr(C)]
pub struct ddTableDealPBN {
    pub cards: [[c_char; 80]; 4],   // [N, E, S, W]; each NUL-terminated PBN hand string
}

#[repr(C)]
pub struct ddTableResults {
    pub resTable: [[c_int; 4]; 5],  // [strain 0=S..4=NT][declarer 0=N..3=W]
}

#[repr(C)]
pub struct deal {
    pub trump: c_int,                // 0=S, 1=H, 2=D, 3=C, 4=NT
    pub first: c_int,                // 0=N, 1=E, 2=S, 3=W
    pub currentTrickSuit: [c_int; 3],
    pub currentTrickRank: [c_int; 3],
    pub remainCards: [[c_uint; 4]; 4],  // [player][suit], 52-bit mask per suit
}

#[repr(C)]
pub struct futureTricks {
    pub nodes: c_int,
    pub cards: c_int,
    pub suit: [c_int; 13],
    pub rank: [c_int; 13],
    pub equals: [c_int; 13],
    pub score: [c_int; 13],
}

// --- Functions ---

extern "C" {
    pub fn SetResources(maxThreads: c_int);
    pub fn CalcDDtablePBN(
        tableDealPBN: *mut ddTableDealPBN,
        tablep: *mut ddTableResults,
    ) -> c_int;
    pub fn SolveBoard(
        dl: deal,
        target: c_int,
        solutions: c_int,
        mode: c_int,
        futp: *mut futureTricks,
        threadIndex: c_int,
    ) -> c_int;
}
```

**`build.rs`** -- compile DDS sources:

```rust
fn main() {
    cc::Build::new()
        .cpp(true)
        .files(&[
            "engine/dds/src/dds.cpp",
            // Additional source files discovered during task 2
        ])
        .include("engine/dds/include")
        .compile("dds");

    println!("cargo:rustc-link-lib=static=dds");
    println!("cargo:rustc-link-lib=dylib=c++");
}
```

If the DDS source tree proves too complex for `cc` (many files, custom build steps), fall back to compiling DDS separately with its own build system. The FFI declarations remain the same either way.

### 4. Domain types (`src/core/`)

Two-layer design: domain types never touch FFI. Conversion between layers happens in `src/dds/solver.rs`.

#### `src/core/deal.rs` -- card and hand types

```rust
/// Bridge suit, in rank order from highest to lowest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Suit {
    Spades,   // "S"
    Hearts,   // "H"
    Diamonds, // "D"
    Clubs,    // "C"
}

impl Suit {
    /// Iterate all four suits in canonical order.
    pub fn all() -> [Suit; 4];
    /// PBN / display character.
    pub fn as_char(self) -> char;
    /// Parse from PBN char.
    pub fn from_char(c: char) -> Option<Suit>;
}

/// Card rank. Ord derives in power order: Two < Three < ... < Ace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Rank {
    Two, Three, Four, Five, Six, Seven, Eight, Nine, Ten, Jack, Queen, King, Ace,
}

impl Rank {
    /// Iterate all thirteen ranks from Two to Ace.
    pub fn all() -> [Rank; 13];
    /// PBN / display character: 2-9, T, J, Q, K, A.
    pub fn as_char(self) -> char;
    /// Parse from PBN char.
    pub fn from_char(c: char) -> Option<Rank>;
}

/// A single playing card.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Card {
    pub suit: Suit,
    pub rank: Rank,
}

impl Card {
    pub fn new(suit: Suit, rank: Rank) -> Self;
}

/// Compass direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction { North, East, South, West }

impl Direction {
    /// Partner of this direction.
    pub fn partner(self) -> Direction;
    /// Next direction clockwise.
    pub fn next(self) -> Direction;
    /// Index for DDS arrays: 0=N, 1=E, 2=S, 3=W.
    pub fn dds_index(self) -> usize;
}

/// A bridge strain (denomination), including No Trump.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Strain { Spades, Hearts, Diamonds, Clubs, NoTrump }

impl Strain {
    /// Index for DDS arrays: 0=S, 1=H, 2=D, 3=C, 4=NT.
    pub fn dds_index(self) -> usize;
    /// PBN / display character.
    pub fn as_char(self) -> char;
    /// Trick value in duplicate scoring (undoubled).
    pub fn trick_value(self) -> u8;
    /// Whether this strain is a major suit.
    pub fn is_major(self) -> bool;
    /// Whether this strain is a minor suit.
    pub fn is_minor(self) -> bool;
}
```

```rust
/// One player's hand, stored as a 52-bit mask. This is a newtype: the bit
/// layout is an internal detail, not exposed through the public API.
///
/// Bit layout (matches DDS convention):
///   bits  0-12  S-A, S-K, ..., S-2  (bit 0 = highest rank)
///   bits 13-25  H-A, H-K, ..., H-2
///   bits 26-38  D-A, D-K, ..., D-2
///   bits 39-51  C-A, C-K, ..., C-2
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hand(u64);

impl Hand {
    /// An empty hand.
    pub fn empty() -> Self { Hand(0) }

    /// A hand containing exactly the given cards.
    pub fn from_cards(cards: &[Card]) -> Self;

    /// Iterate over all cards in this hand, in arbitrary order.
    pub fn cards(&self) -> impl Iterator<Item = Card>;

    /// Whether the hand contains this card.
    pub fn contains(&self, card: Card) -> bool;

    /// Number of cards held.
    pub fn len(&self) -> usize;

    /// Return a new Hand with `card` removed.
    pub fn remove(&self, card: Card) -> Self;

    /// Return a new Hand with `card` added.
    pub fn add(&self, card: Card) -> Self;

    /// Bit mask for a specific suit (S=0, H=1, D=2, C=3).
    /// Each suit occupies 13 bits within the `u64`.
    pub(crate) fn suit_mask(self, suit: Suit) -> u16;

    /// Convert to the per-suit bit masks expected by DDS `remainCards`.
    pub(crate) fn to_dds_masks(self) -> [u32; 4];
}

/// Four hands plus dealer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Deal {
    pub dealer: Direction,
    pub hands: [Hand; 4],  // N, E, S, W order
}
```

The `Hand` newtype uses `u64` internally for zero-copy alignment with DDS, but the public API only speaks `Card` and `Suit`. The bit layout (which bits map to which cards) is enforced inside `Hand::from_cards`, `Hand::cards`, and `Hand::to_dds_masks` -- the rest of the codebase never shifts bits manually.

#### `src/core/tricks.rs` -- tricks matrix

```rust
/// Double-dummy tricks: `data[strain.dds_index()][declarer.dds_index()]`.
#[derive(Debug, Clone, Serialize)]
pub struct TricksMatrix {
    data: [[u8; 4]; 5],
}

impl TricksMatrix {
    /// Create from the DDS `ddTableResults` raw array.
    pub fn from_dds(raw: &[[i32; 4]; 5]) -> Self;

    /// Tricks for a given strain and declarer.
    pub fn get(&self, strain: Strain, declarer: Direction) -> u8;

    /// Best tricks for `side` in `strain` (max of the two partner seats).
    pub fn best_for_side(&self, side_axis: Side, strain: Strain) -> u8;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side { NS, EW }
```

#### `src/core/par.rs` -- par calculation

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vulnerability { None, NS, EW, Both }

#[derive(Debug, Clone, Serialize)]
pub struct ParResult {
    pub contract: String,     // e.g. "4S", "3N"
    pub declarer: Direction,  // N, S, E, or W
    pub level: u8,            // 1-7
    pub strain: Strain,
    pub tricks: u8,           // tricks declarer actually makes
    pub score: i32,           // from NS perspective: positive = NS gain
    pub side: Side,           // which side declares
}
```

**Strategy:** After cloning DDS (task 2), check whether `dds.h` provides a par computation function (`CalcPar` or similar). If DDS provides it, we wrap it directly. If DDS does not, we implement a minimum viable par calculator with the following scope:

- Search undoubled contracts only (levels 1-7, all 5 strains, both sides).
- For each candidate: if `best_tricks >= level + 6`, the contract makes; compute score from trick values + game/slam bonuses.
- If not making: compute the negative penalty score for the undertricks.
- The par contract is the equilibrium point where neither side can improve their score by bidding higher.
- Scoring constants: minor suit = 20/trick, major = 30/trick, NT first = 40 + 30 per additional; game bonus 300 NV / 500 V at 100+ trick points; slam bonuses 500/750 NV/V for small, 1000/1500 for grand.

The doubled/redoubled case is excluded from v1.0 scope -- document this in the output or in CLI `--help` as a known limitation.

### 5. Safe DDS wrapper (`src/dds/solver.rs`)

A safe Rust wrapper that owns the FFI interaction. The rest of the codebase calls only this module, never `ffi` directly.

```rust
use crate::core::deal::Deal;
use crate::core::tricks::TricksMatrix;

pub struct DdsSolver;

impl DdsSolver {
    /// Initialize DDS resources. Safe to call once at startup.
    pub fn init() {
        // unsafe { ffi::SetResources(0); }  // 0 = use default threads
    }

    /// Compute the full 20-result tricks matrix for a fresh deal.
    pub fn solve_table(deal: &Deal) -> Result<TricksMatrix, DdsError> {
        // 1. Convert Deal.hands to ddTableDealPBN (PBN string per hand)
        // 2. Call ffi::CalcDDtablePBN
        // 3. Copy results into TricksMatrix
        // 4. Drop any C-allocated memory if needed
    }
}
```

The conversion from `Deal` to `ddTableDealPBN` uses the PBN serialization already implemented in `core::pbn`. This way the PBN parser is also the DDS format bridge -- single source of truth for hand-to-string conversion.

### 6. PBN parser (`src/core/pbn.rs`)

```rust
/// Parse a PBN deal string into a `Deal`.
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
```

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
3. `parse_deal` -> `DdsSolver::solve_table` -> `par::compute`.
4. If `--format json`: serialize `{ "tricks": ..., "par": ... }` to stdout.
5. If `--format text` (default): minimal human-readable output. One line per strain showing NS tricks and EW tricks, plus one par summary line. Example:

```
     S  H  D  C  N
NS: 10  6  5  8  7
EW:  3  7  8  5  6
Par: NS 4S = (420)
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

All library functions return `Result<T, Error>`. The CLI catches at `main`, prints to stderr, exits non-zero.

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
  - `Suit::all()` and `Rank::all()` produce correct counts.

- `core::par` unit tests:
  - Known textbook par results for sample deals.
  - Game bonus thresholds (3NT vs 4H).
  - Vulnerability impact on scores.

- `dds` integration test (requires DDS compiled):
  - Feed a known deal, assert the tricks matrix matches a hand-computed reference.

### Manual (developer performs)

- `bridge solve "N:AKQJT98..8642 76543..JT97 Q8542..KJ8 .KQJ97632.AQT53" --vul none` prints a tricks table and par line.
- `bridge solve --format json | jq` produces valid JSON with all 20 trick values.
- Pipe via `stdin`: `echo 'N:...' | bridge solve -` works identically.
- Invalid deal prints a clear error to `stderr` and exits non-zero.
