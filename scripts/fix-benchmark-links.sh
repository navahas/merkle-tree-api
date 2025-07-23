#!/usr/bin/env bash
set -e

SRC="target/criterion"
DEST="criterion"
REPORT_SRC="$SRC/report/index.html"
REPORT_DEST="$DEST/index.html"

mkdir -p "$DEST"

echo "Copying benchmark results to ./$DEST"
cp -r "$SRC"/* "$DEST/"

# Rewrite links to assume everything is served from /benchmarks
# So: href="../add_leaf/..." → href="add_leaf/..."
sed -i.bak -E 's|href="\.\./([^"]+)"|href="\1"|g' "$DEST/report/index.html"

# Copy the patched index to top-level so it's served at /benchmarks/index.html
cp "$DEST/report/index.html" "$REPORT_DEST"

echo "Updated index.html and fixed links"
