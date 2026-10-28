# Implementation progress

The numbered milestones in PLAN.md describe capability targets, not completed
conformance claims. Smaller tested increments are committed independently.

## Increment 1: library bootstrap

- Immutable compiled templates, literal results, basic paths, built-in rules.
- Structured diagnostics, explicit execution stack and limits.
- Nine integration tests; format, test and Clippy checks pass.
- Reference Go driver written, not executed: Go is absent from PATH.

## Increment 2: XPath and control-flow foundations

- Dedicated XPath AST/parser/evaluator, predicates, selected axes, namespaces,
  arithmetic, comparisons, functions, sequences and union.
- Shared XPath/instruction budget, lexical variables, global parameters,
  template calls/defaults, named initial entry, for-each/if/choose and AVTs.
- Forward globals, circular dependency diagnostics and static lexical checks.
- 23 integration tests pass, including adapted Go fixtures and scope/focus tests.
- Format and Clippy checks pass. No full suite or Go differential run yet.

## Increment 3: early record streaming

- Added quick-xml pull events and a public fixed-depth record-streaming API.
- Named templates run against one grounded record at a time using the ordinary
  evaluator; inherited namespaces survive detachment and output is incremental.
- Static safety checks include called templates and globals; there is no hidden
  whole-document fallback. Full XSLT streamability is not yet implemented.
- Event/record/depth limits, shared run budgets, ignored-content validation,
  I/O failure handling and observable buffering statistics.
- 34 integration tests pass. Generated 100/10,000-record checks report constant
  peak buffering and output before EOF. No process RSS measurement yet.
- Format and Clippy pass; reference Go execution remains unavailable.

## Increment 4: identity transforms and namespace-aware results

- XPath-backed union/wildcard/namespaced/positional match patterns, correct
  branch priorities, explicit priorities, modes and current-mode propagation.
- Shallow/deep copy, comment/PI copying, pending result start tags and namespace
  fixup (including default undeclarations and conflicting copied prefixes).
- Streamed record-local template dispatch; unsafe match shapes are rejected.
- 45 integration tests pass; format and Clippy checks pass.

## Increment 5: computed nodes and stable sorting

- Computed element/attribute QNames, namespace resolution and dynamic errors;
  body/select content, value separators and sanitized comment/PI construction.
- Explicit-stack string-content capture and shared output/execution limits.
- Stable, budgeted multi-key mergesort; original sort-key focus and sorted
  iteration focus, numeric NaN ordering and static text case-order.
- Streaming checks cover computed-name/content expressions and all sort keys.
- 57 integration tests pass; format and Clippy pass.

## Increment 6: pinned W3C test bridge

- Development JSON-lines driver and partial catalog runner, exact error codes,
  structural XML/string assertions, named environments and explicit status reports.
- Passing-case regression gate records 43 cases at the pinned suite revision.
- First choose/copy selection: 203 cases, 43 pass, 92 engine failures,
  52 harness errors and 16 optional-profile exclusions. This is not a conformance
  percentage. All 43 passing cases pass the committed gate.
- Named initial templates can run without source; source-dependent expressions
  correctly diagnose absent focus rather than seeing a fabricated source tree.
- 58 Rust integration tests, seven runner tests, format and Clippy pass.

## Increment 7: namespace exclusions and whitespace declarations

- Namespace exclusions resolve against each declaring scope; #all is a snapshot,
  and required result-name bindings survive exclusion.
- Source strip/preserve rules with specificity and inherited xml:space; text,
  CDATA and character references are stripped as one logical text node.
- Streamed records inherit xml:space for stripping without injected attributes.
- Explicit UTF-8, XML 1.0 and indent=no output settings are accepted; other
  encodings and indentation remain explicit gaps.
- 67 Rust integration tests, seven harness tests and Clippy/format pass.
- The same 203 W3C cases now have 57 passes, 78 engine failures, 52 harness
  errors and 16 exclusions. All 43 previous passes remain; gate expanded to 57.

## Increment 8: reverse axes, node tests and conditional expressions

- Ancestor, sibling, following and preceding axes; reverse-axis predicate focus
  is distinct from document-order normalization and grouped filters.
- Namespace/local wildcards, named kind tests, PI targets and PI string values.
- Lazy if/then/else expressions; lexical checks still inspect both branches.
- Streaming checks explicitly reject all new nonlocal axes and traverse
  conditionals. Wildcard match priorities follow their specificity.
- 73 Rust integration tests and Clippy/format pass; the selected W3C cases
  improve to 64 pass / 71 engine failures / 52 harness errors / 16 exclusions.
- Previous 57 passing cases remain green; gate expanded to 64.

## Increment 9: XPath catalog assertions

- Catalog parser preserves namespace scopes; supported XPath assertions run via
  a development stylesheet with the principal XML document bound as $result.
- Unsupported assertion expressions remain harness errors, and false assertions
  remain engine failures. No public standalone XPath API was added.
- Ten harness unit tests pass, including namespace shadowing and failed
  assertion-evaluator classification.
- Same 203 cases: 66 passes, 100 engine failures, 21 harness errors, 16 exclusions.
  The additional engine failures were previously blocked by harness support.
- All 64 previous passes remain green; passing gate expanded to 66.

## Increment 10: exact numeric values and constructors

- Arbitrary-precision integer/decimal literals and arithmetic, double promotion,
  numeric digit limits, lexical-cast errors and integer/decimal zero-division.
- Initial xs:string/untypedAtomic/boolean/integer/decimal/double/float constructors
  resolve by namespace URI; node/data atomization preserves untyped values.
- General versus singleton value comparisons, exact numeric comparisons, empty
  arithmetic and selected XPath 1.0 conversions.
- Float constructor rounds to f32 but distinct float type identity remains ahead;
  nonterminating decimal division uses the dependency's 100-digit default.
- 82 Rust integration tests and Clippy/format pass. Selected W3C cases improve to
  73 pass / 93 engine failures / 21 harness errors / 16 exclusions.
- All 66 previous passes remain; gate expanded to 73.

## Increment 11: sequence types and type operators

- Common atomic/node sequence types, occurrence indicators and binding checks;
  function conversion atomizes nodes, casts untyped values and promotes numbers.
- Supplied template parameter mismatches are distinguished from binding errors.
- Distinct float/double values and promotion, instance-of/treat-as/cast/castable.
- Corrected type/union/unary operator precedence against the reference grammar.
- 91 Rust integration tests and Clippy/format pass. Selected W3C cases improve to
  77 pass / 89 engine failures / 21 harness errors / 16 exclusions.
- All 73 previous passes remain; gate expanded to 77.

## Increment 12: owned temporary trees and constructor bodies

- Node handles preserve identity/document order across documents and expose
  fragment wrappers as document nodes without leaking synthetic elements.
- Per-run typed arenas own temporary XML strings/documents and drop them together;
  max_temporary_bytes bounds retained serialized sources (not total RSS).
- Global/local/default/argument constructor bodies execute on the task stack,
  with lexical scope, forward globals, typed children and untyped document values.
- xsl:sequence output and a sole typed sequence constructor preserve the supported
  value semantics; mixed typed top-level sequences remain explicitly unsupported.
- Node comparisons and correct string atomization of comments/PIs.
- 104 Rust integration tests and Clippy/format pass. Streaming arenas reset each
  record; a 1,000-record constructor test succeeds under a 100-byte source budget.
- Selected W3C cases: 90 pass / 76 engine failures / 21 harness errors / 16 exclusions.
  All 77 previous passes remain; gate expanded to 90.

## Increment 13: document and copy controls

- xsl:copy select handles zero/singleton values, replacement focus, ignored atomic
  bodies and XTTE3180; implicit copy rejects absent focus.
- copy-namespaces controls retain bindings required by element/attribute names;
  inherit-namespaces controls result construction and required default resets.
- xsl:document uses per-run document ownership; typed sole-document constructors
  return document nodes, nested documents flatten, document attributes fail.
- Empty namespace URIs are normalized across parser boundaries, XPath and stream
  record selection. New selectors are included in streaming safety checks.
- 113 Rust integration tests and Clippy/format pass. Selected W3C cases:
  101 pass / 63 engine failures / 23 harness errors / 16 exclusions.
  All 90 previous passes remain; two new harness errors became reachable.

Next: sequence-aware simple-content construction, then qualified entry names.
