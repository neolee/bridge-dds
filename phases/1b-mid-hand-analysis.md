# Phase 1b -- Position Analysis

## Goal

Implement `Position`-based double-dummy analysis.

Given remaining hands, the next player to act, optional cards already played to the current trick, and a `trump`, call `SolveBoardPBN` to evaluate legal continuations. For clean residual snapshots, also produce a `next_to_act x strain` matrix similar in shape to the full-deal `DDS` matrix.

This phase is centered on the current position, not on how that position was reached. A position may come from a complete deal at trick one, a manually entered residual deal, or a complete deal plus a `Play` trace.

## Key Decisions

- The core solver interface uses `next_to_act` and `trump`, not `declarer`.
- `declarer`, `dummy`, and defenders are `UI` concepts. They can be layered on top when the user starts from a normal contract.
- The raw `DDS` score from `SolveBoardPBN` is interpreted as `tricks_for_side_to_act`.
- Manual residual input starts only from a clean trick boundary: all four hands have the same card count and `current_trick` is empty.
- Runtime trial play must support `current_trick` lengths from `0` to `3`, because hands become uneven during a trick.
- `Play` trace import is an extra entry path. It derives a `Position`, then reuses the same analysis pipeline.
- Imported `Play` traces warn on follow-suit violations and continue when possible. Interactive `play_card` rejects follow-suit violations as hard errors.
- `Position Matrix` output must label row semantics as `next_to_act`, not `declarer`.

These decisions keep the library model independent from contract scoring. This matters because residual positions often lack the earlier trick history, so total tricks and score may be unknowable and are not required for continuation analysis.

## Reference

- `DDS` `API`: `engine/dds/include/dll.h` -- `SolveBoardPBN`, `dealPBN`, `futureTricks`.
- `DDS` documentation: `engine/dds/doc/dll-description.md`.
- `PBN` specification: <https://www.tistis.nl/pbn/pbn_v21.txt>.
- Project `PBN` input contract: `phases/pbn-input-contract.md`.

## Scope

### In Scope

- Add core `Position` types.
- Add `DDS` `FFI` bindings for `SolveBoardPBN`.
- Convert a `Position` into `dealPBN`.
- Solve one `Position + trump` and return legal card results.
- Return every legal card with a double-dummy score.
- Produce a residual `next_to_act x strain` matrix for clean snapshots.
- Extend the `CLI` with a residual-position entry path.
- Support runtime trial-play state advancement with `current_trick`.
- Extra step: parse a `PBN` `Play` tag and derive a `Position`.

### Out Of Scope

- Contract scoring from residual positions without prior trick counts.
- `AnalysePlayPBN` post-mortem evaluation for every historical card.
- Multi-board batch analysis with positions.
- Web `UI` concepts such as `declarer`, `dummy`, undo history, and color marking. The library should expose enough data for those later.

## Concepts

### `Position`

`Position` is the single state object accepted by continuation analysis.

```rust
pub struct Position {
    /// Remaining cards for each player in `N`, `E`, `S`, `W` order.
    pub hands: [Hand; 4],
    /// The next player who must play a card.
    pub next_to_act: Direction,
    /// Cards already played to the current trick, in play order.
    pub current_trick: Vec<PlayedCard>,
}

pub struct PlayedCard {
    pub player: Direction,
    pub card: Card,
}
```

`current_trick.len()` must be between `0` and `3`. Cards in `current_trick` are already removed from `hands`.

When `current_trick` is empty, `next_to_act` is also the trick leader. When `current_trick` is not empty, `next_to_act` is only the next player to act.

### `Entry Snapshot`

An `Entry Snapshot` is a manually entered residual deal.

Rules:

- All four hands have the same card count.
- `current_trick` is empty.
- `next_to_act` is the trick leader.

This is the first supported residual input mode. It avoids accepting ambiguous mid-trick manual states.

### `Runtime Position`

A `Runtime Position` is an internal state during trial play.

Rules:

- `current_trick.len()` may be `0`, `1`, `2`, or `3`.
- Hand counts may differ.
- `next_to_act` is the next player to act.
- Cards in `current_trick` have already been removed from `hands`.

This state is required after the user plays one or more cards during a trick.

### `Position Matrix`

A `Position Matrix` evaluates a clean residual snapshot across possible `next_to_act` and `strain` values.

- Rows: hypothetical `next_to_act` values: `N`, `E`, `S`, `W`.
- Columns: `S`, `H`, `D`, `C`, `NT`.
- Value: maximum tricks from the current position for the side containing `next_to_act`.

This matrix has the same shape as the full-deal `DDS` matrix, but it has different semantics. Full-deal rows are `declarer` values. Position rows are `next_to_act` values.

### `Continuation Analysis`

`Continuation Analysis` evaluates one `Position + trump`.

Output:

- Every legal card for `next_to_act`.
- `tricks_for_side_to_act` for each card.
- Whether each card is optimal.

Use `SolveBoardPBN` with `solutions = 3`, because the trial-play `UI` needs all legal cards, not only optimal cards.

## Tasks

### 1. Add `Position` Types (`src/core/position.rs`)

Add `Position`, `PlayedCard`, and validation helpers.

```rust
pub struct Position {
    pub hands: [Hand; 4],
    pub next_to_act: Direction,
    pub current_trick: Vec<PlayedCard>,
}

pub struct PlayedCard {
    pub player: Direction,
    pub card: Card,
}

pub enum PositionKind {
    EntrySnapshot,
    Runtime,
}
```

Validation:

- `current_trick.len() <= 3`.
- `EntrySnapshot` requires equal hand counts and empty `current_trick`.
- `Runtime` permits uneven hand counts.
- `current_trick` players must follow clockwise order.
- `next_to_act` must be the next clockwise player after the last `current_trick` player, or the trick leader when `current_trick` is empty.

### 2. Add Core Card Helpers (`src/core/deal.rs`)

Add narrow helpers needed by `Position` and `DDS` conversion.

```rust
impl Rank {
    /// Return the `DDS` rank value used in `currentTrickRank`: `2..14`.
    /// This is distinct from `bit_index()`, which uses `A=0` for bit storage.
    pub fn dds_rank(self) -> i32;
}

impl Card {
    pub fn to_pbn(self) -> String;
}

impl Hand {
    pub fn has_suit(&self, suit: Suit) -> bool;
}
```

Keep existing `Hand` storage unchanged.

### 3. Add `SolveBoardPBN` `FFI` (`src/dds/ffi.rs`)

Declare the `DDS` structs and function exactly as defined in `engine/dds/include/dll.h`.

```rust
#[repr(C)]
pub struct futureTricks {
    pub nodes: c_int,
    pub cards: c_int,
    pub suit: [c_int; 13],
    pub rank: [c_int; 13],
    pub equals: [c_int; 13],
    pub score: [c_int; 13],
}

#[repr(C)]
pub struct dealPBN {
    pub trump: c_int,
    pub first: c_int,
    pub currentTrickSuit: [c_int; 3],
    pub currentTrickRank: [c_int; 3],
    pub remainCards: [c_char; 80],
}

pub fn SolveBoardPBN(
    dlpbn: dealPBN,
    target: c_int,
    solutions: c_int,
    mode: c_int,
    futp: *mut futureTricks,
    thrId: c_int,
) -> c_int;
```

### 4. Convert `Position` To `dealPBN` (`src/dds/solver.rs`)

Add a converter used by all `SolveBoardPBN` calls.

Rules:

- `dealPBN.trump = trump.dds_index()`.
- `dealPBN.first = position.next_to_act.dds_index()`.
- `dealPBN.currentTrickSuit` uses `Suit` indices `0..3`.
- `dealPBN.currentTrickRank` uses ranks `2..14`.
- Empty `currentTrickSuit` and `currentTrickRank` slots are `0`.
- `dealPBN.remainCards` is a null-terminated `PBN` string built from remaining `hands`.
- `dealPBN.remainCards` must start with `<first>:`. The `<first>` value controls how the four hand strings map to `N`, `E`, `S`, and `W`.
- For the first implementation, serialize `remainCards` with `<first> = position.next_to_act`, and emit the four hands clockwise from that direction.
- Hand strings use suit order `S.H.D.C`, and ranks within each suit use descending order `A K Q J T 9 8 7 6 5 4 3 2`.
- Reject `remainCards` strings with length `>= 80`.

Use the existing `DDS_LOCK` around `SolveBoardPBN` calls.

Add a reusable serializer rather than overloading complete-deal serialization:

```rust
pub fn hands_to_dds_pbn(hands: &[Hand; 4], first_hand: Direction) -> String;
```

This mirrors the existing `deal_to_dds_pbn` behavior, but it works for residual `Position` hands that may contain fewer than `13` cards per player.

### 5. Solve One Position (`src/dds/solver.rs`)

Add `DdsSolver::solve_position`.

```rust
impl DdsSolver {
    pub fn solve_position(
        position: &Position,
        trump: Strain,
    ) -> Result<Vec<CardResult>, Error>;
}

pub struct CardResult {
    pub card: Card,
    pub tricks_for_side_to_act: u8,
    pub is_optimal: bool,
}
```

Call `SolveBoardPBN` with:

- `target = -1`
- `solutions = 3`
- `mode = 1`
- `thrId = 0`

Map `futureTricks` entries to `CardResult`. `is_optimal` is true when `tricks_for_side_to_act` equals the best returned score.

Equivalent lower cards encoded by `futureTricks.equals` may be ignored in the first implementation, but the limitation must be documented in code and tests should cover the primary returned card.

### 6. Build A `Position Matrix` (`src/dds/solver.rs`)

Add a method for clean residual snapshots.

```rust
pub fn solve_position_matrix(
    snapshot: &Position,
) -> Result<PositionMatrix, Error>;

pub struct PositionMatrix {
    /// `data[strain][next_to_act]`.
    pub data: [[u8; 4]; 5],
}
```

Requirements:

- Validate `snapshot` as `EntrySnapshot`.
- For each `next_to_act` in `N`, `E`, `S`, `W`, clone the snapshot and set `position.next_to_act`.
- For each `Strain`, call `solve_position`.
- Store the best `tricks_for_side_to_act` for that `next_to_act` and `Strain`.

This matrix is for residual positions. It must not be described as a `declarer` matrix.

### 7. Add State Advancement (`src/core/position.rs`)

Add trial-play mechanics.

```rust
pub fn legal_cards(position: &Position) -> Vec<Card>;

pub fn play_card(
    position: &Position,
    card: Card,
    trump: Strain,
) -> Result<Position, Error>;
```

Rules:

- The card must be in `position.hands[position.next_to_act]`.
- If `current_trick` is not empty and the player has the led suit, the card must follow suit.
- Remove the card from the hand.
- Append `PlayedCard` to `current_trick`.
- If the trick now has `4` cards, determine the winner, clear `current_trick`, and set `next_to_act` to the winner.
- Otherwise set `next_to_act` to the clockwise next player.

This function is the shared basis for later `UI` undo and replay. The first implementation does not need persistent undo history.

### 8. Add Residual Input Parsing (`src/core/pbn.rs`)

Add a parser for short residual hands. Do not reuse `parse_deal_tag`, because that function requires `13` cards per hand.

Proposed residual tag:

```pbn
[Position "N:QJ6.K65.J8.T9 -.J97.AT76.Q K5.T83.KQ9.A AT94.AQ4.-.KJ3"]
[First "S"]
[Trump "S"]
```

Rules:

- `Position` uses the same four-hand clockwise format as `Deal`.
- Each hand may contain fewer than `13` cards.
- All four hands must contain the same number of cards.
- No card may appear twice.
- `First` is required for residual input.
- `Trump` is optional.
- `current_trick` is empty for manual residual input.

The `Position` tag name is phase-local unless `phases/pbn-input-contract.md` is updated during implementation.

### 9. Extend The `CLI` (`src/cli/main.rs`)

Keep existing full-deal behavior unchanged when the input contains `Deal`.

Add residual position behavior when the input contains `Position`.

Required flag:

```rust
#[arg(long)]
trump: Option<String>
```

Behavior:

- `bridge solve` with `Deal` and no `Position` keeps Phase `1a` output.
- `bridge solve --trump S` with `Position` prints continuation analysis for the supplied `First`.
- `bridge solve --format json --trump S` with `Position` emits a `continuation` object.
- `bridge solve --matrix` with `Position` emits the residual `next_to_act x strain` matrix.
- Continuation analysis may read `trump` from `--trump` or an optional `Trump` tag.
- `--trump` overrides `Trump` when both are present.
- Missing both `--trump` and `Trump` for continuation analysis returns a clear error.
- The residual matrix mode does not require `trump`, because it evaluates all strains.

The exact `CLI` flag names may be adjusted during implementation if `clap` ergonomics suggest a cleaner shape, but the distinction between `Position Matrix` and `Continuation Analysis` must remain explicit.

### 10. Extra Step: Import `Play` Trace (`src/core/play.rs`)

This step is part of `Phase 1b`, but it depends on the core `Position` pipeline.

Add a `Play` parser that converts a complete `Deal + Play + trump` into a `Position`.

```rust
pub struct RawPlayTrace {
    pub tag_leader: Option<Direction>,
    pub cards: Vec<Card>,
}

pub fn parse_play_tag(value: &str) -> Result<RawPlayTrace, Error>;

pub fn position_from_play_trace(
    deal: &Deal,
    opening_leader: Direction,
    trump: Strain,
    trace: &RawPlayTrace,
) -> Result<PositionFromTrace, Error>;

pub struct PositionFromTrace {
    pub position: Position,
    pub tricks_won_ns: u8,
    pub tricks_won_ew: u8,
    pub warnings: Vec<PlayWarning>,
}
```

Parsing rules:

- Accept an optional leading direction prefix such as `W:`.
- Parse cards as a flat play-order sequence.
- Whitespace and `=` may both separate cards.
- Do not require the card count to be a multiple of `4`.

Validation rules:

- A played card must belong to the player who is to act.
- A card may not be played twice.
- Follow-suit violations are warnings for imported `Play` traces.
- If the optional `Play` prefix conflicts with the derived opening leader, return a warning or error before deriving the position. Prefer error in `CLI` mode.

This import path may also track `tricks_won_ns` and `tricks_won_ew`, but the core solver must not require those counts.

## Output

### Text Continuation Output

Example:

```text
Trump: S
Next to act: S
Current trick: empty

Suggested plays:
  SK  8 tricks for side to act  optimal
  DQ  8 tricks for side to act  optimal
  S5  7 tricks for side to act
  H3  6 tricks for side to act
```

### Text Matrix Output

Example:

```text
Position matrix: tricks for side to act
      S  H  D  C NT
  N   5  6  5  7  6
  E   8  6  7  5  6
  S   5  6  5  7  6
  W   8  6  7  5  6
```

### `JSON` Continuation Output

```json
{
  "continuation": {
    "trump": "S",
    "next_to_act": "S",
    "current_trick": [],
    "suggested": [
      { "card": "SK", "tricks_for_side_to_act": 8, "optimal": true },
      { "card": "S5", "tricks_for_side_to_act": 7, "optimal": false }
    ]
  }
}
```

### `JSON` Matrix Output

```json
{
  "position_matrix": {
    "row_semantics": "next_to_act",
    "value_semantics": "tricks_for_side_to_act",
    "rows": ["N", "E", "S", "W"],
    "columns": ["S", "H", "D", "C", "NT"],
    "values": {
      "N": { "S": 5, "H": 6, "D": 5, "C": 7, "NT": 6 },
      "E": { "S": 8, "H": 6, "D": 7, "C": 5, "NT": 6 },
      "S": { "S": 5, "H": 6, "D": 5, "C": 7, "NT": 6 },
      "W": { "S": 8, "H": 6, "D": 7, "C": 5, "NT": 6 }
    }
  }
}
```

## Error Handling

Add or reuse error variants in `src/core/error.rs`.

```rust
#[error("invalid trump '{0}'; expected one of: S, H, D, C, NT")]
InvalidTrump(String),

#[error("invalid first player '{0}'; expected one of: N, E, S, W")]
InvalidFirst(String),

#[error("invalid position: {0}")]
InvalidPosition(String),

#[error("invalid play trace: {0}")]
InvalidPlayTrace(String),
```

Warnings should be returned as typed values where possible. Do not model warnings as `Error` variants unless they must stop execution.

## Verification

### Automated (`cargo test`)

- `core::position` unit tests:
  - Valid `EntrySnapshot` with equal hand counts passes.
  - Unequal manual residual hands fail.
  - `Runtime` position with `current_trick` length `1`, `2`, and `3` passes.
  - `next_to_act` validation catches out-of-order current trick states.
  - `legal_cards` enforces follow suit.
  - `play_card` advances within a trick.
  - `play_card` clears a completed trick and sets the winner as `next_to_act`.

- `dds` integration tests:
  - `solve_position` on a full-deal first-trick position matches known `DDS` expectations.
  - `solve_position` with `current_trick` populated succeeds.
  - `solve_position_matrix` returns the same values as the Phase `1a` table when started from a full deal and using equivalent first-player semantics where applicable.
  - `solutions = 3` returns non-optimal legal cards as well as optimal cards.

- `pbn` parser tests:
  - Residual `Position` tag parses short hands.
  - Duplicate cards fail.
  - Unequal hand sizes fail for manual residual input.
  - `First` is required for residual input.
  - Optional `Trump` tag parses.
  - `--trump` overrides `Trump` for continuation analysis.

- Extra `play` tests:
  - `Play` tag parses flat sequences separated by whitespace and `=`.
  - Incomplete final trick derives `current_trick`.
  - Completed final trick derives an empty `current_trick` and winner as `next_to_act`.
  - Duplicate and ownership errors fail.
  - Follow-suit violations return warnings and still derive a best-effort `Position`.

### Manual Checks

- `bridge solve < full-deal.pbn` keeps the Phase `1a` output.
- `bridge solve --matrix < residual-position.pbn` prints a residual `next_to_act x strain` matrix.
- `bridge solve --trump S < residual-position.pbn` prints legal continuation cards.
- `bridge solve --format json --trump S < residual-position.pbn` emits a `continuation` object.
- Invalid residual input exits non-zero with a clear error.

## Implementation Order

1. Add `Position` types and validation.
2. Add `DDS` `FFI` declarations for `SolveBoardPBN`.
3. Implement `Position` to `dealPBN` conversion.
4. Implement `DdsSolver::solve_position`.
5. Implement `legal_cards` and `play_card`.
6. Implement `solve_position_matrix`.
7. Add residual `PBN` parsing.
8. Extend `CLI` output for residual continuation and matrix modes.
9. Add tests for each completed layer.
10. Add the extra `Play` trace import path after the position pipeline is stable.

Work should begin only after this revised plan is reviewed and confirmed.
