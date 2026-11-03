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

DocBook also ships a Schematron schema for glossary references, backed by its
unmodified `standalone-functions.xsl`. The following separate probe places the
unmodified schema and its relative XSLT include in a temporary directory and
checks missing, single, and duplicate glossary entries against SaxonC, using
both embedded glossaries and external files selected by a DocBook processing
instruction:

```sh
PYTHONPATH=/tmp/rust-xslt-saxonche python3 tools/workloads/docbook_schematron.py \
  --docbook /tmp/xslt-workloads/docbook --schxslt2 /tmp/xslt-workloads/schxslt2 --release
```

With DocBook 2.8.4 revision `5210c58dcc982934b51101eb0527d32bda85a865`, both
Rust and Saxon compile the schema and validate all six examples. The probe
compares fired-rule contexts, assertion tests, locations and message text across
both generated validators and both processors. It also exercises the public
example with the schema and source documents in separate directories and checks
that one input document cannot access another input's resource directory. Rust's nested function execution
uses guarded stack growth on Unix x86-64 and AArch64; other targets retain the
previous conservative nesting limits and may still report `XPDY0130` here.

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

Use `--phase NAME` after the schema to choose a phase, `--max-steps N` to set
the per-transformation work limit, and `--max-depth N` to set its execution
depth limit. Each SVRL document is
written to stdout; counts of failed assertions and successful reports are
written to stderr. Invalid input is reported in SVRL and does not itself set a
nonzero exit status; compile and runtime errors do. With multiple inputs,
stdout contains consecutive XML documents separated by newlines. Schema
compilation uses local files under the schema directory and its descendants;
relative includes and fragment identifiers within that tree are supported.
For each input, validation can also read files under that input's directory and
its descendants. Each input has its own resource roots. The resolver
canonicalizes paths, rejects escapes and does not fetch network URLs.
For pipeline use, `--fail-on-findings` returns status 1 if any input produces a
`failed-assert` or `successful-report`. It treats every finding as significant,
including warnings and informational reports, regardless of role or severity.
All input documents are processed and all SVRL reports are written before this
status is returned. Without the option, findings do not change the exit status.
The compiler's XSLT imports and includes are resolved separately within the
compiler's directory and its descendants.
Messages from preprocessing and schema compilation are printed to stderr and
stop the command by default, since they can indicate a partially compiled
schema. Use `--allow-compiler-messages` to continue after reviewing them.
Messages from the validator are also printed. The example rejects output that
is not an SVRL report and warns if no rules fire; an empty result can be valid
for a document with no matching contexts, so that warning does not change the
exit status. For example, the ISO include stage emits a nonterminating message
when an include fragment is missing; the default behavior prevents that case
from silently producing a report with no validation rules.
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

To check a larger input, run:

```sh
PYTHONPATH=/tmp/rust-xslt-saxonche python3 tools/workloads/reference_peppol_scale.py \
  --schxslt2 /tmp/xslt-workloads/schxslt2 \
  --peppol /tmp/xslt-workloads/peppol --release
```

This creates 10-, 20-, and 40-line invoices by copying an upstream invoice line
and removing its profile identifier, then compares the resulting fired rules and
assertion details across Rust and Saxon. All three run within the default
20-million-step limit on the pinned workload. The generated inputs deliberately
contain validation failures and do not claim to be balanced business invoices.
The `--lines` option selects other sizes. With `--lines 100 200 500`, 100
lines fit within the default limit; 200 and 500 require a higher limit on this
workload. The 200-line case agrees across both compilers and processors with
`--max-steps 25000000`; all three agree with `--max-steps 200000000`. These
limits are per transformation.

The same checkout includes the larger CEN EN 16931 UBL schema. Its reference
probe checks a valid invoice and variants with a missing specification ID, an
incorrect monetary total, and an invalid invoice-type code:

```sh
PYTHONPATH=/tmp/rust-xslt-saxonche python3 tools/workloads/reference_cen.py \
  --schxslt2 /tmp/xslt-workloads/schxslt2 \
  --peppol /tmp/xslt-workloads/peppol --release
```

Both processors compile and execute the schema in the default, model, and
code-list phases. The probe compares fired-rule contexts and assertion details
across both processors, checks the expected failure IDs, and runs the public
example with the code-list phase and multiple inputs. All stages fit within the
default 20-million-step limit on these inputs. This covers these examples and
phase selections, not every CEN rule.

For the CII binding, fetch the European validation repository's pinned invoice
examples and run the companion probe:

```sh
git clone --filter=blob:none --sparse https://github.com/ConnectingEurope/eInvoicing-EN16931.git /tmp/xslt-workloads/cen-examples
git -C /tmp/xslt-workloads/cen-examples checkout b17b2695e5e3e021945c38658326b78a76f80f84
git -C /tmp/xslt-workloads/cen-examples sparse-checkout set cii/examples
PYTHONPATH=/tmp/rust-xslt-saxonche python3 tools/workloads/reference_cii.py \
  --schxslt2 /tmp/xslt-workloads/schxslt2 \
  --peppol /tmp/xslt-workloads/peppol \
  --cen-examples /tmp/xslt-workloads/cen-examples --release
```

The probe compares both generated validators on both processors for all 15
upstream examples and three intentionally invalid invoices. It checks model
and code-list phase selection and the public example as well. These inputs fit
within the default 20-million-step limit; they do not cover every CII rule.

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

A second domain workload is the NIEM Schematron-in-XSD rules. Clone the pinned
rules and run the optional SaxonC comparison:

```sh
git clone https://github.com/NIEM/Schematron-in-XSD-Spec.git /tmp/xslt-workloads/niem
git -C /tmp/xslt-workloads/niem checkout 4f71c5cc73e63b91085a6c3c312737bd26b61e1d
PYTHONPATH=/tmp/rust-xslt-saxonche python3 tools/workloads/reference_niem.py \
  --schxslt2 /tmp/xslt-workloads/schxslt2 \
  --niem /tmp/xslt-workloads/niem --release
```

The unmodified schema compiles and runs in rust-xslt against the upstream XSD.
Its generated stylesheet refers to `xs:` without declaring that prefix, so
Saxon rejects it. For the cross-processor comparison the probe adds an `xs`
`sch:ns` declaration to the schema in memory; it does not modify the checkout.
It runs Rust- and Saxon-generated validators on both processors against the
upstream XSD and valid and invalid minimal XSDs. The latter exercise quantified
expressions, rule variables, node identity and namespace inspection. The probe
checks expected findings and compares fired rules and assertion details. All
cases fit within the default 20-million-step limit on the pinned inputs.

The NEMSIS national demographic schema provides a larger healthcare workload
with upstream valid and invalid samples and expected SVRL reports. The optional
probe compiles the original schema with NEMSIS's ISO Schematron compiler in
both Rust and SaxonC, then runs both generated validators in both processors:

```sh
git clone --filter=blob:none --no-checkout https://git.nemsis.org/scm/nep/nemsis_public.git /tmp/xslt-workloads/nemsis
git -C /tmp/xslt-workloads/nemsis checkout 17e66ed740363cba0f947a4b8e09523810a4d112
PYTHONPATH=/tmp/rust-xslt-saxonche python3 tools/workloads/reference_nemsis.py \
  --nemsis /tmp/xslt-workloads/nemsis --schxslt2 /tmp/xslt-workloads/schxslt2 --release
```

All 20 demographic sample documents agree on fired rules and assertion details
between processors and match the upstream expected failure IDs. The comparison
normalizes the current local time and time zone in one diagnostic, since the
processors can use different local time zones. The probe uses the default
20-million-step limit. SchXslt2 generates an invalid call to `path()` for this
schema's multi-node `subject` expressions, so its generated validator does not
compile. Further investigation found that it also omits the schema's embedded
XSLT variable and does not add an `xs` namespace binding to the generated
validator. The NEMSIS ISO compiler provides a working Rust-only pipeline for
these cases.

The public example can run the pinned NEMSIS ISO compiler and validate multiple
documents without SaxonC:

```sh
cargo run --release --example schematron -- \
  /tmp/xslt-workloads/nemsis/Schematron/DevelopmentKit/Schematron/utilities/iso-schematron-xslt2/iso_svrl_for_xslt2.xsl \
  /tmp/xslt-workloads/nemsis/Schematron/rules/DEMDataSet.sch --iso \
  /tmp/xslt-workloads/nemsis/SampleData/Schematron/DEM/xml/DEMDataSet--Base.xml \
  /tmp/xslt-workloads/nemsis/SampleData/Schematron/DEM/xml/DEMDataSet-nemSch_d001_A.xml
python3 tools/workloads/nemsis_cli.py --nemsis /tmp/xslt-workloads/nemsis --release
```

`--iso` supplies the ISO compiler's `allow-foreign` parameter and uses its
unqualified `phase` parameter when `--phase` is supplied. Add
`--iso-preprocess` to run `iso_dsdl_include.xsl` and `iso_abstract_expand.xsl`
from the compiler's directory before compiling the schema. The include stage
uses the schema document URI as the base for relative references; the pinned
ISO stylesheet does not honor an inherited element `xml:base` for those
references. The pinned NEMSIS schemas are already expanded and do not need
preprocessing. If preprocessing them, use `--max-depth 2048`; the patient-care
schema also needs `--max-steps 100000000` for its include stage. The pinned ISO
abstract-pattern processor does not correctly expand the upstream `svrl.sch`
schema even with a higher depth limit; the same issue occurs with SaxonC.

HL7's C-CDA 2.1 repository adds another healthcare workload with an unmodified
schema, a sample document, and an external terminology file. To compare the
public example with SaxonC using the pinned ISO compiler above:

```sh
git clone https://github.com/HL7/CDA-ccda-2.1.git /tmp/xslt-workloads/hl7-ccda
git -C /tmp/xslt-workloads/hl7-ccda checkout 6d3ed96160b45a111895da4df5510c7fad9de01f
PYTHONPATH=/tmp/rust-xslt-saxonche python3 tools/workloads/reference_ccda.py \
  --ccda /tmp/xslt-workloads/hl7-ccda --nemsis /tmp/xslt-workloads/nemsis --release
```

The Rust-only public example and SaxonC independently compile and run the
schema's `errors` phase. They agree on fired-rule contexts and assertion details
for the upstream document and two small documents that test valid and invalid
codes against the upstream 62 MB `voc.xml`. The upstream example fires 172
rules; the invalid-code case reports `a-1098-8746`. The probe uses a
200-million-step limit and depth 2048; compilation exceeds the default
20-million-step limit on this schema. The synthetic documents exercise a
terminology rule and are not complete CDA clinical documents. C-CDA also
requires separate XML Schema validation, which this probe does not perform.

Pass `--dataset ems` to run the same comparison on the larger NEMSIS
patient-care schema and its 198 upstream sample documents and expected reports.
This requires the checkout to include `Schematron/rules/EMSDataSet.sch` and the
`SampleData/Schematron/EMS/xml` and `svrl` directories. The comparison also
normalizes the current local time in the EMS time-stamp diagnostic. All 792
compiler/validator combinations agree and match the expected failure IDs with
the default step limit. SchXslt2 exceeds that limit while transpiling this
schema; with a 100-million-step limit, transpilation completes but its generated
validator still has invalid `path()` calls for multi-node subjects.

The optional diagnostic below tests three *temporary* compatibility changes:
selecting the first subject node for `path()`, passing through the embedded XSLT
variable, and adding an `xs` Schematron namespace declaration. With those
changes, three demographic samples—including both multi-subject failures—agree
across Rust and Saxon compilation and validation and match upstream failure IDs.
The diagnostic does not modify either checkout, and is not a general fix for
SchXslt2 or a substitute for the unmodified NEMSIS workflow above.

```sh
PYTHONPATH=/tmp/rust-xslt-saxonche python3 tools/workloads/schxslt2_nemsis_diagnostic.py \
  --nemsis /tmp/xslt-workloads/nemsis --schxslt2 /tmp/xslt-workloads/schxslt2 --release
```
