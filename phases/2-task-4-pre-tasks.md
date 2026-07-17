# `Phase 2a Task 4` Pre-Tasks

## Goal

Close the remaining `Phase 2a Task 1` and `Task 2` gaps, finalize backward-compatible legacy `Play` behavior, and define the parser/normalization boundary required to begin `Task 4` without temporary duplicate parsing or state-advancement paths.

`Task 4` will be renamed to `Unify PBN Parser And Play Normalization`. It will own parsing, context-dependent play normalization, legality validation, state advancement, and migration of existing `CLI` parsing paths. It will not implement application use cases, call `DDS`, merge `HTTP` input sources, or map errors to `HTTP` responses.

Work on `Task 4` begins only after the changes in this document are implemented, verified, reviewed, and confirmed.

## Confirmed Decisions

### Complete `Task 1` And `Task 2` First

The domain-boundary and regression-test gaps from `Task 1` and `Task 2` must be closed before the unified parser is implemented. The parser must construct validated domain values rather than depend on the current public raw fields.

### Preserve Legacy `Play` Compatibility

Existing documented and tested `CLI` inputs remain supported:

- A legacy inline value with a direction prefix, such as `[Play "E:S3=S5"]`.
- A legacy inline value without a direction prefix, such as `[Play "S3=S5"]`, with the opening leader derived from `--declarer` or `Declarer`.
- An empty legacy value, `[Play ""]`, with the opening leader supplied separately.
- Existing chronological sequences containing more than four cards joined by `=` without a required whitespace trick boundary.
- Existing `--trump`, `--first`, `--declarer`, and `--matrix` argument names and meanings.

For legacy input, whitespace and `=` delimit chronological cards. Trick boundaries are derived by state advancement, not by separator grouping.

Backward compatibility applies to existing valid, non-contradictory inputs. Validation hardening may reject malformed or contradictory inputs that were previously ignored or accepted accidentally, including:

- A final `declarer` that contradicts the final `opening_leader`.
- Duplicate cards, incorrect ownership, or a failure to follow suit.
- Empty card tokens, invalid card tokens, and malformed separators.

When a legacy `Play` prefix and a separately supplied `declarer` are both present, normalization requires `opening_leader == declarer.next()`. Consistent existing calls preserve their behavior; contradictory calls return `conflicting_input`.

### Distinguish Standard And Legacy `Play` Deterministically

After the `Play` tag value is extracted and validated as a quoted string, its value alone determines the representation:

- A value that is exactly one bare `Direction` (`W`, `N`, `E`, or `S`) is a standard `Play` header and introduces a standard section.
- Every other value is legacy inline `Play`, including an empty value, a direction followed by `:`, and an unprefixed card sequence.

The parser does not use the presence of following section data to choose the representation. Non-empty section data following a legacy inline `Play` value returns `invalid_pbn`. A bare direction remains standard `Play` even when its section is empty.

### Move Play Normalization Into `Task 4`

`Task 4` will parse both standard and legacy `Play`, normalize them into one validated chronological representation, and advance a `PlayPosition` to the final state.

The normalization function receives already resolved context. It does not implement source priority or transport merging. The current `CLI` adapter resolves its command-line arguments and parsed `PBN` fields before calling normalization. The later transport layer resolves `URL query > JSON body > PBN` before calling the same normalization function.

`Task 4` normalization may use domain operations such as `PlayPosition::play_card()`. It must not call `DDS` or perform continuation analysis.

### Preserve Standard `Play` Column Identity

A standard `Play` representation retains both:

- `first_column`: the immutable player represented by the first column of the parsed section.
- `opening_leader`: the final resolved player who plays the first chronological card.

A higher-priority source may override `opening_leader`, but it does not change the fixed column ownership established by the original `Play` tag. Normalization uses the current trick leader to read each row in chronological order while using `first_column` to map every token to its fixed player column.

### Retain Fine-Grained Internal Errors Through `Task 4`

`Task 4` does not replace the internal `Error` enum with the public `HTTP` error protocol. It preserves useful fine-grained variants and adds only variants required by the new parser and normalization behavior. `Task 8` remains responsible for the final transport mapping.

The intended future mappings are:

- `PbnParse`, `DuplicatePbnTag`, `InvalidPbnTag`, and `UnsupportedPbnFeature` map to `invalid_pbn`.
- `InvalidDeal` maps to `invalid_deal`.
- `InvalidPosition` maps to `invalid_position`.
- `InvalidPlayTrace` maps to `invalid_play_trace`.
- A new `ConflictingInput(String)` variant maps to `conflicting_input`.
- Missing final required fields map to `missing_field` at the application/transport boundary.

The unified `parse_record()` accepts partial records and therefore does not emit `MissingPbnTag`. The existing `CLI` adapter may continue to use `MissingPbnTag` temporarily for operation-specific required fields until `Task 5` and `Task 8` move those checks to their final boundaries.

## Pre-Task Work

### 1. Close `Task 1` Domain Boundaries

Update `src/core/deal.rs` so `Deal` contains validated `Hands` and cannot be constructed with incomplete or duplicate raw hands:

```rust
pub struct Deal {
    first: Direction,
    hands: Hands,
}

impl Deal {
    pub fn try_new(first: Direction, hands: Hands) -> Result<Self, Error>;
    pub fn first(&self) -> Direction;
    pub fn hands(&self) -> &Hands;
}

impl Hands {
    pub fn iter(&self) -> impl Iterator<Item = (Direction, &Hand)>;
}
```

`Deal::try_new()` validates exactly `13` cards in each hand and `52` cards total. Cross-hand uniqueness and the per-hand limit remain guaranteed by `Hands`.

Update all direct field access in `src/core/pbn.rs`, `src/dds/solver.rs`, `src/cli/main.rs`, and tests to use the constructor and accessors. No parser or solver path may construct `Deal` through public raw fields.

Residual position parsing must construct `Hands` rather than return a raw `[Hand; 4]`. The old residual type is temporary and will be removed by `Task 4`.

### 2. Close `Task 1` And `Task 2` Verification Gaps

Add unit tests around `to_dds_deal()` in `src/dds/solver.rs` that assert:

- `dealPBN.first` equals `trick_leader` for all four leaders.
- `dealPBN.remainCards` always uses the fixed `N` serialization prefix.
- Every incomplete-current-trick card is absent from `remainCards`.
- `currentTrickSuit` and `currentTrickRank` preserve play order for lengths `0..=3`.

Add domain tests that cover:

- `CurrentTrick` lengths `0..=3` for all four leaders.
- Derived players and `next_to_act` for every covered case.
- `SnapshotPosition -> PlayPosition -> SnapshotPosition` round trips.
- Rejection of double removal and double add-back.
- `Hands::try_new()` rejection of more than `13` cards in one hand.
- `Hands::iter()` direction order and ownership.

Add a `DDS` integration fixture where the incomplete current trick changes the exact score or optimal card. Assert exact `tricks_for_score_side`, exact optimal cards, and the relevant non-optimal alternative rather than checking only the returned suit.

Replace or correct `test_mid_trick_score_side_is_next_to_act`. The regression must use a position where `trick_leader` and `next_to_act` are on opposite sides, such as a one-card or three-card current trick, and must assert that the exposed `score_side` is the side containing `next_to_act`.

Add a `CLI` regression that confirms the corrected mid-trick text/`JSON` output. Output remains unchanged except for previously confirmed correctness fixes.

### 3. Amend The `PBN` Contract

Update `phases/pbn-input-contract.md` to define the complete legacy inline grammar:

```text
[Play ""]
[Play "S3=S5"]
[Play "E:"]
[Play "E:S3=S5=S2=SQ=H3=H5"]
```

The optional direction prefix contributes `opening_leader` at the `PBN` source priority. A missing prefix leaves `opening_leader` absent for later derivation from `declarer` or another higher-priority source.

Legacy cards form one chronological sequence. Whitespace and `=` are delimiters; neither delimiter defines a validated trick boundary. Placeholders are not accepted in legacy input.

Add the explicit standard/legacy discrimination rule:

- An exact bare `Direction` value is standard `Play`.
- Every other value is legacy inline `Play`.
- Section data after a legacy inline value is rejected as `invalid_pbn`.

Clarify the error boundary for standard `Play` placeholders:

- Invalid tokens, an invalid row width, an incomplete non-final row, a row containing four `-` tokens, or placeholder syntax outside the supported final row returns `invalid_pbn`.
- A context-dependent chronological gap, a real card in the wrong fixed player column, impossible leader progression, ownership failure, duplicate card, or follow-suit failure returns `invalid_play_trace`.

Document that `Task 4` retains the fine-grained internal `Error` variants and that the stable public error-code mapping remains a `Task 8` responsibility. Add `ConflictingInput(String)` when final `CLI` play fields contradict each other; do not introduce an `HTTP`-specific error type in the parser or normalizer.

Update `phases/2-api-service.md` to rename and expand `Task 4`, add the concrete types and signatures below, and remove duplicate play-normalization/state-advancement work from `Task 5`.

### 4. Confirm The Revised Plan

Review the updated `phases/2-api-service.md`, `phases/pbn-input-contract.md`, and this document together. Confirm the revised plan before implementation of the unified parser begins.

## `Task 4` Implementation Contract

### Files And Responsibilities

- `src/core/pbn.rs`: shared record parser, supported tag parsing, section state machine, parser-side structured types, duplicate detection, and parser-stage errors.
- `src/core/play.rs`: standard/legacy play normalization into a validated chronological play and final `PlayPosition`.
- `src/core/deal.rs`: validated `Deal` and `Hands` boundaries consumed by the parser and normalizer.
- `src/core/position.rs`: domain play advancement through `PlayPosition::play_card()` and shared validated `PlayedCard` output where required.
- `src/cli/main.rs`: parse once, resolve existing `CLI` arguments against parsed fields, select the current operation from `ParsedRecord`, and call shared normalization without scanning raw tag strings.

The implementation may split `src/core/pbn.rs` into a `src/core/pbn/` module if that materially improves clarity. Any such split must keep one public parser entry point and must not expose multiple parsing paths.

### Parser Types

The exact ownership details may be refined during implementation, but the public responsibilities must match these types:

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

pub struct DirectedCard {
    pub player: Direction,
    pub card: Card,
}

pub struct ParsedCurrentTrick {
    pub cards: Vec<DirectedCard>,
}

pub struct ParsedContract {
    pub level: u8,
    pub strain: Strain,
    pub doubling: Doubling,
}

pub enum Doubling {
    Undoubled,
    Doubled,
    Redoubled,
}

pub struct ParsedAuction {
    pub first: Direction,
    pub calls: Vec<AuctionCall>,
}

pub enum AuctionCall {
    Bid { level: u8, strain: Strain },
    Pass,
    Double,
    Redouble,
}

pub enum ParsedPlay {
    Standard {
        first_column: Direction,
        rows: Vec<PlayRow>,
    },
    Legacy {
        opening_leader: Option<Direction>,
        cards: Vec<Card>,
    },
}

pub struct PlayRow {
    pub cards: [Option<Card>; 4],
}

pub fn parse_record(input: &str) -> Result<ParsedRecord, Error>;
```

`ParsedCurrentTrick` is parser-side data. It preserves the player/card pairs supplied by the tag and does not become a position model.

`PlayedCard` in normalized output represents a validated chronological event. If the existing type is retained, move it to the appropriate shared domain or application boundary and expose read-only accessors. Do not reuse an unvalidated parser-side type as a normalized play event.

```rust
pub struct PlayedCard {
    player: Direction,
    card: Card,
}

impl PlayedCard {
    pub fn player(&self) -> Direction;
    pub fn card(&self) -> Card;
}
```

### Normalization Types

```rust
pub struct NormalizedPlay {
    opening_leader: Direction,
    played_cards: Vec<PlayedCard>,
    final_position: PlayPosition,
}

impl NormalizedPlay {
    pub fn opening_leader(&self) -> Direction;
    pub fn played_cards(&self) -> &[PlayedCard];
    pub fn final_position(&self) -> &PlayPosition;
}

pub fn normalize_play(
    play: &ParsedPlay,
    deal: &Deal,
    trump: Strain,
    opening_leader: Direction,
) -> Result<NormalizedPlay, Error>;
```

`normalize_play()` performs no source merging. Its caller supplies final resolved `deal`, `trump`, and `opening_leader` values.

For legacy input, normalization advances cards in their parsed chronological order.

For standard input, normalization:

1. Maps each row column to an absolute player using the immutable `first_column`.
2. Reads cards in chronological order using the current `PlayPosition` leader.
3. Calls `PlayPosition::play_card()` for every real card.
4. Uses completed-trick winners to determine the next row's chronological leader.
5. Validates that the final incomplete row contains one continuous chronological prefix.
6. Returns the validated chronological events and the single final `PlayPosition` produced by the same advancement pass.

No caller may replay the normalized sequence merely to reconstruct the final position. `Task 5` consumes the returned `final_position` and must not introduce a second advancement implementation.

### `CLI` Migration

`Task 4` replaces:

- `parse_record()` as the full-deal-only parser.
- `parse_residual_record()`.
- `parse_play_tag()` as a separate legacy parser.
- `extract_tag_value()`.
- Raw checks such as `input.contains("[Position ")` and `input.contains("[Play ")`.

The `CLI` calls the unified `parse_record()` exactly once. It chooses the existing operation from the presence of structured fields in `ParsedRecord` and retains the existing command-line flags.

During `Task 4`, operation selection preserves the current precedence:

1. A present `Position` selects the residual-position path.
2. Otherwise, a present `Play` selects the play-trace path.
3. Otherwise, the input selects the full-deal path.

Final rejection of records containing fields from multiple incompatible operations belongs to the later shared application/transport validation. Parser migration must not silently select a different operation from the current `CLI` for the same input.

During `Task 4`, the `CLI` adapter preserves its existing operation behavior and does not implement the final `HTTP` endpoint-applicability rules. In particular, parser migration must not introduce unrelated matrix/`First` behavior changes. Final shared application routing and applicability checks remain in `Task 6` and `Task 8`.

For play input:

- `--trump` remains supported and has priority over a parsed `Contract` strain for the `CLI`.
- A parsed `Contract` strain may be used as an additive fallback when `--trump` is absent.
- `--declarer` remains supported and has priority over a parsed `Declarer` value.
- A legacy `Play` prefix supplies a `PBN`-priority `opening_leader`.
- A missing legacy prefix derives `opening_leader` from the final `declarer`.
- Final `declarer` and `opening_leader` values are cross-validated before normalization.

`Task 4` also updates `phases/1b-verification.md` case `10`. Its expected error changes from the current downstream `SnapshotPosition` ownership failure to a parser-stage `CurrentTrick` player-order rejection. The guide must show the actual stable message produced by the implementation and identify that `S` is expected as the second player after leader `E`, but `N` was supplied.

## `Task 5` Boundary After Revision

`Task 5` creates the application use cases and consumes parser/normalizer results. It does not parse tag strings, flatten standard `Play` rows, validate legacy separators, or advance a second play state.

The play portion of `Task 5`:

- Accepts final normalized play input.
- Uses `NormalizedPlay::final_position()` for final-position and continuation work.
- Uses `NormalizedPlay::played_cards()` for later historical evaluation integration.
- Does not call `PlayPosition::play_card()` over the same input again.

Full historical evaluation and `AnalysePlayPBN` remain in `Phase 2b`.

## Verification

### Automated Checks Before `Task 4`

- `cargo fmt --check`.
- `cargo clippy --all-targets --all-features -- -D warnings`.
- `cargo test -- --test-threads=1`.
- New `Deal`, `Hands`, position-conversion, `to_dds_deal()`, exact `mid-trick`, and `score_side` regressions pass.

### Automated Checks For `Task 4`

- Every supported tag parses independently into `ParsedRecord`.
- Duplicate supported tags and sections are rejected.
- Unknown tags follow the documented ignore policy.
- Malformed tag and section syntax is rejected.
- `CurrentTrick` validates entry count, clockwise player order, and duplicate cards intrinsically.
- The invalid-order regression used by `phases/1b-verification.md` case `10` fails in `parse_record()` before `SnapshotPosition` construction.
- Standard `Play` preserves fixed rows and columns at the parser boundary.
- Standard `Play` normalization handles changing trick leaders.
- Valid incomplete-final-row placeholders work for every possible current leader and length `1..=3`.
- Invalid placeholder shapes and chronological gaps return the documented error category.
- Legacy `Play` tests cover prefixed, unprefixed, empty, more-than-four-card `=` sequences, whitespace-separated sequences, and malformed tokens.
- Normalization rejects ownership, duplicate-card, and follow-suit violations.
- `NormalizedPlay::played_cards()` and `final_position()` come from one state-advancement pass.
- The existing documented `CLI` play invocations continue to work.
- The `CLI` contains no raw `PBN` tag scan or separate tag parser.
- The old `parse_residual_record()`, `parse_play_tag()`, and `extract_tag_value()` paths are removed.

### Manual Checks

- Run the existing full-deal, position matrix, continuation, prefixed legacy play, and unprefixed legacy play examples from `phases/1b-verification.md`.
- Run a standard `Play` section whose second trick has a different leader from the opening leader.
- Run standard incomplete-final-row examples for different fixed-column rotations.
- Confirm text and `JSON` output remain unchanged except for previously approved correctness fixes and the explicitly approved parser-stage error change in `phases/1b-verification.md` case `10`.

## Start Gate

`Task 4` is ready to begin only when all of the following are true:

- The `Task 1` domain boundaries are complete.
- The `Task 1` and `Task 2` correctness regressions are present and passing.
- Legacy `Play` compatibility is documented as defined here.
- Placeholder/error classification is consistent across the phase plan and `PBN` contract.
- `Task 4` owns play normalization and `Task 5` no longer duplicates it.
- The revised plan has been reviewed and explicitly confirmed.
- The working tree is clean and the full baseline verification passes.

## Reference

- `phases/2-api-service.md`.
- `phases/pbn-input-contract.md`.
- `phases/1b-verification.md`.
- `phases/2-review-4.md`.
- `engine/dds/doc/dll-description.md`.
- `engine/dds/include/dll.h`.
- `PBN 2.1`: <https://www.tistis.nl/pbn/pbn_v21.txt>.
