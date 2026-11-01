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

Use `--phase NAME` after the schema to choose a phase. Each SVRL document is
written to stdout; counts of failed assertions and successful reports are
written to stderr. Invalid input is reported in SVRL and does not itself set a
nonzero exit status; compile and runtime errors do. With multiple inputs,
stdout contains consecutive XML documents separated by newlines. The example uses local
files and no resource resolver, so schemas needing external includes are not
yet supported by this example. The probe also checks a default phase,
variables, diagnostic references, dynamic message text, an upstream root-rule
fixture, and an upstream fixture containing an XSLT accumulator.

Known message limitation: a whitespace-only separator between adjacent
`sch:name` and `sch:value-of` elements is present in the generated stylesheet
but is stripped when that stylesheet is compiled. The probe reports this as a
failure and therefore exits nonzero. Setting `xml:space="preserve"` on the
assertion retains the separator; the probe also checks that workaround.
