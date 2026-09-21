#!/usr/bin/env bash
#
# Re-fetches the third-party diagram renderers that the HTML viewer embeds.
#
# The produced files are committed, so `cargo build` never touches the network.
# Run this only to bump a version.
#
# Set KEEP_FONTS=1 to retain Excalidraw's embedded handwriting fonts. They are
# 88% of that bundle (17 MB), so they are dropped by default and Excalidraw text
# falls back to a system sans-serif.

set -euo pipefail

MERMAID_VERSION="${MERMAID_VERSION:-12.0.0}"
EXCALIDRAW_VERSION="${EXCALIDRAW_VERSION:-0.1.5}"
DRAWIO_VIEWER_URL="${DRAWIO_VIEWER_URL:-https://viewer.diagrams.net/js/viewer.min.js}"

cd "$(dirname "$0")/.."
dest="templates/vendor"
mkdir -p "$dest"

echo "==> mermaid ${MERMAID_VERSION}"
curl -sSfL "https://cdn.jsdelivr.net/npm/mermaid@${MERMAID_VERSION}/dist/mermaid.min.js" \
    -o "$dest/mermaid.min.js"

echo "==> draw.io viewer"
curl -sSfL "$DRAWIO_VIEWER_URL" -o "$dest/drawio-viewer.min.js"

echo "==> @excalidraw/utils ${EXCALIDRAW_VERSION}"
curl -sSfL "https://cdn.jsdelivr.net/npm/@excalidraw/utils@${EXCALIDRAW_VERSION}/dist/prod/index.js" \
    -o "$dest/excalidraw-utils.raw.js"

# The package ships ESM. Rewrite its export list into a global so the bundle can
# be inlined verbatim, and optionally drop the embedded font payloads.
KEEP_FONTS="${KEEP_FONTS:-0}" python3 - "$dest/excalidraw-utils.raw.js" "$dest/excalidraw-utils.min.js" <<'PY'
import os, re, sys

src, out = sys.argv[1], sys.argv[2]
code = open(src, encoding="utf-8").read()
before = len(code)

if os.environ.get("KEEP_FONTS") != "1":
    code = re.sub(r"data:font/woff2;base64,[A-Za-z0-9+/=]+", "data:font/woff2;base64,AA==", code)

match = re.search(r"export\{([^}]*)\}", code)
if match is None:
    sys.exit("no export statement found - upstream bundle format changed")

pairs = []
for entry in match.group(1).split(","):
    parts = entry.strip().split()
    local, exported = parts[0], parts[-1]
    pairs.append(f'"{exported}":{local}')

shim = (
    "globalThis.ExcalidrawUtils={" + ",".join(pairs) + "};"
    "globalThis.dispatchEvent(new Event('adrscope:excalidraw-ready'));"
)
code = code.replace(match.group(0), shim)
open(out, "w", encoding="utf-8").write(code)
print(f"    {before/1048576:.2f} MB -> {len(code)/1048576:.2f} MB")
PY
rm -f "$dest/excalidraw-utils.raw.js"

# A bundle containing "<!--" followed by "<script" would put the HTML parser
# into an escaped state that swallows the closing tag of the inlined element.
for bundle in "$dest"/*.js; do
    if grep -q '<!--' "$bundle" && grep -q '<script' "$bundle"; then
        echo "error: $bundle mixes '<!--' and '<script' and cannot be inlined safely" >&2
        exit 1
    fi
done

echo
echo "Vendored:"
ls -l "$dest"/*.js | awk '{printf "  %-44s %6.2f MB\n", $NF, $5/1048576}'
