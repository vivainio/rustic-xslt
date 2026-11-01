# First practical Schematron validation

## Goal and current evidence

Prioritize a working Schematron workflow over broad XSLT conformance and the
DocBook workload. A user supplies a Schematron schema and XML document; the
engine compiles the schema to XSLT using unmodified SchXslt2, runs that XSLT,
and returns an SVRL report. The first milestone is a useful subset, not a
claim of full ISO Schematron support.

The pinned SchXslt2 revision is `bd196454305641f67443816fe83bbc120f30ebe1`.
At that revision the unmodified transpiler and generated validator run end to
end for the inventory and typical fixtures, explicit phase selection, and two
upstream SchXslt2 fixtures, including one using an accumulator, as well as a
500-item document with 50 expected failures. The probe checks SVRL assertions,
reports, locations, diagnostics and selected phases. A separate message case
currently fails: a whitespace-only separator between `sch:name` and
`sch:value-of` is stripped when the generated validator is compiled. The probe
also verifies that `xml:space="preserve"` on the assertion retains the space.
This is not yet broad coverage of external schemas. Constructor-based stylesheet
functions have a conservative nesting limit of two; deeply recursive schemas
need a shared trampoline. Select-based accumulator rules run on unstreamed
trees; sequence-constructor rules and record-streamed accumulators remain
unsupported, and repeated queries traverse the tree again.
The stylesheet contains accumulator declarations, functions with instruction-based
bodies, and generated XSLT with error handling. These work in the current
fixtures but do not imply complete coverage. DocBook also exposed namespace-node
support and additional constructor cases; their priority for Schematron is not
yet proven.
See [the workload setup](tools/workloads/README.md) for reproducible sources.

## First useful milestone

A local, reproducible end-to-end example must compile an unmodified SchXslt2
transpiler, transform a Schematron schema into a validator, compile that
validator, and run it on both a valid and an invalid XML document. Assert the
SVRL structure and meaning, including failed assertion test, message and
location, rather than only checking that XML was returned. A valid document
must have no failed assertions; an invalid document must report exactly the
expected failures. A compilation or dynamic error must be distinguishable
from an invalid document and must never be treated as a successful validation.

The initial representative schemas should exercise:

- `schema`, `pattern`, `rule`, `assert` and `report`, including multiple
  patterns and multiple rules, and rule precedence within a pattern;
- namespace declarations, namespaced XML, ordinary XPath predicates,
  attributes and comparisons, and readable assertion messages with
  `value-of` and `name`;
- schema/pattern/rule `let` variables, default and selected phases, and
  diagnostics attached to assertions;
- at least one real upstream fixture in addition to small valid/invalid pairs.

Use `queryBinding="xslt3"` for the initial SchXslt2 path. Verify other query
bindings against that implementation before promising support. Keep schemas
and input documents small enough to fit the existing resource limits, but use
unmodified real transpiler code and generated validator XSLT. Preserve schema
base URIs and namespaces through both compilation stages.

## Work in priority order

1. **Build the end-to-end workload probe.** Extend `tools/workloads` to feed
   schemas to SchXslt2, save generated XSLT for diagnosis, compile it and run
   paired XML fixtures, comparing semantic SVRL results. Record failures by
   stage: transpiler compile, transpilation, generated XSLT compile,
   validation, and incorrect SVRL. Keep a development-only reference
   processor as an optional independent oracle; it must not become a runtime
   dependency. Do not count compilation or fallback as a passing validation.
2. **Namespace aliases (implemented; full Rust regression suite passes).** Support
   `xsl:namespace-alias` for literal result names and namespace bindings,
   including `#default`, declarations across imports/includes and import
   precedence. Verify it by generating an XSLT stylesheet, compiling the
   generated stylesheet and transforming input, then rerun the actual
   SchXslt2 probe. Respect namespace fixup and distinguish literal results
   from computed names. Do not special-case or rewrite SchXslt2.
3. **Follow the real dependency chain.** After each unblock, run the full
   two-stage probe and implement the next XSLT/XPath feature that prevents
   representative schemas from working. Investigate accumulator declarations
   and mode usage, non-expression stylesheet-function bodies, static
   parameters/`use-when`, and generated `xsl:try`/`xsl:catch` as candidates,
   not as a presumed complete or mandatory list. Implement generic semantics
   with focused behavioral tests and an upstream workload that uses them.
   Update this plan with observed blockers and avoid large unrelated
   conformance sweeps.
4. **Cover typical validation behavior.** Grow the passing end-to-end fixture
   set through the features above. Compare relevant SVRL nodes and attributes
   by expanded name and value, allowing inconsequential serialization and
   prefix differences. Include at least one check that the same schema yields
   different correct outcomes for valid and invalid inputs. Check reference
   results where available and pin fixtures and processor versions.
5. **Make it usable through the library.** Provide a documented runnable Rust
   example using the public API: load the transpiler and schema, retain their
   base URIs, compile the generated validator, validate XML and inspect the
   SVRL. Reuse the compiled validator for multiple documents. Surface errors
   separately from validation failures, carry limits and caller-controlled
   resource resolution through both stages, and document how phase parameters
   are supplied. A new public CLI or dedicated Schematron API is not required
   for this milestone; revisit ergonomics after the workflow works.

## Next coverage, after the first milestone

Add include/external-schema resolution, abstract patterns and rule extension,
and richer diagnostics using upstream examples and explicit resolver/base-URI
checks. Then assess common real schemas that use XSLT 2.0 binding, embedded
XSLT functions, and larger documents. Add features in the order these
workloads require; support for streamable validation, schema-aware XSLT,
2025-only Schematron features, and DTD-backed entities is not a prerequisite
unless a selected representative workload demonstrates otherwise.

## Completion and reporting

The milestone is complete only when the unmodified pinned transpiler and its
generated stylesheet both run, the representative feature cases above pass
with correct SVRL for valid and invalid documents, and the documented public
library example can be reproduced. Report the exact supported cases, upstream
revision, outstanding blockers, and resource limits. Unsupported schema
features must fail clearly rather than silently producing a misleading report.
