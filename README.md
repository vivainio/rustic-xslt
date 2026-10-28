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

The public surface includes `Stylesheet::compile`, `transform`,
`transform_with_options`, `transform_with_limits`, a convenience
`rust_xslt::transform`, and structured `Diagnostic` errors. Compiled stylesheets
are immutable and `Send + Sync`. `TransformOptions` accepts string global
parameters, a named initial template, and execution limits.

Supported so far:

- match templates, named templates, global/template parameters, local variables;
- lexical scope, forward global references, circular-global detection;
- child/attribute/self/parent/descendant axes, namespaced XPath name tests,
  wildcards, node/text/comment/PI tests, predicates, union and sequences;
- arithmetic, comparisons, boolean operators, basic string/node/sequence functions;
- `apply-templates`, `call-template`, `value-of`, `text`, `for-each`, `if`, `choose`;
- unnamespaced literal result elements, constant attributes and AVTs;
- XML/text output, XML declaration omission, and shared execution budgets.

Versions 1.0, 2.0 and 3.0 are accepted; complete version semantics are not yet
implemented. Version 1.0 currently enables first-item value-of/AVT conversion
and selected backwards-compatible XPath conversions. Numbers currently use
`f64`; exact decimal/integer types and full atomic type checking remain ahead.
Bindings with content currently support text only, not temporary result trees.
Match patterns currently support unprefixed element-name paths and `/` only.

Unsupported constructs are rejected rather than ignored. Implementation gaps
use `RUST_XSLT_UNSUPPORTED`; syntax/name/function errors use XPath error codes.
Variable references and named-template calls are checked even in dead branches.
No imports, copy instructions, result namespace fixup, streaming, schema
awareness, resource loading, or cancellation yet. Empty result elements use
explicit start/end tags. Unused namespace declarations are omitted.

Parsing uses `roxmltree` with DTD processing disabled and a one-million-node
limit. No external resources are opened. Source trees and selected sequences
are buffered; execution limits are not a hard memory limit. Runtime diagnostic
locations and full W3C error-code fidelity remain ahead. Execution defaults to
128 levels; XPath nesting is capped at 128 even when execution depth is raised.
Template execution uses an explicit task stack. Compilation limits nested
instruction bodies and XPath parsing, and caps expression token counts.

Run `cargo test --offline` and `cargo clippy --offline --all-targets -- -D warnings`.
The lockfile pins the dependency set. Adapted Go fixtures retain attribution in
LICENSE and NOTICE. See [PROGRESS.md](PROGRESS.md) for delivered increments.

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
