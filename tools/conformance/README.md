# Development conformance bridge

This is a **partial** runner for W3C's XSLT catalog, not a published conformance
measurement. The external suite is not distributed with this repository.

The reference source pins `xslt30-test` to
`fddf1cf920087e791f13315d68dfbe882d97dc56`. Clone the official repository,
check out that revision, then run:

```sh
python3 tools/conformance/run.py /path/to/xslt30-test --sets choose,copy,template,variable,param,mode,tunnel,call-template,apply-templates,initial-template,initial-mode,next-match
```

The command builds a development-only Rust JSON-lines example, reads catalog
and test-set files, executes transformations, and writes a JSON report listing
every selected case. Suite checkout must be clean and pinned; a missing suite,
unknown selected set, or empty selection is an error. Full exploratory runs
exit nonzero for engine failures or harness errors. No runtime CLI or standalone
XPath API is added to the shipped library.

For the committed passing-case regression gate:

```sh
python3 tools/conformance/run.py /path/to/xslt30-test \
  --baseline tools/conformance/baseline.json
python3 -m unittest discover -s tools/conformance -p 'test_*.py'
```

The gate fails for missing or regressed baseline cases. Other failures and
harness errors are still named and counted, never reclassified as passes.
The current gate contains 882 passing cases out of 1,572 selected cases: 273 engine
failures, 264 harness errors, 153 optional-profile exclusions. Do not present the
882 cases as an applicable-suite pass rate.

Supported setup: local stylesheet/source files or inline content, local/global
named environments, string/boolean/numeric/empty parameter literals, ordinary/tunnel
initial template/mode parameters and selection,
and serialization selection. Supported assertions: exact W3C error codes
(including explicit `*`), structural `assert-xml`, exact `assert-serialization` (with explicit prefix ignoring),
`serialization-matches` via XPath regexes, exact `assert-serialization-error`,
secondary `assert-result-document` with recursive assertions,
`assert-string-value`, supported XPath `assert` expressions, all-of/any-of/not. XML comparison preserves tree shape,
element/fragment text and whitespace, comments, PIs and prefixes unless ignore-prefixes is requested;
valid document prolog/epilog whitespace is excluded by XML parsing. Serialization
assertions support XML, HTML, XHTML, text, JSON and adaptive methods plus expected-file encodings;
attribute order and redundant namespace declarations do not matter.

Engine errors never satisfy negated non-error assertions. An implementation-gap
code never satisfies an error wildcard. Unsupported assertions are detected
before running any branch, including inside any-of. Missing setup or assertions
are harness errors; engine gaps remain failures. Optional schema-aware, XSLT
streaming and static-typing profiles are excluded explicitly; the record API is
not treated as the XSLT streaming feature.

XPath assertions run through a development-only stylesheet with `$result` bound
to the XML result document and the catalog namespace scope preserved. Unsupported
assertion expressions are harness errors. No standalone XPath product is added.

Still missing: raw XDM result access, complete XPath assertion coverage,
message assertions, general parameter expressions and static catalog
parameters, context-item selectors, package/resource/schema environments,
collations and additional dependency configurations. These appear in reports
as harness errors. This runner is intended to grow alongside the engine.

## Internal XPath / QT3 checks

A separate partial bridge runs QT3 expressions through development stylesheets;
this adds no public XPath API. The runners build the development driver with the
`conformance` feature. QT3 requests select an XPath-only function library so XSLT
extensions such as current() do not alter unknown-function assertions; normal
XSLT requests retain the XSLT library. The hidden compilation hook is absent
from default builds. The suite must be clean at
`201a6e466940cdfc727f4babfedcde5332b9f578`.

```sh
python3 tools/conformance/qt3.py /path/to/qt3tests \
  --baseline tools/conformance/qt3-baseline.json
```

The current selection contains 26,966 cases: 18,735 pass, 390 engine failures, 764
harness errors and 7,077 profile exclusions. All 18,735 passing cases are recorded
in a gate.
Counts describe this selected development workload, not a conformance rate.

The bridge supports absent/local XML context, named environment references,
namespace declarations, source variables and URI-mapped local resources through
the development driver's resolver. It handles scalar/type/count/empty/string assertions,
atomic/node deep equality, permutation assertions with duplicate counts, XPath assertions, exact error codes and composite assertions.
Queries bind their result outside assertion scope, so assertion `$result` cannot
satisfy a query's undeclared variable. Unsupported assertion expressions, setup and dependencies are reported as harness errors. Schema-aware and
XQuery-only and exclusively older XPath version profiles are excluded explicitly;
the bridge evaluates XPath 3.1. Queries are reevaluated for assertions;
raw XDM delivery and nondeterministic functions are outside this initial bridge.

When `--baseline` is supplied without `--sets`, both runners use the baseline
selection, including surveyed sets with zero passes. Explicit set lists remain
available for exploratory runs. Empty regression baselines are rejected.
