#!/usr/bin/env bash
# Satoshi's Razor - tool installer.
# Checks the toolchain, fetches the repo if needed, builds every component,
# and links the binaries onto your PATH. Nothing else: no dataset is written,
# no server is started.
#
#   curl -sSf <host>/install.sh | bash
#   ./install.sh              # from a checkout
set -euo pipefail

REPO_URL="${RAZOR_REPO:-https://github.com/jorikschellekens/satoshis-razor}"

say()  { printf '\033[1;36m▸ %s\033[0m\n' "$*"; }
die()  { printf '\033[1;31m✕ %s\033[0m\n' "$*" >&2; exit 1; }
have() { command -v "$1" >/dev/null 2>&1; }

say "checking toolchain"
have git || die "git is required"

if ! have cargo; then
  die "rust is required - install via https://rustup.rs then re-run"
fi
if [ "$(uname -s)" = "Linux" ] && ! have bwrap; then
  say "note: bubblewrap is not installed - local proof verification will run unsandboxed"
  echo "  fix: sudo apt install bubblewrap  (the checker isolates untrusted proofs with it)"
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

say "putting razor, anvil-harness, and zk-prover on your PATH"
BIN="${CARGO_HOME:-$HOME/.cargo}/bin"
mkdir -p "$BIN"
for t in razor anvil-harness zk-prover; do ln -sf "$PWD/target/release/$t" "$BIN/$t"; done
echo "  linked into $BIN (rebuilds update them in place)"

# Point participation commands at the public registry, unless the user opted
# out or already chose a remote. One file, one url; `razor remote off` or
# --local on any command undoes it.
if [ -z "${RAZOR_NO_REMOTE:-}" ] && [ ! -f "$HOME/.config/razor/remote" ]; then
  mkdir -p "$HOME/.config/razor"
  echo "https://razor.mempoolsurfer.com" > "$HOME/.config/razor/remote"
  say "default remote set to https://razor.mempoolsurfer.com"
  echo "  razor propose / formalize / submit ... publish there, signed by your key"
  echo "  razor remote off (or --local on any command) keeps everything on this machine"
fi

say "done"
echo "  try: razor help"
echo "  the clone already contains the live log; to browse it locally:"
echo "    razor serve      # http://localhost:8420  (./demo.sh first for the scripted walkthrough)"
