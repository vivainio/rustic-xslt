# Real stylesheet workloads

This probes published, unmodified DocBook xslTNG stylesheets and the SchXslt2
Schematron compiler. The source projects and release files are external and are
not vendored into this repository. To reproduce the current run:

```sh
mkdir -p /tmp/xslt-workloads
git clone --branch 2.8.4 --depth 1 https://codeberg.org/DocBook/xslTNG.git /tmp/xslt-workloads/docbook
git clone https://codeberg.org/schxslt/schxslt2.git /tmp/xslt-workloads/schxslt2
git -C /tmp/xslt-workloads/schxslt2 checkout bd196454305641f67443816fe83bbc120f30ebe1
python3 tools/workloads/fetch_docbook.py /tmp/xslt-workloads/docbook /tmp/xslt-workloads/docbook-release/xslt
python3 tools/workloads/run.py \
  --docbook-release /tmp/xslt-workloads/docbook-release/xslt \
  --docbook-source /tmp/xslt-workloads/docbook \
  --schxslt2 /tmp/xslt-workloads/schxslt2 \
  --report /tmp/xslt-workloads/report.json
```

The DocBook source checkout alone lacks generated files required by the real
stylesheets, so the fetch script obtains the published 2.8.4 release modules.
The report records both source revisions. The DocBook stages are tested
individually against the upstream sample; these are not an end-to-end pipeline.
The first stage is checked against the input tree and an additional entityref
case. The XInclude stage reports its own extension fallback as degraded. Stages
without an output oracle are unverified even if they run. SchXslt2 is tested
against an upstream Schematron fixture. The runner exits nonzero on blockers,
failures, or unverified output, so it is a progress probe rather than a passing
CI gate. The driver currently limits each transformation to one million steps.

Unparsed-entity lookup currently only supports documents without DTDs: the XML
parser rejects DTDs, and thus declared unparsed entities cannot yet be resolved.

To run the Schematron-focused, two-stage probe without DocBook:

```sh
python3 tools/workloads/schematron.py \
  --schxslt2 /tmp/xslt-workloads/schxslt2 \
  --generated /tmp/xslt-workloads/validator.xsl
```

It compiles the upstream transpiler, generates a validator for a namespaced
inventory schema, and checks valid and invalid documents against expected SVRL
assertion and report details. It reports the failing stage and exits nonzero
when either compilation, transformation or the expected SVRL fails. Generated
XSLT is saved only after successful transpilation. The `reference_driver`
limits each transformation to one million steps.

The public-library example can compile one schema and validate multiple files:

```sh
cargo run --example schematron -- \
  /tmp/xslt-workloads/schxslt2/src/main/resources/content/transpile.xsl \
  tools/workloads/schematron/typical.sch \
  tools/workloads/schematron/typical-valid.xml \
  tools/workloads/schematron/typical-invalid.xml
```

Use `--phase NAME` after the schema to choose a phase, and `--max-steps N`
to set the per-transformation work limit for larger schemas. Each SVRL document is
written to stdout; counts of failed assertions and successful reports are
written to stderr. Invalid input is reported in SVRL and does not itself set a
nonzero exit status; compile and runtime errors do. With multiple inputs,
stdout contains consecutive XML documents separated by newlines. The example uses local
files and a resolver limited to the schema directory and its descendants;
relative includes and fragment identifiers within that tree are supported. The
resolver canonicalizes paths, rejects escapes and does not fetch network URLs.
The probe also checks a default phase,
variables, diagnostic references, dynamic message text, an upstream root-rule
fixture, and an upstream fixture containing an XSLT accumulator.

Known message limitation: a whitespace-only separator between adjacent
`sch:name` and `sch:value-of` elements is present in the generated stylesheet
but is stripped when that stylesheet is compiled. The probe reports this as a
limitation and therefore exits nonzero. Setting `xml:space="preserve"` on the
assertion retains the separator. SaxonC-HE 12.9 gives the same results, including
when each processor executes the other processor's generated stylesheet.

To repeat that independent cross-check, install SaxonC in a temporary directory
and run the optional reference probe (it is not a runtime dependency):

```sh
python3 -m pip install --target /tmp/rust-xslt-saxonche saxonche==12.9.0
PYTHONPATH=/tmp/rust-xslt-saxonche python3 tools/workloads/reference_whitespace.py \
  --schxslt2 /tmp/xslt-workloads/schxslt2
```

The probe also exercises a two-parameter abstract pattern and external includes
with an inherited `xml:base`, including an ID fragment. For example:

```sh
cargo run --example schematron -- \
  /tmp/xslt-workloads/schxslt2/src/main/resources/content/transpile.xsl \
  tools/workloads/schematron/includes/main-fragment.sch \
  tools/workloads/schematron/invalid.xml
```

A larger, real XSLT 2.0-binding workload is Peppol BIS Billing. To obtain the
pinned rules and example invoice and run the probe:

```sh
git clone --depth 1 --filter=blob:none --sparse https://github.com/OpenPEPPOL/peppol-bis-invoice-3.git /tmp/xslt-workloads/peppol
git -C /tmp/xslt-workloads/peppol fetch --depth 1 origin 5df0eff7c8efafd7c79c0dacce55082d175db759
git -C /tmp/xslt-workloads/peppol checkout --detach FETCH_HEAD
git -C /tmp/xslt-workloads/peppol sparse-checkout set rules/sch rules/examples
python3 tools/workloads/peppol.py \
  --schxslt2 /tmp/xslt-workloads/schxslt2 \
  --peppol /tmp/xslt-workloads/peppol \
  --release \
  --generated /tmp/xslt-workloads/peppol-validator.xsl
```

The probe checks that rules fire, the upstream example has no failed
assertions, and removing its `cbc:ProfileID` triggers `PEPPOL-EN16931-R001`.
It uses a 500-million-step limit per transformation by default; override it
with `--max-steps`. Both generating the pinned validator and validating the
example fit within the normal 20-million-step library limit. On one local run,
a release build took about 0.43 seconds to generate the validator and 2.9–3.0
seconds to validate each example. These are measurements, not performance
guarantees.

To compare SaxonC and rust-xslt on the same Peppol inputs, using the optional
SaxonC installation above:

```sh
PYTHONPATH=/tmp/rust-xslt-saxonche python3 tools/workloads/reference_peppol.py \
  --schxslt2 /tmp/xslt-workloads/schxslt2 \
  --peppol /tmp/xslt-workloads/peppol --release
```

The comparison cross-runs both generated validators through both processors,
checking fired-rule contexts and assertion IDs, tests, locations, and text.
It covers these two inputs, not every Peppol rule.

The independent [Schematron/schema](https://github.com/Schematron/schema)
repository contains real schemas for SVRL reports and additional Schematron
constraints. The following optional reference probe uses the pinned revision,
the SaxonC installation above, and the pinned SchXslt2 checkout:

```sh
git clone https://github.com/Schematron/schema.git /tmp/xslt-workloads/schematron-schema
git -C /tmp/xslt-workloads/schematron-schema checkout 3a94cbe9ee6a9481d38888654cd7411bb450f895
PYTHONPATH=/tmp/rust-xslt-saxonche python3 tools/workloads/reference_svrl.py \
  --schxslt2 /tmp/xslt-workloads/schxslt2 \
  --schema /tmp/xslt-workloads/schematron-schema --release
```

The SVRL schema checks an actual report generated from the inventory fixture
and a version with a missing location and an illegal child. These exercise
nested abstract-rule extensions and parameterized abstract patterns. The
Schematron constraints schema checks the upstream SVRL schema and a version
with a broken abstract-rule reference, exercising `current()` and document-wide
search. Both Rust- and Saxon-generated validators run on both processors;
the probe compares fired-rule contexts and assertion details for all four
inputs. All stages fit within the default 20-million-step limit for the pinned
inputs. This is feature coverage for those cases, not a complete assessment of
all Schematron implementations or schemas.
