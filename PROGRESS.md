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

## Increment 38: raw sequence preservation

- Typed binding/template collectors retain atomic types and selected node/
  attribute identity across sequence instructions, control flow and template calls.
- Nested raw/serialized captures restore their output modes independently;
  conversion runs after collection with binding/parameter/template-specific codes.
- Raw atomics need no serialized temporary-tree bytes. Record collectors reset
  per record, and final output still consumes the shared byte/step budgets.
- Constructed-node and built-in node-result paths that cannot yet participate in
  raw collection fail explicitly; they do not silently stringify their values.
- 272 Rust integration tests, 27 harness tests and Clippy/format pass. XSLT gate
  expands to 373 (373 pass / 160 failures / 144 harness errors / 46 exclusions).
  All 980 QT3 gate cases remain green with unchanged selected-case counts.

## Increment 39: parentless constructed-node boundaries

- Typed constructed roots now have parentless XDM boundaries while retaining
  backing-document ownership. Parent/ancestor/sibling/following axes cannot
  expose parser wrappers or unrelated constructed roots.
- Added root(); absolute paths require a document-rooted tree. Streamed record
  checks reject root navigation before consuming input.
- Parentless name/kind patterns use singleton predicate focus; document-node()
  patterns match actual document roots without broadening node() to documents.
- 279 Rust integration tests, 27 harness tests and Clippy/format pass. All 373
  XSLT gate cases remain green.
- Nineteen QT3 sets: 2,037 cases, 989 pass / 667 engine failures / 21 harness errors /
  360 profile exclusions. All 980 previous passes remain; QT3 gate expands to 989.

## Increment 40: mixed constructed-node sequences

- Raw typed bindings and template results collect constructed elements, documents,
  text, comments and processing instructions alongside atomic/selected values.
- Adjacent constructed text nodes retain separate identities before conversion;
  nested element contents normalize independently of their containing sequence.
- Untyped constructor delegation can atomize into typed template returns. Mixed
  return cardinality errors now use XTTE0505 rather than rejecting the shape.
- 282 Rust integration tests, 27 harness tests and Clippy/format pass. Both pinned
  gates retain all 373 XSLT and 989 QT3 passes without regressions.

## Increment 41: raw copy results

- xsl:copy-of retains atomic types and creates fresh parentless copies of node
  results, including distinct document nodes and repeated selected nodes.
- xsl:copy grounds element/document bodies independently within raw sequences;
  atomic copies retain type identity. Parentless attribute copies fail explicitly.
- 284 Rust integration tests and Clippy/format pass. XSLT gate expands to 375
  (375 pass / 158 failures / 144 harness errors / 46 exclusions); all 989 QT3
  passes remain green.

## Increment 42: raw template dispatch

- Typed collectors accept apply-templates and preserve explicit template results.
- Built-in text-only-copy creates text nodes; shallow/deep-copy preserve node
  kinds and parentless result boundaries, grounding nested content independently.
- 286 Rust integration tests, 27 harness tests and Clippy/format pass. Both pinned
  gates retain all 375 XSLT and 989 QT3 passes without regressions.

## Increment 43: parentless attributes

- Typed collectors retain standalone constructed/copied attributes, including
  named attribute types, template returns and built-in shallow/deep copy rules.
- Attribute roots retain their identity and expose no synthetic parent or axes;
  attributes attached to parentless constructed elements retain their real parent.
- Copied attributes get fresh identities and attach normally to result elements.
- 290 Rust integration tests, 27 harness tests and Clippy/format pass. XSLT gate
  expands to 376 (376 pass / 157 failures / 144 harness errors / 46 exclusions).
  All 989 QT3 gate cases remain green.

## Increment 44: zero-length text-node identity

- Raw text/value constructors retain zero-length nodes with distinct identities,
  empty string values and parentless navigation, backed by budgeted arena nodes.
- Raw copy/copy-of preserves empty text nodes with fresh identities; insertion
  into element/document content still discards empty text during normalization.
- 292 Rust integration tests, 27 harness tests and Clippy/format pass. Both pinned
  gates retain all 376 XSLT and 989 QT3 passes without regressions.

## Increment 45: XPath quantified expressions

- some/every supports dependent multiple bindings, nested quantifiers, QName
  variables, lexical shadowing and unchanged caller focus.
- Effective boolean values short-circuit; empty domains use existential/universal
  identities. Static scope, parser depth and streaming safety inspect all bodies.
- Iteration consumes the shared budget. 300 Rust integration tests, 27 harness
  tests and Clippy/format pass; all 376 XSLT and 989 prior QT3 passes remain.
- Twenty QT3 sets: 2,240 cases, 1,132 pass / 685 engine failures / 21 harness errors /
  402 profile exclusions. Quantified expressions add 143 passes to the gate.

## Increment 46: integer division

- XPath idiv returns exact arbitrary-precision integers, truncating toward zero;
  decimal operands divide scaled integers without intermediate decimal rounding.
- Float/double promotion, empty operands, untyped node atomization, divide-by-zero,
  invalid non-finite operands and overflow preserve the required types/error codes.
- Numeric digit/scale limits apply to integer division. 304 Rust integration tests,
  27 harness tests and Clippy/format pass; all previous regression cases remain.
- Twenty-one QT3 sets: 2,376 cases, 1,229 pass / 713 engine failures / 21 harness
  errors / 413 profile exclusions. Gate grows by 97; XSLT gate expands to 378
  (378 pass / 155 failures / 144 harness errors / 46 exclusions).

## Increment 47: XPath node set operators

- union aliases |; intersect/except bind more tightly and associate left-to-right.
- Node operands retain identity, remove duplicates and return document order;
  atomic operands report XPTY0004, including alongside empty operands.
- Membership scans consume the shared budget. Constructed elements/attributes
  participate without value-based equality. 310 Rust integration tests, 27 harness
  tests and Clippy/format pass; all 378 XSLT and previous QT3 passes remain.
- Twenty-four QT3 sets: 2,605 cases, 1,262 pass / 728 engine failures / 40 harness
  errors / 575 profile exclusions. Gate expands by 33 node-set cases.

## Increment 48: cardinality assertion functions

- exactly-one, zero-or-one and one-or-more preserve atomic types, node identity
  and order, and report FORG0005/FORG0003/FORG0004 for invalid cardinalities.
- Arity is checked statically; item copying consumes the shared budget.
- 312 Rust integration tests, 27 harness tests and Clippy/format pass. All 378
  XSLT passes remain; previous QT3 cases gain four passes.
- Twenty-seven QT3 sets: 2,766 cases, 1,318 pass / 824 engine failures / 40 harness
  errors / 584 profile exclusions. Gate adds 52 cardinality-function cases.

## Increment 49: general XPath path operands

- Paths accept parenthesized expressions, variable/function/literal steps and
  descendant abbreviations, while axis-only paths retain pattern support.
- Each step receives the input item/position/size. Node results sort/deduplicate;
  atomic final results preserve order. Mixed results report XPTY0018 and atomic
  intermediate operands XPTY0019. Static path depth and run budgets remain bounded.
- Lexical and streaming checks traverse both operands. 320 Rust integration tests,
  27 harness tests and Clippy/format pass; all previous gate cases remain green.
- XSLT gate expands to 379 (379 pass / 154 failures / 144 harness errors / 46
  exclusions). Twenty-eight QT3 sets: 2,794 cases, 1,348 pass / 812 engine failures /
  41 harness errors / 593 profile exclusions. QT3 gate gains 30 cases.

## Increment 50: XPath string concatenation

- || atomizes optional singleton operands into one xs:string, with empty inputs
  contributing empty strings and multi-item inputs reporting XPTY0004.
- Precedence lies between comparisons and ranges, retaining the existing type,
  arithmetic, set and map relationships. String copying consumes the shared budget.
- 324 Rust integration tests, 27 harness tests and Clippy/format pass. XSLT gate
  expands to 380 (380 pass / 153 failures / 144 harness errors / 46 exclusions).
- Twenty-nine QT3 sets: 2,886 cases, 1,384 pass / 867 engine failures / 42 harness
  errors / 593 profile exclusions. Previous passes remain; QT3 gate grows by 36.

## Increment 51: Unicode string functions

- upper-case/lower-case use full locale-independent Unicode mapping, including
  expansions; translate uses first-match codepoints and removes unmapped suffixes.
- substring uses Unicode scalar positions, XPath rounding, NaN/infinity rules,
  strict modern function conversion and selected backwards-compatible conversions.
- Traversals consume the shared budget. 331 Rust integration tests, 27 harness
  tests and Clippy/format pass; all prior gate cases remain green.
- XSLT gate expands to 381 (381 pass / 150 failures / 146 harness errors / 46
  exclusions); new assertion reachability exposes two further harness gaps.
- Thirty-three QT3 sets: 3,035 cases, 1,522 pass / 876 engine failures / 44 harness
  errors / 593 profile exclusions. All 48 selected substring cases pass.

## Increment 52: Unicode codepoint conversion

- string-to-codepoints returns exact integer Unicode scalar values; the reverse
  conversion accepts integer sequences and untyped node atomization.
- Invalid XML 1.0 characters (including zero, surrogates and out-of-range values)
  report FOCH0001; invalid argument types report XPTY0004. Traversals are budgeted.
- 334 Rust integration tests, 27 harness tests and Clippy/format pass. All 381
  XSLT gate cases remain green; prior QT3 cases gain nine passes.
- Thirty-five QT3 sets: 3,158 cases, 1,639 pass / 868 engine failures / 52 harness
  errors / 599 profile exclusions. Gate adds 108 codepoint-function cases;
  all 44 selected string-to-codepoints cases pass. A million-item codepoint range
  remains a named execution-budget failure under the conformance driver limits.

## Increment 53: conditional operand grammar

- Unparenthesized if expressions are rejected in arithmetic, logical, comparison,
  concatenation, range and unary operands, matching ExprSingle placement rules.
- Parenthesized forms, nested else conditions, function arguments, sequence items
  and binding bodies remain valid. Contextual keyword element names still work.
- 337 Rust integration tests, 27 harness tests and Clippy/format pass. All 381
  XSLT and previous QT3 gate cases remain green.
- Thirty-six QT3 sets: 3,200 cases, 1,657 pass / 879 engine failures / 52 harness
  errors / 612 profile exclusions. Conditional expressions add 18 gate cases.

## Increment 54: codepoint string comparison

- compare returns integer -1/0/1 using Unicode codepoint order; codepoint-equal
  compares strings without implicit normalization. Empty arguments return ().
- Modern string conversion, static arity checks and explicit codepoint collation
  validation preserve XPTY0004/FOCH0002 diagnostics. Comparison scans are budgeted.
- 339 Rust integration tests, 27 harness tests and Clippy/format pass. All 381
  XSLT gate cases and prior QT3 cases remain green.
- Thirty-eight QT3 sets: 3,332 cases, 1,736 pass / 914 engine failures / 64 harness
  errors / 618 profile exclusions. Gate adds 79 string-comparison cases.

## Increment 55: Unicode normalization

- normalize-unicode supports default NFC, NFD, NFKC, NFKD and empty identity forms;
  normalization names trim XML whitespace and ignore ASCII case.
- Pinned pure Rust unicode-normalization 0.1.25 (Unicode 17.0), available offline.
  Canonical ordering, composition/decomposition and compatibility expansion retain
  scalar correctness; input/output traversals consume the shared execution budget.
- Unsupported forms report FOCH0003; FULLY-NORMALIZED remains an optional gap.
- 343 Rust integration tests, 27 harness tests and Clippy/format pass. All 381
  XSLT and previous QT3 gate cases remain green.
- Thirty-nine QT3 sets: 3,380 cases, 1,771 pass / 917 engine failures / 71 harness
  errors / 621 profile exclusions. Gate grows by 35 normalization-related cases.

## Increment 56: absolute value, floor and ceiling

- abs/floor/ceiling preserve integer, decimal, float and double result types;
  integer/decimal operations remain exact beyond IEEE integer precision.
- Empty modern arguments, untyped node conversion and legacy numeric conversion
  follow the existing numeric rules; floating signs, infinities and NaN survive.
- 346 Rust integration tests, 27 harness tests and Clippy/format pass. All 381
  XSLT and previous QT3 gate cases remain green.
- Forty-two QT3 sets: 3,750 cases, 1,917 pass / 1,129 engine failures / 71 harness
  errors / 633 profile exclusions. Gate grows by 146 unary-numeric cases.
- Remaining selected floor/ceiling assertion failures include the documented
  floating-point lexical formatting gap, alongside unsupported numeric subtypes.

## Increment 57: floating-point string forms

- Float/double strings retain signed zero, normalized E notation and shortest
  round-tripping mantissas, using decimal form only in the specified magnitude range.
- Legacy value-of, AVT, simple-content, atomic output, string/concat and text-sort
  conversions retain plain notation, zero and Infinity/-Infinity spellings.
- Modern casts remain canonical independently of legacy serialization boundaries.
- 350 Rust integration tests, 27 harness tests and Clippy/format pass. All 381
  XSLT gate cases remain green; QT3 gate expands by 33 to 1,950.
- Same 3,750 QT3 cases: 1,950 pass / 1,096 engine failures / 71 harness errors /
  633 profile exclusions. All selected floor/ceiling formatting assertions pass.

## Increment 58: default-code error function

- error() and error((), ...) report FOER0000 without requiring focus; descriptions
  retain caller text. Arguments evaluate normally, including supplied error objects.
- Lazy branches, logical short-circuits and empty iteration domains skip errors;
  static arity, code and description type checks remain enforced.
- Custom QName codes and returned error-object payloads remain explicit gaps.
- 354 Rust integration tests, 27 harness tests and Clippy/format pass. All 381
  XSLT passes remain; prior QT3 cases gain thirteen passes.
- Forty-three QT3 sets: 3,848 cases, 2,005 pass / 1,132 engine failures / 71 harness
  errors / 640 profile exclusions. Gate adds 42 selected error-function cases.

## Increment 59: QName atomic values and custom errors

- QName(uri, lexical-name) retains prefixes and expanded identity, with full
  XML 1.0 Unicode name validation and FOCA0002 lexical/namespace diagnostics.
- xs:QName sequence types, raw typed bindings/template results/copies, equality
  and deep equality preserve atomic values without conflating strings/prefixes.
- error() preserves explicit QName codes, using short standard error names and
  Q{uri}local for custom namespaces (including Q{} for absent namespaces).
- Static-context QName constructors/casts remain explicit implementation gaps.
- 360 Rust integration tests, 27 harness tests and Clippy/format pass. All 381
  XSLT passes remain; forty-five QT3 sets: 3,920 cases, 2,113 pass / 1,085 engine
  failures / 71 harness errors / 651 profile exclusions. Gate grows by 108.
- Public repository configured as https://github.com/vivainio/rustic-xslt;
  crate name remains rust-xslt.

## Increment 60: static-context QName conversion

- xs:QName constructors and cast/castable operators capture declaring namespace
  bindings, resolve lexical prefixes, trim XML whitespace and preserve QName inputs.
- Unbound prefixes report FONS0004; invalid lexical values FORG0001; wrong atomic
  types/cardinalities XPTY0004. Castability converts cast failures to false without
  swallowing resource errors. Optional casts retain empty sequence behavior.
- Static lexical/streaming walkers traverse cast operands. Source namespaces and
  XML default result namespaces do not replace the declaring XPath prefix scope.
- 363 Rust integration tests, 27 harness tests and Clippy/format pass. All 381 XSLT
  passes remain; QT3 gate grows by four to 2,117. Same 3,920 cases: 2,117 pass /
  1,081 engine failures / 71 harness errors / 651 profile exclusions.
- All 25 selected fn:QName cases pass, with optional/XQuery profiles excluded.

## Increment 61: URI atomic values

- xs:anyURI constructor/casts and sequence types preserve distinct atomic values,
  collapse XML whitespace and support promotion to strings for function arguments.
- Comparison, deep equality, min/max, string functions and effective boolean values
  handle URI values with their required string semantics; forbidden casts fail.
- namespace-uri and namespace-uri-from-QName return URI values. QName-sensitive
  function conversion reports XPTY0117 for untyped inputs under modern XPath.
- 368 Rust integration tests, 27 harness tests and Clippy/format pass. All 381 XSLT
  passes remain; fifty QT3 sets: 4,007 cases, 2,219 pass / 1,045 engine failures /
  72 harness errors / 671 profile exclusions. Gate grows by 102.
- All selected anyURI constructor/comparison cases pass. A legacy XPath namespace
  conversion error-code distinction remains a named version-semantics gap.

## Increment 62: precision-aware numeric rounding

- round uses ties toward positive infinity; round-half-to-even uses even ties.
  Both accept optional integer precision and preserve numeric result types.
- Integer/decimal rounding remains exact. Floating inputs round their exact binary
  values via decimal representations, preserving signed zero and non-finite values.
- Extreme precisions short-circuit to unchanged/zero values without allocating
  unbounded powers of ten; numeric digit limits still apply.
- 372 Rust integration tests, 27 harness tests and Clippy/format pass. XSLT gate
  expands to 382 (382 pass / 149 failures / 146 harness errors / 46 exclusions);
  all previous QT3 gate cases remain green.
- Fifty-two QT3 sets: 4,398 cases, 2,529 pass / 1,126 engine failures / 72 harness
  errors / 671 profile exclusions. Rounding adds 310 gate cases; remaining failures
  primarily require numeric subtypes, plus an older-XPath arity distinction.

## Increment 63: integer restriction types

- Added signed byte/short/int/long, unsigned byte/short/int/long, nonpositive,
  negative, nonnegative and positive integer types with exact bounds and hierarchy.
- Constructors/casts preserve subtype identity and distinguish lexical/facet
  FORG0001 errors from numeric-cast FOCA0002 bounds errors.
- Sequence types, function conversion, raw bindings/template results, arithmetic,
  aggregates, ranges, positions, codepoints and rounding accept integer subtypes.
  Arithmetic can produce unrestricted integers beyond the source subtype bounds.
- 378 Rust integration tests, 27 harness tests and Clippy/format pass. All 382
  XSLT gate cases remain green; QT3 gate grows by 853 to 3,382.
- Same 4,398 QT3 cases: 3,382 pass / 273 engine failures / 72 harness errors /
  671 profile exclusions. All selected floor, ceiling, round-half-to-even and
  integer-divide applicable cases pass; older-XPath round arity remains named.

## Increment 64: subtype-preserving extrema

- min/max retain selected integer restriction and decimal/integer values when
  no primitive promotion is needed; float/double mixtures still promote.
- Homogeneous URI extrema retain anyURI type; URI/string mixtures promote to
  strings. Equal extrema may retain either selected input type as permitted.
- 380 Rust integration tests, 27 harness tests and Clippy/format pass. All 382
  XSLT cases remain green; QT3 gate grows by two to 3,384.
- Same 4,398 QT3 cases: 3,384 pass / 271 engine failures / 72 harness errors /
  671 profile exclusions. No previous passes regress.

## Increment 65: string restriction types and QName accessors

- Added normalizedString, token, language, Name, NCName, NMTOKEN, ID, IDREF and
  ENTITY atomic constructors/casts/types with XML whitespace, language and Unicode
  name facets. Subtype substitution preserves identity through typed capture.
- Primitive string operations, URI/QName casts, EBV, deep equality and min/max
  handle derived strings without conflating them with untyped atomic values.
- prefix-from-QName and local-name-from-QName return NCName restriction values;
  optional inputs and namespace-sensitive argument conversion remain precise.
- 386 Rust integration tests, 27 harness tests and Clippy/format pass. All 382
  XSLT gate cases remain green; fifty-five QT3 sets: 4,460 cases, 3,427 pass /
  272 engine failures / 73 harness errors / 688 profile exclusions. Gate adds 43.

## Increment 66: node names and namespace lookup

- Added node-name() returning a QName with the node's expanded name and an
  in-scope prefix when one is available; absent input and unnamed nodes return
  the empty sequence.
- Added namespace-uri-for-prefix(), in-scope-prefixes() and resolve-QName()
  for element nodes, including namespace-sensitive unbound-prefix errors.
- Corrected function registration, optional QName resolution, string argument
  conversion, lexical validation and the implicit xml prefix.
- Namespace lookup handles the implicit xml binding and excludes undeclared
  empty namespace bindings; regression checks cover invalid names and types.
- Full offline Rust tests, formatting and Clippy pass. Full conformance counts
  are not refreshed.



## Increment 67: QT3 permutation assertions and namespace gate

- Added unordered sequence assertions using deep equality with duplicate
  multiplicities, rather than membership-only comparison. Empty and mismatched
  sequences were checked through the development driver.
- Surveyed four namespace function sets and expanded the QT3 gate to 3,458
  passing cases with no regressions among the previous 3,427 cases.
- Selection: 4,626 cases, 3,458 pass / 272 engine failures / 78 harness errors /
  818 profile exclusions at the unchanged pinned revision.
- Remaining namespace failures name older-XPath node-name arity and DTD-supplied
  namespace declarations. XML 1.1 dependency handling remains a harness gap.
- All 27 harness unit tests pass. The prior full Rust checks remain green.



## Increment 68: atomic sequence lookup

- Added index-of() and distinct-values() with atomization, numeric promotion,
  string/untyped/URI equivalence and expanded QName equality. Incomparable types
  remain distinct; NaN collapses only for distinct-values.
- Selected atomic subtypes survive duplicate removal. Optional codepoint
  collations and index-of search cardinality are validated even for empty input.
- Shared deep-equality atomic comparison avoids inconsistent equality rules;
  duplicate scans consume the shared execution budget.
- The two QT3 sets contribute 138 passes; remaining failures require temporal
  or binary atomic types.

## Increment 69: node sequence selection

- Added has-children(), innermost(), outermost() and unordered(). Node extrema
  remove duplicates and return document order while respecting detached roots.
- Ancestry checks include attribute owners and consume shared work budgets.
- Four QT3 sets contribute 69 passes; many remaining cases require source-variable
  environments, function items or namespace-axis support in the harness/engine.

## Increment 70: node languages and generated identifiers

- Added lang() with nearest inherited xml:lang, ASCII case folding and sublanguage
  matching, plus generate-id() for supported node identities.
- Identifier strings distinguish documents, elements and expanded attribute names;
  empty inputs return the empty string.
- Record streaming rejects both functions before I/O: inherited languages and
  identities across records require additional streaming support.
- Two QT3 sets contribute 42 passes; a remaining identifier assertion needs regex.

## Increment 71: string search and joins

- String search functions accept optional codepoint collations, enforce function
  conversion and charge string traversal to the shared budget.
- string-join() accepts its one-argument form and converts supported atomic values
  while validating singleton separators.
- Six QT3 sets contribute 206 passes. Other collations, temporal values and older
  XPath arity/type rules remain explicit failures.

## Increment 72: URI escaping

- Added encode-for-uri(), iri-to-uri() and escape-html-uri() using uppercase
  percent escapes of UTF-8 bytes and their distinct ASCII preservation sets.
- Optional strings, function conversion and shared work budgets are retained.
- Three QT3 sets contribute 106 passes; the three remaining failures require
  current-time().
- Combined selection: 5,517 cases, 4,024 pass / 383 engine failures / 235 harness
  errors / 875 profile exclusions. All prior passing cases remain green.
- All 399 Rust tests, 27 harness tests, formatting and Clippy pass.

## Increment 73: trace, nilled and default collation

- Added trace() (value passthrough with validated label), nilled() (false for
  elements, empty otherwise) and default-collation() (codepoint collation URI).
- Regression tests cover cardinality, type errors and empty inputs.
- QT3 at the pinned revision (clone at /tmp/rustic-qt3): 5,750 cases, 4,256 pass /
  368 engine failures / 229 harness errors / 897 profile exclusions. No regressions
  against the previous 4,245-case gate; the gate now records 4,256.

## Increment 74: ASCII case-insensitive and case-blind collations

- Added a shared collation module with the codepoint, html-ascii-case-insensitive
  and QT3 case-blind collations; unknown URIs still raise FOCH0002 (UCA collations
  remain unsupported).
- contains, starts-with, ends-with, substring-before/after and compare honour the
  collation argument; folded keys map one-to-one to source characters so
  substring results preserve original text.
- QT3: 5,750 cases, 4,268 pass / 356 engine failures / 229 harness errors / 897
  exclusions; the gate records 4,268 with no regressions.

## Increment 75: date, time and duration values

- Added xs:dateTime, xs:date, xs:time, xs:duration, xs:yearMonthDuration and
  xs:dayTimeDuration as atomic values with strict lexical validation (leap years,
  24:00:00, time zone bounds), canonical strings, nanosecond fractions and casts
  between related types (dateTime to date/time, date to dateTime, duration
  subtypes).
- Value comparison normalizes time zones using UTC as the implicit time zone;
  durations compare by months and nanoseconds, xs:duration ordering is a type
  error, and mixed temporal families are XPTY0004. deep-equal and distinct-values
  use the same equality.
- Component accessors (year-from-date and relatives, *-from-duration,
  timezone-from-*), implicit-timezone() and current-dateTime/date/time(), which
  are fixed per execution budget.
- Not yet: duration/date arithmetic, gYear-family types, adjust-*-to-timezone,
  date formatting, min/max/sort over temporal values.
- Pinned QT3 selection: 5,750 cases, 4,440 pass / 184 engine failures / 229 harness
  errors / 897 exclusions; the gate records 4,440 with no regressions.

## Increment 76: temporal arithmetic and time zone functions

- Added date/time/duration arithmetic: dateTime/date/time plus or minus day-time
  durations, dateTime/date plus or minus year-month durations (day clamping),
  differences of same-type moments, duration sums, differences, scaling by numbers
  (rounding half toward positive infinity) and division, including exact
  duration-by-duration ratios.
- Added dateTime(), adjust-dateTime/date/time-to-timezone() and min()/max() over
  same-type temporal values. Ordering across duration subtypes is a type error.
- Bounded years and duration components so hostile lexical values return errors
  instead of overflowing.
- QT3 now selects 8,187 cases (39 more sets): 6,593 pass / 168 engine failures /
  235 harness errors / 1,191 exclusions; the gate records 6,593 with no regressions.

## Increment 77: Gregorian fragment types and Unicode block properties

- Added xs:gYear, xs:gYearMonth, xs:gMonth, xs:gMonthDay and xs:gDay: strict
  lexical forms (leap-day month-days, year limits), canonical strings, casts from
  dateTime/date, time-zone-normalized equality and type-error ordering.
- Regular expressions accept XML Schema block properties (\p{IsBasicLatin} and
  negations, usable inside classes); unknown block names raise FORX0002 and
  surrogate blocks match nothing.
- The QT3 gate now covers 323 sets (26,002 cases): 15,469 pass / 1,205 engine
  failures / 2,388 harness errors / 6,940 exclusions with no regressions among the
  recorded passing cases, and the new regex block cases add to the recorded set.
  Remaining large buckets: higher-order-function dependency (1,604 harness errors),
  maps and arrays, regex backreferences, resolve-uri/id/path/sort.

## Increment 78: casts, contains-token and resolve-uri

- Casting dates, times, durations and Gregorian fragments to numeric, boolean,
  binary or URI types is now XPTY0004; integer-subtype range failures raise
  FORG0001 as the casting rules require.
- Added contains-token() with optional collations and resolve-uri() implementing
  RFC 3986 reference resolution (dot-segment removal, query/fragment handling,
  FORG0002 for malformed references or non-absolute bases, FONS0005 without a base).
- Pinned QT3 run over 26,002 cases: 15,882+ pass with no regressions; the gate
  records every passing case, now including cast, token and URI sets.

## Increment 79: sort and path

- Added fn:sort() with an optional collation: atomized keys, stable merge sort that
  cannot panic on inconsistent orderings, NaN first, type errors for incomparable
  keys, original items returned. Key functions remain unsupported (no function items).
- Added fn:path() producing Q{uri}name[position] steps, @attribute steps, text(),
  comment() and processing-instruction() steps, and the root() prefix for
  parentless trees.

## Increment 80: xsl:function

- Added top-level xsl:function declarations: namespaced names (XTSE0080), typed
  parameters and results (XTTE0790/XTTE0780), arity overloading, duplicate
  detection (XTSE0770), recursion within the XPath depth limit, and static checks of
  body variables.
- Bodies are limited to xsl:variable declarations followed by one xsl:sequence;
  richer sequence constructors are reported as unsupported rather than guessed.
- Function bodies see global variables and their own parameters only; globals are
  also bound under a reserved prefix so a caller's local variable cannot shadow them.
- Calls to unknown stylesheet functions fail at run time with XPST0017.

## Increment 81: xsl:key and key()

- Added xsl:key declarations (match pattern and use expression, several
  declarations per name) and key() with optional top node. Names with prefixes are
  expanded at compile time; unknown keys raise XTDE1260.
- Lookup scans the context tree (elements and attributes) without an index, so
  repeated key() calls over large documents are linear each; collations, composite
  keys and body-style use are rejected explicitly. Record streaming rejects key().

## Increment 82: xsl:message

- Added xsl:message with select or body content: the message is evaluated (so
  errors surface) but not added to the result; terminate="yes" raises XTMM9000
  carrying the message text. There is no message-delivery API yet, and error-code
  is accepted but ignored.

## Increment 83: xsl:for-each-group

- Added xsl:for-each-group with group-by (several keys per item, first-appearance
  order), group-adjacent (single key per item, XTTE1100), group-starting-with and
  group-ending-with (node patterns, XTTE1120), plus current-group() and
  current-grouping-key() with XTDE1061/XTDE1071 outside a suitable context.
- Exactly one grouping attribute is required (XTSE1080). Groups may be sorted with xsl:sort keys that
  see current-group() and current-grouping-key(). Collations and composite keys are
  rejected as unimplemented; record streaming rejects grouping.

## Increment 84: attribute sets

- Added xsl:attribute-set (merged declarations, use-attribute-sets chains) and
  use-attribute-sets on literal result elements and xsl:element. Literal attributes
  and later xsl:attribute instructions override set attributes; unknown sets are
  XTSE0710 and cycles XTSE0720. xsl:copy and record streaming do not apply sets yet.

## Increment 85: xsl:include

- Added Stylesheet::compile_with_resolver(), which loads xsl:include hrefs through a
  caller-supplied closure; included modules contribute every declaration kind
  (templates, variables, functions, keys, attribute sets, whitespace rules, modes).
  Inclusion nesting is limited (XTSE0180) so include cycles fail; plain compile()
  rejects xsl:include. xsl:import is still unsupported because import precedence is
  not implemented.

## Increment 86: function items and higher-order functions

- Added inline functions (closures over visible variables, typed parameters and
  results), named function references (name#arity for built-in, constructor and
  stylesheet functions), dynamic calls, the arrow operator, function-arity(),
  function-lookup(), for-each(), filter(), fold-left(), fold-right(),
  for-each-pair() and sort() with key functions, plus function(*) sequence types.
- QT3: key() and current-grouping-key() are stylesheet functions now, so three
  XPath-only cases that expected them to be unknown (XPST0017) were dropped from the
  gate on purpose, together with the XPath 2.0 `function()` case; the gate records
  every other passing case.

## Increment 87: maps, arrays and partial application

- Added map and array values with constructors (map{}, [ ], array{ }), the lookup
  operator (postfix and unary, names, integers, wildcards, expressions), calls of
  maps and arrays as functions, partial application with `?` placeholders, fn:apply,
  fn:function-name and the map: and array: function libraries (including the
  higher-order members), deep-equal over containers, atomization of arrays in
  comparisons, arithmetic, data() and sort keys, and map(*)/array(*) sequence types.
- Maps keep insertion order and look keys up linearly; duplicate keys follow
  op:same-key (1 and 1.0 collide). Key lookup `?name` takes only NCNames and
  integer literals.
- The QT3 driver now accepts the higherOrderFunctions feature and predeclares the
  XPath 3.1 static prefixes (fn, xs, math, map, array, err). Two XPath 2.0 cases that
  expect `[` and `[]` to be syntax errors were dropped from the gate on purpose.
- Pinned QT3 over 26,002 cases: 17,321+ pass (from 15,999), 798 engine failures,
  890 harness errors, 6,993 profile exclusions.

## Increment 88: math namespace

- Added math:pi, exp, exp10, log, log10, pow, sqrt, sin, cos, tan, asin, acos, atan
  and atan2 with IEEE semantics, empty-sequence handling and function references.
  All 149 selected QT3 math cases pass and are in the gate.

## Increment 89: fn:format-number

- Added format-number() with the default decimal format: positive/negative
  subpictures, prefix/suffix text, grouping (regular and explicit positions),
  percent and per-mille, minimum/maximum digits with round-half-even, exponent
  pictures such as 9.9999e999, NaN and infinity, and FODF1310 for invalid pictures.
- Named decimal formats (xsl:decimal-format) are not implemented and raise XTDE1280.
- 198 of the 269 fn-format-number cases pass and are in the gate.

## Increment 90: xsl:analyze-string

- Added xsl:analyze-string with matching and non-matching bodies, AVT regex and
  flags, regex-group() (unmatched and out-of-range groups are empty strings),
  XTDE1150 for patterns that match the empty string and XTSE1130 for a missing
  body. The fn:analyze-string function still needs element construction in XPath.

## Increment 91: indent="yes"

- xsl:output indent="yes" now re-flows the serialized document: element-only
  content goes onto indented lines (two spaces), elements with character content
  (mixed content) are left byte-for-byte, comments and processing instructions are
  indented like elements, and a newline follows the XML declaration. Whitespace-only
  text between elements of element-only content is dropped. Record streaming does
  not indent.

## Increment 92: xsl:iterate

- Added xsl:iterate with select, typed parameters (select only), xsl:on-completion
  (select or body, absent focus), xsl:next-iteration with with-param overrides
  (unmentioned parameters carry over) and xsl:break with select or body content.
  position()/last() follow the iterated sequence; falling off the end of the body
  continues with unchanged parameters. Tail position is not checked statically, and
  record streaming rejects iteration.

## Increment 93: xsl:map and xsl:perform-sort

- xsl:map (with xsl:map-entry, xsl:sequence, xsl:if, xsl:choose, xsl:for-each and
  xsl:variable children) is translated at compile time into map:merge/map:entry
  expressions; duplicate keys are errors (map:merge reject). on-duplicates, sorting
  inside xsl:map and map-entry bodies beyond one xsl:sequence are rejected.
- xsl:perform-sort orders a select expression or a single-xsl:sequence body by its
  xsl:sort keys.

## Increment 94: xsl:try and xsl:catch

- Added xsl:try with xsl:catch clauses selected by error name tests (`*`, `*:local`,
  `prefix:*`, `Q{uri}local`), error variables err:code/description/value/module/
  line-number/column-number/additional, select or body forms, and error rethrow
  from catch blocks.
- The try body runs as a nested evaluation with its own output, so partial output is
  discarded on error and spliced into the result on success. Resource-limit errors
  are never caught, nesting is capped, and try inside attribute or other
  simple-content constructors, raw sequence variables and record streaming is
  rejected.

## Increment 95: xsl:decimal-format

- Added xsl:decimal-format (default and named formats; decimal, grouping, minus,
  percent, per-mille, zero-digit, digit, pattern and exponent separators; infinity
  and NaN strings) with the third argument of format-number(); pictures are
  normalized against the format, so custom zero-digit families and literal text that
  looks like picture syntax work. Invalid properties are XTSE1295/XTSE1300 and
  conflicting declarations XTSE1290; unknown names raise FODF1280.
- The QT3 driver turns decimal-format environments into declarations: 230 of the 269
  fn-format-number cases now pass.

## Increment 96: fn:parse-json

- Added parse-json() with strict RFC 8259 parsing into maps, arrays, strings,
  doubles and booleans (null is the empty sequence), surrogate-pair escapes,
  U+FFFD for characters outside the XML Char production, the duplicates
  (use-first, use-last, reject), liberal and escape options, FOJS0001/FOJS0003/
  FOJS0005 errors and a nesting limit. The fallback option and the file-reading
  json-doc()/unparsed-text() functions are not implemented.
- 123 of the 154 fn-parse-json cases pass and are in the gate.

## Increment 97: HTML and XHTML output methods

- xsl:output method="html" and "xhtml": HTML void elements lose their end tags
  (`<br>`; XHTML writes `<br />`), script/style content is unescaped in HTML, boolean
  attributes minimize, attribute values escape only & and ", element names match
  case-insensitively, HTML omits the XML declaration by default, and
  doctype-public/doctype-system emit a DOCTYPE before the root element (also for
  XML). Further xsl:output attributes are accepted and ignored (media-type,
  html-version, include-content-type and similar); other encodings, character maps
  and CDATA sections remain unsupported.

## Increment 98: xsl:import and xsl:apply-imports

- Added xsl:import through the resolver closure with import precedence: importing
  modules outrank imported ones and later imports outrank earlier ones. Template
  rules compare precedence before priority; imported global variables, functions and
  named templates are replaced by higher-precedence declarations (same-precedence
  clashes remain XTSE0630/XTSE0660/XTSE0770); multiple xsl:output declarations are
  allowed across precedence levels. xsl:import must precede other declarations
  (XTSE0200); xsl:include shares the including module's precedence.
- xsl:apply-imports applies only rules of lower precedence than the current rule
  (built-in rules otherwise) and accepts with-param.
- Not covered: imports inside included modules, precedence for keys, attribute sets
  and decimal formats (they merge), and use-package.

## Increment 99: regular-expression backreferences

- Patterns with \1-\9 style backreferences now run on a bounded backtracking matcher
  (capturing and non-capturing groups, alternation, greedy and lazy quantifiers,
  counted repeats, anchors, the i/m/x flags); single-character atoms still use the
  linear-time engine. References must name closed groups (FORX0002 otherwise),
  non-participating groups fail the reference, empty loop iterations are cut off, and
  matching consumes the shared work budget. matches, replace, tokenize and
  xsl:analyze-string all go through the same abstraction.
- fn-matches/fn-matches.re/fn-replace/fn-tokenize: 1,268 of 1,333 QT3 cases pass.

## Increment 100: xsl:number

- Added xsl:number: value (rounded, XTDE0980 for negative or NaN), select, level
  single/multiple/any with count and from patterns, format tokens (decimal with zero
  padding and other Unicode digit families, a/A, i/I, w/W/Ww words with hyphenation,
  prefix/suffix/separators), ordinal, grouping-separator and grouping-size.
- Not implemented: start-at, lang and letter-value (accepted, ignored), non-English
  words and traditional numbering.

## Increment 101: fn:format-integer

- Added format-integer() on the shared numbering module: decimal digit patterns
  (mandatory/optional digits, regular and explicit grouping separators, other Unicode
  digit families), a/A, i/I, w/W/Ww, the ordinal modifier, negative numbers, empty
  input and FODF1310 for malformed pictures; unrecognized tokens fall back to "1".
  Language-dependent forms are not implemented.

## Increment 102: format-date, format-time and format-dateTime

- Added the three formatting functions with picture strings: Y, M, D, d, F, W, w, E, C,
  H, h, P, m, s, f, Z and z markers, escaped brackets, numbering modifiers (digit
  families, zero padding, a/A, i/I, w/W/Ww, N/n/Nn names, ordinal), width modifiers,
  truncated fractional seconds, time zone pictures (+hh:mm families, GMT forms,
  military letters, the t modifier) and FOFD1340/FOFD1350 errors for malformed
  pictures or unavailable components. English only; calendars and places are
  accepted but ignored.
- The QT3 driver binds environment parameters as global variables and treats
  language=en as supported; 243 of the 315 format-date/time cases pass.

## Increment 103: evaluate, where-populated and namespace

- Added xsl:evaluate (dynamic XPath with context-item and with-param variables,
  global variables visible, parse errors reported as XTDE3160; namespace-context,
  base-uri and with-params are rejected), xsl:where-populated (the body runs in a
  nested output and survives only when it produced content) and xsl:namespace
  (XTDE0410 without an open start tag, XTDE0920 for bad prefixes, XTDE0430 for
  conflicts with the element's own prefix).
- The static variable check now also covers inline functions, dynamic calls, partial
  applications, map and array constructors, lookups, perform-sort and evaluate.

## Increment 104: xsl:result-document

- Added xsl:result-document (AVT href, method xml/html/xhtml/text, indent,
  omit-xml-declaration, UTF-8 only) and Stylesheet::transform_with_documents(),
  which returns the principal result plus each secondary document with its href.
  Plain transform() ignores secondary results. Duplicate or missing hrefs raise
  XTDE1490; named output definitions (format), other encodings and nested streaming
  are not implemented.

## Increment 105: collected messages

- xsl:message texts are now collected and returned in TransformResult::messages by
  transform_with_documents(); terminate="yes" still raises XTMM9000 with the text.

## Increment 106: resource functions

- TransformOptions::resolver (a ResourceResolver closure) now backs
  unparsed-text(), unparsed-text-lines(), unparsed-text-available() and json-doc();
  failures and a missing resolver report FOUT1170 (availability tests return false).
  doc()/document() still need document loading into the temporary tree arena.

## Increment 107: doc(), document() and doc-available()

- Documents named by literal URIs in the stylesheet's XPath expressions are loaded
  through the resolver before the transformation starts (XML declaration stripped)
  and bound to their URI; doc() and document() return the same node for repeated
  calls, doc-available() reports presence and unknown documents raise FODC0002.
  Dynamically computed URIs are not supported, and relative-URI resolution against a
  base URI is not performed (the string must match the literal).

## Increment 108: UCA collation approximation

- Collation URIs under http://www.w3.org/2013/collation/UCA are accepted with the
  strength, numeric, alternate, caseFirst, caseLevel, lang and fallback parameters.
  The ordering approximates the Unicode Collation Algorithm: symbols before digits
  before letters, base letters at the primary level, accents at the secondary level
  and case (lower first unless caseFirst=upper) at the tertiary level; no DUCET
  tables, tailoring or script reordering are applied.
- Collation-aware search (contains, starts-with, ends-with, substring-before/after,
  contains-token), compare, min, max, index-of, distinct-values and deep-equal use
  the selected collation.

## Increment 109: expand-text

- expand-text="yes" (on xsl: elements) and xsl:expand-text="yes" (on literal result
  elements) turn text nodes and xsl:text content into text value templates:
  {expr} is evaluated and joined with spaces, {{ and }} are literal braces, and the
  nearest declaration wins. Unbalanced braces and invalid expressions are static
  errors. Record streaming rejects value templates.

## Increment 110: use-when and static variables

- use-when (xsl: elements) / xsl:use-when (literal result elements) conditions are
  evaluated at compile time on instructions and top-level declarations; static
  variables (xsl:variable static="yes") are evaluated in order, visible to later
  conditions, and also remain ordinary global variables.
- Added system-property(), element-available(), function-available() and
  type-available() (no static parameters supplied from the API, and no
  available-system-properties()).

## Increment 111: richer match patterns

- Match patterns now support `//` separators (a//b, //p, /doc//p), parenthesized
  unions in any step (chap/(p|q), (a|b)//c) and key()/id()/doc() patterns that match
  the nodes the call selects. Default priorities treat any multi-step, `//` or
  call-based pattern as 0.5. Reverse axes other than the child/attribute pair are still
  rejected as unsupported patterns; record streaming keeps requiring single-step
  patterns.

## Increment 112: expression fuzzing

- Added a deterministic structured fuzzer (tests/expression_fuzz.rs) that builds random
  well-formed XPath expressions from the supported function library, runs them under
  step limits and asserts that nothing panics. About 600,000 expressions over several
  seeds ran clean before the default round count was reduced for routine test runs.

## Increment 113: xsl:on-empty and xsl:on-non-empty

- A sequence constructor containing xsl:on-empty / xsl:on-non-empty runs its other
  instructions in a nested output; the on-empty content replaces an empty result and
  the on-non-empty content follows a non-empty one. Like where-populated, the nested
  evaluation cannot produce attributes or namespaces for the enclosing element.

## Increment 114: output encodings

- xsl:output and xsl:result-document accept encoding="UTF-8", "ISO-8859-1" and
  "US-ASCII". Results stay Rust strings: the XML declaration names the encoding and
  characters outside it become numeric character references (markup methods) or raise
  SERE0008 (text method). Other encodings, and record streaming with a non-UTF-8
  encoding, are rejected.

## Increment 115: cdata-section-elements

- xsl:output cdata-section-elements writes the text content of the listed elements
  (expanded names, accumulated across declarations) as CDATA sections, splitting any
  `]]>` in the content. Not applied to result documents or the text method.

## Increment 116: character maps

- xsl:character-map with xsl:output-character, nested use-character-maps and the
  xsl:output use-character-maps attribute: mapped characters in text and attribute
  values are written verbatim (unescaped); unknown maps are XTSE1590, cycles XTSE1600
  and duplicate names XTSE1580. Maps apply to the principal result only.

## Increment 117: dynamic sort settings and sort collations

- xsl:sort data-type, order, case-order and collation may be attribute value
  templates (evaluated once per sort in the instruction's focus, XTDE0030 for invalid
  values); collation, lang and stable are accepted, with collations applying to text
  keys (XTDE1035 when unsupported).

## Increment 118: xsl:assert

- xsl:assert raises XTMM9001 with its message (select or body) when the test is
  false and does nothing otherwise.

## Increment 119: collation keys and environment functions

- Added collation-key() (byte keys that order like the selected collation, including
  the UCA approximation), default-language() ("en"), environment-variable(),
  available-environment-variables(), static-base-uri() and base-uri()/document-uri()
  (always empty: source documents carry no URI in this API).

## Increment 120: QT3 driver resources

- The development driver accepts a `resources` map backing the resolver, and the QT3
  bridge binds `<source role="$var">` environments through doc() and `<resource>`
  entries by URI.

## Increment 121: QT3 resource regression gate

- Refreshed the pinned 26,966-case selection after resource support: 18,557 pass,
  599 engine failures, 765 harness errors and 7,045 profile exclusions. All 18,511
  previous gate cases still pass; added the 46 new passing cases to the gate.
  Previously blocked cases now reaching the engine account for the increased
  failure count; these remain explicit failures, not passes.
- Added a harness regression covering source-variable URI defaults, explicit
  resource URIs, resource content and UTF-8 BOM removal.
- Validation: cargo test --offline, Clippy with warnings denied, all 28 Python
  harness tests and the pinned QT3 regression gate pass.

## Increment 122: context-defaulting functions and atomization

- data() and nilled() accept the zero-argument forms, using the current item.
  Missing context raises XPDY0002; nilled() still requires a node, and data()
  recursively atomizes arrays while rejecting maps/functions with FOTY0013.
- Fixed the array-flattening fast path that previously let maps and functions
  through atomization when no arrays were present.
- QT3 now selects XPath 3.1-compatible spec dependencies and respects negative
  dependencies. Exclusively XPath 2.0/3.0 cases are profile exclusions rather
  than being evaluated with incompatible version semantics. Six old gate entries
  fall into these profiles; all other previous gate cases retain passing status.
- Added 17 passing cases: the gate now contains 18,568 passes out of 26,966
  selected cases, with 557 engine failures, 764 harness errors and 7,077 profile
  exclusions. Validated the full selection, then reran all 16 sets affected by
  the profile-selection correction and merged their results.
- Validation: full offline Rust tests, Clippy with warnings denied, formatting,
  and all 29 Python harness tests pass.

## Increment 123: documents in global bindings

- Load literal-URI documents before evaluating global bindings, so global
  variables and parameters can use doc(), document() and doc-available(). Each
  URI is resolved once per transformation and shares node identity across global
  bindings, stylesheet functions and template expressions.
- Parse loaded resources as XML documents, preserving top-level comments/PIs
  without creating text nodes from prolog/epilog whitespace. Invalid XML and
  multiple-root fragments remain unavailable; retained source-byte limit failures
  propagate as XPDY0130 instead of being swallowed as missing documents.
- Added regressions for forward global dependencies, shared document identity,
  resolver call counts, missing resolvers, XML declarations/BOMs, document
  structure, malformed resources and source-byte limits.
- Full pinned QT3 gate: 18,729 pass (161 new passes, zero regressions), 396 engine
  failures, 764 harness errors and 7,077 profile exclusions across 26,966 cases.
  All passing cases are recorded in the regression baseline.
- Validation: full offline Rust tests, Clippy with warnings denied, formatting,
  all 29 Python harness tests and the full pinned QT3 regression gate pass.

## Increment 124: Unicode component and XPath names

- Component declarations, external entry names and XPath name tokens now use the
  existing XML NCName rules consistently, including non-ASCII letters, combining
  marks in continuations, and Unicode URIQualifiedName local names.
- Added regressions for Unicode variable/function names, external named-template
  entry, expanded-name paths and local-name wildcards, plus invalid name starts.
- Validation: full offline Rust suite, Clippy with warnings denied, formatting,
  diff whitespace checks and all 29 Python harness tests pass. External W3C
  conformance gates were not rerun for this increment.

## Increment 125: selection-anchored match paths

- Match patterns now support child/descendant paths after key(), id(), and
  document selection anchors. Reverse ancestry matching checks the selected
  anchor by node identity; parenthesized unions retain their anchors.
- Added focused regressions for child versus descendant distance, attributes,
  union paths, default priorities and xml:id anchors. W3C key-033, key-038 through
  key-041 and key-049 now pass in the surveyed key set.
- Surveyed match/key/id and expanded the XSLT gate to all 575 passing cases out of
  1,159 selected cases: 240 engine failures, 195 harness errors and 149 optional
  profile exclusions. All 420 previous gate cases still pass.
- Validation: full offline Rust tests, Clippy with warnings denied, formatting,
  diff whitespace checks and the expanded pinned XSLT regression gate pass.

## Increment 126: typed key values and selected subtrees

- key() compares atomic values using value equality: numbers and strings stay
  distinct, untyped values compare as strings, QNames compare by expanded name,
  temporal values normalize timezones, and NaN never matches. Arrays atomize
  recursively; maps/functions raise FOTY0013. Backwards-compatible declarations
  enable string comparison for all declarations sharing the key name.
- The third argument restricts results to its ancestor-or-self subtree, including
  the top node and attributes. Two-argument lookups still search the document;
  trees without document roots raise XTDE1270.
- Key declarations evaluate in global scope so caller-local bindings cannot
  change their match patterns or use values. Added focused regressions and fixed
  older expectations that had incorrectly blessed subtree/type coercion bugs.
- Pinned XSLT gate: 580 passes (five new, zero regressions), 235 engine failures,
  195 harness errors and 149 profile exclusions across 1,159 selected cases.
- Validation: full offline Rust tests, Clippy with warnings denied, formatting,
  whitespace checks and the expanded XSLT regression gate pass.

## Increment 127: key dependency guards

- Circular key dependencies in match/use expressions now raise XTDE0640 instead
  of recurring until execution limits or the native stack are exhausted. The
  guard distinguishes document roots so the same key may depend on another
  document, and bounds acyclic dependency depth with the shared depth limit.
- Evaluation removes active dependency markers on both success and failure;
  caught key errors do not contaminate later lookups. Added direct/mutual cycle,
  catch/retry, cross-document, repeated lookup and depth-limit regressions.
- Validation: full offline Rust tests, Clippy with warnings denied, formatting,
  all 29 Python harness tests and the 580-case pinned XSLT gate pass.
- Reran the full pinned 26,966-case QT3 selection after increment 124: all 18,729
  baseline cases still pass (396 failures, 764 harness errors, 7,077 exclusions).

## Increment 128: composite keys and explicit key collations

- xsl:key composite="yes" compares the entire ordered sequence of atomic
  components, including empty composite keys. Ordinary keys retain any-value
  matching. Composite declarations sharing a name must agree (XTSE1222).
- Explicit key collations use the existing collation implementation for string
  components. Unknown collations raise XTSE1210; inconsistent collation URIs
  across declarations raise XTSE1220. Collation defaults inherited from stylesheet
  attributes and relative collation URIs remain unsupported.
- Added regressions for component order/type/length, empty keys, array atomization,
  case-insensitive comparisons and declaration consistency.
- Pinned XSLT gate now contains 584 passes (four new, zero regressions), 231 engine
  failures, 195 harness errors and 149 profile exclusions across 1,159 cases.
- Validation: full offline Rust tests, Clippy with warnings denied, formatting,
  whitespace checks and the expanded XSLT gate pass.

## Increment 129: current item semantics

- Added current(), retaining the outer stylesheet expression's context item
  through predicates, path steps and nested XPath focus changes. Patterns retain
  the candidate node, and key use expressions retain the indexed node.
- Missing current items raise XTDE1360. Named current#0 references and
  function-lookup retain the item at creation and report their function metadata.
  Dynamic evaluate establishes its own current item from the supplied context.
- Added focused regressions for nested instruction contexts, atomic current
  items, matching, keys, missing context and retained function references.
- Expanded the pinned XSLT gate to 594 passes (ten new, zero regressions) across
  1,160 cases: 222 engine failures, 195 harness errors, 149 profile exclusions.
- Validation: full offline Rust tests, Clippy with warnings denied, formatting,
  whitespace checks and the expanded pinned XSLT gate pass.

## Increment 130: cached key evaluation

- Cache evaluated key data per expanded key name and document root for each
  transformation. Cached entries own atomic values and retain node IDs rather
  than borrowing source nodes; node handles are reconstructed in the queried tree.
- Lookups preserve atomic/composite/collation rules, identity, document order and
  subtree restrictions. Index construction retains dependency guards and charges
  retained value bytes to the shared execution budget; failed indexes are discarded.
- Added repeated-lookup work-budget, cross-document/subtree and repeated compiled
  stylesheet execution regressions. Candidate value comparisons remain linear in
  cached entries; this is evaluated-value caching, not a hash lookup index.
- Validation: full offline Rust suite, focused final key tests, Clippy with warnings
  denied, formatting, whitespace checks and all 594 pinned XSLT baseline cases pass.
- A full QT3 run after current() identified two XPath-only unknown-function cases
  that now see the XSLT extension through the bridge. These remain reported
  regressions until the development bridge isolates its XPath function profile.

## Increment 131: isolate the QT3 function library

- Added a development-only conformance feature and hidden stylesheet compilation
  hook selecting the XPath function library. QT3 query and assertion requests use
  this profile; ordinary XSLT compilation keeps its extension functions.
- The profile rejects XSLT-only calls/references with XPST0017 and hides them from
  function-lookup at runtime. Functions transferred into XPath, including
  generate-id(), remain available. Compilation restores the thread-local profile
  after success or failure; compiled stylesheets retain their own runtime profile.
- Added profile compilation/restoration/runtime lookup tests and a Python bridge
  regression verifying both query and assertion requests select the profile.
- Full pinned QT3: 18,735 passes (six new, zero regressions), 390 engine failures,
  764 harness errors, 7,077 profile exclusions across 26,966 cases. The two current()
  regressions from increment 129 are resolved; all prior gate cases pass.
- Validation: full all-feature offline Rust suite, default-build checks,
  all-feature Clippy with warnings denied, formatting, all 30 Python harness
  tests, the full QT3 gate and the 594-case XSLT gate pass.

## Increment 132: XPath default element/type namespace

- Implemented inherited xpath-default-namespace, including local empty resets
  and xsl:xpath-default-namespace on literal result elements. Unprefixed element
  paths/patterns, element/document sequence types, atomic type names and QName
  casts use the setting. Attributes, variables, component names, function calls
  and explicitly empty URIQualifiedNames retain their separate namespace rules.
- Whitespace name tests and key declarations use the same static namespace
  context. Namespaced xpath-default-namespace on XSLT elements raises XTSE0090.
- Added focused inheritance, literal-result/AVT, type/cast, key, whitespace and
  invalid-attribute regressions. Surveyed the dedicated W3C namespace set and
  expanded the gate to 618 passes (24 new, zero regressions) across 1,186 cases:
  218 engine failures, 197 harness errors and 153 optional-profile exclusions.
- Validation: full all-feature offline Rust suite, focused final regressions,
  all-feature Clippy with warnings denied, formatting, all 30 Python harness
  tests, expanded XSLT gate and all 18,735 QT3 baseline cases pass.

## Increment 133: structural versus serialized assertions

- assert-serialization now compares serialized text exactly, supporting XML,
  HTML, XHTML, text, JSON and adaptive results. Explicit ignore-prefixes keeps
  structural prefix-insensitive comparison for XML methods. Expected files use
  their declared encoding; invalid settings fail harness preflight.
- XML assertions first parse well-formed documents, excluding prolog/epilog
  whitespace and DTD declarations from the XDM comparison while retaining
  comments/PIs. Fragment fallback preserves all fragment text and element
  whitespace. This resolves seven previously false-negative comparisons.
- Recognized the existing serialization feature dependency and surveyed output
  tests. Expanded the pinned XSLT gate to 640 passes (22 new, zero regressions)
  across 1,418 cases: 239 engine failures, 386 harness errors and 153 exclusions.
- Added exact/non-XML serialization, encoding, prefix, preflight and document/
  fragment whitespace regressions. All 36 Python harness tests, whitespace
  checks and the expanded pinned XSLT regression gate pass.

## Increment 134: serialization regexes and error assertions

- Added serialization-matches using the engine's XPath matches() implementation,
  catalog flags and external string parameters; raw serialized text is never
  interpreted as XML or interpolated into XPath. Pattern files are supported;
  invalid flags/evaluator errors remain explicit harness errors.
- Added assert-serialization-error with exact serialization-code matching. Its
  wildcard accepts only serialization errors, not arbitrary transformation errors
  or implementation-gap diagnostics. Recognized the XPath_3.1 feature dependency.
- The expanded pinned XSLT gate now has 694 passes (54 new, zero regressions),
  347 engine failures, 224 harness errors and 153 profile exclusions across 1,418
  cases. Newly reachable cases account for increased explicit engine failures.
- Added regex parameter/flag, false-match, evaluator-error, invalid-preflight and
  serialization-error regressions. All 39 Python tests, whitespace checks and the
  expanded pinned XSLT regression gate pass.

## Increment 135: XML declarations, standalone and UTF-8 BOMs

- omit-xml-declaration accepts all XSLT boolean lexical forms. XML/XHTML
  declarations serialize standalone=yes/no/omit; incompatible declaration omission
  raises SEPM0009. HTML and text methods ignore XML-declaration settings.
- Added UTF-8 byte-order marks to principal/secondary results and a single stream
  preamble. Non-BOM encodings ignore the BOM request. Secondary results can
  override standalone/BOM settings, and preamble bytes count toward output limits.
- Validate boolean serialization attributes that were previously silently ignored,
  returning XTSE0020 for invalid lexical values. Valid include-content-type,
  escape-uri-attributes and undeclare-prefixes behavior remains incomplete.
- Added regressions for boolean forms, standalone conflicts, HTML/text behavior,
  principal/secondary output, one streamed preamble and output-byte budgets.
- Pinned XSLT gate now has 716 passes (22 new, zero regressions), 325 engine
  failures, 224 harness errors and 153 exclusions across 1,418 cases.
- Validation: full all-feature offline Rust suite, all-feature Clippy with warnings
  denied, formatting, whitespace checks and the expanded XSLT gate pass.

## Increment 136: merge unnamed output declarations

- Multiple unnamed xsl:output declarations now merge properties independently
  using import precedence. Equal normalized boolean/encoding values are compatible;
  conflicting values at the winning precedence raise XTSE1560. Higher-precedence
  values can resolve conflicts in lower-precedence imported declarations.
- Includes merge at the including module's precedence. Missing properties no
  longer reset previously declared methods, encodings or declaration settings.
- CDATA element names accumulate across declarations and resolve unprefixed names
  using the stylesheet XML default namespace, independently of the XPath default.
- Added property merging, boolean alias, conflict, import/include and CDATA
  namespace regressions. Named output definitions remain unsupported.
- Pinned XSLT gate: 724 passes (eight new, zero regressions), 317 engine failures,
  224 harness errors and 153 exclusions across 1,418 cases.
- Validation: full all-feature offline Rust suite, all-feature Clippy with warnings
  denied, formatting, all 39 Python harness tests and expanded XSLT gate pass.

## Increment 137: named output definitions and secondary inheritance

- Named xsl:output definitions merge independently by expanded name/import
  precedence. result-document format supports AVTs and static QName bindings;
  unknown/invalid definitions raise XTDE1460 and AVT variables are checked.
- Secondary documents inherit their selected named or unnamed definition's method,
  encoding, indentation, declaration/BOM, doctype, CDATA and character maps, with
  explicit instruction overrides. Corrected the old secondary declaration test
  that had ignored the unnamed omit-xml-declaration setting.
- Character maps now apply to text/raw HTML text; CDATA content retains the pinned
  suite's unmapped behavior, and HTML output ignores CDATA section settings.
- The development driver returns secondary documents/messages and reads principal
  method metadata from the compiled stylesheet instead of the first output node.
- Added named method/AVT/namespace, merging/conflict, inherited/overridden settings,
  character-map/CDATA and driver metadata regressions. Surveyed result-document;
  expanded gate: 735 passes (11 new, zero regressions), 410 failures, 274 harness
  errors and 153 exclusions across 1,572 cases. All 18,735 QT3 cases still pass.
- Validation: full all-feature Rust suite, final focused tests, all-feature Clippy,
  default-build check, formatting, 39 Python tests and both full pinned gates pass.
  Rebuilt with debug info disabled after clearing generated Cargo artifacts to
  recover disk space; source/dependency configuration was unchanged.

## Increment 138: secondary result assertions

- Added assert-result-document over the development driver's returned secondary
  document map. Result URIs normalize relative to an implementation-defined
  logical base without fetching resources; each selected document runs through
  the ordinary recursive assertion evaluator as an XML result.
- Missing documents and duplicate equivalent URIs are failures. Malformed driver
  responses and unsupported nested assertions remain harness errors, checked
  before execution even inside any-of. Engine errors cannot satisfy negated
  secondary assertions.
- Added URI normalization, nested XML/string assertions, missing/duplicate result,
  malformed response, nested preflight and negated-error regressions.
- Expanded pinned XSLT gate: 741 passes (six new, zero regressions), 414 engine
  failures, 264 harness errors and 153 exclusions across 1,572 cases. Newly
  reaching the engine accounts for four additional explicit failures.
- Validation: all 41 Python harness tests, whitespace checks and the expanded
  pinned XSLT gate pass. No Rust implementation changes in this increment.

## Increment 139: predicate match patterns and item dispatch

- Added bare-dot and dot-predicate match patterns with singleton predicate focus,
  priority handling and static variable-scope validation. Template and grouping
  dispatch now test atomic, map, array and function items as well as nodes.
- Implemented text-only, shallow-skip and copy behavior for non-node items,
  including array-member dispatch, forwarded parameters and serialization errors
  for maps and functions. Record streaming rejects predicate patterns it cannot
  evaluate locally.
- Added coverage for predicate position/last, default priorities and
  next-match, function items, built-in mode policies, array parameters, and atomic
  grouping. Updated the prior grouping expectation for atomic population items.
- Expanded the pinned XSLT gate to 754 passing cases (13 new, zero regressions),
  401 engine failures, 264 harness errors and 153 profile exclusions across 1,572
  selected cases. All 18,735 QT3 baseline cases remain green.
- Validation: all-feature Rust tests, all-target/all-feature Clippy, 41 Python
  harness tests, and both pinned gates pass.

## Increment 140: variable-anchored and filtered patterns

- Match patterns can use global variable references as selection anchors, alone
  or at the head of a path, and can filter selected nodes. Pattern filters are
  applied after the candidate's underlying pattern matches.
- Explicit descendant and descendant-or-self name tests now use the existing
  reverse-ancestry matcher. Added focused coverage for variable identity,
  descendant matching and filtered variable anchors.
- Expanded the pinned XSLT gate to 763 passing cases (nine new, zero regressions),
  392 engine failures, 264 harness errors and 153 profile exclusions across 1,572
  selected cases. All QT3 baseline cases remain green.
- Validation: all-feature Rust tests, all-target/all-feature Clippy, 41 Python
  harness tests, and both pinned gates pass.

## Increment 141: intersect and except match patterns

- Match pattern `intersect` and `except` operators now constrain candidates by
  requiring or excluding a match from the right-hand pattern. Unions on the
  right side remain alternative constraints, and precedence uses the left
  pattern's default priority.
- Variable and predicate expressions inside constraints participate in static
  scope validation. Record streaming rejects constrained patterns until its
  local matcher can evaluate them safely.
- Added coverage for direct-child intersection and nested-child exclusion.
  Expanded the pinned XSLT gate to 766 passing cases (three new, zero regressions),
  389 engine failures, 264 harness errors and 153 profile exclusions across 1,572
  selected cases.
- Validation: focused pattern suites, all-target/all-feature Clippy and the full
  pinned XSLT gate pass.

## Increment 142: self-axis and descendant-axis predicates

- Match patterns now support `self::` steps at the start, middle or end of a path.
  Descendant and descendant-or-self steps evaluate their predicates over the
  selected axis sequence, preserving `position()` and `last()` behavior.
- Root-function selection anchors can be filtered, including parentless element
  roots. Added regression coverage for self-axis matching and `root()[…]`.
- Expanded the pinned XSLT gate to 775 passing cases (nine new, zero regressions),
  380 engine failures, 264 harness errors and 153 profile exclusions across 1,572
  selected cases.
- Validation: all-feature Rust tests, all-target/all-feature Clippy, 41 Python
  harness tests and both pinned gates pass.

## Increment 143: anchored constraints in path patterns

- When a path pattern contains an `intersect` or `except` on its right-hand side,
  its constraints are now prefixed with the left-hand pattern. This preserves
  the axis context for exclusions such as `x/(descendant::a except child::a)`.
- Added a regression using parentless temporary elements to verify that direct
  children and deeper descendants take different template rules. Absolute
  descendant patterns also require a document-rooted input tree.
- Validation: the focused pattern suite and the pinned XSLT gate pass without
  losing any of the 775 recorded cases.

## Increment 144: catalog initial-template entry

- When a catalog test has no source and no explicit initial-template selection,
  the harness now starts the stylesheet's `xsl:initial-template`, if present.
  It leaves source-backed cases and explicit entry selections unchanged.
- Mode declarations now accept and validate `warning-on-multiple-match`; issuing
  that optional warning remains disabled. Added harness and mode-boolean tests.
- The expanded pinned XSLT gate now has 808 passing cases (33 new, zero regressions),
  347 engine failures, 264 harness errors and 153 profile exclusions across 1,572
  selected cases.
- Validation: all 43 Python harness tests, mode and pattern suites and the pinned
  XSLT gate pass.

## Increment 145: evaluate right-hand pattern expressions in context

- Slash patterns with a general right operand now evaluate that expression from
  each candidate left-hand anchor and test whether the result contains the
  candidate. This keeps `except` exclusions relative to the same anchor that
  selected the descendant, including parentless constructed trees.
- Record streaming rejects these context-dependent path patterns. Added coverage
  for a nested descendant that is excluded under its nearest parent but selected
  under an outer matching ancestor.
- The pinned XSLT gate now has 809 passing cases (one new, zero regressions), 346
  engine failures, 264 harness errors and 153 profile exclusions across 1,572
  selected cases.
- Validation: all-feature Rust tests, all-target/all-feature Clippy, 43 Python
  harness tests and the pinned XSLT gate pass.

## Increment 146: versioned match-pattern syntax

- The compiler now rejects XSLT 2.0 parenthesized unions, top-level filtered
  patterns and `intersect`/`except`, plus a predicate directly after `/`. It
  accepts these XSLT 3.0 pattern forms. Leading XPath comments are skipped when
  checking the pattern's opening syntax.
- Expanded the pinned XSLT gate to 813 passing cases (four new, zero regressions),
  342 engine failures, 264 harness errors and 153 profile exclusions across 1,572
  selected cases.
- Validation: all-feature Rust tests, all-target/all-feature Clippy, 43 Python
  harness tests and both pinned gates pass.

## Increment 147: principal result documents

- `xsl:result-document` without `href` or with `href=""` now writes to the
  principal result. A second principal result, including a collision with normal
  template output, raises `XTDE1490`.
- Added tests for both collision orders and updated the pinned XSLT regression
  baseline to 832 passing cases (19 new, zero regressions), 323 engine failures,
  264 harness errors and 153 profile exclusions across 1,572 selected cases.
- Validation: all-feature Rust tests, all-target/all-feature Clippy, 43 Python
  harness tests and both pinned XSLT and QT3 gates pass.

## Increment 148: result-document DOCTYPE overrides

- `xsl:result-document` accepts `doctype-public` and `doctype-system` AVTs and
  applies them to secondary results and href-less principal results. Public
  identifiers are ignored when the resulting system identifier is absent.
- The result-document selection gained eight passes; the full pinned XSLT gate
  now has 840 passing cases (eight new, zero regressions), 315 engine failures,
  264 harness errors and 153 profile exclusions across 1,572 selected cases.
- Validation: focused result-document tests and the pinned XSLT gate pass.

## Increment 149: result-document CDATA lists

- Result documents accept dynamic `cdata-section-elements` QName lists, resolving
  prefixes in the declaring stylesheet and adding them to output CDATA settings.
  DOCTYPE AVTs now also participate in static variable-scope validation.
- The pinned XSLT gate has 842 passing cases (two new, zero regressions),
  313 engine failures, 264 harness errors and 153 profile exclusions.
- Validation: focused result-document tests, all-target/all-feature Clippy and
  the full pinned XSLT gate pass. Named-format selection on href-less results
  remains a separate serialization gap.

## Increment 150: unified result-document serialization

- Principal and secondary result documents now share named-format resolution,
  serialization overrides, encoding, declarations, indentation and CDATA merging.
  Explicit principal results bypass the implicit result serializer.
- The pinned XSLT gate has 864 passing cases (22 new, zero regressions),
  291 engine failures, 264 harness errors and 153 profile exclusions.
- Validation: all-feature Rust tests, all-target/all-feature Clippy, formatting
  and the full pinned XSLT gate pass.

## Increment 151: result-document character maps

- Result-document instructions accept `use-character-maps`, supplementing the
  selected output definition and overriding inherited mappings for the same
  character. Referenced map composition uses the existing cycle checks.
- The pinned XSLT gate has 868 passing cases (four new, zero regressions),
  287 engine failures, 264 harness errors and 153 profile exclusions.
- Validation: character-map and result-document tests, all-target/all-feature
  Clippy and the full pinned XSLT gate pass.

## Increment 152: static result-document character-map checks

- Result-document character-map references are validated during compilation,
  including instructions in branches that never execute. Unknown maps report
  `XTSE1590` before transformation begins.
- Added a regression for an unknown map under `xsl:if test="false()"`.
- Validation: character-map tests, all-target/all-feature Clippy and the full
  868-case pinned XSLT regression gate pass without regressions.

## Increment 153: boolean result-document AVTs

- `indent`, `omit-xml-declaration` and `byte-order-mark` on result documents
  accept AVTs, validate dynamic boolean values and participate in static scope
  checks. Literal invalid values retain compile-time validation.
- The pinned XSLT gate has 872 passing cases (four new, zero regressions),
  283 engine failures, 264 harness errors and 153 profile exclusions.
- Validation: result-document tests, all-target/all-feature Clippy and the full
  pinned XSLT regression gate pass.

## Increment 154: standalone result-document AVTs

- Result-document `standalone` accepts AVTs, including `omit`, and validates
  dynamic values before rendering the document. Expressions participate in
  static variable-scope checks.
- The pinned XSLT gate has 874 passing cases (two new, zero regressions),
  281 engine failures, 264 harness errors and 153 profile exclusions.
- Validation: result-document tests, all-target/all-feature Clippy and the full
  pinned XSLT regression gate pass.

## Increment 155: result-document encoding AVTs

- Result-document `encoding` accepts AVTs and evaluates them in the caller's
  context before serialization. Encoding expressions participate in static
  variable-scope checks; supported literal encodings retain early validation.
- Added a dynamic US-ASCII regression that verifies character-reference output.
- Validation: result-document tests, all-target/all-feature Clippy and the full
  874-case pinned XSLT regression gate pass without regressions.

## Increment 156: result-document method AVTs

- Result-document `method` accepts AVTs and selects the supported XML, HTML,
  XHTML or text serializer from the evaluated value. Expressions participate
  in static scope checks. Added a dynamic text-method regression.
- The pinned XSLT gate has 875 passing cases (one new, zero regressions),
  280 engine failures, 264 harness errors and 153 profile exclusions.
- Validation: result-document tests, all-target/all-feature Clippy and the full
  pinned XSLT regression gate pass.

## Increment 157: reserve result destinations before evaluation

- Secondary result URIs are reserved before their bodies execute, so a nested
  result-document instruction cannot reuse its still-open parent destination.
  Duplicate destinations report `XTDE1490` without waiting for serialization.
- Added a regression for nested result documents targeting the same URI.
- Validation: all six result-document tests, all-target/all-feature Clippy and
  the full 875-case pinned XSLT regression gate pass without regressions.

## Increment 158: result-document output-version AVTs

- Result-document `output-version` accepts AVTs for XML 1.0 output. Text output
  ignores this parameter; other serialization versions remain explicit gaps.
  Version expressions participate in static variable-scope checks.
- The pinned XSLT gate has 878 passing cases (three new, zero regressions),
  277 engine failures, 264 harness errors and 153 profile exclusions.
- Validation: result-document tests, all-target/all-feature Clippy and the full
  pinned XSLT regression gate pass.

## Increment 159: serialization regression audit

- Rechecked the accumulated result-document serialization changes against the
  full all-feature Rust suite and pinned QT3 workload: all 18,735 recorded QT3
  cases remain green across 26,966 selected cases.
- Removed obsolete secondary-depth bookkeeping after unifying explicit principal
  and secondary serialization, and corrected the principal-output state comment.
- Validation: full all-feature Rust tests, all-target/all-feature Clippy,
  formatting, 43 Python harness tests and the pinned QT3 gate pass.

## Increment 160: XML public-only DOCTYPE handling

- XML and XHTML output ignore a public DOCTYPE identifier when there is no
  system identifier. HTML retains its public-only declaration behavior.
- Added a regression comparing XML and HTML output for this parameter pair.
- The pinned XSLT gate has 880 passing cases (two new, zero regressions),
  275 engine failures, 264 harness errors and 153 profile exclusions.
- Validation: HTML/XML serializer tests, all-target/all-feature Clippy and the
  full pinned XSLT regression gate pass.

## Increment 161: preserve HTML result-document public identifiers

- Result-document DOCTYPE overrides retain public identifiers independently of
  system identifiers, leaving method-specific suppression to the serializer.
  Public-identifier AVTs are evaluated even when the system identifier is absent.
- Added a regression for an HTML result document with a public-only DOCTYPE.
- Validation: seven result-document tests, all-target/all-feature Clippy and
  the full 880-case pinned XSLT regression gate pass without regressions.

## Increment 162: dynamic CDATA QName diagnostics

- Invalid QName lists produced by result-document CDATA AVTs now report
  `XTDE0030` in the run phase, including undeclared prefixes. Shared QName
  resolution no longer leaks compile-phase diagnostics into this dynamic path.
- Added coverage for malformed dynamic QNames and undeclared dynamic prefixes.
- Validation: eight result-document tests, all-target/all-feature Clippy and
  the full 880-case pinned XSLT regression gate pass without regressions.

## Increment 163: defer implicit declaration serialization

- The implicit principal preamble is serialized after execution only when no
  explicit principal result was selected. Explicit result-document overrides
  can therefore replace otherwise-conflicting implicit declaration settings.
- Prefix bytes remain included in output limits. Added a regression overriding
  an implicit standalone/omitted-declaration conflict.
- Validation: declaration and result-document tests, all-target/all-feature
  Clippy and the full 880-case pinned XSLT gate pass without regressions.

## Increment 164: XML whitespace in dynamic CDATA lists

- Dynamic result-document CDATA QName lists split only on XML whitespace.
  Nonbreaking spaces remain part of a token and trigger invalid-QName errors.
- Extended dynamic QName coverage with a nonbreaking-space input.
- Validation: full all-feature Rust tests before this change; focused
  result-document tests, all-target/all-feature Clippy and the full 880-case
  pinned XSLT gate pass after the change without regressions.

## Increment 165: CDATA and character-map interaction regression

- Confirmed the serializer follows the W3C character-map rule that CDATA text
  suppresses character mapping. Added a regression covering CDATA text together
  with mapped attributes and ordinary mapped text in the same result.
- Reference: https://www.w3.org/TR/xslt-xquery-serialization-30/#character-maps
- Validation: all four character-map tests, all-target/all-feature Clippy,
  formatting and diff checks pass. No runtime behavior changed.
