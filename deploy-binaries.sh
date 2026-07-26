#!/usr/bin/env bash
#
# Build backend + frontend from a pushed git ref and copy the closures to the
# deployment host.
#
# Usage:
#   ./deploy-binaries.sh <git-ref> [host]
#
#   <git-ref>  tag, branch or commit that exists on GitHub (e.g. v1.1.0, main)
#   [host]     ssh target, defaults to $DEPLOY_HOST or shifty.nebenan-unverpackt.de
#
# The ref must be pushed to GitHub first — this builds from the archive URL,
# not from your working copy, so the deployed artifact is reproducible.
#
# After this script finishes, activate the new closure on the host by pointing
# the NixOS config at the same ref and running nixos-rebuild there.

set -euo pipefail

REF="${1:-}"
HOST="${2:-${DEPLOY_HOST:-shifty.nebenan-unverpackt.de}}"

if [ -z "$REF" ]; then
  echo "usage: $0 <git-ref> [host]" >&2
  exit 1
fi

ZIP_URL="https://github.com/neosam/haushalt/archive/${REF}.zip"

echo "==> ref:  $REF"
echo "==> src:  $ZIP_URL"
echo "==> host: $HOST"
echo

echo "==> Fetching SHA256 ..."
nix-prefetch-url --unpack "$ZIP_URL"
echo

for pkg in backend frontend; do
  echo "==> Building $pkg ..."
  # --no-link so we never clobber ./result in a working copy; the out path is
  # read from stdout instead.
  OUT=$(nix build "${ZIP_URL}#${pkg}" --no-link --print-out-paths)
  echo "    $OUT"
  echo "==> Copying $pkg closure to $HOST ..."
  nix-copy-closure --to "$HOST" "$OUT"
  echo
done

echo "==> Done. Both closures are on $HOST."
echo "    Now point that host's NixOS config at ref '$REF' and run nixos-rebuild switch."
