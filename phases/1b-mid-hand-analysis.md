# Phase 1b -- Mid-Hand Analysis

## Goal

Extend the CLI and library to accept a partial play trace. Given a deal, a contract, and the first `k` tricks already played, derive the residual state (remaining cards, current leader, partial trick state) and call `SolveBoardPBN` to return the optimal continuation from the current position.

## Reference

- DDS API: `engine/dds/include/dll.h` -- `SolveBoardPBN`, `dealPBN`, `futureTricks`.
- DDS documentation: `engine/dds/doc/dll-description.md`.
- PBN specification: <https://www.tistis.nl/pbn/pbn_v21.txt> (section 3.3 Play data).
- Project PBN input contract: `phases/pbn-input-contract.md`.

## Scope

### In scope

- Parse a PBN `Play` tag value into a structured play trace (sequence of cards in play order).
- Accept `--trump` and `--declarer` CLI flags to specify the contract.
- Derive the residual state after `k` cards have been played:
  - Remaining cards per hand (52 minus played cards).
  - Current leader (winner of last completed trick, or next player if mid-trick).
  - Cards already played to the current incomplete trick (0-3 cards).
  - Tricks already won by each side.
- Call `SolveBoardPBN` with the residual state.
- Display the suggested cards and their double-dummy scores (tricks from this position).
- Validate the play trace:
  - Every played card belongs to the player who is claimed to have played it.
  - No card is played twice.
  - (Warning only) follow-suit violations.

### Out of scope

- `AnalysePlayPBN` (post-mortem evaluation of each card in sequence). Useful but not required for "what should I play next?"
- Parsing the `Contract` or `Auction` PBN tags. The contract is provided via CLI flags.
- Multi-board batch analysis with play traces.

## Tasks

### 1. Play trace parsing (`src/core/play.rs`)

Parse the PBN `Play` tag value.

**Format:**

```pbn
[Play "W:S6=S4=SJ=SQ=S3=S7=S9=SK"]
```

- Tricks are separated by whitespace.
- Within each trick, cards are separated by `=`.
- Each card is a suit letter (`S`/`H`/`D`/`C`) followed by a rank character.
- The first card of the first trick is played by the opening leader.

**Data type:**

```rust
/// A parsed play trace. Cards in play order, each annotated with the
/// direction that played it (derived from the opening leader and trick winners).
#[derive(Debug, Clone)]
pub struct PlayTrace {
    /// Cards in the order they were played, with the player who played each.
    pub plays: Vec<(Direction, Card)>,
    /// The opening leader (derived from the first card's player tag, or
    /// from declarer if we use `--declarer`).
    pub opening_leader: Direction,
}
```

**Parsing logic:**

```rust
pub fn parse_play_tag(value: &str, opening_leader: Direction) -> Result<PlayTrace, Error>;
```

- Split value by whitespace to get tricks.
- For each trick, split by `=` to get individual cards.
- Parse each card string: suit char + rank char → `Card`.
- Each trick must have exactly 4 cards (no incomplete tricks in the trace).
- Track play order: first card of each trick is played by the current leader.
  Determine winner of each trick (apply follow-suit and trump rules), next
  trick's leader is the winner.

### 2. Validate the play trace (`src/core/play.rs`)

```rust
pub fn validate_trace(
    trace: &PlayTrace,
    deal: &Deal,
    trump: Strain,
) -> Result<(), Vec<PlayWarning>>;
```

**Hard errors** (returned as `Err`):
- A card is not held by the player who supposedly played it.
- The same card appears more than once in the trace.

**Warnings** (returned in the `Vec<PlayWarning>`):
- Follow-suit violation: a player had a card in the led suit but played another suit.
- The computed trick winner differs from who led the next trick in the trace
  (may indicate the trace is incorrectly formatted, or the opener's tag is wrong).

These warnings do not block the analysis — they are displayed to the user and
the residual state is derived on a best-effort basis.

### 3. Derive the residual state (`src/core/play.rs`)

```rust
pub struct ResidualState {
    /// Remaining cards for each player (N, E, S, W order).
    pub hands: [Hand; 4],
    /// The player who leads to the next trick (or the current player if mid-trick).
    pub leader: Direction,
    /// Suits of cards already played to the current trick (0-3 entries).
    /// All zero if starting a new trick.
    pub current_trick_suits: [Suit; 3],
    /// Ranks of cards already played to the current trick.
    pub current_trick_ranks: [Rank; 3],
    /// Number of cards played to the current trick (0-3).
    pub cards_in_trick: usize,
    /// Tricks won by NS so far.
    pub tricks_ns: u8,
    /// Tricks won by EW so far.
    pub tricks_ew: u8,
}

pub fn derive_residual(
    deal: &Deal,
    trace: &PlayTrace,
    trump: Strain,
) -> Result<ResidualState, Error>;
```

Algorithm:

1. Start with `hands = deal.hands.clone()`, `leader = trace.opening_leader`.
2. Process cards in play order, grouping into tricks (4 cards per trick).
3. For each card: verify it exists in `hands[player]`, remove it.
4. For each trick: determine the winner using follow-suit and trump rules.
   Winner leads the next trick.
5. After processing all complete tricks, record the partial trick state
   (cards already played but trick not yet complete) in `current_trick_suits`/`ranks`.
6. Track tricks won: increment `tricks_ns` or `tricks_ew` based on winner.

**Follow-suit and trick-winner logic:**

- The first card of a trick establishes the led suit.
- Each subsequent player must follow suit if possible; otherwise may play any card.
- If no trump is played, the highest card in the led suit wins.
- If trump is played, the highest trump wins.
- If the contract is NoTrump, there is no trump suit; the led suit always wins.

This logic is needed both for deriving the residual state (determining the next leader)
and for validating the play trace.

### 4. Call `SolveBoardPBN` (`src/dds/solver.rs`)

Extend `DdsSolver` with a mid-hand solve method:

```rust
impl DdsSolver {
    /// Solve from a mid-hand position. Returns the suggested cards and
    /// the number of tricks the declaring side can win from this point.
    pub fn solve_mid_hand(
        state: &ResidualState,
        trump: Strain,
        leader: Direction,
    ) -> Result<Vec<SuggestedCard>, Error>;
}

pub struct SuggestedCard {
    pub card: Card,
    /// Tricks for the declaring side if this card is played.
    /// `None` if the target cannot be reached.
    pub score: Option<u8>,
    /// Whether this card is optimal.
    pub is_optimal: bool,
}
```

Implementation:

1. Convert `ResidualState.hands` to PBN format for `dealPBN.remainCards[80]`.
2. Set `dealPBN.trump`: DDS trump index (0=S, 1=H, 2=D, 3=C, 4=NT).
3. Set `dealPBN.first`: DDS direction index of the leader.
4. Set `dealPBN.currentTrickSuit[0..cards_in_trick]` and `currentTrickRank[0..cards_in_trick]`.
5. Call `SolveBoardPBN` with `target = -1` (find max tricks), `solutions = 2` (return all optimal cards), `mode = 1` (always search).
6. The `futureTricks.score[i]` values are **from the current position** (we verified this semantics during Phase 1a review). Each entry gives the number of tricks the declaring side can win if that specific card is played now.

The `score` in `futureTricks` is relative to the declaring side (the side that is to play). For the output we display it as-is: "if you play this card, you can win N more tricks."

### 5. Extend CLI (`src/cli/main.rs`)

Add new flags to `bridge solve`:

```rust
#[derive(Subcommand)]
enum Command {
    Solve {
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,

        /// Trump suit: S, H, D, C, or NT (required if Play tag is present)
        #[arg(long)]
        trump: Option<String>,

        /// Declarer: N, E, S, or W (required if Play tag is present)
        #[arg(long)]
        declarer: Option<String>,
    },
}
```

**Input contract for Phase 1b:**

The PBN record may now include an optional `Play` tag:

```pbn
[Deal "N:..."]
[Dealer "N"]
[Vulnerable "None"]
[Play "W:S6=S4=SJ=SQ=S3=S7=S9=SK"]
```

When a `Play` tag is present:
- `--trump` and `--declarer` are required. The CLI rejects the input if they are missing.
- The opening leader is derived from `declarer` (the player to declarer's left: `declarer.next()`).
- The play trace opening leader tag (first character of the Play value, e.g. `W:`) is cross-checked against the derived leader and a warning is emitted if they differ.
- Parse the `Play` tag, validate, derive residual state, call `solve_mid_hand`.

When no `Play` tag is present: behavior is identical to Phase 1a.

### 6. Output format

When a play trace is provided, output extends the current format:

**Text output example:**

```
Trump: S  Declarer: N
Tricks played: 4  Already won: NS 2  EW 1
Leader: S  Cards in trick: 0

 N:  QJ6 K65 J8 T9
 E:  - J97 AT76 Q
 S:  K5 T83 KQ9 A
 W:  AT94 AQ4 - KJ3

Suggested plays for S:
  SK  (8 tricks)
  S5  (7 tricks)
  H3  (6 tricks)
  DQ  (8 tricks)
  ...
```

**JSON output** extends the Phase 1a shape:

```json
{
  "tricks": { ... },
  "par": { ... },
  "continuation": {
    "leader": "S",
    "tricks_played": 4,
    "won_ns": 2,
    "won_ew": 1,
    "cards_in_trick": 0,
    "hands": {
      "N": "QJ6.K65.J8.T9",
      "E": "-.J97.AT76.Q",
      "S": "K5.T83.KQ9.A",
      "W": "AT94.AQ4.-.KJ3"
    },
    "suggested": [
      { "card": "SK", "score": 8, "optimal": true },
      { "card": "S5", "score": 7, "optimal": false }
    ]
  }
}
```

### 7. DDS return-value semantics

The DDS `futureTricks.score[i]` field is documented as "Target of maximum number of tricks"
when `target = -1`. From DDS source analysis (the `SolveBoard` function computes tricks
the declaring side can win from the current position, not total tricks for the full deal),
the returned values are **tricks from the current position**.

Since we already track `tricks_ns` and `tricks_ew` in the residual state, the total tricks
the declaring side will ultimately win is `tricks_already_won + score[i]`. Both the
per-card score and the already-won counts are displayed to the user.

A unit test in `tests/dds_integration.rs` will verify this semantics by feeding a known
position and asserting the returned futureTricks values against expected results from
the DDS example files.

### 8. Error handling additions

New error variants for `src/core/error.rs`:

```rust
#[error("invalid trump '{0}'; expected one of: S, H, D, C, NT")]
InvalidTrump(String),

#[error("invalid declarer '{0}'; expected one of: N, E, S, W")]
InvalidDeclarer(String),

#[error("Play tag present but --trump and --declarer are required for mid-hand analysis")]
MissingContractFlags,

#[error("play trace validation error: {0}")]
PlayValidation(String),

#[error("play trace warning: {0}")]
PlayWarning(String),
```

## Verification

### Automated (`cargo test`)

- `core::play` unit tests:
  - Round-trip: parse a PBN `Play` tag, verify card count matches.
  - Opening leader derived correctly from declarer.
  - Residual state after 1, 2, 4 complete tricks: remaining cards, next leader, tricks won.
  - Partial trick: 1-3 cards played, residual state shows correct current trick info.
  - Follow-suit validation: correct play passes, violation produces warning.
  - Card ownership error: card not held by claimed player produces hard error.
  - Duplicate card error: same card twice produces hard error.

- `dds` integration test:
  - Use a known deal from `engine/dds/examples/hands.cpp` with a known play trace.
  - Derive residual state and call `solve_mid_hand`.
  - Assert the returned scores match expected values from the DDS example.
  - Verify the trick continuation semantics (already-won + future = total).

### Manual (developer performs)

- `bridge solve --trump S --declarer N < examples/mid-hand.pbn` prints continuation analysis.
- JSON output includes `continuation` block.
- Missing `--trump` or `--declarer` when `Play` tag is present produces a clear error.
- An invalid play trace (card not in hand) prints an error and exits non-zero.
