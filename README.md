# bridge-dds

A double-dummy bridge solver and analysis tool. Given a deal in PBN format, it computes the
maximum tricks each side can win in every strain, derives the par contract, and (with a play
trace) provides optimal continuation from any point in the play. Available as a standalone
CLI tool and as a self-contained Web application.

## Build

Prerequisites: `git`, `g++` (or `clang++`), `Rust` toolchain.

```bash
git clone --recurse-submodules https://github.com/<org>/bridge-dds.git
cd bridge-dds

# Build the DDS C library
./scripts/build-dds-macos.sh

# Build the CLI
cargo build --release
```

The binary is at `target/release/bridge`.

## Usage

```bash
echo '[Deal "N:QJ6.K652.J85.T98 873.J97.AT764.Q4 K5.T83.KQ9.A7652 AT942.AQ4.32.KJ3"]
[Dealer "N"]
[Vulnerable "None"]
' | ./target/release/bridge solve
```

JSON output:

```bash
echo '...' | ./target/release/bridge solve --format json
```

## Tests

```bash
cargo test
```
