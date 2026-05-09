#!/bin/bash
# Build DDS static library for macOS (clang, GCD + STL threading, no Boost).

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
DDS_DIR="$PROJECT_DIR/engine/dds"
LIB_DIR="$DDS_DIR/lib"

echo "==> Cleaning previous build..."
make -C "$DDS_DIR/src" -f Makefiles/Makefile_Mac_clang_static clean

echo "==> Compiling DDS..."
make -C "$DDS_DIR/src" -f Makefiles/Makefile_Mac_clang_static \
  THREADING="-DDDS_THREADS_GCD -DDDS_THREADS_STL" \
  THREAD_LINK="" \
  WARN_FLAGS="-Wall -Wextra -Werror -Wno-unused -Wno-deprecated-declarations -Wno-sign-conversion -Wno-array-parameter -Wno-missing-field-initializers" \
  macos

echo "==> Copying libdds.a to $LIB_DIR/"
mkdir -p "$LIB_DIR"
cp "$DDS_DIR/src/libdds.a" "$LIB_DIR/libdds.a"

echo "==> Done. Library at $LIB_DIR/libdds.a"
