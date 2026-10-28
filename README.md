# rust-xslt

Native Rust port of the [go-xslt](https://github.com/tim-riep/go-xslt) engine.
Early implementation: **not yet an XSLT-conformant processor**. See [PLAN.md](PLAN.md).

```rust
use rust_xslt::Stylesheet;

let sheet = Stylesheet::compile(r#"
<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3.0">
  <xsl:output omit-xml-declaration="yes"/>
  <xsl:template match="/catalog">
    <titles><xsl:apply-templates select="book"/></titles>
  </xsl:template>
  <xsl:template match="book"><title><xsl:value-of select="title"/></title></xsl:template>
</xsl:stylesheet>"#)?;
let output = sheet.transform("<catalog><book><title>Rust</title></book></catalog>")?;
assert_eq!(output, "<titles><title>Rust</title></titles>");
# Ok::<(), rust_xslt::Diagnostic>(())
```

The public surface currently consists of `Stylesheet::compile`, `transform`,
`transform_with_limits`, a convenience `rust_xslt::transform`, and structured
`Diagnostic` errors. Compiled stylesheets are immutable and `Send + Sync`.

Supported in this first slice:

- `xsl:stylesheet` / `xsl:transform`, match templates and built-in template rules;
- unprefixed element-name paths, absolute paths, `.`, and `node()` selections;
- `xsl:apply-templates`, `xsl:value-of`, `xsl:text`;
- unnamespaced literal result elements and constant unnamespaced attributes;
- XML or text output, and XML declaration omission;
- step, execution-depth, and serialized-output-byte limits.

Versions 1.0, 2.0 and 3.0 are accepted as headers only; their complete semantics
are not implemented. Unsupported expressions, instructions and attributes fail
at compilation with the implementation-specific code `RUST_XSLT_UNSUPPORTED`.
No imports, parameters, predicates, copy instructions, namespace result fixup,
streaming, schema awareness, resource loading or cancellation yet. Empty result
elements use explicit start/end tags. Unused namespace declarations are omitted.

Parsing uses `roxmltree` with DTD processing disabled and a one-million-node
limit. No external resources are opened. Source trees and selected sequences
are buffered in memory; execution limits are not a hard memory limit. Runtime
diagnostic source locations and complete W3C error-code fidelity remain future
work. The default execution depth is conservatively 128; template dispatch uses
an explicit task stack. Compilation limits literal-result nesting to 128.

Run `cargo test --offline` and `cargo clippy --offline --all-targets -- -D warnings`.
The lockfile pins the initial dependency set. The catalog fixture is adapted
from the Go engine tests; license and attribution are in LICENSE and NOTICE.

## Go reference driver (development only)

`tools/go-reference` reads JSON objects from stdin and returns one JSON response
per request. It uses the sibling `../go-xslt` checkout, whose recorded baseline
is `1dd67a33e3a1db1e43251378ef0deccf9a4a7f39` (clean when inspected).
It does not enforce that pin: check the checkout before differential runs.

```sh
cd tools/go-reference
go run -mod=mod . < requests.jsonl
```

Requests use the Go engine request fields (`stylesheet`, `source`, `params`,
`baseDir`, etc.) plus `operation`: `compile` or `transform` (default).
Transform responses preserve the engine's output, messages, secondary outputs
and diagnostics. Compilation returns diagnostics. The driver requires the Go
version/dependencies declared by the reference checkout; it is not a Rust runtime
dependency. Full automated differential and W3C catalog runners are not built yet.

Reference suite pins from the baseline script: XSLT `fddf1cf9`, QT3 `201a6e46`.
The Go driver has not been compiled here because `go` is not installed/on PATH.
