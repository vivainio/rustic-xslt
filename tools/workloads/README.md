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
