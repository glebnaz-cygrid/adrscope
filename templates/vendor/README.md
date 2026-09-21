# Vendored diagram renderers

These bundles are embedded verbatim into the generated HTML viewer so that it
stays a single self-contained file that works from `file://` with no network.

Refresh them with `scripts/vendor-diagrams.sh` — never edit them by hand.

| File | Upstream | Version | License |
| --- | --- | --- | --- |
| `mermaid.min.js` | [mermaid-js/mermaid](https://github.com/mermaid-js/mermaid) | 12.0.0 | MIT |
| `drawio-viewer.min.js` | [jgraph/drawio](https://github.com/jgraph/drawio) | `viewer.diagrams.net` build | Apache-2.0 |
| `excalidraw-utils.min.js` | [`@excalidraw/utils`](https://www.npmjs.com/package/@excalidraw/utils) | 0.1.5 | MIT |
| `drawio-prelude.js` | ADRScope | — | MIT |

## Local modifications

`excalidraw-utils.min.js` is the upstream ESM bundle with two changes applied by
the vendoring script:

1. Its trailing `export{...}` is rewritten into `globalThis.ExcalidrawUtils`,
   because the viewer inlines the bundle rather than importing it.
2. The 231 embedded WOFF2 payloads (17 MB, 88% of the bundle) are replaced with
   empty stubs, so Excalidraw text renders in a system sans-serif instead of the
   Excalifont handwriting face. Re-run the script with `KEEP_FONTS=1` to keep
   them at the cost of ~17 MB per generated viewer.

## Attribution

The draw.io viewer is Apache-2.0 licensed; see `drawio-viewer.NOTICE`.
