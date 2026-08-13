#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
required_version="cargo-about 0.9.1"

if [ "$(cargo about --version 2>/dev/null || true)" != "$required_version" ]; then
  echo "cargo-about 0.9.1 is required: cargo install --locked --features cli --version 0.9.1 cargo-about" >&2
  exit 1
fi

cd "$repo_root"
cargo about generate \
  --locked \
  --fail \
  --output-file THIRD_PARTY_LICENSES.html \
  about.hbs
