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
parameters, a named initial template/mode, and execution limits. API component
names use a local name or `Q{uri}local` (also for `StreamOptions::template` and
parameter keys); lexical prefixes require a stylesheet namespace scope and are
rejected at API boundaries. Stylesheet component names and XPath variable
references resolve prefixes at their declaring element; unprefixed names have
no namespace, regardless of the default XML namespace. Component names currently
use the documented ASCII name-character subset.

Supported so far:

- match templates, named templates, modes/current-mode, explicit/default priority,
  global/template parameters and local variables, with namespace-qualified names;
- lexical scope, forward global references, circular-global detection;
- child/attribute/self/parent/descendant/ancestor/sibling/following/preceding
  axes, namespace wildcards, named node-kind/PI tests, predicates, union,
  sequences, lazy if/then/else, instance-of/treat-as/cast/castable;
- exact integer/decimal arithmetic, numeric promotion, general/value comparisons,
  basic xs constructors, atomization and string/node/sequence functions;
- `apply-templates`, `call-template`, `value-of`, `text`, `for-each`, `if`, `choose`;
- namespace-aware literal results, constant attributes, AVTs and namespace fixup;
- selected/shallow `copy`, namespace-copy controls, deep `copy-of`, document
  construction, computed elements/attributes, comment/PI
  construction, and sequence-aware simple content with value/body separators;
- stable multi-key sorting for for-each/apply-templates with numeric/text keys,
  static ascending/descending order and case-order;
- source strip-space/preserve-space rules, inherited xml:space, and namespace
  exclusions resolved at the declaring scope;
- UTF-8 XML/text output, explicit indent="no", XML declaration omission, and
  shared execution budgets.

Versions 1.0, 2.0 and 3.0 are accepted; complete version semantics are not yet
implemented. Version 1.0 currently enables first-item value-of/AVT conversion
and selected backwards-compatible XPath conversions. Integer and decimal literals/arithmetic now use arbitrary-precision values,
with a 10,000-digit implementation limit; exponent literals use IEEE doubles.
Non-terminating decimal division currently uses bigdecimal’s default precision
(100 digits). Full numeric/atomic type coverage and canonical double formatting
remain ahead. Floats and doubles have distinct type identity and numeric-promotion rules.
Bindings support common atomic/node sequence types and occurrence indicators,
atomization and function conversion. Select expressions and temporary result-tree constructor bodies work. Untyped
constructors return document nodes; typed node constructors return their children
and apply function conversion. A sole typed xsl:sequence retains its atomic value.
Mixed typed top-level atomic/node constructor sequences, standalone attribute
sequences and zero-length constructed text-node identity remain incomplete.
Match patterns support child/attribute paths, namespaces, wildcards, node tests,
predicates, `/` and union. Descendant patterns remain ahead.

Unsupported constructs are rejected rather than ignored. Implementation gaps
use `RUST_XSLT_UNSUPPORTED`; syntax/name/function errors use XPath error codes.
Variable references and named-template calls are checked even in dead branches.
No imports, schema awareness,
resource loading, or cancellation yet. Record streaming is described below. Empty result elements use
explicit start/end tags. Literal results preserve in-scope non-XSLT namespaces except declared exclusions;
required element/attribute namespace bindings are always retained. XML indentation
and non-UTF-8 output remain unsupported.

Parsing uses `roxmltree` with DTD processing disabled and a one-million-node
limit. No external resources are opened. Source trees and selected sequences
are buffered; execution limits are not a hard memory limit. Runtime diagnostic
locations and full W3C error-code fidelity remain ahead. Execution defaults to
128 levels; XPath nesting is capped at 128 even when execution depth is raised.
Template execution, result-content capture and binding constructors use an
explicit task stack. Per-run arenas own temporary documents and source strings;
node handles preserve parent/root navigation and identity across documents.
`Limits::max_temporary_bytes` caps retained serialized temporary-tree source bytes
(default 64 MiB), with fresh arenas per streamed record. Parsed-node storage and
other allocations are additional, so this is not a hard memory bound.
Namespace-axis/raw namespace-node support remains ahead; serialized XML 1.0
cannot represent all XDM namespace inheritance boundaries.
Computed QName validation currently accepts letters, digits, underscore, hyphen
and dot, rather than the full XML name-character tables. Sort settings are
static; locale collations and sort-setting AVTs remain ahead. Compilation limits nested
instruction bodies and XPath parsing, and caps expression token counts.

Run `cargo test --offline` and `cargo clippy --offline --all-targets -- -D warnings`.
The lockfile pins the dependency set. Adapted Go fixtures retain attribution in
LICENSE and NOTICE. See [PROGRESS.md](PROGRESS.md) for delivered increments.

## Early record streaming

`Stylesheet::stream_records(input, &mut output, &StreamOptions)` incrementally
reads a UTF-8 XML 1.0 document, selects a fixed-depth path such as
`/catalog/book`, and invokes a named template with each book as context.
This follows the reference engine's grounded-record approach: hold one complete
record subtree, reuse the ordinary evaluator, write the result, then discard it.

```rust
use rust_xslt::{Stylesheet, StreamOptions};
use std::io::Cursor;
let sheet = Stylesheet::compile(r#"
<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3.0">
  <xsl:output method="text"/>
  <xsl:template name="record"><xsl:value-of select="title"/><xsl:text>;</xsl:text></xsl:template>
</xsl:stylesheet>"#)?;
let options = StreamOptions { record_path: "/catalog/book".into(), ..Default::default() };
let mut output = Vec::new();
let stats = sheet.stream_records(Cursor::new("<catalog><book><title>Rust</title></book></catalog>"), &mut output, &options)?;
assert_eq!(output, b"Rust;");
assert_eq!(stats.records, 1);
# Ok::<(), rust_xslt::Diagnostic>(())
```

This is an explicit record API, **not yet the XSLT `streamable` feature**.
It emits a sequence of record results (XML fragments for XML output); it adds
no outer wrapper. The XML declaration, when enabled, is written once. Use
`StreamOptions::namespaces` for prefixed record paths. Inherited input namespace
bindings are preserved when a record is detached.

Static checks reject absolute paths, parent navigation, `last()`, source-dependent
globals in the reachable record-template call graph, before
reading input or writing output. Rejection is conservative, including `last()`
in grounded inner loops. `position()` is the selected record's one-based index.
Record-local template dispatch supports single-step, predicate-free match patterns
(including identity transforms); more complex dispatch patterns are rejected.
There is no silent fallback to whole-document buffering.

The stream has explicit event-byte, record-byte and input-depth limits, plus
one execution/output budget across all records. Malformed ignored content is
still checked. The library retains the ancestor namespace/name stack, one XML
event, one record and one record's result; it does not accumulate prior records
or results. XML events use `quick-xml` 0.38.4; grounded records use `roxmltree`.
Output may be partial on parse, execution or I/O failure. Caller-owned input
and output buffers are outside the library's buffering limits.

An integration test generates 100 and 10,000 records without holding the input
or output, checks that reported peak record/event buffering stays constant,
and verifies that output begins before input is exhausted. These counters are
not a measurement of process RSS. General XSLT streamability analysis,
streamable modes/accumulators and automatic plan selection remain ahead.

## W3C development checks

A pinned, partial catalog runner and a 214-case passing regression gate are in
[tools/conformance](tools/conformance/README.md). Selected-case reports distinguish
engine failures, missing harness support and optional-profile exclusions. These
are development checks, not a conformance claim. The example JSON-lines driver
and its JSON dependency are development-only.

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
