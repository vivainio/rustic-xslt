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

## Increment 14: sequence-aware simple content

- Simple-content capture merges adjacent text nodes while preserving boundaries
  of atomized elements/documents/comments/PIs/attributes and atomic values.
- Attribute body separators (including AVTs) are supported; default attribute
  body separator remains empty, comment/PI separator remains a space.
- Value-of/AVT/select simple content also merges adjacent real text nodes.
- Adjacent atomic result values normalize across instruction/template boundaries;
  separator expansion is checked before allocating terminal joined results.
- 122 Rust integration tests and Clippy/format pass. Both targeted W3C comment/PI
  cases now pass: 103 pass / 61 engine failures / 23 harness errors / 16 exclusions.
- All 101 previous passes remain; gate expanded to 103.

## Increment 15: qualified component names

- Template, mode, variable and parameter names resolve prefixes in the declaring
  element's scope; aliases share identity and rebound prefixes remain distinct.
- XPath variable references, forward globals, lexical checks, template arguments
  and duplicate detection use expanded names, without a default namespace.
- Initial template/mode, external global parameters and streamed record entries
  accept local names or `Q{uri}local`; API lexical prefixes are rejected explicitly.
- 131 Rust integration tests, ten harness tests and Clippy/format pass. All 103
  W3C gate cases remain green (103 pass / 61 fail / 23 harness errors / 16 exclusions).

## Increment 16: qualified catalog setup and expanded regression gate

- Catalog initial template/mode and global parameter names resolve namespace
  prefixes at each setup element, then use the API's expanded-name notation.
- Missing prefixes, invalid names and aliased duplicate catalog parameters are
  harness errors; string-only and initial-parameter limitations remain explicit.
- Four new harness tests cover aliases, shadowing, default namespace independence
  and URIQualifiedNames. All 14 harness tests pass; 131 Rust tests remain green.
- Surveyed nine related W3C sets: 575 cases, 214 pass / 219 engine failures /
  97 harness errors / 45 exclusions. All 103 previous passes remain; the gate
  now records all 214 passing cases at the unchanged suite revision.

## Increment 17: URIQualifiedNames

- `Q{uri}local` component names share identity with lexical QName aliases.
- XPath accepts URIQualifiedNames in variable references, name tests, supported
  function calls and atomic sequence types; namespace wildcards include `Q{}*`.
- Explicitly unnamespaced function names do not acquire the default function
  namespace. Malformed braced names fail statically without lexer panics.
- 137 Rust integration tests, 14 harness tests and Clippy/format pass. W3C selection:
  215 pass / 218 engine failures / 97 harness errors / 45 exclusions; the 214-case
  gate remains green and expands to 215.

## Increment 18: template application parameters

- `apply-templates` accepts ordinary `with-param` bindings after sort keys;
  expanded-name duplicate checks and lexical validation also inspect dead code.
- Argument expressions and constructor bodies execute on the task stack in the
  caller's focus/scope, then values are shared across selected template dispatch.
- Supplied-parameter type checks, per-template defaults and modern built-in-rule
  forwarding are preserved; explicit nested applications do not leak arguments.
  Version 1.0 built-in rules retain their no-parameter behavior.
- Record streaming traverses argument expressions/constructors before consuming
  input; safe record-local arguments work with fresh per-record arenas.
- 146 Rust integration tests, 14 harness tests and Clippy/format pass. Ten W3C
  sets: 625 cases, 245 pass / 224 engine failures / 110 harness errors / 46 exclusions.
  All 215 previous passes remain; gate expanded to 245 cases.

## Increment 19: required parameters and binding diagnostics

- Explicit required global/template parameters report XTDE0050/XTDE0700 when
  absent. Named calls missing required arguments report XTSE0690 statically,
  including dead branches; undeclared arguments report XTSE0680 outside 1.0 mode.
- Required parameters reject defaults and invalid booleans; boolean attributes
  now strip surrounding whitespace. Disallowed required attributes are XTSE0090.
- Parameter default/supplied type errors remain distinct through constructor
  capture, with standard duplicate global/template/parameter diagnostics.
- 154 Rust integration tests, 14 harness tests and Clippy/format pass. W3C:
  262 pass / 206 engine failures / 111 harness errors / 46 exclusions. One newly
  reachable assertion lacks harness support; all 245 previous passes remain.
  Gate expanded to 262.

## Increment 20: reserved component namespaces

- Declared templates, modes, variables and parameters reject reserved namespaces
  with XTSE0080, while xsl:initial-template retains its standard exception.
- The implicit xml prefix resolves for stylesheet component checks and XPath
  source-name tests without an explicit xmlns:xml declaration.
- 157 Rust integration tests and Clippy/format pass. W3C: 263 pass / 205 engine
  failures / 111 harness errors / 46 exclusions; all 262 prior passes remain.
  Gate expanded to 263.

## Increment 21: atomic and initial-entry parameter API

- Added ParameterValue for strings, untyped values, booleans, exact integers and
  decimals, floats/doubles and empty sequences. Existing string globals remain
  compatible; typed_parameters and initial_parameters are separate maps.
- Named/matched initial templates and streamed record entries accept ordinary
  parameters independently of globals; aliased duplicate API names are errors.
- Parameter conversion consumes the shared budget and numeric lexical limits.
  Nodes and general parameter sequences remain outside this initial atomic API.
- The development driver/catalog supports simple atomic setup literals and
  ordinary initial parameters; complex expressions/static/tunnel setup remain
  explicit harness gaps.
- 164 Rust integration tests, 17 harness tests and Clippy/format pass. The 263-case
  W3C gate remains green; selected counts are unchanged.

## Increment 22: implicitly required parameters

- Typed parameter declarations without an explicit default become implicitly
  required when their type excludes the empty sequence. Missing values report
  XTDE0610 in 2.0 and XTDE0700 in 3.0, including stylesheet parameters.
- Explicit empty defaults still undergo default-value type checking; optional
  empty types retain their valid defaults. Implicit requirements remain dynamic
  for named calls, distinct from explicit required=yes static checks.
- 167 Rust integration tests and Clippy/format pass; all 263 W3C gate cases remain
  green with unchanged selected-case counts.

## Increment 23: tunnel parameters

- Tunnel bindings flow through named/matched templates and built-in rules using
  owned shared frames; with-param overrides are scoped to their called branch.
- Ordinary parameters do not intercept tunnel values. Receiver defaults and
  type conversions remain local; explicit required tunnel parameters are dynamic.
- Added initial_tunnel_parameters to tree/record APIs and the development
  protocol; the catalog keeps ordinary and tunnel initial bindings separate.
- 176 Rust integration tests, 18 harness tests and Clippy/format pass. Eleven W3C
  sets: 683 cases, 295 pass / 210 engine failures / 132 harness errors / 46 exclusions.
  All 263 previous gate cases remain; gate expands to 295.
- Downloaded the complete QT3 suite at 201a6e466940cdfc727f4babfedcde5332b9f578
  for subsequent offline XPath work. Full pinned XSLT suite is also cached.

## Increment 24: URIQualifiedNames inside AVTs

- AVT scanning recognizes braced URI literals independently of string literals,
  so qualified variable/function references work in literal attributes, computed
  names and separator expressions. Escaped literal braces retain their meaning.
- Invalid or unmatched AVT delimiters now report XTSE0350.
- 179 Rust integration tests and Clippy/format pass. W3C: 296 pass / 209 engine
  failures / 132 harness errors / 46 exclusions. All 295 previous passes remain;
  gate expands to 296.

## Increment 25: scoped default modes

- Standard default-mode attributes resolve at their own declaring element and
  apply to template/apply-templates defaults, including literal result scopes.
- The principal default controls initial match dispatch; #unnamed explicitly
  selects the unnamed mode in stylesheets and the API. #current remains dynamic.
- Empty/duplicate expanded-name mode lists and invalid special tokens are checked
  statically, preserving unbound-prefix diagnostics.
- 184 Rust integration tests and Clippy/format pass. W3C: 316 pass / 189 engine
  failures / 132 harness errors / 46 exclusions. All 296 previous passes remain;
  gate expands to 316.

## Increment 26: mode declarations and built-in node policies

- Declared modes support text-only-copy, shallow/deep-copy, shallow/deep-skip and
  fail policies on nodes; document deep-skip still processes document children.
- Shallow policies dispatch attributes and children with separate focus lists;
  copies retain namespace fixup and share execution/output limits.
- Compatible explicit mode settings merge; conflicts are XTSE0545. Highest
  priority ambiguities can fail with XTDE0540; overlapping branches of one
  template do not count as distinct competing rules.
- True streamability and accumulator requirements remain explicit gaps.
- 191 Rust integration tests and Clippy/format pass. W3C: 327 pass / 177 engine
  failures / 133 harness errors / 46 exclusions. All 316 previous passes remain;
  gate expands to 327.

## Increment 27: initial-mode eligibility and visibility

- Initial named modes must exist as explicit declarations or template rule modes;
  #all does not create arbitrary eligible initial names. Checks use expanded names.
- Public/private/final visibility merges with mode declarations. Named private
  modes remain callable internally but are ineligible entries, with the standard
  exceptions for default and unnamed modes. Invalid unnamed visibility is static.
- Package declared-mode enforcement and abstract mode components remain deferred
  with package support rather than being applied to ordinary stylesheet modules.
- 195 Rust integration tests and Clippy/format pass. W3C: 330 pass / 174 engine
  failures / 133 harness errors / 46 exclusions. All 327 previous passes remain;
  gate expands to 330.

## Increment 28: next-match

- Ranked dispatch tracks the current rule through named calls; next-match selects
  lower priority or earlier declarations in the same mode, with fresh ordinary
  arguments and inherited tunnel values. Exhaustion uses the mode's built-in rule.
- Union alternatives remain separate continuation candidates unless an explicit
  priority unifies them; they still do not trigger multiple-template ambiguity.
- for-each and copy with explicit select clear the current rule. Missing-rule
  calls report XTDE0560. Static and streaming checks inspect argument bodies.
- A 600-rule continuation test uses the explicit task stack and verifies the
  shared execution budget. Imports/apply-imports remain unsupported.
- 204 Rust integration tests and Clippy/format pass. Twelve W3C sets: 723 cases,
  356 pass / 177 engine failures / 144 harness errors / 46 exclusions. All 330
  previous passes remain; gate expands to 356.

## Increment 29: XPath ranges and simple maps

- Exact integer range bounds support untyped atomization, optional empty values,
  descending empty ranges and arbitrary precision without double conversion.
- Range cardinality is checked against the remaining shared execution budget
  before allocation; generated items and simple-map iterations consume that budget.
- Simple maps retain order/duplicates and set position/last against their input
  sequence; empty inputs skip their right operand. Dot now denotes any context
  item rather than implicitly requiring a node self-axis.
- Precedence and non-associative range grammar are checked explicitly.
- 212 Rust integration tests and Clippy/format pass. W3C: 357 pass / 176 engine
  failures / 144 harness errors / 46 exclusions. All 356 previous passes remain;
  gate expands to 357.

## Increment 30: internal QT3 development bridge

- Added a pinned, development-only QT3 runner that evaluates expressions through
  XSLT. Absent/local XML context, named environments and prefixes are supported.
- Scalar/type/count/string/empty assertions, atomic deep equality, composites
  and exact errors are checked without silently accepting unsupported assertions.
  Unsupported setups/assertions remain harness errors; query and assertion scopes
  are separate, and evaluator failures do not become engine passes.
- Seven additional harness tests plus six live positive/negative controls pass;
  25 harness tests total. The 212 Rust tests and 357-case XSLT gate remain green.
- Selected op-to/op-bang: 182 cases, 111 pass / 69 engine failures / two XQuery-only
  exclusions; all 14 simple-map cases pass. Recorded a separate 111-case QT3 gate.
- Node deep equality, general setup/parameters and nondeterministic-query delivery
  remain explicit harness gaps. No standalone XPath runtime API was introduced.

## Increment 31: XPath sequence functions

- Added reverse/head/tail, remove/insert-before and subsequence with atomic/node
  identity preservation and shared execution-budget charges.
- Exact integer insertion/removal positions avoid machine-integer overflow;
  subsequence uses XPath half-toward-positive-infinity rounding and IEEE NaN/
  infinity endpoint rules. Function conversion rejects wrong argument types.
- 218 Rust integration tests, 25 harness tests and Clippy/format pass. All 357
  XSLT gate cases remain green.
- Expanded QT3 selection to eight sets: 467 cases, 310 pass / 144 engine failures /
  four harness errors / nine profile exclusions. All 111 previous passes remain;
  QT3 gate expanded to 310.

## Increment 32: numeric aggregate functions

- Added avg/min/max and the optional sum zero argument. Numeric aggregates apply
  common integer/decimal/float/double promotion, preserve arbitrary precision
  where applicable, and handle NaN and empty sequences explicitly.
- Untyped inputs atomize and cast to double; incomparable types report FORG0006.
  Min/max also compare strings with codepoint collation and booleans; unsupported
  collations report FOCH0002. Duration/date coverage remains ahead.
- 225 Rust integration tests, 25 harness tests and Clippy/format pass. XSLT gate
  expands to 358 (358 pass / 175 fail / 144 harness errors / 46 exclusions).
- Twelve QT3 sets: 1,343 cases, 716 pass / 575 engine failures / four harness errors /
  48 profile exclusions. All 310 previous passes remain; QT3 gate expands to 716.

## Increment 33: XPath comments and QName normalization

- Nested XPath comments are scanned iteratively with a 128-level limit, including
  comment-aware AVT delimiters. Strings/braced URIs retain literal comment markers.
- Version 3.0 AVTs accept empty/comment-only expressions; earlier versions keep
  their nonempty grammar. The lexer ignores XML whitespace only.
- Braced URI literals collapse XML whitespace in names and wildcards, including
  external and catalog names. XPath forbids the xmlns URI with XQST0070 while
  stylesheet declaration checks retain XTSE0080.
- 233 Rust integration tests, 26 harness tests and Clippy/format pass. All 358
  XSLT gate cases remain green.
- Fourteen QT3 sets: 1,437 cases, 753 pass / 587 engine failures / ten harness errors /
  87 profile exclusions. All 716 previous passes remain; QT3 gate expands to 753.

## Increment 34: XPath for/let expressions

- Lexical for/let bindings support sequential comma-separated bindings, QName
  variables, shadowing and ordered Cartesian iteration without changing focus.
- Empty for inputs skip their bodies; let binds whole sequences. Static checks
  reject forward/out-of-scope references even in unexecuted branches.
- Assignment/numeric token boundaries are checked; lowered multi-binding depth
  counts toward the 128-level XPath bound. Streaming checks traverse both sides.
- 242 Rust integration tests and Clippy/format pass. XSLT gate expands to 362:
  362 pass / 171 engine failures / 144 harness errors / 46 exclusions.
- Sixteen QT3 sets: 1,715 cases, 857 pass / 577 engine failures / 14 harness errors /
  267 profile exclusions. All 753 previous passes remain; QT3 gate expands to 857.

## Increment 35: deep sequence equality

- Added iterative deep-equal for supported atomics/untyped nodes, including NaN,
  expanded names, unordered attributes and ordered children. Comment/PI nodes
  compare directly but are excluded from document/element child comparisons.
- Node/atomic distinctions and codepoint collation validation remain explicit;
  traversal consumes the shared execution budget without native tree recursion.
- QT3 deep assertions now use the native function, including node sequences.
  Baseline commands infer selected sets while retaining zero-pass survey sets.
- 249 Rust integration tests, 27 harness tests and Clippy/format pass. XSLT gate
  expands to 363 (363 pass / 170 failures / 144 harness errors / 46 exclusions).
- Seventeen QT3 sets: 1,978 cases, 959 pass / 666 engine failures / 19 harness errors /
  334 profile exclusions. All 857 previous passes remain; QT3 gate expands to 959.

## Increment 36: declared template result types

- Template as attributes validate sole sequence/document returns and element-only
  constructor bodies (including supported control flow), reporting XTTE0505.
- Direct sequences retain atomic types for function conversion. Sole documents
  retain document type through capture; unconstrained item()* needs no conversion.
- Other constrained bodies remain explicit raw-collection gaps, avoiding silent
  type erasure or adjacent-text merging before required-type checks.
- Tree, initial-entry and record-template invocation share the same conversion
  path and temporary-tree/output budgets.
- 256 Rust integration tests, 27 harness tests and Clippy/format pass. XSLT gate
  expands to 368 (368 pass / 165 failures / 144 harness errors / 46 exclusions).
  All 959 QT3 gate cases remain green with unchanged selected-case counts.

## Increment 37: named node sequence types

- Added element/attribute QName constraints and document-node(element(...))
  constraints, including wildcards, prefix aliases and occurrence indicators.
- Binding/default/supplied/template return checks and instance/treat operators
  share the same expanded-name matching. Document constraints enforce a single
  matching element child, excluding text and allowing comments/PIs.
- Sequence type syntax preserves token boundaries; node schema annotations remain
  an explicit implementation gap rather than being silently discarded.
- 262 Rust integration tests and Clippy/format pass. All 368 XSLT gate cases remain.
- Eighteen QT3 sets: 1,999 cases, 980 pass / 666 engine failures / 19 harness errors /
  334 profile exclusions. All 21 selected sequence-type cases pass; previous 959
  passes remain and the QT3 gate expands to 980.

Next: raw sequence preservation across bindings and template calls, followed by
constructed-node collection and broader XPath grammar coverage.
