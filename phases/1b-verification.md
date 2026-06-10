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

Rows are `next_to_act`, not declarers. Values differ from the full-deal matrix because the
opening leader differs.

### 3. Full deal via Position tag (continuation)

```bash
echo '[Position "N:QJ6.K652.J85.T98 873.J97.AT764.Q4 K5.T83.KQ9.A7652 AT942.AQ4.32.KJ3"]
[First "N"]
[Trump "S"]
' | cargo run -- solve --trump S
```

### 4. Residual position, clean trick start

Each player holds a complete suit. North leads with NoTrump.

```bash
echo '[Position "N:AKQJ... .AKQJ.. ..AKQJ. ...AKQJ"]
[First "N"]
[Trump "NT"]
' | cargo run -- solve --trump NT
```

Expected:

```text
Trump: NT
First: N
Current tricks: (empty)
Next to act: N

N plays for NS side tricks:
4: SA SJ SQ SK
```

Matrix variant:

```bash
echo '[Position "N:AKQJ... .AKQJ.. ..AKQJ. ...AKQJ"]
[First "N"]
' | cargo run -- solve --matrix
```

### 5. Mid-trick (1 card played)

North led S-A. East is next to act. N is the trick leader (NS side), but next_to_act
is East (EW side). `score_side` is EW — DDS scores from the side to play.

```bash
echo '[Position "N:AKQJ... .AKQJ.. ..AKQJ. ...AKQJ"]
[First "E"]
[Trump "NT"]
[CurrentTrick "N:SA"]
' | cargo run -- solve --trump NT
```

Expected: East's heart plays.

```text
Trump: NT
First: N
Current tricks: NSA
Next to act: E

E plays for EW side tricks:
0: HA HJ HQ HK
```

### 6. Mid-trick (2 cards played)

North led S-A, East played H-A. South is next (S is NS side).

```bash
echo '[Position "N:AKQJ... .AKQJ.. ..AKQJ. ...AKQJ"]
[First "S"]
[Trump "NT"]
[CurrentTrick "N:SA E:HA"]
' | cargo run -- solve --trump NT
```

Expected:

```text
Trump: NT
First: N
Current tricks: NSA EHA
Next to act: S

S plays for NS side tricks:
4: DA DJ DQ DK
```

### 7. Mid-trick (3 cards played)

North S-A, East H-A, South D-A. West is last (W is EW side).

```bash
echo '[Position "N:AKQJ... .AKQJ.. ..AKQJ. ...AKQJ"]
[First "W"]
[Trump "NT"]
[CurrentTrick "N:SA E:HA S:DA"]
' | cargo run -- solve --trump NT
```

Expected:

```text
Trump: NT
First: N
Current tricks: NSA EHA SDA
Next to act: W

W plays for EW side tricks:
0: CA CJ CQ CK
```

### 8. Mid-trick, alternate trick leader (East leads)

East led H-A, South played D-A. West is next (EW side). `score_side` is EW.

```bash
echo '[Position "N:AKQJ... .AKQJ.. ..AKQJ. ...AKQJ"]
[First "W"]
[Trump "NT"]
[CurrentTrick "E:HA S:DA"]
' | cargo run -- solve --trump NT
```

Expected: West's club plays.

```text
Trump: NT
First: E
Current tricks: EHA SDA
Next to act: W

W plays for EW side tricks:
4: CA CJ CQ CK
```

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

### 10. CurrentTrick validation: invalid play order (error)

East leads H-A, but the second card is listed as N:SA. In clockwise order from E,
the second player is S, not N.

```bash
echo '[Position "N:AKQJ... .AKQJ.. ..AKQJ. ...AKQJ"]
[First "S"]
[Trump "NT"]
[CurrentTrick "E:HA N:SA"]
' | cargo run -- solve --trump NT
```

Expected: `error: invalid position: SnapshotPosition: South does not hold SA (current trick card 2)`.

### 11. --first overrides [First] tag

```bash
echo '[Position "N:AKQJ... .AKQJ.. ..AKQJ. ...AKQJ"]
[First "N"]
[Trump "NT"]
' | cargo run -- solve --first E
```

### 12. JSON output

```bash
echo '[Position "N:AKQJ... .AKQJ.. ..AKQJ. ...AKQJ"]
[First "N"]
[Trump "NT"]
' | cargo run -- solve --trump NT --format json
```

Expected: JSON object with `score_side`, `next_to_act`, `current_trick`, `suggested` fields.

### 13. Play trace (with prefix, no --declarer needed)

```bash
echo '[Deal "N:QJ6.K652.J85.T98 873.J97.AT764.Q4 K5.T83.KQ9.A7652 AT942.AQ4.32.KJ3"]
[Dealer "N"]
[Vulnerable "None"]
[Play "E:S3=S5=S2=SQ"]
' | cargo run -- solve --trump S
```

Expected: continuation analysis showing N won the trick and leads next.

### 14. Play trace (no prefix, --declarer required)

```bash
echo '[Deal "N:QJ6.K652.J85.T98 873.J97.AT764.Q4 K5.T83.KQ9.A7652 AT942.AQ4.32.KJ3"]
[Dealer "N"]
[Vulnerable "None"]
[Play "S3"]
' | cargo run -- solve --trump S
```

Expected: error (`--declarer is required when Play tag has no direction prefix`).

### 15. Error cases

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
