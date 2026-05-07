# `PBN` Input Contract

## Purpose

This document defines the `PBN` input subset accepted by `bridge-dds`. The contract applies across the `CLI`, `REST` API, tests, and future frontend workflows.

## Scope

`bridge-dds` treats `PBN` as the canonical board input format. The implementation intentionally supports a small, explicit subset first, then expands only when a feature needs it.

The initial contract supports one complete board per request. Multi-board files are reserved for batch analysis.

## Required Tags

Each board must include:

- `Deal`: complete four-hand deal data.
- `Dealer`: the game's dealer.
- `Vulnerable`: the board vulnerability.

Unknown tags are ignored in the initial implementation.

Each required tag may appear at most once. Duplicate required tags are errors.

## Line Format

Accepted line endings:

- `LF`
- `CRLF`

Accepted tag format:

```pbn
[Tag "Value"]
```

Tag names are case-sensitive in the initial implementation. Required tags must be written exactly as `Deal`, `Dealer`, and `Vulnerable`.

The initial implementation does not support:

- Escaped quotes inside tag values.
- Tag value inheritance.
- Section data.
- Comments.
- Multiple boards in one request.
- `Auction` parsing.
- `Play` parsing.

`Auction` and `Play` tags may be present but are ignored until the phase that implements them.

## `Dealer`

`Dealer` must be one of:

- `N`
- `E`
- `S`
- `W`

`Dealer` is the only source for the `DealerPar` dealer argument. The `Deal` tag's `<first>` value must not be used as the dealer.

## `Vulnerable`

Accepted values:

- `None`, `Love`, and `-`: no vulnerability.
- `NS`: `North-South` vulnerable.
- `EW`: `East-West` vulnerable.
- `All` and `Both`: both sides vulnerable.

When serializing normalized board data, prefer `None`, `NS`, `EW`, and `All`.

## `Deal`

`Deal` must use the standard `PBN` deal-tag value format:

```text
<first>:<1st_hand> <2nd_hand> <3rd_hand> <4th_hand>
```

`<first>` must be one of `N`, `E`, `S`, or `W`. The four hands are listed clockwise from `<first>`.

Each hand must contain four suit fields in `S.H.D.C` order. A void suit is represented by an empty field.

Example:

```pbn
[Deal "N:QJ6.K652.J85.T98 873.J97.AT764.Q4 K5.T83.KQ9.A7652 AT942.AQ4.32.KJ3"]
```

The initial implementation requires complete deals:

- Four hands must be present.
- No hand may be `-`.
- Each hand must contain exactly `13` cards.
- The board must contain exactly `52` cards.
- No card may appear more than once.
- Rank characters are `A`, `K`, `Q`, `J`, `T`, `9`, `8`, `7`, `6`, `5`, `4`, `3`, and `2`.
- Suit fields may be supplied in any rank order on input.

Normalized deal output uses:

- The original `<first>` direction.
- Hands emitted clockwise from `<first>`.
- Suit order `S.H.D.C`.
- Descending rank order within each suit.

## Error Policy

Invalid `PBN` input must fail before calling `DDS`.

Missing required tags, duplicate required tags, unsupported features, invalid tag values, invalid deals, and oversized `DDS` buffers should produce distinct errors where practical.
