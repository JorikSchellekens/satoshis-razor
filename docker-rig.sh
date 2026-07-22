#!/usr/bin/env bash
# Bring a Docker container to the anvil as a benchmark rig.
#
# Builds a Linux image containing the benchmark harness, registers it as a
# rig whose runner is `docker run`, and measures every registered challenge
# on it. On macOS and Windows the container runs inside Docker's Linux
# virtual machine, so this adds a genuinely different environment to the
# boards: Linux scores next to the host's native scores, from one laptop.
#
# Usage:  ./docker-rig.sh [challenge ids...]     (default: all challenges)
set -euo pipefail
cd "$(dirname "$0")"

RAZOR=${RAZOR:-target/release/razor}
IMAGE=satoshis-anvil-rig
LOG=registry/data/events.jsonl

command -v docker >/dev/null 2>&1 || { echo "docker is not installed" >&2; exit 1; }
docker info >/dev/null 2>&1 || { echo "docker is installed but the daemon is not running" >&2; exit 1; }
[ -x "$RAZOR" ] || { echo "$RAZOR not built - run ./install.sh or cargo build --release" >&2; exit 1; }

# With a remote registry configured (install.sh sets the public one), the
# CLI publishes there and keeps a cached copy of its log; read that copy.
# RAZOR_REMOTE="" opts out, like it does for every razor command.
REMOTE_ACTIVE=
if [ -n "${RAZOR_REMOTE:-}" ]; then
  REMOTE_ACTIVE=1
elif [ -z "${RAZOR_REMOTE+x}" ] && [ -s "$HOME/.config/razor/remote" ]; then
  REMOTE_ACTIVE=1
fi
if [ -n "$REMOTE_ACTIVE" ]; then
  $RAZOR status > /dev/null   # refreshes the cached remote log
  LOG=registry/data/remote.jsonl
fi
[ -f "$LOG" ] || { echo "no registry log at $LOG - run ./demo.sh or ./seed.sh first" >&2; exit 1; }

echo "==> building the rig image (the first build compiles the harness inside the container - a few minutes; cached afterwards)"
docker build -q -t "$IMAGE" -f anvil/rig/docker/Dockerfile .

ARCH=$(docker run --rm --entrypoint uname "$IMAGE" -m)   # e.g. aarch64, x86_64
RIG="docker-linux-$ARCH"

if ! grep -q "\"register_rig\".*\"id\":\"$RIG\"" "$LOG"; then
  echo "==> registering rig $RIG"
  $RAZOR rig --id "$RIG" --owner "${RIG_OWNER:-${USER:-local}}" --arch "$ARCH-linux" --tier native \
    --runner "docker run --rm $IMAGE" \
    --note "Linux container: scores measured inside Docker's Linux VM, not on the host"
else
  echo "==> rig $RIG already registered"
fi

if [ $# -gt 0 ]; then
  CHALLENGES="$*"
else
  CHALLENGES=$(grep '"type":"register_challenge"' "$LOG" | sed -E 's/.*"id":"([^"]+)".*/\1/')
fi
[ -n "$CHALLENGES" ] || { echo "no challenges on this dataset - ./demo.sh registers four" >&2; exit 1; }

for c in $CHALLENGES; do
  echo "==> benching $c on $RIG"
  $RAZOR bench --challenge "$c" --rig "$RIG"
done

if [ -n "$REMOTE_ACTIVE" ]; then
  echo
  echo "Done. The scores are published - the public site picks them up on its next poll."
else
  # Re-export with whatever dataset label the current site data carries.
  DATASET=$(python3 -c "import json;print(json.load(open('site/data.json')).get('dataset','demo'))" 2>/dev/null || echo demo)
  $RAZOR export --out site/data.json --dataset "$DATASET"
  echo
  echo "Done. The $ARCH-linux boards are live - razor serve, then open the anvil page."
fi
