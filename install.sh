#!/usr/bin/env bash
# Satoshi's Razor - installer.
# Checks the toolchain, fetches the repo if needed, builds every component,
# seeds the live registry, replays the demo once to prove the stack works,
# and serves the site.
#
#   curl -sSf <host>/install.sh | bash
#   ./install.sh              # from a checkout
set -euo pipefail

REPO_URL="${RAZOR_REPO:-https://github.com/jorikschellekens/satoshis-razor}"
PORT="${RAZOR_PORT:-8420}"

say()  { printf '\033[1;36m▸ %s\033[0m\n' "$*"; }
die()  { printf '\033[1;31m✕ %s\033[0m\n' "$*" >&2; exit 1; }
have() { command -v "$1" >/dev/null 2>&1; }

say "checking toolchain"
have git || die "git is required"
have python3 || die "python3 is required (serves the site)"

if ! have cargo; then
  die "rust is required - install via https://rustup.rs then re-run"
fi
if ! have elan && ! have lake; then
  say "installing elan (Lean toolchain manager)"
  curl -sSf https://raw.githubusercontent.com/leanprover/elan/master/elan-init.sh | sh -s -- -y
  export PATH="$HOME/.elan/bin:$PATH"
fi
if have rustup && ! rustup target list --installed | grep -q wasm32-unknown-unknown; then
  say "adding wasm32-unknown-unknown target (Tier-1 deterministic scoring)"
  rustup target add wasm32-unknown-unknown
fi

# locate or fetch the repo
if [ -f lean/lakefile.toml ] && [ -f demo.sh ]; then
  say "using current checkout: $(pwd)"
else
  say "cloning $REPO_URL"
  git clone "$REPO_URL" satoshis-razor
  cd satoshis-razor
fi

say "building (Lean package + registry + harness + zk prover + wasm submissions)"
(cd lean && lake build)
cargo build --release
cargo build --release --target wasm32-unknown-unknown \
  -p popcount-naive -p popcount-swar -p sum-loop -p sum-closed -p evm-ref -p evm-tos

say "replaying the demo walkthrough (real proofs, real benchmarks, real SNARKs)"
./demo.sh

say "seeding the live registry (real corpora, real open problems)"
./seed.sh

say "putting razor, anvil-harness, and zk-prover on your PATH"
BIN="${CARGO_HOME:-$HOME/.cargo}/bin"
mkdir -p "$BIN"
for t in razor anvil-harness zk-prover; do ln -sf "$PWD/target/release/$t" "$BIN/$t"; done
echo "  linked into $BIN (rebuilds update them in place)"

say "done - serving the registry at http://localhost:$PORT"
echo "  try: razor help"
razor serve --port "$PORT"
