# `Phase 1a` Review 1

## Status

`Phase 1a` is functionally complete, but not fully closed. The `bridge solve` command can read one `PBN` record from `stdin`, parse `Deal`, `Dealer`, and `Vulnerable`, call `DDS`, print the full `4x5` double-dummy tricks matrix, and print the `DealerPar` result in text or `JSON`.

The remaining work is engineering cleanup: formatting, `clippy`, automated `DDS` integration tests, small `PBN` contract strictness gaps, and documentation updates.

## Verified

The repository was clean before review.

`cargo test` passed with `16` tests.

`cargo build --release` passed and produced `target/release/bridge`.

Manual `CLI` verification passed for the `DDS` example deals documented in `phases/1a-verification.md`:

- Deal `1`: `Par: 2S-EW; -110`.
- Deal `2`: `Par: 4S*-EW-1; +100`.
- Deal `3`: `Par: 5H*-NS-2; -300`.

`bridge solve --format json` produced the documented shape with `tricks` and `par`.

Error paths returned exit code `1` and clear messages for:

- Missing `Dealer`.
- Missing `Vulnerable`.
- Invalid `Vulnerable`.
- Duplicate `Dealer`.
- Partial `Deal` using `-`.

`Deal.first` and `Dealer` are handled independently in the implementation. The verification guide example under `Deal.first differs from Dealer` is incorrect because it changes `<first>` to `E` without rotating the four listed hands. A corrected rotated input preserves the expected matrix and par result.

## Implementation Notes

The project skeleton exists: `Cargo.toml`, `src/lib.rs`, `src/cli/main.rs`, and `build.rs`.

The `DDS` submodule exists under `engine/dds`, and `engine/dds/lib/libdds.a` exists.

The hand-written `FFI` declarations in `src/dds/ffi.rs` match the required `DDS` functions and structs for this phase: `ddTableDealPBN`, `ddTableResults`, `parResultsDealer`, `CalcDDtablePBN`, `DealerPar`, `SetMaxThreads`, and `ErrorMessage`.

The safe wrapper in `src/dds/solver.rs` owns the unsafe calls and checks the `ddTableDealPBN.cards` buffer length before calling `DDS`.

The `PBN` parser in `src/core/pbn.rs` supports the required phase subset and rejects duplicate required tags, missing required tags, partial deals, invalid ranks, duplicate cards, and incomplete full-board deals.

The tricks matrix in `src/core/tricks.rs` preserves all `20` `DDS` values and converts them into the `JSON` output shape.

## Open Issues

`cargo fmt --check` fails. The affected files include `src/core/pbn.rs`, `src/dds/ffi.rs`, and `src/dds/solver.rs`.

`cargo clippy --all-targets --all-features -- -D warnings` fails. Current findings are:

- `Hand` exposes `len()` but not `is_empty()`.
- `src/core/tricks.rs` has `needless_range_loop` warnings.

The planned automated `DDS` integration tests are missing. Current tests cover domain logic and `PBN` parsing, but there is no test that directly asserts `engine/dds/examples/hands.cpp` values such as `DDtable`, `dealerScore`, and `dealerContract`.

The `PBN` parser is looser than `phases/pbn-input-contract.md` in a few places:

- `parse_dealer_tag()` accepts values such as `North` because it only reads the first character.
- `parse_deal_tag()` accepts a multi-character `<first>` prefix when the first character is valid.
- `parse_tag_line()` accepts unquoted tag values.

`Hand::from_cards()` and `Hand::add()` differ from the phase plan signature. The plan describes fallible APIs returning `Result`, while the implementation returns `Self`. The current `PBN` path still checks duplicates at the board level, but the public domain API is less defensive than planned.

The phase plan mentions a root `Makefile`, but the repository currently uses `scripts/build-dds-macos.sh` for the `DDS` build. The chosen build entrypoint should be documented consistently.

`PLAN.md` still shows the `Phase 1a` checklist as incomplete. `README.md` is only a short project summary and does not document build or usage commands.

## Recommended Closeout

1. Run `cargo fmt` and commit the formatting changes.

   This is a straightforward cleanup item. The current implementation works, but the repository should pass `cargo fmt --check` before `Phase 1a` is closed.

2. Fix `clippy` by adding `Hand::is_empty()` and rewriting the flagged loops in `src/core/tricks.rs`.

   This is also a straightforward cleanup item. The target is for `cargo clippy --all-targets --all-features -- -D warnings` to pass.

3. Add automated `DDS` integration tests for the documented `DDS` example deals and `DealerPar` results.

   The tests should assert the `DDtable`, `dealerScore`, and `dealerContract` expectations from `engine/dds/examples/hands.cpp`. This moves the current manual `CLI` verification into repeatable coverage and protects the `FFI` layout, `DDS` indexing, vulnerability mapping, and `DealerPar` conversion.

4. Tighten `PBN` parsing to match `phases/pbn-input-contract.md`.

   This is a confirmed implementation defect, not a documentation issue. The parser should be stricter:

   - `parse_dealer_tag()` must require exactly one character and only accept `N`, `E`, `S`, or `W`.
   - `parse_deal_tag()` must require `<first>` to be exactly one direction character before `:`.
   - `parse_tag_line()` must require the standard `[Tag "Value"]` form and reject unquoted values.
   - Tests should cover rejected values such as `North`, `NORTH`, `North:...`, and `[Dealer N]`.

   The reason is that `PBN` is the shared input contract for the `CLI`, later `REST` API, tests, and frontend workflows. Accepting undocumented variants now creates accidental compatibility obligations later. Strict validation also matches the phase rule that invalid `PBN` input fails before calling `DDS`.

5. Make the `Hand` construction APIs defensive as planned.

   This is a confirmed implementation defect. Change the public APIs to the planned fallible form:

   ```rust
   pub fn from_cards(cards: &[Card]) -> Result<Self, Error>;
   pub fn add(&self, card: Card) -> Result<Self, Error>;
   ```

   `Hand::from_cards()` should reject duplicate cards in the input slice. `Hand::add()` should reject adding a card already contained in the hand. `Hand::remove()` can remain infallible because removing an absent card does not introduce hidden duplicate state.

   The reason is that `Hand` uses a bitset, so duplicate cards are otherwise silently collapsed. The current `PBN` parsing path checks duplicates at the board level, but the domain type itself should not make invalid data look valid. This matters for later `Play` parsing, manual card selection, and `REST` request handling.

6. Keep `scripts/build-dds-macos.sh` as the `DDS` build entrypoint and update the documentation accordingly.

   The existing script is acceptable and should remain the supported local build path. The cleanup is to remove or revise references that imply a root `Makefile` owns the stable `DDS` build. Update `phases/1a-full-deal-dds.md`, `phases/1a-verification.md`, and `README.md` so they consistently describe `scripts/build-dds-macos.sh` and `engine/dds/lib/libdds.a`.

   Also correct the `Deal.first differs from Dealer` example in `phases/1a-verification.md` by rotating the hand list when `<first>` changes. The current example changes the board instead of only changing the presentation order.

7. Update `PLAN.md` after the closeout items are completed.

   Once items `1` through `6` are finished and verified, mark the `Phase 1a` checklist entries as complete. Do not mark `Phase 1a` complete before formatting, `clippy`, automated `DDS` integration tests, parser strictness, `Hand` API fixes, and documentation alignment are done.
