#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"

npm --prefix "$ROOT/apps/web" ci
npm --prefix "$ROOT/apps/web" run build

if ! command -v rustup >/dev/null 2>&1; then
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain none
fi
rustup toolchain install 1.97.1 --profile minimal
rustup toolchain install nightly-2026-10-05 --profile minimal --component rust-src --target wasm32-unknown-unknown
if [[ "$(trunk --version 2>/dev/null || true)" != "trunk 0.21.14" ]]; then
  cargo +1.97.1 install trunk --version 0.21.14 --locked
fi
cd "$ROOT/apps/web"
RUSTUP_TOOLCHAIN=nightly-2026-10-05 trunk build --release --locked
