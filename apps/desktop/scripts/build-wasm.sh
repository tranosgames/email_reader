#!/bin/sh
# Compile le moteur Rust en WebAssembly et génère la liaison JS dans src/wasm/ (feature 003).
# Prérequis : rustup target add wasm32-unknown-unknown ; cargo install wasm-bindgen-cli --version 0.2.129
set -eu
ROOT=$(cd "$(dirname "$0")/../../.." && pwd)
cd "$ROOT"
# MF_BUILD_STD=1 : reconstruit la std depuis rust-src (environnements sans le composant rust-std wasm).
if [ "${MF_BUILD_STD:-0}" = "1" ]; then
  RUSTC_BOOTSTRAP=1 cargo build -p mf-wasm --profile wasm --target wasm32-unknown-unknown -Zbuild-std=std,panic_abort
else
  cargo build -p mf-wasm --profile wasm --target wasm32-unknown-unknown
fi
wasm-bindgen --target web --out-dir apps/desktop/src/wasm \
  target/wasm32-unknown-unknown/wasm/mf_wasm.wasm
ls -l apps/desktop/src/wasm/mf_wasm_bg.wasm
