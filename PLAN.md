# Rust XSLT engine port

## Scope and baseline

Build a native Rust library that ports the transformation engine in
`../go-xslt`, baseline commit `1dd67a33e3a1db1e43251378ef0deccf9a4a7f39`.
No CLI, desktop app, website, standalone XPath API, or standalone XSD API.
No Go runtime, subprocess engine, or C/JVM engine in the shipped library.
The Go implementation is a development reference, not a runtime dependency.

Target basic XSLT 3.0 with internal XPath 3.1 first, then streaming.
Support XSLT 1.0 compatibility as a distinct, tested capability. Defer
schema-aware XSLT to a separate optional milestone: excluding a public XSD
API does not remove the schema compiler and validator needed by
`xsl:import-schema`, typed nodes, and validation instructions.

The repository is currently empty. This document plans the port; it does not
claim any implemented functionality or measured Rust conformance.

## Size and source map

Production Go lines, excluding tests, from the inspected checkout:

| Source | Files | Lines | Rust responsibility |
| --- | ---: | ---: | --- |
| `internal/xmltree` | 10 | 6,672 | XML parsing, XDM trees, serialization |
| `internal/xpath` | 58 | 33,389 | Expressions, values, functions, patterns |
| `internal/xslt` | 48 | 33,832 | Compilation and transformation |
| `internal/xsd` | 24 | 16,840 | Deferred schema-aware support |

These are dependency inventories, not estimates of Rust code size. The first
three packages contain about 74,000 production lines before the public
facade and resource helpers. XPath is a substantial part of the engine and
cannot be removed just because it has no standalone public API.

Also inspect and selectively port `internal/engine`, `internal/fsaccess`,
`internal/fileuri`, and `internal/netfetch`. Exclude validation-only and
standalone XPath facade code.

## Architecture

Start with one library crate, `rust-xslt`, with private modules:

- `diagnostic`: structured W3C error QName, severity, phase, source URI and span.
- `resource`: URI resolution, caller-provided resource resolver, filesystem
  policy, document cache, optional network adapter.
- `xml`: tokenizer/parser, namespace bindings, source locations and tree arena.
- `xdm`: node handles, expanded QNames, atomic values, sequences, maps, arrays,
  functions and sequence types.
- `xpath`: lexer, parser, static context, evaluator, function library and patterns.
- `compile`: module loading, declarations, static checks and compiled instructions.
- `runtime`: focus, variable frames, template dispatch, keys, accumulators,
  shared execution budget and per-transformation caches.
- `serialize`: XML/HTML/XHTML/text/JSON/adaptive output and serialization settings.
- `stream`: streamability analysis and event-based execution, added later.

Use arena indices for nodes, with handles retaining an `Arc<Document>` for
ownership; node identity must include document identity. Preserve parent and
axis navigation, document order, namespace nodes, base URIs and locations.
Temporary result documents use the same ownership model. Avoid cyclic
`Rc<RefCell<Node>>` graphs and references borrowing short-lived parser buffers.

Use enums for AST, instructions and XDM values. Keep compiled stylesheets
immutable and `Send + Sync`, sharing them through `Arc`; every run owns its
mutable execution state. Store feature configuration explicitly with modules
and static contexts rather than reproducing Go's process-wide registries.
Use persistent/owned environment frames for closures and avoid function-value
ownership cycles. Prototype closure lifetime and reclamation before fixing
this representation.

Use an explicit execution stack or trampoline for recursive templates and
functions. Do not translate Go's 10,000-level recursion default into Rust
native stack recursion. Parsing and tree traversal need depth checks too.

Choose XML, numeric, Unicode, URI, encoding and regex dependencies through
small compatibility probes before committing to them. A generic regex engine
alone does not implement XPath regex semantics. Decimal/integer precision,
NaN, signed zero, collations and Unicode versions need deliberate policies.
Preserve behavior rather than Go data structures or file boundaries.

## Public contract

Expose compilation plus repeated transformations, with convenience one-shot
transformation. Separate `CompileOptions` from `TransformOptions`.

The API should cover:

- stylesheet/source text and reader inputs, explicit base URIs;
- named global parameters, initially strings and later typed XDM values;
- initial template and mode, with initial function support as required by XSLT;
- immutable compiled stylesheets reusable concurrently;
- principal output as bytes or a `std::io::Write` sink;
- secondary outputs returned to the caller, plus messages and diagnostics;
- per-compile feature settings and per-run resource policy;
- step, depth and aggregate output-byte limits, cancellation and deadlines.

Return typed errors for failures; retain messages and useful diagnostics on
failure. Serialization may produce non-UTF-8 bytes. Writer calls may leave
partial output on failure; document this explicitly. A reader input API alone
does not imply bounded-memory streaming.

Default to no network and no filesystem access without an explicit allowed
base/directory. Check resolved paths, symlinks, URI encodings and redirects.
Apply policy to imports, includes, entities, doc/document, collections,
unparsed text, source-document and nested transformations. `result-document`
must never write arbitrary files. Nested `fn:transform` and `xsl:evaluate`
inherit policy and consume the same budget. Check limits during allocation
and output, rather than only after constructing a large result.

## Delivery milestones

### 0. Freeze the reference and build the test bridge

Record the source commit, dirty-tree status, Apache-2.0 license and attribution.
Inventory behavior from source/tests rather than treating README percentages
as a verified baseline; local conformance documentation has differing counts.
Pin the W3C suite revisions using the source conformance script.

Create a test-only Go driver supporting compilation and transformation through
a structured request/response format. Keep it outside the shipped Rust API.
Run fixtures against both engines, comparing outputs, secondary documents,
messages and structured error codes. Use W3C expected assertions as the
conformance authority; do not copy a Go bug merely to match it.

Exit: reproducible baseline report and a small fixture corpus covering success,
static error, dynamic error, imports and multiple outputs.

### 1. XML, XDM, diagnostics and resource policy

Implement parsing, node identity and order, namespaces, entities, URI handling,
source spans, basic atomic values/sequences and XML/text serialization.
Port targeted parser and security fixtures. Establish resolver and budget
interfaces now, so later features cannot bypass them.

Exit: namespace/axis-ready trees, deterministic diagnostics, malformed/deep XML
rejected safely, entity expansion bounded, and denied-resource tests passing.

### 2. First complete transformation path

Implement the XPath subset needed for paths, predicates, variables, literals,
comparisons, arithmetic and common string/node functions. Add stylesheet
compilation, match patterns, built-in templates, template priority, literal
result elements, apply-templates, call-template, value-of, text, copy,
copy-of, variables/parameters, if/choose and for-each.

Exit: identity transform, parameterized report and recursive templates pass
against expected results and the Go reference. This is a useful subset,
explicitly not an XSLT 1.0 or 3.0 conformance claim.

### 3. Practical XSLT 1.0 behavior

Complete axes/patterns, import precedence, include/import, modes, sorting,
keys, numbering, whitespace rules, namespace fixup, AVTs and XML/HTML/text
output. Implement version-dependent XPath conversion and XSLT backwards
compatibility deliberately, including result tree fragment semantics.

Exit: documented XSLT 1.0 feature matrix, differential regressions and relevant
W3C cases passing, including error-code checks and serialization edge cases.

### 4. XPath 3.1 and XSLT 2.0 foundations

Complete sequences/types, numeric and date/time semantics, casts, regex,
collations, function library, grouping, stylesheet functions, temporary trees,
tunnel parameters, next-match/apply-imports, multiple result documents and
serialization options. Then add maps, arrays, higher-order functions and JSON.
Use QT3 internally to validate the XPath required by XSLT; this does not add a
public XPath product.

Exit: explicit per-feature QT3/XSLT reports with no silent assertion skips.

### 5. Basic XSLT 3.0

Implement packages and component visibility/overrides, static parameters and
use-when, iterate/break/next-iteration, try/catch, assertions/messages,
merge, non-streamed accumulators, evaluate, source-document, fn:transform,
initial entry variants and remaining basic-processor requirements.

Exit: pinned basic XSLT 3.0 suite run; every failure/unsupported dependency
named and explained, strict expected error codes, and no regression in the
passing set. Concurrency and nested-budget tests pass.

### 6. Streaming

Port streamability analysis alongside the event executor, then streamed
accumulators, grouping, fork and merge behavior. Integrate incremental output
where semantics permit it. Distinguish non-streamed fallback from guaranteed
streamable execution and report capability properties accurately.

Exit: applicable streaming suite cases pass and increasing-input benchmarks
show bounded memory for declared streaming workloads. Reader/writer APIs
alone do not satisfy this gate.

### 7. Optional schema-aware XSLT

If required, port the necessary `internal/xsd` implementation behind a Cargo
feature. Add schema loading/imports, type annotations, schema-aware matching,
validation modes and schema component bindings into XPath. Expose no standalone
validation API. Keep non-schema builds free of this implementation.

Exit: schema-aware XSLT cases and focused XSD regressions pass; capability
claims and disabled-feature diagnostics match the enabled build.

## Validation and release gates

Port meaningful existing Go fixtures alongside their implementation slices.
Build the XSLT catalog runner early and expand it rather than waiting until
feature completion. Support catalog environments, parameters, initial entries,
secondary results and all assertion kinds; an unimplemented assertion is a
harness failure. Structural XML comparison and exact serialization comparison
must remain separate. Track pass/fail/unsupported counts and case names by
feature, with exact reasons and suite revisions.

Run formatting, lint and unit/integration checks in CI. Add parser/evaluator
fuzzing, hostile-input and filesystem-policy tests, concurrent stylesheet
reuse, cancellation during long XPath work, and writer failure propagation.
Keep full conformance runs separate from quick checks, and fail release runs
if required suite checkouts are missing.

Benchmark cold compile, warm compiled execution, serialization, peak memory
and genuinely streamed execution separately. Use representative Go benchmark
fixtures; do not promise a speedup before measurement.

Release the first subset with an honest support matrix. Claim basic XSLT 3.0,
streaming or schema awareness only after their own gates pass. Retain Apache
license notices for ported source and follow the test suites' separate terms.

## Recommended first implementation batch

1. Create the library crate, license/notice files and diagnostic types.
2. Pin the Go/suite baselines and implement the test-only reference bridge.
3. Prototype arena node handles, closure ownership and iterative execution.
4. Implement XML parsing plus minimal XPath and template dispatch.
5. Deliver the milestone-2 end-to-end fixture set before broadening features.

A calendar estimate should follow that first batch: the major risks are XPath
semantics, Rust ownership of XDM/functions, serialization fidelity and streaming,
not the number of public API methods.
