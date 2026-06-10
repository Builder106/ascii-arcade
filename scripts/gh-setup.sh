#!/bin/zsh
set -euo pipefail

# Sets GitHub repo metadata (description + topics) once the repo is pushed.
# Requires the `gh` CLI authenticated against the repo's remote.
#
#   ./scripts/gh-setup.sh

gh repo edit \
  --description "macOS live-wallpaper customizer — a spinning ASCII donut, a helix, or playable text-mode DOOM as your desktop background." \
  --add-topic swift \
  --add-topic macos \
  --add-topic appkit \
  --add-topic ascii-art \
  --add-topic wallpaper \
  --add-topic doom \
  --add-topic terminal \
  --add-topic donut \
  --add-topic freedoom \
  --add-topic vapor \
  --add-topic pseudo-terminal

echo "Repo metadata updated. (No homepage set — this is a desktop app with no hosted demo.)"
