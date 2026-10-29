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
parameters, atomic `typed_parameters`, separate `initial_parameters`, a named
initial template/mode, and execution limits. `ParameterValue` represents strings,
untyped atomic values, booleans, exact integer/decimal lexical values, floats,
doubles and the empty sequence. The same parameter fields are available for
streamed record entry. Supplying aliased duplicate names is an API error. API component
names use a local name or `Q{uri}local` (also for `StreamOptions::template` and
parameter keys); `initial_mode: Some("#unnamed".into())` selects the unnamed mode
when a stylesheet default is declared. Lexical prefixes require a stylesheet namespace scope and are
rejected at API boundaries. Stylesheet component names and XPath variable
references accept `Q{uri}local` and resolve prefixes at their declaring element; unprefixed names have
no namespace, regardless of the default XML namespace. Component names currently
use the documented ASCII name-character subset.

Supported so far:

- match templates, named templates, modes/current-mode, scoped default-mode,
  explicit/default priority, declared node-mode copy/skip/fail policies and
  on-multiple-match fail/use-last handling,
  global/template parameters and local variables, with namespace-qualified names;
- lexical scope, forward global references, circular-global detection;
- child/attribute/self/parent/descendant/ancestor/sibling/following/preceding
  axes, namespace wildcards, named node-kind/PI tests, predicates, function/parenthesized path steps, union/intersect/except node sets,
  sequences, budgeted exact integer ranges, focus-aware simple maps (`!`),
  lexical for/let bindings, some/every quantified expressions, lazy if/then/else, bounded nested XPath comments, string concatenation (`||`),
  instance-of/treat-as/cast/castable;
- exact integer/decimal arithmetic and integer division (`idiv`), integer subtype
  constructors/bounds/hierarchy, derived string/name types and whitespace facets,
  subtype-preserving min/max, numeric promotion, general/value comparisons,
  basic xs constructors, QName/anyURI atomic values and error(), atomization and string/node/sequence functions, including
  reverse/head/tail/unordered, index-of/distinct-values, exactly-one/zero-or-one/one-or-more, remove/insert-before, subsequence, numeric sum/avg/min/max, exact abs/floor/ceiling and precision-aware rounding,
  Unicode upper/lower case, translate/substring, codepoint conversion/comparison, Unicode normalization, URI escaping,
  codepoint string search, optional-separator string joins,
  node-name/namespace lookups, has-children/innermost/outermost, lang/generate-id,
  and iterative deep equality of supported atomic/untyped node sequences;
- URIQualifiedNames in component names, variables, node tests/wildcards, supported
  function calls and atomic sequence types;
- `apply-templates`, `call-template` and `next-match` with parameters (select or
  constructor bodies), plus `value-of`, `text`, `for-each`, `if`, `choose`;
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

XPath 3.1 and XSLT 3.0 additions beyond the core above:

- values and types: dates, times, durations and Gregorian fragments with arithmetic,
  time zones and formatting (`format-date`, `format-time`, `format-dateTime`,
  `format-integer`, `format-number` with `xsl:decimal-format`), hex/base64 binary,
  maps, arrays, function items (inline functions, `name#arity`, partial application,
  arrow operator), higher-order functions, `parse-json`, `math:` functions, UCA-style
  collations (approximate), regular expressions with Unicode blocks and
  backreferences;
- instructions: `xsl:function`, `xsl:key`, `xsl:for-each-group`, `xsl:iterate`,
  `xsl:try`/`xsl:catch`, `xsl:analyze-string`, `xsl:number`, `xsl:map`,
  `xsl:perform-sort`, `xsl:evaluate`, `xsl:where-populated`, `xsl:on-empty`/`xsl:on-non-empty`, `xsl:namespace`,
  `xsl:message` (collected by `transform_with_documents`), `xsl:result-document`,
  `xsl:attribute-set`, `xsl:import`/`xsl:apply-imports`, `use-when`, static
  variables and `expand-text`.

Not yet implemented: schema awareness, packages, accumulators, `xsl:merge`,
`xsl:fork`, `xsl:source-document`, other encodings than UTF-8/ISO-8859-1/US-ASCII, dynamic
`doc()` URIs, and the XSLT `streamable` feature.

Versions 1.0, 2.0 and 3.0 are accepted; complete version semantics are not yet
implemented. Version 1.0 currently enables first-item value-of/AVT conversion
and selected backwards-compatible XPath conversions. Integer and decimal literals/arithmetic now use arbitrary-precision values,
with a 10,000-digit implementation limit; exponent literals use IEEE doubles.
Non-terminating decimal division currently uses bigdecimal’s default precision
(100 digits). Unicode case conversion uses the Rust standard library’s Unicode tables.
Normalization supports NFC/NFD/NFKC/NFKD and the empty identity form using
unicode-normalization 0.1.25 (Unicode 17.0); FULLY-NORMALIZED remains unsupported.
Float/double strings preserve signed zero and use normalized exponent notation
outside the ordinary decimal range. Legacy value-of/AVT/string conversions and
text sort keys retain plain notation and legacy infinity/zero spellings.
Full numeric/atomic type coverage remains ahead. QName(uri, lexical-name) retains
prefixes and supports namespace-based equality, deep equality and xs:QName types.
QName construction accepts XML 1.0 Unicode names. Prefix/local-name accessors
return xs:NCName values with the correct string restriction hierarchy. anyURI values collapse XML
whitespace, promote to strings for function conversion/comparison, and retain
their distinct type for namespace-uri and namespace-uri-from-QName results. xs:QName
constructors/casts resolve prefixes in their declaring XPath namespace scope. error() reports FOER0000 for absent codes
and preserves explicit QName codes; returned error-object payloads remain ahead. Floats and doubles have distinct type identity and numeric-promotion rules.
Bindings support common atomic/node sequence types, named element/attribute tests,
document element constraints and occurrence indicators,
atomization and function conversion. Select expressions and temporary result-tree constructor bodies work. Untyped
constructors return document nodes; typed node constructors return their children
and apply function conversion. A sole typed xsl:sequence retains its atomic value.
Template result types apply function conversion with XTTE0505 diagnostics for
sole sequence/document returns and element-only constructors (including supported
control flow). Unconstrained item()* templates retain ordinary output behavior.
Sequence instructions, supported control flow and template delegation also
retain raw values across typed bindings and returns. Mixed sequences support
constructed elements, documents, text, comments and processing instructions;
constructed element contents are normalized independently. Parentless attribute constructors and copies preserve expanded names and values.
Zero-length constructed text nodes retain distinct identities and empty string
values until element/document content normalization discards them.
Raw apply-templates preserves explicit template results and built-in text/copy
results, including parentless element/document copies.
Selected node/attribute identity is preserved through raw sequence capture.
Raw xsl:copy-of allocates fresh node identities; xsl:copy constructs shallow
element/document results. Copied attributes have fresh identities and can be attached to constructed elements.
Match patterns support child/attribute paths, namespaces, wildcards, node tests,
predicates, `/`, `//`, parenthesized unions and `key()`/`id()` calls.

Mode declarations support node-based text-only-copy, shallow/deep-copy,
shallow/deep-skip and fail policies. Compatible declarations merge; conflicting
settings fail statically. Named private modes are internal entry points; unknown
initial modes fail with XTDE0045. Default and unnamed modes remain eligible.
Public/private/final mode visibility is supported; abstract modes and package
declared-mode enforcement remain ahead. Full streamability and atomic-item patterns remain ahead.

Unsupported constructs are rejected rather than ignored. Implementation gaps
use `RUST_XSLT_UNSUPPORTED`; syntax/name/function errors use XPath error codes.
Variable references and named-template calls are checked even in dead branches.
Template application arguments use the caller's focus/scope, including when
sorting changes the selected-node order. Modern built-in template rules forward
ordinary supplied parameters; version 1.0 built-ins use their original parameter
behavior. Explicit required global/template parameters are supported; missing named-call
arguments are checked statically. Parameters with nonempty required types and no default are implicitly required;
XSLT 2.0 reports XTDE0610 and 3.0 reports XTDE0700 when they are missing.
Tunnel parameters pass through undeclared intermediate templates and built-in
rules; ordinary and tunnel values remain separate. Initial tunnel values use
`initial_tunnel_parameters`.
No schema awareness or cancellation yet; `xsl:import` and `xsl:include` load modules through
`Stylesheet::compile_with_resolver`, and `TransformOptions::resolver` backs
`unparsed-text()`, `json-doc()` and literal-URI `doc()`. Record streaming is described below. Empty result elements use
explicit start/end tags. Literal results preserve in-scope non-XSLT namespaces except declared exclusions;
required element/attribute namespace bindings are always retained. `indent="yes"`,
`method="html"`/`"xhtml"` and DOCTYPE declarations are supported; non-UTF-8 output is not.

Parsing uses `roxmltree` with DTD processing disabled and a one-million-node
limit. No external resources are opened. Source trees and selected sequences
are buffered; execution limits are not a hard memory limit. Runtime diagnostic
locations and full W3C error-code fidelity remain ahead. Execution defaults to
128 levels; XPath nesting is capped at 128 even when execution depth is raised.
Template execution, result-content capture and binding constructors use an
explicit task stack. Per-run arenas own temporary documents and source strings;
node handles preserve parent/root navigation and identity across documents.
Typed constructed nodes are parentless, and their backing parser wrapper/siblings
are hidden. `root()` returns the actual root; absolute paths require a document
root. Parentless nodes and document nodes participate in supported match patterns.
`Limits::max_temporary_bytes` caps retained serialized temporary-tree source bytes
(default 64 MiB), with fresh arenas per streamed record. Parsed-node storage and
other allocations are additional, so this is not a hard memory bound.
Namespace-axis/raw namespace-node support remains ahead; serialized XML 1.0
cannot represent all XDM namespace inheritance boundaries.
Computed QName validation currently accepts letters, digits, underscore, hyphen
and dot, rather than the full XML name-character tables. Sort settings are
may be attribute value templates and may name any supported collation; locale-specific tailoring remains ahead. Compilation limits nested
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

Static checks reject absolute paths, `root()`, parent navigation, `last()`, source-dependent
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

A pinned, partial catalog runner and an 808-case passing regression gate are in
[tools/conformance](tools/conformance/README.md). Selected-case reports distinguish
engine failures, missing harness support and optional-profile exclusions. These
are development checks, not a conformance claim. An initial internal QT3 bridge
also gates more than 18,000 passing XPath cases at the pinned XPath suite revision.
The example JSON-lines driver
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
