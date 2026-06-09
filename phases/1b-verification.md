# Phase 1b Verification Guide

## Prerequisites

```bash
cd /Users/neo/Code/ML/bridge-dds
./scripts/build-dds-macos.sh
cargo build --release
```

## Automated tests

```bash
cargo test -- --test-threads=1
```

## Manual verification

### 1. Full deal via Deal tag (Phase 1a path, unchanged)

```bash
echo '[Deal "N:QJ6.K652.J85.T98 873.J97.AT764.Q4 K5.T83.KQ9.A7652 AT942.AQ4.32.KJ3"]
[Dealer "N"]
[Vulnerable "None"]
' | cargo run -- solve
```

### 2. Full deal via Position tag (position matrix)

```bash
echo '[Position "N:QJ6.K652.J85.T98 873.J97.AT764.Q4 K5.T83.KQ9.A7652 AT942.AQ4.32.KJ3"]
[First "N"]
' | cargo run -- solve --matrix
```

Rows are `next_to_act`, not declarers. Values differ from the full-deal matrix because the opening leader differs.

### 3. Full deal via Position tag (continuation)

```bash
echo '[Position "N:QJ6.K652.J85.T98 873.J97.AT764.Q4 K5.T83.KQ9.A7652 AT942.AQ4.32.KJ3"]
[First "N"]
[Trump "S"]
' | cargo run -- solve --trump S
```

### 4. Residual position, clean trick start (continuation + matrix)

Each player holds a complete suit. North leads with NoTrump.

```bash
echo '[Position "N:AKQJ... .AKQJ.. ..AKQJ. ...AKQJ"]
[First "N"]
[Trump "NT"]
' | cargo run -- solve --trump NT
```

Expected: `N plays for NS side tricks:`, then spade cards grouped by score.

Matrix variant:

```bash
echo '[Position "N:AKQJ... .AKQJ.. ..AKQJ. ...AKQJ"]
[First "N"]
' | cargo run -- solve --matrix
```

### 5. Mid-trick (1 card played)

North led S-A. East is next to act.

```bash
echo '[Position "N:AKQJ... .AKQJ.. ..AKQJ. ...AKQJ"]
[First "E"]
[Trump "NT"]
[CurrentTrick "N:SA"]
' | cargo run -- solve --trump NT
```

Expected: East's heart plays. `E plays for NS side tricks:` (trick leader = N).

### 6. Mid-trick (2 cards played)

North led S-A, East played H-A. South is next.

```bash
echo '[Position "N:AKQJ... .AKQJ.. ..AKQJ. ...AKQJ"]
[First "S"]
[Trump "NT"]
[CurrentTrick "N:SA E:HA"]
' | cargo run -- solve --trump NT
```

Expected: South's diamond plays. `S plays for NS side tricks:`.

### 7. Mid-trick (3 cards played)

North S-A, East H-A, South D-A. West is last.

```bash
echo '[Position "N:AKQJ... .AKQJ.. ..AKQJ. ...AKQJ"]
[First "W"]
[Trump "NT"]
[CurrentTrick "N:SA E:HA S:DA"]
' | cargo run -- solve --trump NT
```

Expected: West's club plays. `W plays for NS side tricks:` (trick leader = N).

### 8. Mid-trick, alternate trick leader (East leads)

```bash
echo '[Position "N:AKQJ... .AKQJ.. ..AKQJ. ...AKQJ"]
[First "S"]
[Trump "NT"]
[CurrentTrick "E:HA N:SA"]
' | cargo run -- solve --trump NT
```

East led H-A, North discarded S-A. Expected: `S plays for EW side tricks:` (trick leader = E).

### 9. CurrentTrick validation: card not held (error)

East claims S2 but only holds hearts:

```bash
echo '[Position "N:AKQJ... .AKQJ.. ..AKQJ. ...AKQJ"]
[First "W"]
[Trump "NT"]
[CurrentTrick "N:SA E:S2 S:S3"]
' | cargo run -- solve --trump NT
```

Expected: `error: invalid position: CurrentTrick: East does not hold S2`.

### 10. --first overrides [First] tag

```bash
echo '[Position "N:AKQJ... .AKQJ.. ..AKQJ. ...AKQJ"]
[First "N"]
' | cargo run -- solve --matrix --first E
```

### 11. JSON output

```bash
echo '[Position "N:AKQJ... .AKQJ.. ..AKQJ. ...AKQJ"]
[First "N"]
[Trump "NT"]
' | cargo run -- solve --trump NT --format json
```

### 12. Play trace (with prefix, no --declarer needed)

```bash
echo '[Deal "N:QJ6.K652.J85.T98 873.J97.AT764.Q4 K5.T83.KQ9.A7652 AT942.AQ4.32.KJ3"]
[Dealer "N"]
[Vulnerable "None"]
[Play "E:S3=S5=S2=SQ"]
' | cargo run -- solve --trump S
```

Expected: continuation analysis showing N won the trick and leads next.

### 13. Play trace (no prefix, --declarer required)

```bash
echo '[Deal "N:QJ6.K652.J85.T98 873.J97.AT764.Q4 K5.T83.KQ9.A7652 AT942.AQ4.32.KJ3"]
[Dealer "N"]
[Vulnerable "None"]
[Play "S3"]
' | cargo run -- solve --trump S
```

Expected: error (`--declarer is required when Play tag has no direction prefix`).

### 14. Error cases

```bash
# Missing --trump and no [Trump] tag
echo '[Position "N:AKQJ... .AKQJ.. ..AKQJ. ...AKQJ"]
[First "N"]
' | cargo run -- solve

# Missing [First] tag and no --first
echo '[Position "N:AKQJ... .AKQJ.. ..AKQJ. ...AKQJ"]
' | cargo run -- solve --trump NT

# Unequal hand sizes
echo '[Position "N:AKQJ... .AKQJ.. ..AKQJ. ..."]
[First "N"]
[Trump "NT"]
' | cargo run -- solve --trump NT
```
