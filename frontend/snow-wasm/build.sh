#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "$0")" && pwd)"
OUTPUT_DIR="$ROOT_DIR/pkg"
TARGET_DIR="$ROOT_DIR/target/wasm32-unknown-unknown/release"
WASM_ARTIFACT="$TARGET_DIR/snow_wasm.wasm"
WASM_BINDGEN_BIN="${WASM_BINDGEN:-wasm-bindgen}"

set -euo pipefail

echo "[snow-wasm] building release wasm artifact..."
cargo build --manifest-path "$ROOT_DIR/Cargo.toml" --target wasm32-unknown-unknown --release

if [[ ! -f "$WASM_ARTIFACT" ]]; then
  echo "error: expected wasm artifact at $WASM_ARTIFACT"
  exit 1
fi

echo "[snow-wasm] generating JS bindings with $WASM_BINDGEN_BIN..."
rm -rf "$OUTPUT_DIR"
mkdir -p "$OUTPUT_DIR"

"$WASM_BINDGEN_BIN" \
  --target web \
  --out-dir "$OUTPUT_DIR" \
  --out-name snow_sim \
  "$WASM_ARTIFACT"

echo "[snow-wasm] ready! pkg contents:"
ls -1 "$OUTPUT_DIR"
