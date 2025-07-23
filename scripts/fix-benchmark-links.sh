#!/usr/bin/env bash

set -e

SRC="target/criterion"
DEST="$SRC/benchmarks"
FILE="target/criterion/report/index.html"

mkdir -p "$DEST"

for dir in "$SRC"/*/; do
  name=$(basename "$dir")
  if [ "$name" != "report" ] && [ "$name" != "benchmarks" ]; then
    echo "Moving $name → benchmarks/"
    mv "$dir" "$DEST/"
  fi
done

echo "Benchmark folders moved to: $DEST"

sed -i.bak -E 's|href="\.\./([^"]+)"|href="/benchmarks/\1"|g' "$FILE"
cp "$FILE" "$DEST/index.html"

echo "Updated links in $FILE (backup saved as $FILE.bak)"
