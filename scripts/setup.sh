#!/bin/zsh
set -euo pipefail

ROOT="$(cd "$(dirname "$0")"/..; pwd)"
cd "$ROOT"

if [ ! -d doom-ascii ]; then
  git clone https://github.com/wojciech-graj/doom-ascii.git
fi
cd doom-ascii
make
cd "$ROOT"
cp doom-ascii/_$(uname | tr A-Z a-z)/game/doom_ascii bin/doom_ascii || true

echo "Built doom_ascii to $ROOT/bin/doom_ascii"
