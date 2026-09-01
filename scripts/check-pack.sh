#!/usr/bin/env bash
set -euo pipefail
out=$(npm pack --dry-run 2>&1)
echo "$out"
# Fail if leaked files appear in tarball contents
if echo "$out" | grep -qE "src/.*\.test\.ts"; then
  echo "pack leakage: test files in tarball"
  exit 1
fi
if echo "$out" | grep -qE "(^|[[:space:]])cre/"; then
  echo "pack leakage: cre/ in tarball"
  exit 1
fi
if echo "$out" | grep -qE "AGENTS\.md|LINUS\.md"; then
  echo "pack leakage: AGENTS/LINUS in tarball"
  exit 1
fi
if echo "$out" | grep -qE "(^|[[:space:]])\.env"; then
  echo "pack leakage: .env in tarball"
  exit 1
fi
echo "pack ok — no tests/cre/AGENTS/LINUS/.env"
