#!/usr/bin/env bash
# Build the current working tree as a local macOS application.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

if [[ "$(uname -s)" != Darwin ]]; then
    echo "Local app packaging requires macOS." >&2
    exit 1
fi

export CARGO_TARGET_DIR="$PWD/target"
cargo build --release --locked
# cargo-bundle has no --locked option. Reuse the resolved dependencies offline.
CARGO_NET_OFFLINE=true cargo bundle --release

release_app="$CARGO_TARGET_DIR/release/bundle/osx/gpui-tetris.app"
# Finish the local bundle after cargo-bundle has copied its resources.
codesign --force --sign - "$release_app"
codesign --verify --deep --strict "$release_app"
printf '\nLocal release: %s\n' "$release_app"
