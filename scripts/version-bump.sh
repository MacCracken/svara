#!/usr/bin/env bash
# version-bump.sh — Set the crate version. `cyrius.cyml` reads the version from
# the VERSION file via `${file:VERSION}`, so this single write keeps the manifest,
# distlib, and any consumer in sync. Remember to update CHANGELOG.md + roadmap.
set -euo pipefail
[ $# -ne 1 ] && echo "Usage: $0 <version>" && exit 1
NEW_VERSION="$1"
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
echo "$NEW_VERSION" > "$REPO_ROOT/VERSION"
echo "Bumped VERSION to ${NEW_VERSION} (cyrius.cyml picks it up via \${file:VERSION})."
