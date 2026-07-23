# Phase 2 — `API` Service

## Goal

Expose `bridge-dds` as an `HTTP API` so the Phase 3 `Web` front-end can compute double-dummy results via `HTTP`. The `API` is a thin wrapper around a shared application layer — no solver logic lives in the `HTTP` layer.

This phase has two sub-steps, executed sequentially:

- **`2a` — Shared application layer.** Fix existing correctness bugs in the `mid-trick` `DDS` conversion, establish unified type boundaries and a single `PBN` parser, harden solver boundaries, define the full public contract for four `endpoint`s, and refactor the `CLI` to share application logic and `JSON` output with the `API`.
- **`2b` — `HTTP` service.** Add the `bridge-server` binary with `endpoint`s, bounded-queue worker model, admission control, error protocol, `AnalysePlayPBN` integration, and full service verification.

`Phase 2b` may only begin after `Phase 2a` is verified.

## Reference

- Core library: `src/core/` and `src/dds/` (Phase 1a + 1b).
- `DDS` functions: `CalcDDtablePBN`, `DealerPar`, `SolveBoardPBN`, `SetMaxThreads`, `ErrorMessage`.
- `Phase 2b` additions: `AnalysePlayPBN`, `playTracePBN`, `solvedPlay` (`engine/dds/include/dll.h`).
- `DDS` documentation: `engine/dds/doc/dll-description.md`.
- `PBN` specification: `phases/pbn-input-contract.md` and <https://www.tistis.nl/pbn/pbn_v21.txt>.
- Dependencies: latest stable `axum` (<https://docs.rs/axum>), `tower-http` (<https://docs.rs/tower-http>), `tokio` (<https://docs.rs/tokio>) with minimal features.

## Key Decisions (Confirmed)

### 1. User-Facing Position Semantics

The public input model (`SnapshotPosition`):

- `hands`: the cards each player held **before** the current trick began. All four hands have equal count.
- `current_trick`: `0` to `3` cards already played to this trick, in play order. These cards are **still present** in `hands`. Player identity is derived from `trick_leader` and clockwise order — not repeated per card.
- `trick_leader`: the player who led the current trick.
- `next_to_act`: the player whose turn it is now, derived from `trick_leader` and `current_trick.len()`.

Before calling `DDS`, the application layer converts to an internal `PlayPosition`:

- Validate `current_trick` ownership, order, and follow-suit.
- Validate any externally supplied `next_to_act` against the player derived from `trick_leader` and `current_trick.len()`.
- Remove `current_trick` cards from `hands`.
- `dealPBN.first = trick_leader` (not `next_to_act`).
- `dealPBN.currentTrickSuit` / `dealPBN.currentTrickRank` filled in play order from `current_trick`.
- `dealPBN.remainCards` contains only the cards still in `hands` after removal.
- The application derives the player that `DDS` will treat as next to act as `trick_leader.advance(current_trick.len())`.

`DDS` returns tricks for the **actual side to play** — the side containing the derived next player. Therefore `score_side` is the side of `next_to_act` (not `trick_leader`). The response field `tricks_for_score_side` reflects this.

Shared position data is represented by two reusable value types:

- `Hands`: four hands indexed by absolute direction. It guarantees no card occurs in more than one hand and no hand contains more than `13` cards.
- `CurrentTrick`: the incomplete current trick's leader and `0..=3` cards in play order. It derives each card's player and `next_to_act`; these values are not stored separately.

The user-facing `SnapshotPosition` and internal `PlayPosition` are separate types with explicit conversion boundaries:

- `SnapshotPosition` uses snapshot semantics: current-trick cards remain in `hands`, and all four hands have equal counts.
- `PlayPosition` uses remaining-hand semantics: current-trick cards have been removed from `remaining_hands`. It is the only mutable position type and is also the validated, read-only position accepted by the `DDS` wrapper.

No type carries ambiguous hand semantics across different code paths.

### 2. Input Sources And Override Rules

Each `endpoint` accepts input from three sources, merged with this priority:

```text
`URL query` > `JSON body` fields > `PBN` fields
```

Override and conflict rules:

- **Parse before merge**: every provided representation must independently pass syntax validation and intrinsic field-value validation before participating in the merge. A valid higher-priority field cannot hide an invalid lower-priority field.
- **Cross-source override**: the same field name appearing in different sources is resolved by priority — `query` overrides `body`, `body` overrides `PBN`. No error is produced.
- **Final cross-field semantic validation**: relationships between fields are validated only after field-level source overrides. Related final values that contradict each other (e.g. final `declarer` and `opening_leader` values where `opening_leader != declarer.next()`) produce a `400` error. A higher-priority override may therefore resolve a cross-field conflict present in a lower-priority source, but it cannot hide invalid syntax or an intrinsically invalid field value.
- **Duplicate keys within a single representation**: behavior depends on the underlying parser and is not guaranteed to be detected.
- **Unknown `JSON` fields**: rejected with a `400` error.
- **Explicit `null`**: rejected with a `400` error. Only field omission means "not provided from this source."
- **`URL query` fields not listed for the current `endpoint`**: rejected with a `400` error.
- **Known `PBN` tags not applicable to the current `endpoint`**: rejected with a `400` error. Unknown `PBN` tags are silently ignored.

Simple fields allowed in `URL query` vary by `endpoint`. Complex fields (`pbn`, `deal`, `hands`, `current_trick`, `play`) must go in the `JSON body`.

After merging, the application layer performs full validation on the final normalized input.

### 3. Public Endpoints

```text
`POST /api/v1/analyze/deal`
`POST /api/v1/analyze/position/matrix`
`POST /api/v1/analyze/position/continuation`
`POST /api/v1/analyze/play`
```

Each `endpoint` has an independent request `DTO` and response type.

### 4. `JSON` Representation

- `Card`: `"SA"` (suit letter + rank character).
- `Direction`: `"N"`, `"E"`, `"S"`, `"W"`.
- `Strain`: `"S"`, `"H"`, `"D"`, `"C"`, `"NT"`.
- `Vulnerability`: `"None"`, `"NS"`, `"EW"`, `"All"`.
- `hands`: a map from direction to an array of card strings.
- `current_trick`: an array of card strings in play order.

The public `JSON`/query input accepts only the canonical forms listed above (case-sensitive). `PBN` input may accept its documented aliases.

The public protocol must never expose the internal `u64` bit layout of `Hand`. Request `DTO`s reject unknown `JSON` fields.

### 5. Error Protocol

All `HTTP` errors use:

```json
{
  "error": {
    "code": "invalid_position",
    "message": "current_trick has 4 cards, max 3"
  }
}
```

Stable error codes (minimum set):

| Code | Meaning |
|------|---------|
| `invalid_request` | Unknown `JSON` field, explicit `null`, or unknown query field |
| `invalid_pbn` | `PBN` parse or tag error |
| `invalid_deal` | Deal completeness, duplicates, or count errors |
| `invalid_position` | Position invariant violation |
| `invalid_play_trace` | Play trace card ownership, duplicate, or follow-suit error |
| `invalid_trump` | Invalid trump/strain value |
| `invalid_direction` | Invalid direction value |
| `invalid_vulnerability` | Invalid vulnerability value |
| `conflicting_input` | Cross-field semantic conflict after merging |
| `missing_field` | Required field absent after merging |
| `solver_error` | `DDS` internal error |
| `body_too_large` | Request body exceeds the configured limit (`413`) |
| `queue_overloaded` | Solver queue full (`503`) |
| `request_timeout` | Request timed out (`504`) |
| `internal_error` | Unexpected server error (`500`) |

HTTP status codes: `400` for input/validation errors, `413` for body too large, `500` for internal, `503` for queue overload, `504` for timeout.

### 6. Server Runtime

- `bridge-server` calls the public application-level `initialize_solver()` once at startup; the low-level `DdsSolver` remains internal.
- An `async` worker task holds a bounded `mpsc::Receiver<SolveJob>`. Each `SolveJob` carries a request payload and a `oneshot::Sender` for the response.
- On receiving a job, the worker calls `tokio::task::spawn_blocking()` to run the synchronous application/`DDS` call. The result is sent back through the `oneshot` channel.
- If the `oneshot` receiver is already closed (request timed out), the worker skips the job before calling `spawn_blocking`. An already-started `DDS` call cannot be cancelled.
- Defaults: queue capacity `16`, request timeout `10s`, body size limit `1 MB`. These are configurable for tests.
- The global `DDS_LOCK` serializes all solver calls. Queue capacity provides backpressure; when full, `503` is returned immediately.
- Development `CORS` allows only the configured `Vite` origin, defaulting to `http://localhost:5173`, with `POST` and `Content-Type` enabled. The origin is configurable for development and tests; no wildcard origin is used.

### 7. Unified `PBN` Parser And Supported Play Records

All `PBN` input paths use one shared single-record parser. The parser handles both tag lines and supported section data, and returns a shared `ParsedRecord` with optional structured fields. `CLI`, application, transport, and `HTTP` code must not independently scan or parse tag strings.

The parser supports:

- Tag-only fields: `Deal`, `Dealer`, `Vulnerable`, `Position`, `First`, `Trump`, `CurrentTrick`, `Contract`, and `Declarer`.
- Standard `Auction` and `Play` section headers followed by section data until the next tag or end of record.
- Standard play input:

```pbn
[Contract "4S"]
[Declarer "N"]
[Play "E"]
S3 S5 S2 SQ
```

- The existing legacy inline play forms for backward compatibility:

```pbn
[Play "E:S3=S5=S2=SQ"]
[Play "S3=S5=S2=SQ"]
[Play ""]
```

A `Play` value that is exactly one bare `Direction` is standard `Play`; every other value is legacy inline `Play`. Legacy input retains its optional direction prefix and chronological card sequence. Whitespace and `=` delimit cards but do not define trick boundaries. The application normalization layer converts the standard and legacy forms into the same chronological structured play sequence after source merging. Duplicate `Play` definitions or a legacy inline `Play` tag followed by section data are errors.

The standard `Play` section is parsed as trick rows with four fixed player columns. The direction in the `Play` tag identifies the first column, and the remaining columns proceed clockwise. The parser preserves this row/column structure; normalization combines it with the final merged `deal` and `trump`, validates that each card appears in the correct player's column, determines each completed trick's winner, and emits cards in chronological play order. It must not flatten rows directly because the chronological leader can change between tricks.

Phase 2 accepts explicit card tokens and the standard `-` placeholder only where required to represent seats that have not yet played in the incomplete final trick. Normalization rejects unknown-card gaps, a real card after a missing chronological turn, placeholders in completed tricks, and any play data after the incomplete trick. Claims, annotations, comments, and all other placeholder uses are outside the supported subset and return `invalid_pbn`.

`CurrentTrick` parses into an ordered sequence of player/card pairs. The parser validates syntax, duplicate tags, and the intrinsic value checks: entry count, clockwise player order derived from the first entry, and duplicate cards within the value. Continuation normalization derives `trick_leader` and the public card-only `current_trick`, then validates card ownership, follow-suit, and consistency with `First`/`next_to_act`.

`Contract` parsing supports levels `1` to `7`, strains `S`, `H`, `D`, `C`, and `NT`, with optional `X` or `XX`. The play endpoint extracts only the strain; level and doubling are preserved in `ParsedRecord` but do not affect double-dummy play analysis. Passed-out contract values are rejected for play analysis.

`Auction` section data is captured by the shared parser so complete records are accepted, but semantic auction analysis is outside Phase 2. Endpoint-specific validation decides which parsed tags and sections are applicable.

---

## Endpoint Contracts

### `POST /api/v1/analyze/deal`

**Purpose**: Compute the full `4x5` double-dummy tricks matrix and `DealerPar` result.

**Accepted `PBN` tags**: `Deal`, `Dealer`, `Vulnerable`.

**Allowed `URL query` fields**: `dealer`, `vulnerable`.

**`JSON` request schema**:

```json
{
  "pbn": "[Deal \"N:QJ6.K652.J85.T98 873.J97.AT764.Q4 K5.T83.KQ9.A7652 AT942.AQ4.32.KJ3\"]\n[Dealer \"N\"]\n[Vulnerable \"None\"]",
  "deal": "N:QJ6.K652.J85.T98 873.J97.AT764.Q4 K5.T83.KQ9.A7652 AT942.AQ4.32.KJ3",
  "dealer": "N",
  "vulnerable": "None"
}
```

All fields optional. After merging, `deal`, `dealer`, and `vulnerable` are required.

**Normalized application command**: `AnalyzeDeal { deal: Deal, dealer: Direction, vulnerable: Vulnerability }`.

**`JSON` response schema**:

```json
{
  "tricks": {
    "N": { "S": 5, "H": 6, "D": 5, "C": 7, "NT": 6 },
    "E": { "S": 8, "H": 6, "D": 7, "C": 5, "NT": 6 },
    "S": { "S": 5, "H": 6, "D": 5, "C": 7, "NT": 6 },
    "W": { "S": 8, "H": 6, "D": 7, "C": 5, "NT": 6 }
  },
  "par": {
    "score": -110,
    "contracts": ["2S-EW"]
  }
}
```

`tricks` rows are declarers. Values are tricks for the declaring side. `par.score` is from the `NS` perspective: positive means `NS` gain, negative means `EW` gain.

**Validation**: complete deal (52 unique cards, 13 per hand), valid direction, valid vulnerability.

### `POST /api/v1/analyze/position/matrix`

**Purpose**: Compute a `next_to_act x strain` matrix from a clean residual snapshot.

**Accepted `PBN` tags**: `Position`.

**Allowed `URL query` fields**: none.

**`JSON` request schema**:

```json
{
  "pbn": "[Position \"N:AKQJ... .AKQJ.. ..AKQJ. ...AKQJ\"]",
  "hands": {
    "N": ["SA", "SK", "SQ", "SJ"],
    "E": ["HA", "HK", "HQ", "HJ"],
    "S": ["DA", "DK", "DQ", "DJ"],
    "W": ["CA", "CK", "CQ", "CJ"]
  }
}
```

All fields optional. After merging, `hands` is required. `current_trick` is not accepted; any `current_trick` field (in body or `PBN` tag) is rejected as unknown/inapplicable.

**Normalized application command**: `AnalyzePositionMatrix { hands: Hands }`. No `first` or `trick_leader` — the use case internally iterates over all four `next_to_act` values.

**`JSON` response schema**:

```json
{
  "matrix": {
    "row_semantics": "next_to_act",
    "value_semantics": "tricks_for_score_side",
    "values": {
      "N": { "S": 4, "H": 0, "D": 0, "C": 0, "NT": 4 },
      "E": { "S": 0, "H": 4, "D": 0, "C": 0, "NT": 4 },
      "S": { "S": 0, "H": 0, "D": 4, "C": 0, "NT": 4 },
      "W": { "S": 0, "H": 0, "D": 0, "C": 4, "NT": 4 }
    }
  }
}
```

Rows are `next_to_act` values. Values are tricks for the side containing `next_to_act`.

**Validation**: `Hands` general invariants and four equal hand counts.

### `POST /api/v1/analyze/position/continuation`

**Purpose**: Evaluate all legal continuations from a position with a specified `trump`.

**Accepted `PBN` tags**: `Position`, `First`, `Trump`, `CurrentTrick`. `First` maps to `next_to_act`. When `CurrentTrick` is non-empty, its first entry's player maps to `trick_leader`; when empty, `trick_leader = next_to_act`.

**Allowed `URL query` fields**: `trump`, `trick_leader`, `next_to_act`.

**`JSON` request schema**:

```json
{
  "pbn": "[Position \"N:AKQJ... .AKQJ.. ..AKQJ. ...AKQJ\"]\n[First \"E\"]\n[Trump \"NT\"]\n[CurrentTrick \"N:SA\"]",
  "hands": {
    "N": ["SA", "SK", "SQ", "SJ"],
    "E": ["HA", "HK", "HQ", "HJ"],
    "S": ["DA", "DK", "DQ", "DJ"],
    "W": ["CA", "CK", "CQ", "CJ"]
  },
  "trump": "NT",
  "trick_leader": "N",
  "current_trick": ["SA"],
  "next_to_act": "E"
}
```

After merging, `trump`, `next_to_act`, and `hands` are required. `trick_leader` defaults to `next_to_act` when `current_trick` is empty. `current_trick` defaults to empty.

**Normalized application command**: `AnalyzeContinuation { position: SnapshotPosition, trump }`. Transport normalization validates the externally supplied `next_to_act`, then constructs `CurrentTrick` and `SnapshotPosition`; the application command does not store a duplicate `next_to_act`.

**`JSON` response schema**:

```json
{
  "continuation": {
    "trump": "NT",
    "trick_leader": "N",
    "current_trick": ["SA"],
    "next_to_act": "E",
    "score_side": "EW",
    "suggested": [
      { "card": "HA", "tricks_for_score_side": 4, "optimal": true }
    ]
  }
}
```

`score_side` is the side of `next_to_act` (the side `DDS` reports tricks for). `tricks_for_score_side` is the number of tricks that side can achieve from this position.

**Validation**: `Hands` general invariants, four equal hand counts including `current_trick` cards, `current_trick` length `0..3`, cards held by correct players (derived clockwise from `trick_leader`), `next_to_act` matches derived next player, follow-suit enforced, `CurrentTrick` tag entry order validated intrinsically by the parser.

### `POST /api/v1/analyze/play`

**Purpose**: Analyze a played sequence. Returns per-card evaluation and continuation from the final position.

**Accepted `PBN` tags and sections**: `Deal`, `Dealer`, `Vulnerable`, `Contract`, `Declarer`, `Auction`, and `Play`. Standard `Auction`/`Play` section data and the legacy inline `Play` form are accepted through the unified parser. The `Play` direction serves as `opening_leader` at the `PBN` priority level. `Contract` and `Declarer` provide `trump` and `declarer` respectively.

**Allowed `URL query` fields**: `trump`, `declarer`, `opening_leader`.

**`JSON` request schema**:

```json
{
  "pbn": "[Deal \"N:QJ6.K652.J85.T98 873.J97.AT764.Q4 K5.T83.KQ9.A7652 AT942.AQ4.32.KJ3\"]\n[Dealer \"N\"]\n[Vulnerable \"None\"]\n[Contract \"4S\"]\n[Declarer \"N\"]\n[Play \"E\"]\nS3 S5 S2 SQ",
  "deal": "N:QJ6.K652.J85.T98 873.J97.AT764.Q4 K5.T83.KQ9.A7652 AT942.AQ4.32.KJ3",
  "trump": "S",
  "declarer": "N",
  "opening_leader": "E",
  "play": ["S3", "S5", "S2", "SQ"]
}
```

After merging, `trump`, `deal`, and the play sequence are required. `opening_leader` is required; if absent, it is derived from `declarer` as `opening_leader = declarer.next()`. If both `declarer` and `opening_leader` are provided, cross-validation requires `opening_leader == declarer.next()`. Conversely, from an `opening_leader`, `declarer` is derived as `opening_leader.previous()`. `play` length is `0..=52`; empty plays are valid (returns the opening position without historical evaluation).

**Normalized application command**: `AnalyzePlay { deal, trump, opening_leader, played_cards }`.

**`JSON` response schema**:

```json
{
  "play_analysis": {
    "trace": [
      {
        "player": "E",
        "card": "S3",
        "tricks_before": 5,
        "tricks_after": 5,
        "delta_for_declarer": 0,
        "is_mistake": false
      },
      {
        "player": "S",
        "card": "S5",
        "tricks_before": 5,
        "tricks_after": 5,
        "delta_for_declarer": 0,
        "is_mistake": false
      },
      {
        "player": "W",
        "card": "S2",
        "tricks_before": 5,
        "tricks_after": 5,
        "delta_for_declarer": 0,
        "is_mistake": false
      },
      {
        "player": "N",
        "card": "SQ",
        "tricks_before": 5,
        "tricks_after": 5,
        "delta_for_declarer": 0,
        "is_mistake": false
      }
    ],
    "final_position": {
      "hands": {
        "N": ["SJ", "S6", "HK", "H6", "H5", "H2", "DJ", "D8", "D5", "CT", "C9", "C8"],
        "E": ["S8", "S7", "HJ", "H9", "H7", "DA", "DT", "D7", "D6", "D4", "CQ", "C4"],
        "S": ["SK", "HT", "H8", "H3", "DK", "DQ", "D9", "CA", "C7", "C6", "C5", "C2"],
        "W": ["SA", "ST", "S9", "S4", "HA", "HQ", "H4", "D3", "D2", "CK", "CJ", "C3"]
      },
      "trick_leader": "N",
      "current_trick": [],
      "next_to_act": "N"
    },
    "final_continuation": {
      "trump": "S",
      "trick_leader": "N",
      "current_trick": [],
      "next_to_act": "N",
      "score_side": "NS",
      "suggested": [
        { "card": "S6", "tricks_for_score_side": 4, "optimal": true },
        { "card": "DJ", "tricks_for_score_side": 4, "optimal": true },
        { "card": "D8", "tricks_for_score_side": 4, "optimal": true },
        { "card": "D5", "tricks_for_score_side": 4, "optimal": true },
        { "card": "CT", "tricks_for_score_side": 4, "optimal": true },
        { "card": "C9", "tricks_for_score_side": 4, "optimal": true },
        { "card": "C8", "tricks_for_score_side": 4, "optimal": true },
        { "card": "SJ", "tricks_for_score_side": 3, "optimal": false },
        { "card": "HK", "tricks_for_score_side": 3, "optimal": false },
        { "card": "H6", "tricks_for_score_side": 3, "optimal": false },
        { "card": "H5", "tricks_for_score_side": 3, "optimal": false },
        { "card": "H2", "tricks_for_score_side": 3, "optimal": false }
      ]
    }
  }
}
```

`trace` contains exactly one entry per played card. For the first entry, `tricks_before` is the double-dummy result before the opening lead. `tricks_after` for entry `i` is the result after card `i` is played. No separate initial-state entry.

`final_position` uses the user snapshot model: hands are equal count, `current_trick` cards (if any) are present. When the trace ends on a complete trick boundary, `current_trick` is empty and all hands have equal count. When it ends mid-trick, the already-played cards of the incomplete trick are added back to their owners' hands and appear in `current_trick`.

`final_continuation` is absent when all 52 cards are played.

All examples containing concrete solver values must be generated from and checked against a verified `DDS` fixture.

**Mistake detection**: `AnalysePlayPBN` returns tricks from the declarer's perspective. `is_mistake` is:

- Declarer-side player: true when `delta_for_declarer < 0`.
- Defender-side player: true when `delta_for_declarer > 0`.
- `delta_for_declarer == 0` is never a mistake.

**Validation**: complete deal, follow-suit enforced, no duplicate cards, each played card held by the correct player at the time of play.

---

## Phase 2a — Shared Application Layer

Tasks are listed in priority order. Bug fixes and correctness-critical items come first.

### Task 1: Establish Type Boundaries And Fix `mid-trick` DDS Conversion

In `src/core/deal.rs`, add a shared four-hand value type:

```rust
pub enum HandsError {
    TooManyCards { direction: Direction, count: usize },
    DuplicateCard { card: Card },
    CardNotHeld { direction: Direction, card: Card },
}

pub struct Hands {
    hands: [Hand; 4],
}
```

`Hands` represents cards currently assigned to the four absolute directions. Its fields are private, and every construction and mutation path must preserve these general invariants:

- Exactly four hands are present, indexed by `N`, `E`, `S`, and `W`; the `[Hand; 4]` storage guarantees this internally.
- Each hand contains at most `13` cards.
- No card occurs in more than one hand.
- A single `Hand` contains no duplicate card, as already guaranteed by `Hand`.

Provide and test:

```rust
impl Hands {
    pub fn try_new(hands: [Hand; 4]) -> Result<Self, HandsError>;
    pub fn get(&self, direction: Direction) -> &Hand;
    pub fn iter(&self) -> impl Iterator<Item = (Direction, &Hand)>;
    pub fn counts(&self) -> [usize; 4];
    pub fn total_count(&self) -> usize;
    pub fn owner_of(&self, card: Card) -> Option<Direction>;
    pub fn remove(&self, direction: Direction, card: Card) -> Result<Self, HandsError>;
    pub fn add(&self, direction: Direction, card: Card) -> Result<Self, HandsError>;
}
```

`Hands::remove()` fails unless the specified player owns the card. `Hands::add()` fails if the card is already owned by any player or if adding it would exceed `13` cards in that hand.

`HandsError` is domain-neutral because `Hands` is shared by deal and position use cases. A `Deal` construction boundary maps it to `invalid_deal`; a position construction boundary maps it to `invalid_position`.

`Deal`, residual position parsing, application commands, and position types use `Hands` instead of a raw `[Hand; 4]`. `Deal` adds its own complete-deal invariant: every hand has exactly `13` cards and the total is `52`. Equal hand counts are not a `Hands` invariant because a valid mid-trick `PlayPosition` temporarily has unequal remaining-hand counts. `Deal` fields become private and are created through a validated constructor; read-only accessors expose the original `PBN` serialization direction and hands:

```rust
impl Deal {
    pub fn try_new(first: Direction, hands: Hands) -> Result<Self, Error>;
    pub fn first(&self) -> Direction;
    pub fn hands(&self) -> &Hands;
}
```

Also add and test:

```rust
impl Direction {
    pub fn advance(self, seats: usize) -> Direction {
        Direction::from_dds_index((self.dds_index() + seats) % 4).unwrap()
    }
}
```

In `src/core/position.rs`, replace the semantic-overloaded `Position` and `PositionKind` models with a shared current-trick value type and two position types:

```rust
pub struct CurrentTrick {
    leader: Direction,
    cards: Vec<Card>,
}

pub struct SnapshotPosition {
    hands: Hands,
    current_trick: CurrentTrick,
}

pub struct PlayPosition {
    remaining_hands: Hands,
    current_trick: CurrentTrick,
}
```

`CurrentTrick` guarantees that `cards.len() <= 3`. Player identity and the next player are derived rather than stored:

```rust
impl CurrentTrick {
    pub fn try_new(leader: Direction, cards: Vec<Card>) -> Result<Self, Error>;
    pub fn leader(&self) -> Direction;
    pub fn cards(&self) -> &[Card];
    pub fn player_at(&self, index: usize) -> Option<Direction>;
    pub fn next_to_act(&self) -> Direction;
    pub fn led_suit(&self) -> Option<Suit>;
}
```

When `CurrentTrick` is empty, its `leader` is the player who leads the next trick and is therefore also `next_to_act`. During transport normalization, any separately supplied `next_to_act` is cross-validated against `CurrentTrick::next_to_act()` before constructing an application command.

`SnapshotPosition` is the public input and output model. Its `hands` include the incomplete current trick's cards and have equal counts. It validates:

- `Hands` general invariants.
- Equal hand counts.
- Every current-trick card is held by its player derived from `CurrentTrick`.
- No current-trick card occurs in another player's hand.
- Follow-suit for every played current-trick card.

`PlayPosition` is the internal advancement model and the only position type accepted by the `DDS` wrapper. Current-trick cards have already been removed from `remaining_hands`; adding them back to their derived owners reconstructs a valid `SnapshotPosition`. It validates:

- `Hands` general invariants.
- Current-trick cards are absent from all `remaining_hands`.
- Adding current-trick cards back to their derived owners produces equal hand counts.
- Follow-suit reconstructed from the pre-trick hands.

All fields of `Hands`, `CurrentTrick`, `SnapshotPosition`, and `PlayPosition` are private. They expose only validated constructors, read-only accessors, and mutation methods that preserve their invariants. Transport `DTO`s may use maps and arrays, but must convert them into these domain types before reaching application use cases.

Provide validated constructors and the read-only accessors required by application, transport, output, and solver code:

```rust
impl SnapshotPosition {
    pub fn try_new(hands: Hands, current_trick: CurrentTrick) -> Result<Self, Error>;
    pub fn hands(&self) -> &Hands;
    pub fn current_trick(&self) -> &CurrentTrick;
}

impl PlayPosition {
    pub fn try_new(
        remaining_hands: Hands,
        current_trick: CurrentTrick,
    ) -> Result<Self, Error>;
    pub fn remaining_hands(&self) -> &Hands;
    pub fn current_trick(&self) -> &CurrentTrick;
    pub fn legal_cards(&self) -> Vec<Card>;
    pub fn play_card(&mut self, card: Card, trump: Strain) -> Result<(), Error>;
}
```

Implement and test these explicit conversions:

```rust
impl TryFrom<SnapshotPosition> for PlayPosition;
impl TryFrom<&PlayPosition> for SnapshotPosition;
```

- `SnapshotPosition -> PlayPosition` validates snapshot invariants and removes each current-trick card exactly once from its derived owner.
- `PlayPosition -> SnapshotPosition` validates reconstructable invariants and adds incomplete current-trick cards back to their derived owners.
- Neither conversion may silently remove or add a card twice.

Move `legal_cards()`, `play_card()`, and trick-winner advancement from the existing `Position` implementation to `PlayPosition`. `PlayPosition::play_card()` is the only public state-mutation path and must validate ownership and follow-suit itself before removing the card; callers must not be responsible for pre-validating legality.

In `src/dds/solver.rs`, make continuation solving accept only a read-only `PlayPosition` and add:

```rust
fn to_dds_deal(position: &PlayPosition, trump: Strain) -> Result<dealPBN, Error>;
```

`to_dds_deal()` must:

- Set `dealPBN.first = position.current_trick().leader().dds_index()`.
- Fill `currentTrickSuit` and `currentTrickRank` in play order.
- Serialize only `position.remaining_hands()`.
- Use a fixed `Direction::North` prefix when calling `hands_to_dds_pbn()`. The `PBN` prefix controls only hand serialization order and is independent from `dealPBN.first`.

Replace current `Position` callers in `src/cli/main.rs`, `src/cli/output.rs`, `src/dds/solver.rs`, and `tests/dds_integration.rs` with `SnapshotPosition` or `PlayPosition` according to their hand semantics. Remove `PositionKind`. Keep `PlayedCard` only as a temporary residual-`PBN` parser representation until `Task 4`; it must not be used as one of the new position models.

Then fix two current solver errors that mask each other:

1. `dealPBN.first` is set to `next_to_act` instead of `trick_leader`.
2. `current_trick` cards remain in `dealPBN.remainCards` instead of being removed.

Because all four hands stay equal, `DDS` interprets the position as a clean trick boundary and ignores `currentTrick`. The wrong `first` value happens to return cards from the correct player, so card-suit-only tests pass. Actual scores and optimality may be wrong.

Fix:

- `dealPBN.first = trick_leader`.
- Remove `current_trick` cards from `hands` before building `remainCards`.
- Derive `next_to_act` from `CurrentTrick`; transport normalization validates any externally supplied value, and domain position types do not store a second value that can contradict it.
- Only `SnapshotPosition -> PlayPosition` removes `current_trick` cards. The `DDS` conversion must not remove already-removed cards again.
- Fix the current `CLI` play-trace path immediately: advance one `PlayPosition` through the complete trace, convert that final state to `SnapshotPosition` when needed for output, and never reconstruct the final position from the original complete deal.

Verification additions:

- `Hands` unit tests in `src/core/deal.rs`: per-hand size limit, cross-hand duplicate rejection, ownership lookup, and validated add/remove behavior.
- Conversion unit tests in `src/core/position.rs`: cover all `0..=3` current-trick lengths, all four leaders, derived players and `next_to_act`, add-back round trips, follow-suit, ownership, and rejection of double removal.
- `DDS` conversion tests in `src/dds/solver.rs`: assert `dealPBN.first = trick_leader`, the `remainCards` prefix is `N`, and `current_trick` cards are absent from `remainCards`.
- `Hands`, `CurrentTrick`, `SnapshotPosition`, and `PlayPosition` invariant tests and tests proving neither conversion path removes or adds a card twice.
- Integration test in `tests/dds_integration.rs`: use a deal where `current_trick` changes the optimal play, assert exact scores and optimal card.
- `CLI` regression: run existing mid-trick cases and verify output changes to reflect the corrected position.

### Task 2: Fix Continuation Score Side and Response Semantics

After Task 1, `DDS` returns tricks for the actual side to play — the side of `next_to_act`. Update:

- `score_side` in continuation response to reflect `next_to_act`'s side.
- Rename `tricks_for_side_to_act` to `tricks_for_score_side` in `CardResult` and all response `DTO`s.
- Update `CLI` continuation output label to use the corrected side.

Verification:

- Test a mid-trick position where `trick_leader` and `next_to_act` are on opposite sides; assert `score_side` matches `next_to_act`'s side.

### Task 3: Finalize `PBN` Contract Before Parser Implementation

Before implementing the unified parser, update `phases/pbn-input-contract.md` to define:

- Supported tag lines, including `CurrentTrick`, and supported `Auction`/`Play` section data.
- `CurrentTrick` syntax, ordered player/card representation, and continuation normalization rules.
- Standard play records and backward-compatible legacy inline `Play`.
- Standard `Play` section fixed-player-column semantics, chronological normalization after merging with `deal`/`trump` context, and the supported and unsupported `-` placeholder cases.
- Unsupported claims, annotations, comments, and other out-of-scope `PBN` features.
- The supported `Contract` subset and how `Contract`, `Declarer`, and `Play` derive play-analysis fields.
- Duplicate tag/section behavior, unknown tag policy, and malformed section errors.
- Complete-record parsing versus endpoint-specific partial-record validation.
- Endpoint-specific tag and section applicability.

Also fix `phases/1b-verification.md` case 8 so `CurrentTrick` players follow clockwise order and add the invalid-order case as an expected error example.

`Task 3` documentation changes must be reviewed before `Task 4` parser implementation begins.

### Task 4: Unify PBN Parser And Play Normalization

Replace the three current parsing paths (`parse_record`, `parse_residual_record`, `CLI` play-trace string scanning) with a single shared parser:

- Implement the explicitly documented Phase 2 subset, not a complete `PBN 2.1` implementation. Unsupported syntax must fail explicitly rather than be silently misinterpreted.
- Unified tag-line and supported section-data parsing, duplicate-tag/section detection, unknown-tag policy, and known-tag parsing.
- Output a shared `ParsedRecord` type where all fields are optional. A standard `Play` section is retained as structured trick rows with fixed player columns; the legacy inline form retains an optional opening leader and chronological card sequence.
- Classify a `Play` value deterministically: an exact bare `Direction` is standard; every other value is legacy inline. The parser must not use following section data to choose the representation.
- Preserve legacy compatibility for prefixed, unprefixed, empty, whitespace-delimited, and more-than-four-card `=` sequences.
- After source merging supplies the required `deal`, `trump`, and `opening_leader`, `normalize_play()` converts either play representation into validated chronological `PlayedCard` values and advances one `PlayPosition` to the final state. The parser itself must not flatten a standard `Play` section.
- `endpoint`s and the `CLI` select allowed tags, apply source overrides, and validate the merged result.
- No `CLI`, application, or `HTTP` code may parse or scan tag strings directly.

Implement these parser-side boundaries in `src/core/pbn.rs`:

```rust
pub struct ParsedRecord {
    pub deal: Option<Deal>,
    pub dealer: Option<Direction>,
    pub vulnerable: Option<Vulnerability>,
    pub position: Option<Hands>,
    pub first: Option<Direction>,
    pub trump: Option<Strain>,
    pub current_trick: Option<ParsedCurrentTrick>,
    pub contract: Option<ParsedContract>,
    pub declarer: Option<Direction>,
    pub auction: Option<ParsedAuction>,
    pub play: Option<ParsedPlay>,
}

pub enum ParsedPlay {
    Standard { first_column: Direction, rows: Vec<PlayRow> },
    Legacy { opening_leader: Option<Direction>, cards: Vec<Card> },
}

pub fn parse_record(input: &str) -> Result<ParsedRecord, Error>;
```

`ParsedCurrentTrick` retains player/card pairs after intrinsic entry-count, clockwise-order, and duplicate-card validation. Ownership, follow-suit, and cross-field validation remain post-merge responsibilities. `ParsedContract` retains level, strain, and doubling. `ParsedAuction` retains its first direction and tokenized calls. The full supporting type shapes are specified in `phases/2a-task-4-pre-tasks.md`.

Implement these normalization boundaries in `src/core/play.rs`:

```rust
pub struct NormalizedPlay {
    opening_leader: Direction,
    played_cards: Vec<PlayedCard>,
    final_position: PlayPosition,
}

pub fn normalize_play(
    play: &ParsedPlay,
    deal: &Deal,
    trump: Strain,
    opening_leader: Direction,
) -> Result<NormalizedPlay, Error>;
```

`normalize_play()` receives final resolved context and does not perform source merging or call `DDS`. Standard normalization retains the immutable `first_column`, maps fixed columns to absolute players, reads each row using the current trick leader, validates placeholders, and uses `PlayPosition::play_card()` for one state-advancement pass. `Task 5` consumes the returned chronological events and final position without replaying the cards.

Keep fine-grained internal `Error` variants through `Task 4`. Add `ConflictingInput(String)` for contradictory final fields. `Task 8` maps internal variants to the stable public error protocol; parser and normalization code must not depend on `HTTP` types.

During `Task 4`, the `CLI` parses once and preserves its existing operation precedence (`Position`, then `Play`, then full deal), argument names, and non-contradictory behavior. `Task 6` moves `CLI` source resolution into a shared input layer while preserving `CLI` compatibility. `Task 8` applies the stricter per-`endpoint` applicability rules to future `HTTP` requests.

### Task 5: Define Shared Application Use Cases

Create `src/application/` with:

```text
src/application/
├── mod.rs
├── deal.rs
├── position.rs
└── play.rs
```

Define the application commands, results, and functions explicitly:

```rust
pub struct AnalyzeDeal {
    pub deal: Deal,
    pub dealer: Direction,
    pub vulnerable: Vulnerability,
}

pub struct DealAnalysis {
    pub tricks: TricksMatrix,
    pub par: ParResult,
}

pub struct AnalyzePositionMatrix {
    pub hands: Hands,
}

pub struct PositionMatrixAnalysis {
    values: [[u8; 4]; 5],
}

pub struct AnalyzeContinuation {
    pub position: SnapshotPosition,
    pub trump: Strain,
}

pub struct SuggestedCard {
    pub card: Card,
    pub tricks_for_score_side: u8,
    pub is_optimal: bool,
}

pub struct ContinuationAnalysis {
    pub position: SnapshotPosition,
    pub trump: Strain,
    pub score_side: Side,
    pub suggested: Vec<SuggestedCard>,
}

pub struct AnalyzeFinalPlay {
    pub normalized: NormalizedPlay,
    pub trump: Strain,
}

pub struct FinalPlayAnalysis {
    pub final_position: SnapshotPosition,
    pub final_continuation: Option<ContinuationAnalysis>,
}

pub fn analyze_deal(command: AnalyzeDeal) -> Result<DealAnalysis, Error>;
pub fn analyze_position_matrix(
    command: AnalyzePositionMatrix,
) -> Result<PositionMatrixAnalysis, Error>;
pub fn analyze_continuation(
    command: AnalyzeContinuation,
) -> Result<ContinuationAnalysis, Error>;
pub fn analyze_final_play(
    command: AnalyzeFinalPlay,
) -> Result<FinalPlayAnalysis, Error>;
pub fn initialize_solver();
```

The exact field visibility may use constructors and read-only accessors, but the ownership and responsibilities must remain as shown. `PositionMatrixAnalysis` exposes typed accessors rather than its raw layout. Public application results must not expose `DDS` `FFI` structs or solver-specific raw result types.

Application commands and results derive `Debug`, `Clone`, and `PartialEq` where their fields support those traits so tests can compare typed values directly.

The application layer must not import `axum`, `HTTP` status codes, `Json`, or `serde_json::Value`. Every use case calls the internal `DdsSolver::init()` through its idempotent `Once` boundary so direct application callers cannot accidentally use an uninitialized solver. `initialize_solver()` exposes the same idempotent initialization through the application facade so `bridge-server` can initialize eagerly at startup.

All validation required by an application command is present when the use case is first introduced. In particular, `AnalyzePositionMatrix` rejects unequal hand counts before calling `DDS`, and `AnalyzeContinuation` accepts only a validated `SnapshotPosition`.

`Task 4` supplies validated play normalization, chronological events, state advancement, and the final `PlayPosition`. `Task 5` consumes that result to expose internal `final_position` and `final_continuation` results without parsing or advancing the cards again. The full `AnalyzePlay` use case returning `PlayAnalysis` (with historical `trace`) is completed in `Phase 2b`.

`analyze_final_play()` converts `NormalizedPlay::final_position()` to one `SnapshotPosition` and reuses the continuation use case. It must not replay `NormalizedPlay::played_cards()`. When all cards have been played and no legal continuation exists, `final_continuation` is `None`; the application must not call `SolveBoardPBN` on a terminal position. `Phase 2a` does not define or serialize a partial public play-response `DTO`.

### Task 6: Define Shared Input Normalization And Refactor `CLI`

Create a transport-neutral input layer before moving orchestration out of the `CLI`:

```text
src/input/
├── mod.rs
├── fields.rs        # source-neutral optional fields
├── merge.rs         # field-level precedence and conflicts
└── command.rs       # operation-specific application command builders
```

This layer contains no `axum`, `HTTP` status, or `serde_json::Value` dependency. It accepts already parsed source fields, applies field-level precedence, performs final cross-field validation, and builds the `Task 5` application commands. `Task 6` implements the `CLI` source policy `CLI arguments > PBN`; `Task 8` later supplies `URL query > JSON body > PBN` to the same merger.

Refactor `src/cli/main.rs` so every `CLI` operation uses the shared input layer and delegates solving to the application layer:

- Full-deal analysis calls `AnalyzeDeal`.
- Position matrix calls `AnalyzePositionMatrix`.
- Continuation calls `AnalyzeContinuation`.
- Play-trace import normalizes once and calls `AnalyzeFinalPlay`.
- Remove duplicated solver orchestration and play advancement from the `CLI`.
- Keep argument parsing, `stdin` reading, operation selection, and text rendering in the `CLI` adapter.
- Keep one `parse_record()` call and the existing operation precedence: `Position`, then `Play`, then full deal.

`CLI` compatibility is distinct from future `HTTP` endpoint applicability:

- Preserve all existing option names and non-contradictory behavior for `--trump`, `--first`, `--declarer`, `--matrix`, and `--format`.
- Existing matrix calls containing `[First]` or `--first` remain accepted. `AnalyzePositionMatrix` receives only `Hands`; the compatibility-only value is validated and then discarded. Omitting `First` for matrix analysis may become valid as an additive behavior.
- Do not make existing `CLI` calls fail merely because an otherwise harmless field is inapplicable to the corresponding future `HTTP` endpoint.
- Keep the correctness fixes from Tasks 1, 2, and 4, including `CurrentTrick` order validation and `declarer`/`opening_leader` conflict detection.

After this task, `src/cli/main.rs` must not call `DdsSolver` directly, construct application results, duplicate position normalization, or implement a second field-merging path.

### Task 7: Audit Domain, Application, And `DDS` Boundaries

Audit the `Task 5` boundaries and remove remaining ways to bypass them:

- `Hands` general invariants: at most `13` cards per hand and no cross-hand duplicates.
- Full-deal completeness: exactly `13` cards per hand and `52` total.
- Snapshot and matrix equal-hand-count requirements.
- `current_trick` length `0..3`, card ownership, play order, and follow-suit.
- `CurrentTrick` derives player sequence and `next_to_act` from `trick_leader`.
- `DdsSolver::solve_position_matrix()` accepts validated matrix `Hands`, not a semantically unrelated `SnapshotPosition` with a fabricated leader.
- Low-level `DdsSolver` methods and raw `DDS` result types become `pub(crate)`; the application layer is the public solving facade.
- Existing external solver integration tests migrate to application calls. Private conversion and `FFI` boundary tests remain next to the solver.
- Successful `DDS` output is range-checked before indexing arrays or converting signed counts and scores to unsigned values.
- `DDS_LOCK` poisoning and impossible `DDS` metadata return `Error` rather than panic.

Source precedence, `declarer`/`opening_leader` conflicts, and endpoint applicability do not belong in the solver. They remain in the shared input and transport layers. The `CLI`, future `HTTP` handlers, and direct application callers receive the same domain and solver safety because validation lives at domain construction and application-use-case boundaries.

### Task 8: Define Request DTOs, Public Errors, And Transport Merging

Create `src/transport/` without importing `axum`:

```text
src/transport/
├── mod.rs
├── error.rs         # stable transport-neutral error codes
└── request.rs       # endpoint-specific JSON body and query DTOs
```

Define independent body and query `DTO`s for each endpoint. Use `#[serde(deny_unknown_fields)]` and a shared deserializer that distinguishes an omitted field from an explicit `null`; omitted means absent, while `null` returns `invalid_request`.

The request conversion layer must:

- Parse and perform syntax and intrinsic field-value validation on every provided source before merge; invalid lower-priority input is not hidden by a valid override.
- Parse `PBN` only through `parse_record()`.
- Parse public `JSON` and query values using the canonical case-sensitive forms in this document.
- Provide a `pub(crate)` Deal-value parser for the standalone JSON `deal` field rather than synthesizing a `[Deal "..."]` record. `parse_record()` remains the only public record parser.
- Accept `JSON body` standalone fields and allowed `URL query` fields.
- Merge by field-level `query > body > PBN` priority.
- Reject supported but endpoint-inapplicable `PBN`, body, and query fields.
- Feed the merged fields into the shared `Task 6` command builders.
- Preserve a standard `Play` section's immutable `first_column` when a higher-priority source overrides `opening_leader`.
- Add and test `Direction::previous()` for `opening_leader -> declarer`; do not duplicate inverse-direction arithmetic in transport code.

`Task 8` also defines the stable transport-neutral error-code mapping from internal `Error` variants to the minimum public codes in this document. It does not assign `HTTP` status codes or implement `IntoResponse`; `Phase 2b Task 4` maps these errors to statuses and middleware responses.

Tests cover every endpoint independently, all source combinations, explicit `null`, unknown fields, applicability, parse-before-merge, valid conflict resolution by override, and rejection of contradictory final values.

### Task 9: Define Shared Response DTOs And Stable Output Order

Add strong response types to `src/transport/response.rs` for the three complete `Phase 2a` use cases: deal, position matrix, and continuation. Derive `Serialize`, `Deserialize`, `Debug`, and `PartialEq` so future `HTTP` and current `CLI` tests compare typed values rather than raw `serde_json::Value` trees.

Response conversion implements the final stable order before any new golden fixture is recorded:

- Cards in `hands` arrays: suits in `SHDC` order and descending rank within each suit.
- `suggested`: descending by `tricks_for_score_side`, then suit in `SHDC` order, then rank descending.
- Equivalent cards from the `equals` bitmask are expanded, deduplicated, and sorted by the same comparator.
- `par.contracts` remain in `DDS` order.

For these three use cases, `CLI --format json` serializes the exact response types later used by the `HTTP API`. The response types include the documented `matrix` and `continuation` outer fields; they do not use `serde_json::Value` internally.

- `CLI` text output remains independently formatted.
- Pre-refactoring `CLI` JSON fixtures are not required to stay byte-identical; they are updated to match the final `API` spec.
- Text fixtures stay byte-identical unless a correctness fix changes them.
- Full play-analysis JSON synchronization is completed in `Phase 2b` when the final `PlayAnalysis` result exists.
- `Phase 2a` does not expose a partial play response with an empty or fabricated `trace`.

Stable ordering is a response-conversion responsibility. Do not reorder low-level `DDS` results merely to format `JSON`; this keeps existing text output independent from the public array order.

### Task 10: Lock Response Contracts And Golden Fixtures

- Add typed serialization round-trip tests for every `Phase 2a` response `DTO`.
- Add structural golden fixtures for `CLI --format json`, including the required `matrix` and `continuation` wrappers.
- Add byte-identical text fixtures for existing full-deal, matrix, continuation, prefixed legacy play, and unprefixed legacy play output.
- Record any intentional JSON migration from the pre-`Phase 2a` shape. This task must not introduce a second sorting implementation or change the comparators defined in `Task 9`.

### Task 11: Update Project Documents

- `PLAN.md`: mark `Phase 1b` complete, record completed `Phase 2a` work, replace the obsolete `/api/solve` and `/api/analyze` entries with the four accepted endpoint contracts, and keep `AnalysePlayPBN` historical evaluation in `Phase 2b`.
- `INIT.md`: remove the outdated suggestion that `AnalysePlayPBN` belongs to `Phase 1b` and describe the application layer as the public solving facade.
- `README.md`: describe only currently available binaries and behavior; keep `bridge-server` and the self-contained web application identified as future `Phase 2b`/`Phase 3` work until implemented.
- Verify links to `phases/2a-task-4-pre-tasks.md` and `phases/2a-task-4-verification.md`.
- Do not repeat the `phases/1b-verification.md` case `10` update; it was completed by `Task 4`.

### Task 12: Phase 2a Verification

Automated:

- `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test -- --test-threads=1`.
- Conversion layer tests (`dealPBN.first = trick_leader`, current-trick card removal, score semantics).
- Integration tests with exact mid-trick scores (not just suit checks).
- `Hands` / `CurrentTrick` / `SnapshotPosition` / `PlayPosition` invariant tests.
- Unified `PBN` parser tests for all supported tags, standard and legacy play forms, fixed player columns, legal incomplete-final-trick placeholders, supported section data, and error paths.
- Application use-case tests (3 complete use cases + play advancement/final-state tests).
- Empty-play and complete-`52`-card terminal-play tests; terminal play returns no continuation and does not call `SolveBoardPBN`.
- Shared `CLI` input tests for argument priority, legacy option compatibility, and operation precedence.
- Transport request `DTO`, explicit-`null`, unknown-field, applicability, input-merging, override, and conflict tests.
- Parse-before-merge tests proving invalid syntax or intrinsically invalid lower-priority `PBN`/body fields are rejected even when a higher-priority source provides a valid override.
- Post-merge semantic tests proving a higher-priority override may resolve a lower-priority cross-field conflict, while contradictory final values are rejected.
- Four-direction `declarer -> opening_leader` and `opening_leader -> declarer` derivation tests, including same-source and cross-source conflict cases.
- Solver-boundary tests for impossible `DDS` counts, indices, scores, and poisoned-lock handling without panics.
- Typed response `DTO` round trips and equality tests.
- `CLI` golden fixtures: text byte-identical except approved correctness fixes; JSON uses the final API response `DTO`s.
- Stable output-order and equivalent-card expansion/deduplication tests independent of raw `DDS` order.
- Architectural checks: one public `PBN` record parser, no direct `DdsSolver` calls from the `CLI`, no `axum` imports in `application`, `input`, or transport `DTO` code, and no `serde_json::Value` in application or response types.

Manual:

- Run full-deal, residual, mid-trick, and play-trace `CLI` examples.
- Verify corrected mid-trick scores.
- Verify existing matrix calls containing `[First]` or `--first` remain accepted.
- Verify prefixed, unprefixed, empty, and standard-section play inputs.
- Verify `CLI --format json` matches documented `API` response shapes.

`Phase 2b` may begin only after the full verification passes, project documents are current, the working tree is clean, and the developer explicitly confirms the `Phase 2a` result.

---

## Phase 2b — HTTP Service

### Task 1: Add AnalysePlayPBN

- `FFI` declarations for `AnalysePlayPBN`, `playTracePBN`, `solvedPlay`.
- Safe wrapper returning double-dummy result at each position.
- `PlayEvaluation` with correct `is_mistake` logic:
  - Declarer-side: `delta_for_declarer < 0` → mistake.
  - Defender-side: `delta_for_declarer > 0` → mistake.
  - No change → not a mistake.
- `declarer = opening_leader.previous()` (not `.next()`).

### Task 2: Add Server Binary And Router

- `[[bin]] name = "bridge-server"` in `Cargo.toml`.
- `src/server/main.rs` with `axum` `Router`, four `endpoint`s.
- Call application-level `initialize_solver()` once at startup.

### Task 3: Add Bounded-Queue Worker

- `async` worker holds `mpsc::Receiver<SolveJob>`.
- On receiving a job, checks `oneshot` receiver is still open, then calls `spawn_blocking()` for the synchronous solver call.
- Queue full → `503`. Timeout → `504` (drops receiver; in-flight call not cancelled).
- Defaults: `queue_capacity = 16`, `request_timeout = 10s`, `body_limit = 1MB`. Configurable.

### Task 4: Add Unified HTTP Errors And Middleware

- Map domain, transport, extractor, and runtime errors to the unified `{ error: { code, message } }` format.
- Map body-limit rejection to `413` with code `body_too_large`.
- Body size limit enforcement and request tracing.
- Restricted `CORS` for the configured `Vite` dev origin, default `http://localhost:5173`; allow only required methods and headers, with configuration override for tests.

### Task 5: Complete Play Analysis Endpoint

`POST /api/v1/analyze/play` returns `trace` (from `AnalysePlayPBN`), `final_position`, and `final_continuation`.

### Task 6: Phase 2b Verification

Automated:

- `Router::oneshot()` tests for all four `endpoint`s (success and error).
- Input combination, override, and conflict tests via `HTTP`.
- `AnalysePlayPBN` historical evaluation and mistake-detection tests.
- Complete play analysis: `trace` + `final_position` + `final_continuation`.
- For the same normalized input, `CLI JSON` and the corresponding `HTTP` response body deserialize to equal response values produced from the shared result type.
- Concurrency, queue-overload (`503`), timeout (`504`), body-too-large (`413`), restricted-`CORS`, and error-format tests.
- Random-port smoke test.

Manual:

- `curl` all four `endpoint`s.
- Verify override and conflict behavior.
- Verify play analysis identifies mistakes.
- Verify `Vite` dev server `CORS` access.
