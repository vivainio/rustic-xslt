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

## Increment 166: encoding references outside CDATA

- Restricted output encodings close CDATA before emitting an unrepresentable
  character as a reference, then reopen CDATA for following text. Comment and
  processing-instruction markers do not start CDATA state in the encoding pass.
- Added a US-ASCII CDATA regression. The pinned XSLT gate now has 882 passes
  (two new, zero regressions), 273 failures, 264 harness errors and 153 exclusions.
- Validation: ten result-document tests, all-target/all-feature Clippy and the
  full pinned XSLT regression gate pass.

## Increment 167: encoding errors in comments and processing instructions

- Restricted encodings report `SERE0008` for unrepresentable characters inside
  comments and processing instructions, where character references cannot
  preserve the node value. Representable characters remain unchanged.
- Added US-ASCII failure cases and a representable Latin-1 comment regression.
- Validation: encoding and result-document tests, all-target/all-feature Clippy
  and the full 882-case pinned XSLT gate pass without regressions.

## Increment 168: restricted encodings in markup names

- The encoding pass distinguishes markup names from quoted attribute values.
  Unrepresentable element and attribute names report `SERE0008`, while attribute
  values retain character-reference serialization.
- Added regressions for non-ASCII element names, attribute names and values.
- Validation: encoding and result-document tests, all-target/all-feature Clippy
  and the full 882-case pinned XSLT gate pass without regressions.

## Increment 169: final principal serialization size limit

- The implicit principal result checks its byte limit after indentation and
  encoding, catching expansion from character references and CDATA boundaries.
  Explicit result-document serialization already checks its final size.
- Added a regression where raw output fits but US-ASCII references exceed the
  configured output limit and must report `XPDY0130`.
- Validation: encoding and declaration tests, all-target/all-feature Clippy and
  the full 882-case pinned XSLT gate pass without regressions.

## Increment 170: encoding errors in DOCTYPE identifiers

- The encoding pass tracks DOCTYPE declarations separately from attribute values
  and rejects unrepresentable identifiers with `SERE0008`, preserving identifier
  semantics instead of inserting character references into literals.
- Added US-ASCII rejection and Latin-1 preservation regressions.
- Validation: seven encoding tests, all-target/all-feature Clippy and the full
  882-case pinned XSLT gate pass without regressions.

## Increment 171: aggregate streaming serialization limit regression

- Added a two-record XML regression confirming that escaped output bytes count
  toward the aggregate streaming limit and that a failing second record leaves
  only the first completed record in the writer. Successful runs report the
  actual escaped output size.
- Record streaming remains restricted to UTF-8 output.
- Validation: all twelve streaming tests and formatting pass; all-target and
  all-feature Clippy passed during this test increment.

## Increment 172: preserve quoted encoding state around markup-like text

- The encoding pass recognizes comment, PI and CDATA openings only outside
  markup tags, so marker-like text inserted into quoted attribute values by
  character maps does not change the encoding context.
- Added a regression combining an inserted comment marker and an unrepresentable
  ordinary attribute character under US-ASCII output.
- Validation: encoding and character-map tests, all-target/all-feature Clippy
  and the full 882-case pinned XSLT gate pass without regressions.

## Increment 173: preserve carriage returns in CDATA serialization

- CDATA text emits carriage returns as references outside CDATA sections,
  preventing XML end-of-line normalization from changing their value to LF.
- Added a regression constructing CR text with `codepoints-to-string`.
- Validation: nine encoding tests, all-target/all-feature Clippy and the full
  882-case pinned XSLT gate pass without regressions.

## Increment 174: safe DOCTYPE system-identifier quoting

- DOCTYPE system identifiers containing double quotes use single-quoted literals.
  Values containing both delimiters report invalid serialization parameter
  `SEPM0016` instead of producing a malformed declaration.
- Added a regression for a double quote in an XML system identifier.
- Validation: eight HTML/XML serializer tests, all-target/all-feature Clippy and
  the full 882-case pinned XSLT gate pass without regressions.

## Increment 175: validate DOCTYPE public identifiers

- DOCTYPE public identifiers are checked against XML's PubidChar set and report
  `SEPM0016` for invalid characters before a declaration is emitted.
- Added error regressions for invalid public identifiers and system identifiers
  containing both quote delimiters, including runtime diagnostic phases.
- Validation: result-document and HTML/XML serializer tests, all-target/all-feature
  Clippy and the full 882-case pinned XSLT gate pass without regressions.

## Increment 176: serializer regression audit

- Rechecked the accumulated CDATA, encoding, output-limit and DOCTYPE changes
  against the full all-feature Rust test suite and pinned QT3 workload.
- All 18,735 recorded QT3 cases remain green across 26,966 selected cases
  (390 engine failures, 764 harness errors and 7,077 profile exclusions).
- All 43 Python harness tests, formatting and diff checks pass. The worktree
  was clean before recording this audit. The XSLT baseline remains 882 cases.
- Remaining result-document serialization gaps include URI-attribute escaping
  and HTML content-type metadata settings.

## Increment 177: URI serialization requirements from the W3C specification

- Inspected the remaining URI-escaping fixtures and serializer attribute path.
  Recorded the complete URI attribute/element table, normalization and escaping
  order, namespace constraints and character-map interactions in
  tools/conformance/URI_SERIALIZATION.md for the next implementation slice.
- No runtime behavior changed. The implementation must propagate settings
  through both named output definitions and result-document AVTs.

## Increment 178: HTML and XHTML URI attribute escaping

- URI attributes apply NFC normalization and UTF-8 percent encoding before
  markup escaping. Settings propagate through output definitions, result-document
  boolean AVTs and nested captures; URI escaping suppresses character maps.
- Ordinary input values remain text; URL input values use URI escaping.
- The pinned XSLT gate has 896 passes (14 new, zero regressions), 259 failures,
  264 harness errors and 153 exclusions.
- Validation: HTML and result-document tests, all-target/all-feature Clippy and
  the full pinned XSLT regression gate pass.

## Increment 179: URI escaping and HTML attribute character maps

- HTML attributes apply character maps unless URI escaping takes precedence.
  Disabled URI escaping now restores character maps for URI attributes.
- Added regressions for the toggle, NFC normalization, relative URI punctuation,
  ordinary attributes and namespaced attributes.
- The pinned XSLT gate has 897 passes (one new, zero regressions), 258 failures,
  264 harness errors and 153 exclusions. Focused tests and Clippy pass.

## Increment 180: case-sensitive XHTML URI attribute recognition

- XHTML URI recognition uses exact element and attribute local names in the
  XHTML namespace. HTML keeps case-insensitive recognition. Differently cased
  XHTML names no longer inherit lowercase URI serialization rules.
- Added a regression comparing lowercase href with uppercase element and
  attribute names under the XHTML serializer.
- Validation: eleven HTML/XHTML serializer tests, all-target/all-feature Clippy
  and the full 897-case pinned XSLT gate pass without regressions.

## Increment 181: named URI settings and nested capture regression

- Added a regression combining named HTML output definitions, a dynamic URI
  escaping override and nested try capture. The companion result verifies that
  an unoverridden named setting remains disabled across the same capture path.
- Validation: all twelve result-document tests, the full all-feature Rust suite,
  formatting and diff checks pass. Clippy passed during this test increment.

## Increment 182: URI escaping in record serialization

- Added a two-record streaming regression for enabled and disabled URI escaping.
  Enabled output normalizes decomposed text before percent encoding; disabled
  output preserves it. Both modes verify record and serialized-byte counts.
- Inspected temporary HTML capture construction and confirmed URI settings are
  inherited through all serializer-preserving capture paths.
- Validation: thirteen streaming tests, all-target/all-feature Clippy, formatting
  and diff checks pass. No runtime behavior changed in this increment.

## Increment 183: URI method and XHTML input-type boundaries

- XHTML URL-input recognition requires the exact `type` attribute name; HTML
  retains case-insensitive matching. Uppercase XHTML TYPE remains ordinary data.
- Added a regression proving XML output ignores URI escaping and preserves
  decomposed URI text, alongside exact XHTML input-type matching.
- Validation: twelve HTML/XHTML serializer tests, all-target/all-feature Clippy
  and the full 897-case pinned XSLT gate pass without regressions.

## Increment 184: serialization boolean diagnostics

- Dynamic boolean serialization errors identify the parameter and invalid value,
  including URI escaping, indentation, XML declaration omission and BOM settings.
- Added URI-escaping regressions for runtime invalid values and compile-time
  undeclared variables inside an unexecuted branch.
- Validation: thirteen result-document tests, all-target/all-feature Clippy,
  formatting and diff checks pass. Serialization behavior is unchanged.

## Increment 185: HTML metadata implementation requirements

- Inspected remaining metadata fixtures and the delayed-start-tag serializer.
  Recorded insertion, existing-meta replacement, namespace, capture and output
  limit requirements in tools/conformance/HTML_METADATA.md using the W3C rules.
- Metadata insertion must use node-aware suppression of existing direct child
  declarations rather than broad serialized-string replacement. Default-method
  detection remains separate from explicit HTML/XHTML serialization.
- No runtime behavior changed in this increment.

## Increment 186: HTML and XHTML content-type metadata

- HTML/XHTML head serialization inserts encoding/media metadata and suppresses
  existing direct meta children declaring Content-Type. Disabled insertion
  preserves existing metadata. Generated bytes count toward output limits.
- Include-content-type and media-type settings propagate through output
  definitions, result-document AVTs and serializer-preserving captures.
- Added direct replacement and disabled-setting regressions. The pinned XSLT
  gate has 928 passes (31 new, zero regressions), 227 failures, 264 harness errors
  and 153 exclusions. Focused serializer/result-document tests and Clippy pass.

## Increment 187: XHTML metadata namespace and case handling

- Generated XHTML meta elements reuse the head element's prefix, preserving
  their XHTML namespace when no default namespace is available.
- Metadata insertion and replacement use exact XHTML element names while HTML
  retains case-insensitive recognition. Added a prefixed-head/uppercase-head
  regression confirming exactly one generated metadata element.
- Validation: fourteen serializer tests, all-target/all-feature Clippy and the
  full 928-case pinned XSLT gate pass without regressions.

## Increment 188: exact XHTML metadata attribute recognition

- XHTML metadata replacement requires the exact unnamespaced `http-equiv`
  attribute name. HTML keeps case-insensitive attribute-name recognition.
- Added a regression retaining uppercase and namespaced attributes while
  replacing a matching lowercase declaration with case/space-varied value.
- Validation: fifteen serializer tests, all-target/all-feature Clippy and the
  full 928-case pinned XSLT gate pass without regressions.

## Increment 189: metadata replacement through nested captures

- Serializer-preserving captures inherit surrounding head context, allowing
  direct meta children inside try/populated captures to participate in metadata
  replacement even when the head itself belongs to the outer output.
- Added a failing-then-fixed regression for try capture inside an HTML head.
- Validation: sixteen serializer tests, all-target/all-feature Clippy and the
  full 928-case pinned XSLT gate pass without regressions.

## Increment 190: character maps on generated metadata

- Generated metadata attribute values use mapped attribute escaping, applying
  character maps to the Content-Type value and configured media/encoding text.
- Added a regression for a mapped character in a generated media-type value.
- Validation: character-map and serializer tests, all-target/all-feature Clippy
  and the full 928-case pinned XSLT gate pass without regressions.

## Increment 191: metadata subtree and byte-limit regressions

- Added copied-node coverage proving replacement discards the complete subtree
  of an existing Content-Type meta while preserving unrelated metadata.
- Added an output-limit comparison showing generated metadata bytes trigger
  `XPDY0130` while the same document fits when insertion is disabled.
- Validation: eighteen serializer tests, all-target/all-feature Clippy,
  formatting and diff checks pass. No runtime behavior changed.

## Increment 192: XHTML 1.0 result-document version selection

- Result-document output-version 1.0 accepts the existing XHTML serializer,
  alongside XML 1.0. Other unsupported version selections remain explicit gaps.
- The pinned XSLT gate has 929 passes (one new, zero regressions), 226 failures,
  264 harness errors and 153 exclusions. Focused result-document tests and
  all-target/all-feature Clippy pass.
- The metadata implementation audit passed the full all-feature Rust suite and
  all 18,735 recorded QT3 cases before this version-guard correction.

## Increment 193: exact XHTML void-element recognition

- The shared HTML-name helper preserves XHTML local-name case, preventing
  uppercase XHTML names from receiving lowercase void-element serialization.
  HTML retains case-insensitive behavior.
- Added a regression comparing br/img with uppercase BR/IMG in XHTML output.
- Validation: nineteen serializer tests, all-target/all-feature Clippy and the
  full 929-case pinned XSLT gate pass without regressions.

## Increment 194: reject charset parameters in media-type

- Media-type validation rejects explicit charset parameters with `SEPM0016`,
  keeping generated encoding metadata tied to the actual serializer encoding.
  Parsing respects quoted parameter values and escaped characters.
- Added dynamic charset rejection and quoted-semicolon preservation regressions.
- Validation: fourteen result-document tests, all-target/all-feature Clippy and
  the full 929-case pinned XSLT gate pass without regressions.

## Increment 195: validate streaming media types before output

- Streaming validates the selected media type before constructing its reader or
  writing its preamble. Invalid charset parameters fail even with zero records.
- Added a BOM-enabled regression verifying that rejection leaves output empty.
- Validation: fourteen streaming tests, all-target/all-feature Clippy, formatting
  and diff checks pass.

## Increment 196: explicit principal media-type overrides

- Implicit principal media-type validation is deferred until that result is
  selected, allowing explicit principal result documents to replace unused
  invalid implicit settings. Explicit result settings remain validated directly;
  streaming retains validation before preamble output.
- Added a regression overriding an implicit charset-bearing media type.
- Validation: result-document and streaming tests, all-target/all-feature Clippy
  and the full 929-case pinned XSLT gate pass without regressions.

## Increment 197: malformed media-type quoted parameters

- Media-type validation rejects unterminated quoted parameters with runtime
  `SEPM0016` instead of emitting an invalid metadata value.
- Added a regression for a result-document media type with an open quote.
- Validation: sixteen result-document tests, all-target/all-feature Clippy and
  the full 929-case pinned XSLT gate pass without regressions.

## Increment 198: dynamic metadata settings across result destinations

- Added a regression combining dynamic media type, encoding and metadata toggle
  through try capture for both principal and secondary result documents.
- The test verifies the actual US-ASCII charset, escaped title content and one
  generated metadata declaration per result.
- Validation: seventeen result-document tests, all-target/all-feature Clippy,
  formatting and diff checks pass. No runtime behavior changed.

## Increment 199: metadata replacement across streamed records

- Added a two-record regression for metadata insertion and replacement with a
  configured media type. Disabled insertion preserves each copied declaration;
  enabled insertion generates one declaration per record without duplicates.
- The regression also checks preserved titles, record counts and output bytes.
- Validation: fifteen streaming tests, all-target/all-feature Clippy, formatting
  and diff checks pass. No runtime behavior changed.

## Increment 200: namespace undeclaration serialization parameters

- Output definitions and result-document AVTs carry undeclare-prefixes. XML 1.0
  and XHTML reject enabled prefix undeclarations with SEPM0010; HTML and text
  ignore the parameter. Streaming validates before emitting its preamble.
- Added dynamic XML rejection and text-method regressions. The pinned XSLT gate
  has 931 passes (two new, zero regressions), 224 failures, 264 harness errors
  and 153 exclusions. Focused result-document tests and Clippy pass.

## Increment 201: prefix undeclaration override and streaming regressions

- Added coverage for explicit principal results disabling inherited prefix
  undeclarations, with implicit XML rejection as the control case.
- Added a zero-record streaming regression proving SEPM0010 precedes BOM output.
- Validation: full all-feature Rust suite, all-target/all-feature Clippy,
  formatting and diff checks pass. No runtime behavior changed.

## Increment 202: media-type type/subtype validation

- Serialization media types require nonempty MIME token type and subtype names.
  Missing components, embedded whitespace and additional slashes report runtime
  SEPM0016 rather than generating malformed Content-Type metadata.
- Added five malformed media-type regressions.
- Validation: twenty result-document tests, all-target/all-feature Clippy and
  the full 931-case pinned XSLT gate pass without regressions.

## Increment 203: serialization media-type parameter grammar

- Media-type parameters require valid names and token or quoted values.
  Missing separators/values, unquoted spaces, malformed quotes and control
  characters report SEPM0016; quoted spaces and escaped characters remain valid.
- Added five invalid-parameter cases and a quoted-space success case.
- Validation: twenty-one result-document tests, all-target/all-feature Clippy
  and the full 931-case pinned XSLT gate pass without regressions.

## Increment 204: RFC 2045 MIME token punctuation

- Media-type tokens use RFC 2045's printable ASCII minus tspecials rule rather
  than a narrower HTTP-style allowlist, preserving legal extension punctuation.
- Added a regression for braces in subtype and parameter values.
- Reference: https://www.rfc-editor.org/rfc/rfc2045.html#section-5.1
- Validation: twenty-two result-document tests, all-target/all-feature Clippy
  and the full 931-case pinned XSLT gate pass without regressions.

## Increment 205: ASCII whitespace in MIME parameter parsing

- Media-type validation trims only ASCII MIME whitespace rather than Unicode
  whitespace, preserving nonbreaking spaces as invalid unquoted token content.
- Added nonbreaking-space regressions for a media-type name and parameter value.
- Validation: the full all-feature Rust suite passed before this change;
  twenty-three result-document tests, all-target/all-feature Clippy and the full
  931-case pinned XSLT gate pass afterward without regressions.

## Increment 206: numbering catalog regression coverage

- Surveyed the pinned number catalog set: 108 passes, 49 engine failures and
  114 harness errors across 271 cases. Added all passing cases to the gate and
  retained the entire set in the selection so numbering gaps remain visible.
- The expanded gate has 1,039 passes across 1,843 selected cases, with 273 engine
  failures, 378 harness errors and 153 profile exclusions.
- Validation: the full expanded pinned XSLT gate passes with zero regressions.
  No runtime behavior changed.

## Increment 207: start-at numbering implementation analysis

- Traced the number catalog's largest explicit implementation gap to compiler
  rejection and unsigned runtime/formatter interfaces. Recorded signed-list AVT,
  repeated-offset rebasing, scope and streaming requirements in
  tools/conformance/NUMBER_START_AT.md using the W3C numbering rules.
- The implementation must handle signed rebasing and formatting together;
  existing large-number narrowing remains a separately identified runtime gap.
- No runtime behavior changed in this increment.

## Increment 208: signed start-at numbering

- xsl:number accepts start-at AVTs and rebases each place-marker component,
  repeating the last offset. Signed arbitrary-precision offsets avoid unsigned
  wraparound; formatting preserves negative signs. Literal and dynamic invalid
  lists report XTSE0020 and XTDE0030 respectively, with static scope validation.
- Added signed/repeated-offset and syntax regressions. The gate has 1,047 passes
  (eight new, zero regressions), 265 failures, 378 harness errors, 153 exclusions.
- Validation: numbering tests, all-target/all-feature Clippy and the full pinned
  XSLT gate pass. Existing supplied-value floating-point narrowing remains open.

## Increment 209: exact supplied numbering values

- Supplied numeric values round through the numeric implementation and retain
  arbitrary-precision integers through start-at rebasing. Large decimal formatting
  preserves digit families, width and grouping instead of narrowing to u64.
- String and boolean supplied values use numeric conversion. The gate has 1,048
  passes (one new, zero regressions), 264 failures, 378 harness errors and 153
  exclusions. Numbering tests and all-target/all-feature Clippy pass.

## Increment 210: supplied numbering conversion coverage

- Derived-string and URI atomic values use numeric conversion in xsl:number,
  alongside ordinary strings, untyped values, booleans and numeric values.
- Added regressions for exact rebasing beyond u64 with grouping, decimal ties,
  booleans and derived-string supplied values.
- Validation: full all-feature Rust suite, all-target/all-feature Clippy and the
  full 1,048-case pinned XSLT gate pass without regressions.

## Increment 211: punctuation-only numbering pictures

- Number pictures containing no alphanumeric token retain their punctuation as
  both prefix and suffix, with the default decimal token between them.
- Added multi-component punctuation-only and empty-picture regressions.
- Validation: eight numbering tests, all-target/all-feature Clippy and the full
  1,048-case pinned XSLT gate pass without regressions.

## Increment 212: affixes for empty numbering place markers

- Empty numbering place markers pass through picture formatting rather than
  returning early, preserving prefixes and suffixes such as the dot in A.
- Added computed-empty and supplied-empty marker regressions. The gate has
  1,072 passes (24 new, zero regressions), 240 failures, 378 harness errors and
  153 exclusions. Nine numbering tests and all-target/all-feature Clippy pass.

## Increment 213: count parentless roots once in any-level numbering

- Any-level numbering no longer prepends the root to a traversal that already
  includes it, avoiding duplicate counts for matching parentless element roots.
- Added a temporary-tree regression. The gate has 1,074 passes (two new, zero
  regressions), 238 failures, 378 harness errors and 153 exclusions.
- Validation: ten numbering tests, all-target/all-feature Clippy and the full
  pinned XSLT regression gate pass.

## Increment 214: any-level numbering from attribute nodes

- Any-level numbering reaches attribute context nodes via their parent boundary
  and considers the selected attribute itself without counting unrelated prior
  attributes. Preceding and ancestor element counts remain in document order.
- Added ancestor-count and default attribute-count regressions. The gate has
  1,077 passes (three new, zero regressions), 235 failures, 378 harness errors
  and 153 exclusions. Eleven numbering tests and Clippy pass.

## Increment 215: inclusive numbering from-boundary regression

- Confirmed the W3C rule includes the from-boundary node when it also matches
  count. Added a regression spanning single, multiple and any numbering levels,
  including sibling positions of the matching boundary itself.
- Reference: https://www.w3.org/TR/xslt-30/#number
- Validation: twelve numbering tests, all-target/all-feature Clippy, formatting
  and diff checks pass. No runtime behavior changed.

## Increment 216: numbering letter-value validation

- xsl:number retains letter-value AVTs, validates literals as alphabetic or
  traditional, validates dynamic values at runtime and checks expression scope.
  Existing supported numbering sequence formatting remains unchanged.
- Added literal, dynamic and unexecuted-branch scope error regressions.
- Validation: thirteen numbering tests, all-target/all-feature Clippy and the
  full 1,077-case pinned XSLT gate pass without regressions.

## Increment 217: numbering language AVT validation

- xsl:number retains and evaluates language AVTs, validates language syntax
  with the existing language facets and checks static variable scope. Valid
  unsupported languages retain the existing formatter fallback.
- Added literal, dynamic and scope regressions. The gate has 1,078 passes
  (one new, zero regressions), 234 failures, 378 harness errors, 153 exclusions.
- Validation: fourteen numbering tests, all-target/all-feature Clippy and the
  full pinned XSLT gate pass.

## Increment 218: numbering regression audit and implementation status

- Rechecked numbering changes against the full all-feature Rust suite and QT3
  workload: all 18,735 recorded QT3 cases remain green across 26,966 selected
  cases. All 43 Python harness tests, formatting and diff checks pass.
- Updated the start-at analysis document to reflect implemented signed rebasing,
  exact supplied numeric conversion, current streaming rejection and remaining
  large non-decimal formatting limits. No runtime behavior changed.

## Increment 219: arbitrary-precision alphabetic numbering

- Alphabetic numbering pictures use arbitrary-precision bijective base-26
  conversion above u64, retaining uppercase/lowercase sequences instead of
  falling back to decimal digits. Updated remaining formatter-limit notes.
- Added large uppercase and lowercase regressions.
- Validation: fifteen numbering tests, all-target/all-feature Clippy and the
  full 1,078-case pinned XSLT gate pass without regressions.

## Increment 220: shared arbitrary-precision integer formatting

- fn:format-integer shares the large decimal/alphabetic token formatter with
  xsl:number instead of rejecting values outside u64. Decimal signs, grouping,
  width and digit families remain intact; large ASCII ordinals retain suffixes.
- Added large grouped and negative format-integer regressions.
- Validation: numbering/numeric tests, all-target/all-feature Clippy, the full
  1,078-case XSLT gate and all 18,735 QT3 baseline cases pass without regressions.

## Increment 221: zero-only numbering picture extension regression

- Confirmed zero-only decimal pictures are an existing supported numbering
  extension, including the pinned large-number fixture. Added direct coverage
  retaining padding for both zero-only and zero-followed-by-one pictures.
- A stricter token experiment regressed number-0111 and was discarded; shared
  runtime formatting remains unchanged.
- Validation: sixteen numbering tests and the full 1,078-case pinned XSLT gate
  pass without regressions. Formatting and diff checks pass.

## Increment 222: grouping catalog regression coverage

- Surveyed the pinned for-each-group set: 40 passes, 39 engine failures and six
  harness errors across 85 cases. Added passing cases to the gate while keeping
  the complete set selected and failures visible.
- The expanded gate has 1,118 passes across 1,928 cases, with 273 engine failures,
  384 harness errors and 153 profile exclusions.
- Validation: the full expanded XSLT regression gate passes without regressions.
  No runtime behavior changed.

## Increment 223: composite grouping sequence keys

- Group-by and group-adjacent support composite sequence keys, comparing whole
  atomized sequences including empty keys. Non-composite cardinality behavior
  and first-appearance/consecutive grouping order remain intact.
- Added sequence-key and empty-adjacent-key regressions. The gate has 1,124 passes
  (six new, zero regressions), 267 failures, 384 harness errors, 153 exclusions.
- Validation: six grouping tests, all-target/all-feature Clippy and the full
  pinned XSLT regression gate pass.

## Increment 224: codepoint grouping collation and declaration checks

- Grouping accepts an explicit codepoint collation, matching its existing key
  comparator. Other collations remain explicit implementation gaps.
- Grouping-attribute cardinality is checked before collation support; collation
  with starting/ending patterns reports XTSE1090. Added diagnostic-order and
  explicit-codepoint regressions.
- Validation: seven grouping tests, all-target/all-feature Clippy and the full
  1,124-case pinned XSLT gate pass without regressions.

## Increment 225: modern atomic pattern grouping compatibility regression

- Added coverage confirming atomic populations work for pattern grouping in both
  version 2.0 and 3.0 stylesheets under the XSLT 3.0 processor behavior.
- A stylesheet-version restriction experiment regressed the paired W3C 046a
  fixture and was discarded. The catalog's processor-profile distinction must
  not be inferred from the stylesheet version.
- Validation: eight grouping tests, formatting and diff checks pass. No runtime
  behavior changed.

## Increment 226: grouping key array atomization

- Group-by and group-adjacent flatten array keys through the checked atomization
  path before cardinality checks and comparison. Maps/functions report FOTY0013
  instead of reaching atomic-key comparison as unsupported item kinds.
- Added composite-array, adjacent single-array and map-key error regressions.
- Validation: nine grouping tests, all-target/all-feature Clippy and the full
  1,124-case pinned XSLT gate pass without regressions.

## Increment 227: grouping key string conversion and composite exposure

- Untyped grouping keys convert to strings before comparison and storage,
  ensuring current-grouping-key exposes xs:string rather than xs:untypedAtomic.
- Added key-type coverage and a composite NaN regression checking complete key
  exposure and equal-NaN component grouping.
- Validation: ten grouping tests, all-target/all-feature Clippy and the full
  1,124-case pinned XSLT gate pass without regressions.

## Session checkpoint: 2026-10-30

- Implementation milestones through increment 227 are committed and pushed.
- Final pinned XSLT gate: 1,124 passes across 1,928 selected cases, zero baseline
  regressions; 267 engine failures, 384 harness errors, 153 profile exclusions.
- A final full Rust test rerun could not finish: the linker reported ENOSPC
  (no space left on device). Earlier focused grouping tests and Clippy passed
  as recorded above; the full rerun remains outstanding after freeing space.
- Next investigation: simplified stylesheets with literal result element roots,
  which currently hide additional grouping successes and static diagnostics.

### Checkpoint validation follow-up

- Cleared generated Cargo build artifacts with `cargo clean` to resolve the
  disk-space failure, then rebuilt with incremental compilation and debug info
  disabled and two build jobs.
- The full `cargo test --quiet -j 2` run passes. All 43 Python conformance harness
  unit tests also pass. The outstanding full Rust rerun above is now resolved.

## Increment 228: simplified principal stylesheets

- Literal result element roots with xsl:version compile as an implicit document
  template in the unnamed mode, using the existing sequence constructor and
  static validation. Missing xsl:version reports XTSE0150.
- Preserve namespaces, local variables, attribute value templates and version
  1.0 backwards compatibility. Literal xsl:version attributes are excluded from
  output; literal default-mode does not relocate the implicit template.
- Added five direct regressions. The pinned gate gains 21 cases (18 grouping,
  two numbering and one variable), reaching 1,145 passes across 1,928 cases with
  246 engine failures, 384 harness errors and 153 profile exclusions.
- Validation: full Rust suite, all-target/all-feature Clippy, and the expanded
  pinned XSLT gate pass without regressions. Simplified imported/included module
  handling remains a separate declaration-loader gap.

## Increment 229: simplified imported and included modules

- The declaration loader accepts literal result element modules, creating the
  same implicit document template used by simplified principal stylesheets.
  Module root/version validation and implicit-template construction are shared.
- Imported literal templates retain lower precedence and can be reached with
  apply-imports. Included literal templates share the including module's
  precedence and follow declaration order. Loader errors preserve XTSE0150 and
  static variable diagnostics.
- Validation: eight simplified stylesheet tests, four import tests,
  all-target/all-feature Clippy and the full 1,145-case pinned XSLT gate pass
  without regressions. This resolves the module-loader gap noted in increment
  228; broader mixed-version runtime behavior remains unchanged.

## Increment 230: checked sort stability settings

- Only the first sibling sort key may specify stable; later declarations report
  XTSE1017. Literal boolean values are checked at compile time (XTSE0020), and
  attribute value templates are evaluated and checked at runtime (XTDE0030).
- Stable ordering is retained for equal keys even when stable is false, an
  allowed implementation choice. All dynamic sort settings now receive static
  variable-scope validation, including settings in perform-sort expressions.
- Added literal/computed boolean, equal-key order, misplaced attribute and
  undeclared-variable regressions. Two grouping catalog cases join the gate,
  reaching 1,147 passes, 244 failures, 384 harness errors and 153 exclusions.
- Validation: thirteen focused sorting/grouping tests, all-target/all-feature
  Clippy, the XSLT gate and all 18,735 pinned QT3 cases pass without regressions.

## Increment 231: grouping function restrictions in patterns

- Parsed patterns reject current-group and current-grouping-key references with
  XTSE1060 and XTSE1070, including nested predicates, conditional branches,
  arrays/maps and function references. The shared parser covers template,
  numbering, key and grouping patterns.
- AST inspection preserves harmless function text inside strings/comments and
  recognizes built-in namespace aliases. Indirect calls through user functions
  still require separate dynamic-context handling.
- Validation: 34 focused pattern/grouping tests, all-target/all-feature Clippy
  and the pinned XSLT gate pass without regressions. Two grouping catalog cases
  join the gate: 1,149 passes, 242 engine failures, 384 harness errors and 153
  profile exclusions across 1,928 selected cases.

## Increment 232: composite declaration restrictions

- Pattern grouping rejects an explicitly supplied composite attribute with
  XTSE1090, including false/no/zero values. The restriction concerns attribute
  presence, rather than whether composite grouping was enabled.
- Shared checking covers both starting and ending patterns and retains the
  existing collation restriction. Explicit false composite settings remain
  valid for group-by and group-adjacent.
- Validation: eleven grouping tests, all-target/all-feature Clippy and the full
  1,149-case pinned XSLT gate pass without regressions.

## Increment 233: grouping context at function boundaries

- Inline functions exclude current group/key entries from closures. Dynamic
  calls through named function values clear the caller's grouping context,
  reporting XTDE1061/XTDE1071 for current-group/current-grouping-key calls.
- Explicit lexical variables holding group/key values remain available to
  closures. Scope filtering avoids copying discarded group sequences, and
  ordinary named calls avoid scope copies when no grouping context exists.
- Added direct named, inline and higher-order call regressions plus lexical
  capture coverage. Two catalog cases join the XSLT gate: 1,151 passes, 240
  engine failures, 384 harness errors and 153 exclusions across 1,928 cases.
- Validation: full Rust suite, all-target/all-feature Clippy, the expanded XSLT
  gate and all 18,735 pinned QT3 cases pass without regressions.

## Grouping profile audit

- The remaining for-each-group-015a, -046 and -081a expectations target an
  XSLT 2.0 processor. Their paired XSLT 3.0 cases test the same stylesheets and
  already pass; stylesheet version must not be used as processor version.
- The catalog runner currently accepts both processor profiles. Restricting it
  to XSLT 3.0 also excludes three existing XSLT 2.0-only passing baseline cases
  (match-039, match-082 and match-087). A profile correction should preserve
  their regression coverage separately and report profile changes explicitly.

## Increment 234: catalog processor profile selection

- Catalog spec dependencies now select an XSLT 3.0 processor, respecting exact
  versions, minimum-version suffixes, alternatives and negative dependencies.
  Older-processor cases are excluded rather than reported as engine failures.
- Removed three XSLT 2.0-only pattern cases from the applicable catalog baseline;
  their exact pattern checks remain in the existing Rust versioned syntax test.
- Validation: 44 Python harness tests, the Rust pattern regression and the full
  revised XSLT gate pass. Across 1,928 selected cases: 1,148 passes, 220 engine
  failures, 359 harness errors and 201 profile exclusions; no applicable baseline
  regressions. The changed totals reflect profile selection, not engine gains.

## Increment 235: grouped tunnel parameter whitespace regression

- Investigated for-each-group-085: the engine constructs and tunnels the two
  expected grouped nodes, but also copies the source's whitespace-only text via
  built-in template rules. The catalog's expected XML omits that whitespace.
- Added a focused regression for typed grouped sequences, tunnel parameters and
  preservation of source whitespace. The case remains an XML mismatch in the
  catalog; changing comparison or source whitespace policy requires separate
  evidence about the suite's intended comparison/input configuration.
- Validation: all thirteen grouping tests pass. No runtime behavior changed.

### Increment 235 follow-up: catalog comparison evidence

- The pinned suite's `admin/catalog-schema.xsd` specifies canonical XML or
  deep-equal comparison for assert-xml. Its `runner/compare.xsl` compares text
  nodes and child counts explicitly, without stripping whitespace. The harness's
  strict comparison is therefore consistent with that documented assertion.
- The fixture's expected tree omits whitespace present in the supplied XML and
  copied by built-in templates. Keep the mismatch visible; the preceding Rust
  regression preserves the source-whitespace behavior.

## Increment 236: obsolete grouping binding attributes

- Reject the former draft attributes bind-group and bind-grouping-key on
  xsl:for-each-group with XTSE0090, rather than reporting an implementation
  limitation. Other unsupported attributes retain their existing diagnostics.
- Validation: fourteen grouping tests, all-target/all-feature Clippy and the
  pinned XSLT gate pass without regressions. One catalog case joins the baseline,
  reaching 1,149 passes, 219 engine failures, 359 harness errors and 201 profile
  exclusions among 1,928 selected cases.

## Increment 237: pure branching stylesheet function bodies

- Lower pure function sequence constructors to XPath expressions, supporting
  multiple xsl:sequence instructions, xsl:if and xsl:choose, and local select
  bindings with branch-local scope. Existing expression-only recursive functions
  avoid extra nesting and retain their previous behavior.
- Added branch, sequence order, variable scope and static reference checks.
  General instruction and node-construction function bodies remain unsupported;
  the grouping catalog's recursive constructor case still requires that work.
- Validation: full Rust suite, all-target/all-feature Clippy and the 1,149-case
  pinned XSLT gate pass without regressions.

## Increment 238: typed function-local variables

- Preserve `as` declarations when lowering local select bindings in stylesheet
  functions. Values now use sequence-type function conversion and report
  XTTE0570 when conversion fails, including inside conditional branches.
- Added untyped-node conversion, invalid value and unexecuted-branch coverage.
  This also corrects typed local bindings accepted before increment 237.
- Validation: five stylesheet-function tests, all-target/all-feature Clippy
  and the full 1,149-case pinned XSLT gate pass without regressions.

## Increment 239: pure for-each in stylesheet functions

- Lower xsl:for-each with pure sequence-constructor bodies into an instruction
  map, preserving population order, optional sorting, position/size and the
  per-instruction current item, including in nested loops.
- Added sorted, empty and nested atomic-population regressions. General node
  construction and grouping instructions in functions remain unsupported.
- Validation: six stylesheet-function tests, all-target/all-feature Clippy,
  the 1,149-case XSLT gate and all 18,735 pinned QT3 cases pass without
  regressions.

## Increment 240: stylesheet function catalog coverage

- Surveyed and selected the full stylesheet function declaration set: 52
  passes, 24 engine failures, 12 harness errors and 22 profile exclusions across
  110 cases. Added all passing cases to the pinned gate.
- The expanded gate now records 1,201 passes, 243 engine failures, 371 harness
  errors and 223 profile exclusions across 2,038 selected cases. Remaining
  failures include general constructors, recursion depth and declaration checks.
- Validation: the full expanded XSLT gate passes without regressions. No runtime
  behavior changed.

## Increment 241: stylesheet function declaration values

- Validate function override, override-extension-function and cache as boolean
  attributes; check that the two override synonyms agree. Validate new-each-time
  as boolean or `maybe`. Invalid settings report XTSE0020.
- Added valid aliases, conflicting synonyms, invalid spellings and `maybe`
  coverage. Four catalog cases join the baseline: 1,205 passes, 239 engine
  failures, 371 harness errors and 223 exclusions across 2,038 selected cases.
- Validation: seven stylesheet-function tests, all-target/all-feature Clippy
  and the expanded pinned XSLT gate pass without regressions.

## Increment 242: function parameter declarations

- Accept explicit required=yes on stylesheet function parameters, reject false
  required and true tunnel/static settings with XTSE0020, and reject defaults
  or preserved non-empty content with XTSE0760. Unqualified function names now
  report the specified XTSE0740 instead of XTSE0080.
- Added required aliases, default-value, preserved whitespace and no-namespace
  regressions. Three catalog cases join the baseline: 1,208 passes, 236 engine
  failures, 371 harness errors and 223 exclusions across 2,038 selected cases.
- Validation: eight stylesheet-function tests, all-target/all-feature Clippy and
  the expanded pinned XSLT gate pass without regressions.

## Increment 243: function declaration placement

- Nested xsl:function declarations and function parameters after body content
  report XTSE0010. Added direct regressions for both placement rules.
- Validation: nine stylesheet-function tests, all-target/all-feature Clippy and
  the pinned XSLT gate pass. Two catalog cases join the baseline: 1,210 passes,
  234 engine failures, 371 harness errors and 223 profile exclusions across
  2,038 selected cases.

## Increment 244: obsolete xsl:sequence type attribute

- Reject the obsolete `as` attribute on xsl:sequence with XTSE0090 across
  sequence constructors, including stylesheet functions and templates.
- Validation: ten stylesheet-function tests, all-target/all-feature Clippy and
  the pinned XSLT gate pass. One catalog case joins the baseline: 1,211 passes,
  233 engine failures, 371 harness errors and 223 profile exclusions across
  2,038 selected cases.

## Investigation: availability function namespace context

- The `function-0302b` catalog failure includes an unprefixed dynamically
  computed name evaluated where the default namespace is the XSLT namespace.
  `element-available()` currently recognizes the literal `xsl:` prefix and
  the XSLT URIQualifiedName spelling instead of resolving the supplied name
  against the call site's static namespace context. This also misclassifies
  aliases and a rebound `xsl` prefix.
- A complete fix must retain the static context for dynamically computed names
  and first-class function references, validate names and unbound prefixes,
  and check default namespace handling. The related `system-property()`,
  `function-available()` and `type-available()` implementations also contain
  prefix-based dispatch and need independent review of their namespace rules.
  No behavior or conformance baseline has changed in this investigation.

## Increment 245: direct element-available namespace resolution

- Direct calls resolve dynamically computed names using the expression's static
  namespace bindings, including the default namespace; invalid or unbound names
  report XTDE1440. Added alias, rebound prefix and default namespace regressions.
- The `function-0302b` catalog case joins the baseline: 1,212 passes, 232 engine
  failures, 371 harness errors and 223 exclusions among 2,038 selected cases.
  The previous 1,211-case gate had no regressions. All-target/all-feature Clippy
  and the use-when integration tests pass.
- First-class references to `element-available` still do not capture this static
  context, and the other availability functions still need namespace review.

## Increment 246: element-available function references

- Named references and partial applications of `element-available` retain their
  creation site's namespace bindings, including when invoked from another
  context. Added reference, partial application and function-name coverage.
- Six use-when tests, all-target/all-feature Clippy and the 1,212-case XSLT
  baseline pass. Dynamic `function-lookup` and the other availability functions
  still require review for static-context capture.

## Increment 247: lookup context for element-available

- `function-lookup` captures the lookup site's namespace bindings for returned
  `element-available` and `function-lookup` items, including recursive lookup.
  Immediate dynamic calls and lookup references retain those bindings.
- Added direct, referenced and immediate lookup coverage, and strengthened the
  named-reference test by rebinding the prefix at the invocation site. Seven
  use-when tests, all-target/all-feature Clippy and the 1,212-case XSLT gate pass.
- Other namespace-sensitive availability functions remain to be reviewed.

## Increment 248: system-property namespace resolution

- Resolve `system-property` names using static namespace bindings for direct
  calls, named references, partial applications and dynamic lookup. Unprefixed
  names remain in no namespace even when a default namespace is in scope;
  invalid or unbound names report XTDE1390.
- Added alias, rebinding, default namespace and lookup regressions. Eight
  use-when tests, all-target/all-feature Clippy and the 1,212-case XSLT gate pass.
  `function-available` and `type-available` still need namespace review.

## Increment 249: type-available namespace resolution

- Resolve type names against static namespaces, including the default namespace,
  for direct calls, references, partial applications and dynamic lookup. Invalid
  or unbound names report XTDE1428. Built-in type checks use expanded names.
- Consolidated namespace-sensitive expression handling after an initial catalog
  run revealed a recursion stack overflow from a separate evaluator branch. The
  revised evaluator passes the previously failing catalog run.
- Nine use-when tests, all-target/all-feature Clippy and the 1,212-case XSLT
  gate pass. Imported schema types remain outside the current implementation;
  `function-available` still needs namespace review.

## Increment 250: function-available namespace resolution

- Resolve function names using the call site's namespace bindings, with the
  standard function namespace for unprefixed names and XTDE1400 for invalid or
  unbound names. References, partial applications and dynamic lookup retain
  those bindings. Stylesheet functions are checked by expanded name and arity.
- Added alias, rebound prefix, stylesheet function, arity and lookup coverage.
  Ten use-when tests, all-target/all-feature Clippy and the 1,212-case XSLT
  gate pass. Broader function signature conversion and package visibility
  behavior remain subject to the engine's existing implementation limits.

## Increment 251: function-available arity conversion

- Require exactly one integer when an arity is supplied, convert untyped XML
  values to integers, and return false for negative or oversized arities instead
  of treating them as omitted or matching a variadic function.
- Eleven use-when tests, all-target/all-feature Clippy and the 1,212-case XSLT
  gate pass. Added invalid type/cardinality and untyped-attribute coverage.

## Increment 252: availability EQName validation (corrected in increment 255)

- Added malformed-name and unbound-prefix checks for the four availability and
  system-property functions. The original whitespace rejection was incorrect;
  increment 255 restores the specification’s whitespace handling.

## Increment 253: format-integer argument conversion

- Validate the required picture and optional language argument types before
  returning for an empty integer. Invalid types and language cardinalities now
  report XPTY0004; empty language remains accepted. Language-specific formatting
  continues to use the existing fallback behavior.
- Focused integer-format tests, all-target/all-feature Clippy and the 1,212-case
  XSLT gate pass. Catalog recursion cases still need deeper execution and caching
  support rather than a higher unchecked call-stack limit.

## Increment 254: QName default namespace regression

- Keep XML's default namespace available to XSLT availability functions without
  applying it to unprefixed `xs:QName()` casts; those use the XPath default
  element/type namespace. An existing QName scope regression caught the issue.
- The complete all-feature Rust suite, all-target/all-feature Clippy, the
  1,212-case XSLT gate and the 18,735-case QT3 gate pass. An initial test run
  exhausted disk space during linking; clearing generated Cargo artifacts and
  disabling incremental compilation allowed the full run to complete.


## Increment 255: extension attributes and EQName whitespace

- Ignore non-XSLT namespaced extension attributes on XSLT elements, as permitted
  by the specification. This removes the optimization-hint obstacle in catalog
  function-1201, though its constructed function result remains unsupported.
- Correct increment 252: XSLT EQNames allow surrounding XML whitespace, which is
  now stripped before validation. Malformed names and unbound prefixes still
  report the applicable dynamic errors.
- The full all-feature Rust suite, all-target/all-feature Clippy and the
  1,212-case XSLT gate pass without regressions.

## Increment 256: built-in schema type availability

- Recognize the built-in `xs:anyType`, `xs:anySimpleType`, `xs:untyped` and
  `xs:untypedAtomic` names when checking type availability, alongside the
  existing atomic and temporal types. Unknown names continue to return false.
- Surveyed the seven-case type-available catalog set: two pass, two are harness
  errors for unsupported XSD-version dependencies, and three are profile
  exclusions. Added both passing cases to the gate, which now has 1,214 passes,
  232 failures, 373 harness errors and 226 exclusions across 2,045 selected cases.
- Focused type-availability tests, all-target/all-feature Clippy and the expanded
  XSLT gate pass. User-imported schema types remain outside the basic processor.

## Increment 257: function-available catalog coverage

- Surveyed all ten function-available catalog cases: six pass, three fail due to
  unsupported general instructions in stylesheet functions, and one is a harness
  error for an unrecognized higher-order-functions dependency. The latter also
  checks a capability property whose conformance claim needs separate review.
- Added the six passing cases to the pinned gate. It now records 1,220 passes,
  235 engine failures, 374 harness errors and 226 profile exclusions across
  2,055 selected cases. The expanded gate passes without regressions.

## Increment 258: xsl:evaluate static function library

- Parse xsl:evaluate target expressions with the XPath function library, so
  direct XSLT-only function calls and references produce XTDE3160, while XPath
  built-ins and stylesheet functions remain available. The normal stylesheet
  function library is restored after parsing.
- Surveyed all 27 system-property cases: 15 pass, ten have a harness error for
  the higher-order-functions dependency, and two are profile exclusions. Added
  the 15 passes to the gate: 1,235 passes, 235 engine failures, 384 harness
  errors and 228 exclusions across 2,082 selected cases.
- Dynamic-instruction tests, all-target/all-feature Clippy and the expanded
  XSLT gate pass. Dynamic function lookup and target variable visibility still
  need separate review against the xsl:evaluate static context rules.

## Increment 259: xsl:evaluate target variable scope

- Validate target-expression variable references against supplied parameters and
  lexical bindings, including in branches that are not executed. Unbound names
  now produce XTDE3160, and stylesheet globals are not directly in scope.
  Called stylesheet functions still retain access to their own globals.
- Reused the existing lexical-reference validator for dynamically parsed XPath.
  Five dynamic-instruction tests, all-target/all-feature Clippy and the 1,235-case
  XSLT gate pass. Dynamic with-params maps remain unsupported.


## Increment 260: xsl:evaluate parameter maps

- Support `with-params` maps keyed by QNames, with map values overriding matching
  child parameters and static scope checks including map keys. Wrong map types
  or non-QName keys report XTTE3165; map values can be arbitrary sequences.
- Recognize the enabled dynamic-evaluation dependency in the catalog runner. The
  57-case evaluate survey has 27 passes, 14 failures, one harness error and 15
  profile exclusions. Added the passes to the expanded gate: 1,262 passes, 249
  failures, 385 harness errors and 243 exclusions across 2,139 selected cases.
- Extracted dynamic evaluation from the main evaluator after an initial run
  exposed a recursive stack overflow. Focused tests, runner tests, Clippy and
  the expanded XSLT gate then passed. Namespace context, base URI and typed
  parameters remain among the unsupported xsl:evaluate features.

## Increment 261: empty dynamic context

- An empty `xsl:evaluate` context-item now leaves the dynamic context item
  absent; accessing it reports XPDY0002. Multiple items still report XTTE3210.
- The dynamic-instruction integration tests pass, including the empty and
  multiple-item cases. Namespace context and base URI remain unsupported.

## Increment 262: dynamic namespace context

- Support `xsl:evaluate` namespace-context using the selected node's in-scope
  bindings and default element/type namespace, independently of the stylesheet
  bindings. Non-node or non-singleton results report XTTE3170. Static variable
  validation includes the namespace-context expression.
- Four additional evaluate catalog cases pass and are added to the gate, which
  now has 1,266 passes, 245 failures, 385 harness errors and 243 exclusions
  across 2,139 selected cases. The full all-feature Rust suite, all-target
  Clippy and the expanded gate pass. Base URI and typed parameters remain gaps.

## Increment 263: dynamic evaluation type conversion

- Apply declared types to select-based child parameters and to the result of
  `xsl:evaluate`, including normal function conversion and type errors. Map
  parameters still override child values. Validate the target XPath string's
  type, and evaluate the target with XPath 1.0 compatibility disabled.
- Added one passing evaluate case to the gate: 1,267 passes, 244 failures, 385
  harness errors and 243 exclusions across 2,139 selected cases. Focused tests,
  all-target/all-feature Clippy and the expanded gate pass. Base URI and
  sequence-constructor child parameters remain unsupported.

## Increment 264: dynamic function visibility

- Check direct stylesheet function calls and references in dynamic target
  expressions against function visibility, including in unexecuted branches.
  Private and hidden functions report XTDE3160; public and final functions
  remain available. Dynamic function-lookup visibility still needs review.
- Added the passing evaluate-045 case to the gate, now 1,268 passes, 243
  failures, 385 harness errors and 243 exclusions across 2,139 selected cases.
  Focused tests, all-target/all-feature Clippy and the expanded gate pass.
  General static base URI propagation and dynamic resource resolution remain
  unfinished.

## Increment 265: dynamic function lookup

- Apply the target XPath function library and stylesheet function visibility to
  `function-lookup`, including lookup function items called after they leave
  `xsl:evaluate`. Ordinary stylesheet lookups keep their own function library.
- Focused all-feature dynamic-instruction and XPath-profile tests, all-target
  Clippy, and the 1,268-case XSLT gate pass. Base URI support remains unfinished.

## Increment 266: explicit dynamic base URI

- Evaluate the `base-uri` attribute as an AVT and carry its value into the
  target's `static-base-uri()`, implicit `resolve-uri()`, and text/JSON resource
  resolution. Function references capture that context for later calls.
- Two additional evaluate cases pass and are included in the 1,270-case gate;
  the selected set has 241 failures, 385 harness errors and 243 exclusions.
  Focused all-feature tests, all-target Clippy and the expanded gate pass.
  Inherited stylesheet base URIs and dynamic XML document loading remain gaps.

## Increment 267: stylesheet xml:base for dynamic evaluation

- When no `base-uri` attribute is supplied, derive the target's base URI from
  an absolute stylesheet `xml:base` and any nested relative `xml:base` values.
  An explicit `base-uri` still takes precedence.
- Focused all-feature tests, all-target/all-feature Clippy and the 1,270-case
  gate pass. The compile API does not supply a stylesheet document URI, so
  location-based fallback and dynamic XML document loading remain gaps.

## Increment 268: source-node base URI

- Resolve absolute and nested relative `xml:base` values for `base-uri()` on
  source nodes, including attributes and text. Both `base-uri()` and
  `document-uri()` now check their argument type and cardinality. A dynamic
  evaluation regression checks a source-node base URI used in an AVT.
- The selected QT3 gate now contains 18,742 passes across 27,020 cases, with
  406 failures, 764 harness errors and 7,108 exclusions. Focused all-feature
  tests, Clippy and the 1,270-case XSLT gate pass. Document-origin URIs and
  dynamic XML document loading remain unsupported.

## Increment 269: loaded document origins

- Preserve an absolute resolver URI on preloaded document nodes and propagate it
  to descendants. `document-uri()` returns the origin for document nodes, and
  `base-uri()` resolves descendant `xml:base` values against that origin.
  Relative resolver names still have no known absolute document URI.
- The all-feature Rust suite, all-target/all-feature Clippy, 1,270-case XSLT
  gate and 18,742-case QT3 gate pass. Loading XML documents whose URIs are
  discovered only during evaluation remains unsupported.

## Increment 270: document function argument checks

- Validate the optional URI arguments of `doc()` and `doc-available()` using
  function conversion, including empty sequences and cardinality errors.
- Added the 24 passing `fn-doc` and `fn-doc-available` QT3 cases to the gate:
  18,766 passes, 417 failures, 784 harness errors and 7,115 exclusions across
  27,082 cases. Focused tests, all-target/all-feature Clippy and both catalog
  gates pass. Dynamic document loading still requires an evaluation context
  that can access the per-transformation arena without changing node lifetimes.

## Increment 271: stylesheet function argument errors

- Use XPath's XPTY0004 when stylesheet function argument conversion fails,
  including direct calls, function references and calls from `xsl:evaluate`.
- Added evaluate-022 to the XSLT gate, now 1,271 passes, 240 failures, 385
  harness errors and 243 exclusions across 2,139 selected cases. Focused
  all-feature tests, all-target/all-feature Clippy and the expanded gate pass.

## Increment 272: expanded-text namespace scope

- Compile expressions in expanded text with their parent element's namespace
  bindings, so prefixed names and XSLT availability functions resolve correctly
  inside text value templates, including those in `xsl:comment`.
- Added evaluate-052 to the XSLT gate, now 1,272 passes, 239 failures, 385
  harness errors and 243 exclusions across 2,139 selected cases. Focused
  all-feature tests, all-target/all-feature Clippy and the expanded gate pass.

## Increment 273: dynamic evaluation parameter checks

- Reject duplicate `xsl:evaluate` child parameters by expanded QName with
  XTSE0670, and reject tunnel parameters. Explicit `tunnel="no"` remains valid.
- Focused all-feature tests, all-target/all-feature Clippy and the 1,272-case
  XSLT gate pass. General function-body execution, source document URIs and
  default collation support remain among the dynamic evaluation gaps.

## Increment 274: dynamic evaluation schema-aware attribute

- Evaluate and validate `schema-aware` as a boolean AVT, with static and dynamic
  errors for invalid values and ordinary lexical-scope checks. The non-schema-
  aware processor continues to expose only its built-in schema types.
- Focused all-feature tests, all-target/all-feature Clippy and the 1,272-case
  XSLT gate pass. Schema import and validation remain outside this milestone.

## Increment 275: constructed dynamic parameters

- Allow `xsl:evaluate` child parameters to use sequence constructors, reusing
  the existing binding and temporary-tree machinery. Typed values, caller
  scope, and map overrides are covered; type failures report XTTE0590.
- The full all-feature Rust suite, all-target/all-feature Clippy and the
  1,272-case XSLT gate pass. Existing general constructor limitations and
  stylesheet-function body limitations still apply.

## Increment 276: principal source URI

- Add `TransformOptions::source_uri` for an absolute principal source URI.
  `document-uri()` and inherited `base-uri()` use it, and looking the URI up
  with `doc()` returns the original source document without invoking the resolver.
- Pass file-backed source URIs through both development catalog runners. Added
  evaluate-015 and 27 QT3 cases to their gates: 1,273 XSLT and 18,793 QT3
  passing cases. The full all-feature Rust suite, runner tests, Clippy and both
  expanded gates pass. Arbitrary dynamically discovered XML URIs still need a
  document-loading context integrated with the per-transformation arena.

## Increment 277: dynamic evaluation fallback and static exclusion

- Ignore `xsl:fallback` children when dynamic evaluation is enabled, and apply
  `use-when` exclusion before validating `xsl:evaluate` child parameters.
- Focused all-feature tests, all-target/all-feature Clippy and the 1,273-case
  XSLT gate pass. A node-only loader callback was investigated but still made
  XPath values lifetime-invariant; dynamic XML loading remains unfinished.

## Increment 278: stylesheet module URI context

- Add compilation entry points accepting an absolute stylesheet URI, with an
  optional module resolver. Includes and imports resolve their URIs against the
  containing module and `xml:base`; dynamic expressions inherit their module's
  base URI. Compilation restores module context after nested modules or errors.
- Focused all-feature dynamic-evaluation and module tests, all-target/all-feature
  Clippy and the 1,273-case XSLT gate pass. Ordinary stylesheet expressions do
  not yet receive general static-base-URI propagation; dynamic XML loading also
  remains unfinished.

## Increment 279: static base in XPath resource calls

- Carry the containing module's static base into ordinary `static-base-uri()`,
  `resolve-uri()`, `doc()`, `doc-available()`, text and JSON resource calls,
  including function references and lookup. Resolve literal `doc()` URIs before
  preloading and preserve `doc()`-anchored patterns.
- Extracted the context-aware call path from the main evaluator after an
  initial full run exposed a recursive stack overflow. The full all-feature Rust
  suite, all-target/all-feature Clippy, 1,273-case XSLT gate and 18,793-case QT3
  gate pass. Dynamically discovered XML URIs and general `document()` URI
  resolution remain unfinished.

## Increment 280: document URI resolution

- Resolve `document()` string arguments against the call site's static base,
  node arguments against their node base, and all arguments against an explicit
  base node when supplied. Check argument types and return documents in order
  without duplicates. Function items retain their lexical static base.
- The full all-feature Rust suite, all-target/all-feature Clippy, the 1,273-case
  XSLT gate and the focused QT3 URI-accessor sets pass. A 64-case XSLT document
  survey has five failures and 59 harness errors. Loading newly discovered
  URIs, fragment selection, and strict missing-base handling remain gaps;
  legacy relative resolver names still work when no base is known.

## Increment 281: missing document base errors

- Report XTDE1162 for relative `document()` references when the applicable
  static or node base URI is absent, rather than passing the relative name to
  the resolver. Absolute references and empty sequences remain supported.
- Focused all-feature resource and static-base tests, all-target/all-feature
  Clippy and the 1,273-case XSLT gate pass. Dynamic XML loading and document
  fragment selection remain unfinished.

## Increment 282: XML document fragments

- Resolve `document()` fragments against the cached XML document using XPointer
  shorthand `xml:id` and `element()` paths, including percent decoding and
  ordered fallback across pointer parts. Unsupported or unresolved fragments
  report XTDE1160. Literal document preloading strips the fragment first.
- The full all-feature Rust suite, all-target/all-feature Clippy and the
  1,273-case XSLT gate pass. Other XPointer schemes, schema/DTD ID types and
  dynamically loading newly discovered XML URIs remain unsupported.

## Increment 283: XSLT catalog resource environments

- Load URI-mapped local catalog sources and resources, pass stylesheet module
  URIs into compilation, and expose mapped modules to the development driver.
  Unsupported source-variable bindings, network resources and non-UTF-8 files
  remain explicit harness errors.
- The document set now has 12 passes, 43 failures and nine harness errors;
  previously 59 of its 64 cases were harness errors. Added these 12 passes and
  34 newly exposed passes from existing sets to the expanded gate: 1,319 passes,
  284 failures, 357 harness errors and 243 exclusions across 2,203 cases.
  All 46 runner tests, all-target/all-feature Clippy and the expanded gate pass.

## Increment 284: explicit axes in patterns

- Preserve explicit axis syntax in parsed steps so that `child::document-node()`
  cannot match a document root, while a bare `document-node()` still can. Treat
  an explicit `descendant-or-self::node()` as an axis rather than a `//`
  separator.
- The expanded XSLT gate passes 1,320 cases, with 283 failures, 357 harness
  errors and 243 exclusions across 2,203 selected cases. The full all-feature
  Rust suite and all-target/all-feature Clippy pass.

## Increment 285: static stylesheet function resolution

- Resolve named stylesheet function calls and references by expanded name and
  arity while validating stylesheet expressions, including patterns, keys,
  function bodies and unexecuted branches. Forward and private declarations
  remain available in ordinary stylesheet expressions; dynamic evaluation
  continues to enforce its separate visibility rules.
- Added the repaired match case to the XSLT gate: 1,321 passes, 282 failures,
  357 harness errors and 243 exclusions across 2,203 selected cases. The full
  all-feature Rust suite, focused final-code tests and all-target/all-feature
  Clippy pass.

## Increment 286: implicit attribute axes

- Parse an unprefixed `attribute()` kind test with the implicit attribute axis,
  while retaining an explicitly written axis. This applies consistently to
  XPath expressions and match patterns, including positional predicates.
- Added nine repaired match cases to the XSLT gate: 1,330 passes, 273 failures,
  357 harness errors and 243 exclusions across 2,203 selected cases. The full
  all-feature Rust suite, all-target/all-feature Clippy, and the 18,793-case
  QT3 baseline gate pass.
