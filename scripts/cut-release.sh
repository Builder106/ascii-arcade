#!/bin/zsh
set -euo pipefail

# Cut a release: bump the workspace version, tag it, and push. Pushing the
# tag is what triggers .github/workflows/release.yml, which does the actual
# building/testing/publishing — this script's only job is the version bump
# and a safe, validated tag + push.
#
# Usage:
#   ./scripts/cut-release.sh 0.2.0
#
# Deliberately does NOT run a local build/test gate before tagging — that's
# release.yml's `verify` job's responsibility on the exact tagged commit, and
# duplicating it here would slow down the one command this script exists to
# keep fast.

ROOT="$(cd "$(dirname "$0")"/..; pwd)"
cd "$ROOT"

VERSION="${1:-}"
if [ -z "$VERSION" ]; then
	echo "usage: $0 X.Y.Z" >&2
	exit 1
fi
if ! [[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
	echo "error: '$VERSION' is not semver X.Y.Z (no leading 'v')" >&2
	exit 1
fi
TAG="v$VERSION"

echo "Checking preflight conditions…"

if [ -n "$(git status --porcelain)" ]; then
	echo "error: working tree is not clean — commit or stash first" >&2
	exit 1
fi

BRANCH="$(git rev-parse --abbrev-ref HEAD)"
if [ "$BRANCH" != "main" ]; then
	echo "error: on '$BRANCH', not 'main'" >&2
	exit 1
fi

git fetch origin main --quiet
if [ "$(git rev-parse HEAD)" != "$(git rev-parse origin/main)" ]; then
	echo "error: local main differs from origin/main — pull or push first" >&2
	exit 1
fi

if git rev-parse -q --verify "refs/tags/$TAG" >/dev/null; then
	echo "error: tag $TAG already exists locally" >&2
	exit 1
fi
if git ls-remote --exit-code --tags origin "refs/tags/$TAG" >/dev/null 2>&1; then
	echo "error: tag $TAG already exists on origin" >&2
	exit 1
fi

# Numeric (not lexicographic) semver compare, so 0.9.0 < 0.10.0 sorts right.
version_gt() {
	[ "$1" != "$2" ] && [ "$(printf '%s\n%s\n' "$1" "$2" | sort -t. -k1,1n -k2,2n -k3,3n | tail -1)" = "$1" ]
}

CARGO_VERSION="$(grep -m1 '^version = "' Cargo.toml | cut -d'"' -f2)"
LATEST_TAG_VERSION="$(git tag -l 'v*' | sed 's/^v//' | sort -t. -k1,1n -k2,2n -k3,3n | tail -1)"

if [ -n "$LATEST_TAG_VERSION" ]; then
	# A previous release exists: the new version must be strictly newer.
	FLOOR="$LATEST_TAG_VERSION"
	if ! version_gt "$VERSION" "$FLOOR"; then
		echo "error: $VERSION is not greater than the latest release v$FLOOR" >&2
		exit 1
	fi
else
	# Bootstrap case (no tags yet): allow VERSION to equal Cargo.toml's
	# current version, since that's exactly what the first release does.
	FLOOR="$CARGO_VERSION"
	if version_gt "$FLOOR" "$VERSION"; then
		echo "error: $VERSION is lower than Cargo.toml's current $FLOOR" >&2
		exit 1
	fi
fi

echo
echo "About to:"
echo "  1. Bump Cargo.toml: $CARGO_VERSION -> $VERSION"
echo "  2. Commit as 'chore(release): $TAG'"
echo "  3. Create annotated tag $TAG"
echo "  4. Push main and $TAG to origin (triggers the release workflow)"
echo
printf "Continue? [y/N] "
read -r REPLY
if ! [[ "$REPLY" =~ ^[Yy]$ ]]; then
	echo "Aborted."
	exit 1
fi

MATCHES=$(grep -c '^version = "' Cargo.toml)
if [ "$MATCHES" -ne 1 ]; then
	echo "error: expected exactly one top-level 'version = \"...\"' line in Cargo.toml, found $MATCHES" >&2
	exit 1
fi
sed -i '' -E "s/^version = \"[0-9]+\.[0-9]+\.[0-9]+\"\$/version = \"$VERSION\"/" Cargo.toml

# Re-resolve the workspace graph so Cargo.lock's own version fields for the
# path-dependency members pick up the bump without a full compile. Not
# --offline: a dependency that's in the lockfile but never actually been
# fetched locally (e.g. a Windows-only transitive dep on a macOS dev machine)
# would otherwise hard-fail metadata resolution instead of just downloading it.
cargo metadata --format-version=1 >/dev/null

git add Cargo.toml Cargo.lock
git commit -m "chore(release): $TAG"
git tag -a "$TAG" -m "$TAG"
git push origin main
git push origin "$TAG"

REMOTE_URL="$(git remote get-url origin)"
REPO_SLUG="${REMOTE_URL#https://github.com/}"
REPO_SLUG="${REPO_SLUG%.git}"
echo
echo "Pushed $TAG. Release workflow: https://github.com/$REPO_SLUG/actions/workflows/release.yml"
