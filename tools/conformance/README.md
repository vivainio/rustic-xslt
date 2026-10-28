# Development conformance bridge

This is a **partial** runner for W3C's XSLT catalog, not a published conformance
measurement. The external suite is not distributed with this repository.

The reference source pins `xslt30-test` to
`fddf1cf920087e791f13315d68dfbe874d97dc56`. Clone the official repository,
check out that revision, then run:

```sh
python3 tools/conformance/run.py /path/to/xslt30-test --sets choose,copy,template,variable,param,mode,call-template,apply-templates,initial-template,initial-mode
```

The command builds a development-only Rust JSON-lines example, reads catalog
and test-set files, executes transformations, and writes a JSON report listing
every selected case. Suite checkout must be clean and pinned; a missing suite,
unknown selected set, or empty selection is an error. Full exploratory runs
exit nonzero for engine failures or harness errors. No runtime CLI or standalone
XPath API is added to the shipped library.

For the committed passing-case regression gate:

```sh
python3 tools/conformance/run.py /path/to/xslt30-test --sets choose,copy,template,variable,param,mode,call-template,apply-templates,initial-template,initial-mode \
  --baseline tools/conformance/baseline.json
python3 -m unittest discover -s tools/conformance -p 'test_*.py'
```

The gate fails for missing or regressed baseline cases. Other failures and
harness errors are still named and counted, never reclassified as passes.
The current gate contains 262 passing cases out of 625 selected cases: 206 engine
failures, 111 harness errors, 46 optional-profile exclusions. Do not present the
262 cases as an applicable-suite pass rate.

Supported setup: local stylesheet/source files or inline content, local/global
named environments, string parameter literals, initial template/mode selection,
and serialization selection. Supported assertions: exact W3C error codes
(including explicit `*`), structural `assert-xml`/`assert-serialization`,
`assert-string-value`, supported XPath `assert` expressions, all-of/any-of/not. XML comparison preserves tree shape,
text/whitespace, comments, PIs and prefixes unless ignore-prefixes is requested;
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

Still missing: raw XDM result access, complete XPath assertion coverage, regex assertions,
secondary-result/message assertions, typed/static/tunnel/initial-entry catalog
parameters, context-item selectors, package/resource/schema environments,
collations and additional dependency configurations. These appear in reports
as harness errors. This runner is intended to grow alongside the engine.
