#!/usr/bin/env bash
set -euo pipefail
# Fail if CRE vendored lib drifts from src source.
# Strips the "// keep in sync with src/*.ts" header before diff.
strip_header() { sed '1{/^\/\/ keep in sync/d;}'; }

ok=1
for pair in "src/isStale.ts cre/lib/isStale.ts" "src/quote.ts cre/lib/quote.ts"; do
  set -- $pair
  src=$1; dst=$2
  if ! diff -u <(strip_header < "$src") <(strip_header < "$dst") >/dev/null; then
    echo "drift: $src != $dst (strip header then diff)"
    diff -u <(strip_header < "$src") <(strip_header < "$dst") || true
    ok=0
  fi
done

if [ "$ok" -eq 0 ]; then
  echo "CRE sync failed — update cre/lib/*.ts to match src/*.ts (keep header)"
  exit 1
fi
echo "CRE sync ok"
