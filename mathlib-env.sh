#!/usr/bin/env bash
# Fetch and build the Mathlib verification environment. Run this once,
# with disk to spare: the prebuilt cache for mathlib v4.31.0 is several
# gigabytes. Until it has been run, sorries registered with --env mathlib
# cannot be verified locally (the registry will say so).
set -euo pipefail
cd "$(dirname "$0")/lean-mathlib"
lake exe cache get
lake build
echo "mathlib environment ready: sorries with env=mathlib can now be verified"
