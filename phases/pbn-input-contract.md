# `PBN` Input Contract

## Purpose

Defines the `PBN` subset accepted by `bridge-dds` across the `CLI`, `REST API`, tests, and front-end workflows. The contract covers tag-only partial records and records with supported section data.

## Scope

One board is accepted per request. Multi-board files are reserved for future batch analysis. The shared `parser` implements only the subset documented here. Unsupported syntax must return `invalid_pbn` rather than be ignored or silently misinterpreted.

All input paths use the same `parser` and produce a `ParsedRecord` whose fields are optional. Parsing a record does not require the fields needed by any specific `endpoint`. `Endpoint` applicability, source merging, required-field checks, and cross-field semantic validation happen after parsing.

## Validation Stages

Validation is divided into three stages:

1. The shared `parser` validates record syntax, duplicate supported tags and sections, and the intrinsic value syntax of every supplied supported field.
2. The `transport` or `CLI` adapter rejects supported fields that are not applicable to the selected operation, then merges sources by `URL query > JSON body > PBN`.
3. The `normalization` and `domain` layers validate required final fields and relationships between fields, including card ownership, follow-suit, `First` consistency, and `Declarer`/`Play` consistency.

A higher-priority source may resolve a lower-priority cross-field conflict. It may not hide malformed syntax or an intrinsically invalid field value in the lower-priority source.

## `Parser` Output Boundary

The shared `parser` returns one `ParsedRecord` with optional structured fields. It does not select an `endpoint`, apply source priority, require `endpoint`-specific fields, or construct final `domain` commands.

The `parser` preserves representations that need merged context:

- `CurrentTrick` remains an ordered list of `Direction`/`Card` pairs.
- `Auction` remains an opening direction plus tokenized calls.
- Standard `Play` remains its parsed first-column direction plus fixed-column rows.
- Legacy inline `Play` remains an opening leader plus a chronological card sequence.

The `parser` must not flatten standard `Play` rows, derive completed-trick winners, infer missing fields, or cross-validate separate tags. Those operations belong to post-merge `normalization`.

## `PBN` Source Field Mapping

Before merging with other sources, the parsed `PBN` representation contributes fields independently:

- `Deal` contributes `deal`.
- `Dealer` contributes `dealer`.
- `Vulnerable` contributes `vulnerable`.
- `Position` contributes `hands`.
- `First` contributes `next_to_act`.
- `Trump` contributes `trump`.
- A non-empty `CurrentTrick` contributes `trick_leader` from its first pair and contributes card-only `current_trick`; an empty `CurrentTrick` contributes only an empty `current_trick`.
- `Contract` contributes `trump` from its strain.
- `Declarer` contributes `declarer`.
- `Play` contributes `opening_leader` from its prefix and contributes its standard or legacy play representation. Standard `Play` also retains the prefix as its immutable first-column direction.

These fields participate independently in field-level source priority. After merging, the final `opening_leader` controls the chronological first player for both play representations. A higher-priority `opening_leader` does not change the standard representation's fixed player columns. The final `trick_leader`, `current_trick`, and `next_to_act` values control continuation `normalization`. Any contradictory final relationship returns `conflicting_input`.

## Record And Line Format

Accepted line endings are `LF` and `CRLF`. Leading and trailing whitespace around a line is ignored. A tag line has this exact structure after trimming:

```pbn
[Tag "Value"]
```

Tag names are case-sensitive. Supported tags must be written exactly as listed below. Tag values must be quoted. Escaped quotes, tag value inheritance, comments, and multiple boards are unsupported.

Lines beginning with `;` or `%` after trimming and brace-delimited comment text are unsupported comments and return `invalid_pbn`. A non-empty non-tag line is valid only as data belonging to an immediately preceding supported `Auction` or standard `Play` section. Otherwise it is malformed section data and returns `invalid_pbn`.

Each supported tag or section may appear at most once. Repeating a supported tag, repeating a supported section, or providing both standard and legacy definitions of `Play` returns `invalid_pbn`. Syntactically valid unknown tags are ignored only when they contain no globally unsupported feature. Unknown tags do not introduce a section; non-tag data following an unknown tag therefore returns `invalid_pbn`.

## Common Value Syntax

`Direction` is one of `N`, `E`, `S`, or `W`.

`Card` is exactly two uppercase characters: a suit from `S`, `H`, `D`, or `C`, followed by a rank from `A`, `K`, `Q`, `J`, `T`, `9`, `8`, `7`, `6`, `5`, `4`, `3`, or `2`. Forms such as `S10`, lowercase cards, unknown cards, and card annotations are unsupported.

A hand uses four dot-separated suit fields in `S.H.D.C` order. A void suit is an empty field. Rank order on input is unrestricted. A hand may not use `-`.

## Supported Tags

### `Deal`

`Deal` is a complete four-hand deal:

```text
<first>:<hand1> <hand2> <hand3> <hand4>
```

`<first>` is a `Direction`. The four hands are listed clockwise from `<first>`. The value must contain four hands, exactly `13` cards per hand, and `52` unique cards total. The `Deal` prefix controls only hand serialization order and must not be interpreted as `Dealer`.

### `Dealer`

`Dealer` is a `Direction`.

### `Vulnerable`

Accepted values are `None`, `Love`, `-`, `NS`, `EW`, `All`, and `Both`. `None`, `Love`, and `-` represent no vulnerability. `All` and `Both` represent both sides vulnerable.

### `Position`

`Position` uses the same four-hand clockwise format as `Deal`, but each hand may contain fewer than `13` cards. It must contain four hands with equal card counts and no duplicate card across hands. Cards already played to an incomplete `CurrentTrick` remain in their owners' hands.

Equal counts and cross-hand uniqueness are intrinsic `Position` value checks and are performed before source merging. Relationships between `Position` and `CurrentTrick` are checked after source merging.

### `First`

`First` is a `Direction` and represents `next_to_act` for continuation input.

The `parser` validates only that its value is a `Direction`. After source merging, continuation `normalization` requires final `next_to_act` and validates it against final `trick_leader` and `current_trick`. `First` is not required or applicable to the position-matrix operation.

### `Trump`

`Trump` is one of `S`, `H`, `D`, `C`, or `NT`. Values are case-sensitive.

### `CurrentTrick`

`CurrentTrick` is a whitespace-separated sequence of ordered `Direction:Card` pairs:

```pbn
[CurrentTrick "N:SA E:HA"]
```

An empty value is allowed. A non-empty value contains `1..=3` entries. The first entry identifies `trick_leader`; each later entry's direction must be the next clockwise player. Player order, entry count, and duplicate cards within the value are intrinsic checks performed by the `parser`.

After source merging, continuation `normalization` validates final `trick_leader`, card-only `current_trick`, and `next_to_act`:

- Final `next_to_act` matches the player derived from `trick_leader` and `current_trick`.
- Every card is held by its derived owner in the final `Position`.
- No current-trick card is assigned to another player.
- Every player follows suit when able.

### `Contract`

`Contract` accepts levels `1` through `7`, strains `S`, `H`, `D`, `C`, and `NT`, and optional doubling suffixes `X` or `XX`:

```pbn
[Contract "4S"]
[Contract "3NT"]
[Contract "1HX"]
[Contract "7CXX"]
```

Values are case-sensitive. Lowercase doubling suffixes, passed-out values, and all other contract forms are unsupported and return `invalid_pbn`. The `parser` preserves the level, strain, and doubling. For play analysis, the final merged `Contract` strain supplies `trump`; level and doubling do not affect the `solver`.

### `Declarer`

`Declarer` is a `Direction`. For play analysis, the final merged value derives `opening_leader = declarer.next()`.

The `parser` validates only the direction. If final merged input contains both `declarer` and `opening_leader`, `normalization` requires them to agree.

### `Auction`

`Auction` is a supported section header whose value is a `Direction`:

```pbn
[Auction "N"]
1C Pass 1H Pass
```

The `parser` preserves the opening direction and tokenized calls for future use. An empty `Auction` section is allowed. Accepted call tokens are bids from `1C` through `7NT`, `Pass`, `X`, and `XX`. Values are case-sensitive. The `parser` does not validate bidding order, sufficient bids, doubles, declarer, or any other auction semantics in `Phase 2`.

Every non-empty `Auction` section line is split on whitespace. Any token outside the accepted call set, including comments, annotations, note references, and tag-value inheritance, returns `invalid_pbn`.

### `Play`

`Play` supports a standard section form and a backward-compatible legacy inline form. The `parser` preserves their distinct structures. The play `normalization` layer converts either representation into the same chronological card sequence after source merging supplies the final `deal`, `trump`, and opening-leader context.

#### Standard `Play`

A standard `Play` tag contains only the opening-leader `Direction` and introduces a section:

```pbn
[Play "W"]
S6 S4 SJ SQ
S3 S7 S9 SK
```

Each non-empty section line is one trick row containing exactly four whitespace-separated tokens. Columns are fixed for the entire section: the first column belongs to the opening leader named by the `Play` tag, and the remaining columns proceed clockwise. The `parser` retains these four fixed columns and must not flatten rows into chronological order.

Each token is either a `Card` or `-`. A completed row contains four cards. A row containing `-` must be the final row and must contain `1..=3` cards. No row may follow it. An empty standard `Play` section represents an empty play sequence.

Because columns are fixed while the leader may change between tricks, valid placeholders are not necessarily trailing columns. After determining each completed trick's winner, `normalization` reads the final incomplete row in chronological order from its actual leader. Real cards must form one continuous chronological prefix; every seat that has not yet played must contain `-`.

For example, with fixed columns `W N E S`, if `N` leads the incomplete final trick and only `N` and `E` have played, the valid row shape is:

```pbn
- H2 H3 -
```

`Normalization` rejects a real card after a missing chronological turn, a card in the wrong player's column, card ownership violations, duplicate played cards, follow-suit violations, and any impossible winner or leader transition.

#### Legacy Inline `Play`

Legacy inline `Play` contains an opening-leader `Direction`, a colon, and chronological cards:

```pbn
[Play "E:S3=S5=S2=SQ H3=H5"]
```

Cards within a trick are separated by `=` and tricks are separated by whitespace. Every group except the final group must contain exactly four cards. The final group may contain `1..=4` cards. Placeholders are not accepted. After validating group syntax, the `parser` retains the opening leader and one chronological card sequence; semantic `normalization` occurs after source merging.

An empty play sequence must use an empty standard `Play` section. An inline value containing both inline cards and following section data returns `invalid_pbn`.

## Section Boundaries And Unsupported Features

`Auction` and standard `Play` introduce section data. Their section continues until the next tag line or end of record. Blank lines within a section are ignored. A line that begins with `[` after trimming is treated as a tag line and must satisfy tag syntax.

The following features are outside the supported subset and return `invalid_pbn` wherever they occur:

- Comments beginning with `;` or `%` and brace-delimited comments.
- Claims and claim markers.
- Annotations, glyphs, note references, and embedded commentary.
- Escaped tag values and tag-value inheritance.
- Unknown-card placeholders and any placeholder other than the supported standard-`Play` use of `-`.
- A second board in the same request. Blank lines do not start a new record; repeated supported tags remain duplicate errors.
- Section data outside `Auction` or standard `Play`.

## `Endpoint` Applicability

The `parser` accepts complete or partial records without selecting an `endpoint`. After parsing, the selected operation rejects any supported tag or section that is not applicable.

| Tag or section | `Deal` analysis | `Position` matrix | Continuation | `Play` analysis |
|---|---|---|---|---|
| `Deal` | Allowed | Not allowed | Not allowed | Allowed |
| `Dealer` | Allowed | Not allowed | Not allowed | Allowed |
| `Vulnerable` | Allowed | Not allowed | Not allowed | Allowed |
| `Position` | Not allowed | Allowed | Allowed | Not allowed |
| `First` | Not allowed | Not allowed | Allowed | Not allowed |
| `Trump` | Not allowed | Not allowed | Allowed | Not allowed |
| `CurrentTrick` | Not allowed | Not allowed | Allowed | Not allowed |
| `Contract` | Not allowed | Not allowed | Not allowed | Allowed |
| `Declarer` | Not allowed | Not allowed | Not allowed | Allowed |
| `Auction` | Not allowed | Not allowed | Not allowed | Allowed |
| `Play` | Not allowed | Not allowed | Not allowed | Allowed |

Unknown tags are ignored. A supported but inapplicable tag or section returns `invalid_pbn`.

## Post-Merge Requirements

Required fields are checked only after all allowed sources are merged:

- Deal analysis requires `deal`, `dealer`, and `vulnerable`.
- Position-matrix analysis requires `hands`.
- Continuation analysis requires `hands`, `trump`, and `next_to_act`. `current_trick` defaults to empty; when empty, `trick_leader` defaults to `next_to_act`.
- Play analysis requires `deal`, `trump`, `opening_leader`, and a play sequence. `opening_leader` may be derived from `declarer`, and an empty play sequence is valid.

Missing final fields return `missing_field`. Contradictory final cross-field values return `conflicting_input`.

## Error Policy

- Malformed tag lines, malformed sections, duplicate supported tags or sections, invalid supported tag values, and unsupported `PBN` features return `invalid_pbn`.
- Complete-deal count, completeness, and uniqueness violations return `invalid_deal`.
- Intrinsic `Position` and `CurrentTrick` invariant violations, and final position invariant violations, return `invalid_position`.
- Final play-sequence ownership, duplicate-card, order, and follow-suit violations return `invalid_play_trace`.
- Final cross-field contradictions after source merging return `conflicting_input`.
- Missing required fields after source merging return `missing_field`.
- Unknown tags are ignored.
- Supported but `endpoint`-inapplicable tags and sections return `invalid_pbn`.

All final normalized input must be validated before reaching `DDS`.
