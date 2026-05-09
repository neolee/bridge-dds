# Phase 1a Verification Guide

## Prerequisites

Build the project once:

```bash
cd /Users/neo/Code/ML/bridge-dds

# Build DDS C library (one-time)
make -C engine/dds/src -f Makefiles/Makefile_Mac_clang_static clean
make -C engine/dds/src -f Makefiles/Makefile_Mac_clang_static \
  THREADING="-DDDS_THREADS_GCD -DDDS_THREADS_STL" \
  THREAD_LINK="" \
  WARN_FLAGS="-Wall -Wextra -Werror -Wno-unused -Wno-deprecated-declarations -Wno-sign-conversion -Wno-array-parameter -Wno-missing-field-initializers" \
  macos
mkdir -p engine/dds/lib
cp engine/dds/src/libdds.a engine/dds/lib/libdds.a

# Build Rust CLI
cargo build --release
```

The binary is at `target/release/bridge`.

## Automated tests

```bash
cargo test
```

Expected: 16 tests pass. You can also run `cargo test -- --nocapture` to see test names.

## Manual CLI verification

All examples use the binary from the build directory. Run from the project root.

### 1. Basic solve (text output)

```bash
echo '[Deal "N:QJ6.K652.J85.T98 873.J97.AT764.Q4 K5.T83.KQ9.A7652 AT942.AQ4.32.KJ3"]
[Dealer "N"]
[Vulnerable "None"]
' | cargo run -- solve
```

Expected output:

```
   S  H  D  C  N
N  5  6  5  7  6 
E  8  6  7  5  6 
S  5  6  5  7  6 
W  8  6  7  5  6 
NS 5  6  5  7  6 
EW 8  6  7  5  6 
Par: 2S-EW; -110
```

### 2. JSON output

```bash
echo '[Deal "N:QJ6.K652.J85.T98 873.J97.AT764.Q4 K5.T83.KQ9.A7652 AT942.AQ4.32.KJ3"]
[Dealer "N"]
[Vulnerable "None"]
' | cargo run -- solve --format json
```

Expected: valid JSON with `tricks` object (four directions, each with S/H/D/C/NT) and `par` object with `score` and `contracts` array.

### 3. Deal 2 from DDS example set

```bash
echo '[Deal "E:QJT5432.T.6.QJ82 .J97543.K7532.94 87.A62.QJT4.AT75 AK96.KQ8.A98.K63"]
[Dealer "E"]
[Vulnerable "NS"]
' | cargo run -- solve
```

Expected:

```
   S  H  D  C  N
N  4 10  8  6  9 
E  9  2  3  7  3 
S  4 10  8  6  9 
W  9  2  3  7  3 
NS 4 10  8  6  9 
EW 9  2  3  7  3 
Par: 4S*-EW-1; +100
```

### 4. Deal 3 from DDS example set

```bash
echo '[Deal "N:73.QJT.AQ54.T752 QT6.876.KJ9.AQ84 5.A95432.7632.K6 AKJ9842.K.T8.J93"]
[Dealer "N"]
[Vulnerable "None"]
' | cargo run -- solve
```

Expected:

```
   S  H  D  C  N
N  3  9  8  3  4 
E 10  4  4  9  8 
S  3  9  8  3  4 
W 10  4  4  9  8 
NS 3  9  8  3  4 
EW 10  4  4  9  8 
Par: 5H*-NS-2; -300
```

### 5. Deal.first differs from Dealer

Verify that `<first>` in the `Deal` tag and the `Dealer` tag are handled independently.
This deal has the same hands as Deal 1 (section 1), but hands are rotated so `<first>` is `E`
instead of `N`. Dealer is still `N`.

```bash
echo '[Deal "E:873.J97.AT764.Q4 K5.T83.KQ9.A7652 AT942.AQ4.32.KJ3 QJ6.K652.J85.T98"]
[Dealer "N"]
[Vulnerable "None"]
' | cargo run -- solve
```

Expected: tricks matrix identical to Deal 1 (same hands, just rotated presentation), but par uses Dealer=N.

### 6. Vulnerable aliases

Test each accepted `Vulnerable` value and verify the score sign changes:

```bash
# None
echo '[Deal "N:QJ6.K652.J85.T98 873.J97.AT764.Q4 K5.T83.KQ9.A7652 AT942.AQ4.32.KJ3"]
[Dealer "N"]
[Vulnerable "None"]
' | cargo run -- solve --format json | grep score

# Both
echo '[Deal "N:QJ6.K652.J85.T98 873.J97.AT764.Q4 K5.T83.KQ9.A7652 AT942.AQ4.32.KJ3"]
[Dealer "N"]
[Vulnerable "Both"]
' | cargo run -- solve --format json | grep score
```

Expected: scores may differ between `None` and `Both` because game and slam bonuses change with vulnerability.

### 6. Error cases

Each of these should print an error to stderr and exit with code 1:

```bash
# Missing Dealer
echo '[Deal "N:QJ6.K652.J85.T98 873.J97.AT764.Q4 K5.T83.KQ9.A7652 AT942.AQ4.32.KJ3"]
' | cargo run -- solve

# Missing Vulnerable
echo '[Deal "N:QJ6.K652.J85.T98 873.J97.AT764.Q4 K5.T83.KQ9.A7652 AT942.AQ4.32.KJ3"]
[Dealer "N"]
' | cargo run -- solve

# Invalid Vulnerable
echo '[Deal "N:QJ6.K652.J85.T98 873.J97.AT764.Q4 K5.T83.KQ9.A7652 AT942.AQ4.32.KJ3"]
[Dealer "N"]
[Vulnerable "XYZ"]
' | cargo run -- solve

# Duplicate required tag
echo '[Deal "N:QJ6.K652.J85.T98 873.J97.AT764.Q4 K5.T83.KQ9.A7652 AT942.AQ4.32.KJ3"]
[Dealer "N"]
[Dealer "S"]
[Vulnerable "None"]
' | cargo run -- solve

# Partial deal (hand is "-")
echo '[Deal "N:QJ6.K652.J85.T98 - K5.T83.KQ9.A7652 AT942.AQ4.32.KJ3"]
[Dealer "N"]
[Vulnerable "None"]
' | cargo run -- solve
```

### 7. Using the release binary directly

```bash
cargo build --release
echo '[Deal "N:QJ6.K652.J85.T98 873.J97.AT764.Q4 K5.T83.KQ9.A7652 AT942.AQ4.32.KJ3"]
[Dealer "N"]
[Vulnerable "None"]
' | ./target/release/bridge solve
```
