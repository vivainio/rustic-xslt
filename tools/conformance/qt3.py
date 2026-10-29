#!/usr/bin/env python3
"""Partial, development-only QT3 bridge through XSLT; no public XPath API."""
import argparse
from collections import Counter
import json
import re
from pathlib import Path
import subprocess
import sys
from xml.sax.saxutils import quoteattr
from run import Driver, HarnessError, ProfileExcluded, ROOT, baseline_sets, parse_catalog

NS = "{http://www.w3.org/2010/09/qt-fots-catalog}"
PIN = "201a6e466940cdfc727f4babfedcde5332b9f578"

def tag(node):
    return node.tag.removeprefix(NS)

def text(node):
    return "".join(node.itertext())

def literal(value):
    return "'" + value.replace("'", "''") + "'"

SUPPORTED_FEATURES = {"higherOrderFunctions"}

def dependencies(node):
    for d in node.findall(NS + "dependency"):
        kind, value = d.get("type"), d.get("value", "")
        if kind == "spec":
            xpath = any(v.startswith("XP") for v in value.split())
            if not xpath or d.get("satisfied", "true") == "false":
                raise ProfileExcluded(f"requires another language/version profile: {value}")
        elif kind == "feature" and value == "schema-aware":
            if d.get("satisfied", "true") != "false":
                raise ProfileExcluded("optional schema-aware processor")
        elif kind == "feature" and value in SUPPORTED_FEATURES:
            if d.get("satisfied", "true") == "false":
                raise ProfileExcluded(f"test requires the feature to be absent: {value}")
        else:
            raise HarnessError(f"unsupported QT3 dependency: {kind}={value}")

def environment(node, registry, seen=()):
    result = {"source": "", "namespaces": {}, "decimal_formats": []}
    if node is None:
        return result
    reference = node.get("ref")
    if reference:
        if reference not in registry or reference in seen:
            raise HarnessError(f"unresolved/cyclic QT3 environment: {reference}")
        result = environment(registry[reference][0], registry, (*seen, reference))
    base = registry.get(id(node), (None, Path()))[1]
    for part in node:
        kind = tag(part)
        if kind == "namespace":
            prefix = part.get("prefix", "")
            if not prefix:
                raise HarnessError("QT3 default element/type namespace")
            result["namespaces"][prefix] = part.get("uri", "")
        elif kind == "source":
            if part.get("role") != "." or part.get("validation", "skip") != "skip":
                raise HarnessError("QT3 source variable or validation")
            if not part.get("file") or "://" in part.get("file"):
                raise HarnessError("QT3 source requires a local file")
            result["source"] = (base / part.get("file")).read_text(encoding="utf-8-sig")
        elif kind == "decimal-format":
            result.setdefault("decimal_formats", []).append(dict(part.attrib))
        elif kind == "schema":
            raise ProfileExcluded("optional schema-aware environment")
        else:
            raise HarnessError(f"unsupported QT3 environment component: {kind}")
    return result

# The static context of XPath 3.1 predeclares these prefixes.
STANDARD_NAMESPACES = {
    "fn": "http://www.w3.org/2005/xpath-functions",
    "xs": "http://www.w3.org/2001/XMLSchema",
    "math": "http://www.w3.org/2005/xpath-functions/math",
    "map": "http://www.w3.org/2005/xpath-functions/map",
    "array": "http://www.w3.org/2005/xpath-functions/array",
    "err": "http://www.w3.org/2005/xqt-errors",
}

def stylesheet(expression, bindings, assertion="count($result)", decimal_formats=()):
    # The query is global; assertion-only $result cannot satisfy a free query
    # variable. Generated namespace prefixes cannot appear in the query.
    xsl, value = "_qt3_xsl", "_qt3_value"
    while xsl in bindings or xsl + ":" in expression:
        xsl += "_"
    while value in bindings or value + ":" in expression:
        value += "_"
    namespaces = {**STANDARD_NAMESPACES, **bindings}
    namespaces[xsl] = "http://www.w3.org/1999/XSL/Transform"
    namespaces[value] = "urn:rust-xslt:qt3:result"
    declarations = " ".join(f"xmlns:{p}={quoteattr(uri)}" for p, uri in namespaces.items() if p != "xml")
    formats = "".join(
        f"<{xsl}:decimal-format " + " ".join(f"{k}={quoteattr(v)}" for k, v in fmt.items()) + "/>"
        for fmt in decimal_formats)
    start = (f'<{xsl}:stylesheet version="3.0" {declarations}>'
             f'<{xsl}:output method="text"/>{formats}'
             f'<{xsl}:variable name="{value}:result" as="item()*" select={quoteattr(expression)}/>'
             f'<{xsl}:template name="entry"><{xsl}:variable name="result" as="item()*" select="${value}:result"/>')
    start += f'<{xsl}:value-of select={quoteattr(assertion)}/>'
    return start + f'</{xsl}:template></{xsl}:stylesheet>'

LEAVES = {"error", "assert", "assert-eq", "assert-deep-eq", "assert-permutation", "assert-string-value", "assert-empty", "assert-true", "assert-false", "assert-count", "assert-type"}

def preflight(node):
    kind = tag(node)
    allowed = {"error": {"code"}, "assert-string-value": {"normalize-space"}}.get(kind, set())
    if set(node.attrib) - allowed:
        raise HarnessError(f"unsupported QT3 {kind} attributes: {sorted(set(node.attrib) - allowed)}")
    if kind == "assert-string-value" and node.get("normalize-space", "false") not in {"true", "false"}:
        raise HarnessError("invalid QT3 normalize-space flag")
    if kind in {"any-of", "all-of", "not"}:
        if not len(node) or kind == "not" and len(node) != 1:
            raise HarnessError(f"invalid QT3 {kind} assertion")
        for a in node:
            preflight(a)
    elif kind not in LEAVES:
        raise HarnessError(f"unsupported QT3 assertion: {kind}")

def assertion(a, probe, expression, env, driver):
    kind = tag(a)
    codes = [e.get("code", "") for e in probe.get("diagnostics", [])]
    if kind == "error":
        expected = a.get("code", "")
        return bool(codes) and all(code == expected or expected == "*" and not code.startswith("RUST_XSLT_") for code in codes)
    if kind in {"any-of", "all-of", "not"}:
        if kind == "not" and codes:
            return False
        results = [assertion(c, probe, expression, env, driver) for c in a]
        return any(results) if kind == "any-of" else all(results) if kind == "all-of" else not results[0]
    if codes:
        return False
    value = text(a)
    if kind == "assert-permutation":
        condition = (f"let $_expected := ({value}) return "
                     "count($result) eq count($_expected) and "
                     "(every $_item in ($result, $_expected) satisfies "
                     "count($result[deep-equal(., $_item)]) eq "
                     "count($_expected[deep-equal(., $_item)]))")
        style = stylesheet(expression, env["namespaces"], assertion=condition, decimal_formats=env.get("decimal_formats", ()))
    elif kind == "assert-deep-eq":
        style = stylesheet(expression, env["namespaces"], assertion=f"deep-equal($result, ({value}))", decimal_formats=env.get("decimal_formats", ()))
    else:
        condition = {
            "assert": f"boolean(({value}))",
            "assert-eq": f"$result eq ({value})",
            "assert-empty": "empty($result)",
            "assert-count": f"count($result) eq ({value})",
            "assert-type": f"$result instance of {value}",
            "assert-true": "$result instance of xs:boolean and $result eq true()",
            "assert-false": "$result instance of xs:boolean and $result eq false()",
        }.get(kind)
        if kind == "assert-string-value":
            actual = "string-join($result ! string(.), ' ')"
            if a.get("normalize-space") == "true":
                actual, value = f"normalize-space({actual})", " ".join(re.findall(r"[^ \t\r\n]+", value))
            condition = f"{actual} eq {literal(value)}"
        style = stylesheet(expression, env["namespaces"], assertion=condition, decimal_formats=env.get("decimal_formats", ()))
    response = driver.call({"stylesheet": style, "source": env["source"], "initialTemplate": "entry"})
    if response.get("diagnostics"):
        raise HarnessError("QT3 assertion evaluator: " + "; ".join(e.get("message", "") for e in response["diagnostics"]))
    output = response.get("output", "")
    if output not in {"true", "false"}:
        raise HarnessError("invalid assertion evaluator response")
    return output == "true"

def run_case(case, base, registry, driver):
    dependencies(case)
    result = case.find(NS + "result")
    if result is None or not len(result):
        raise HarnessError("QT3 case has no assertions")
    for a in result:
        preflight(a)
    setup = case.find(NS + "environment")
    if setup is not None:
        registry[id(setup)] = setup, base
    env = environment(setup, registry)
    test = case.find(NS + "test")
    if test is None:
        raise HarnessError("missing QT3 test expression")
    expression = (base / test.get("file")).read_text() if test.get("file") else text(test)
    probe = driver.call({"stylesheet": stylesheet(expression, env["namespaces"], decimal_formats=env.get("decimal_formats", ())), "source": env["source"], "initialTemplate": "entry"})
    answers = [assertion(a, probe, expression, env, driver) for a in result]
    reason = "; ".join(e.get("code", "") + ": " + e.get("message", "") for e in probe.get("diagnostics", []))
    return ("pass", "") if all(answers) else ("fail", reason or "QT3 assertion is false")

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("suite", type=Path)
    parser.add_argument("--sets", help="exact set names; defaults to baseline sets or op-to,op-bang")
    parser.add_argument("--report", type=Path, default=Path("conformance-results/qt3.json"))
    parser.add_argument("--baseline", type=Path)
    args = parser.parse_args()
    baseline = json.loads(args.baseline.read_text()) if args.baseline else None
    if baseline is not None and not baseline.get("passing_cases"):
        parser.error("QT3 baseline has no passing cases")
    if not (args.suite / "catalog.xml").is_file():
        parser.error("missing QT3 catalog")
    revision = subprocess.check_output(["git", "-C", str(args.suite), "rev-parse", "HEAD"], text=True).strip()
    dirty = subprocess.check_output(["git", "-C", str(args.suite), "status", "--porcelain"], text=True).strip()
    if revision != PIN or dirty:
        parser.error(f"QT3 must be clean and pinned to {PIN}")
    subprocess.run(["cargo", "build", "--offline", "--example", "reference_driver"], cwd=ROOT, check=True)
    root = parse_catalog(args.suite / "catalog.xml")
    registry = {e.get("name"): (e, args.suite) for e in root.findall(NS + "environment")}
    for e in root.findall(NS + "environment"):
        registry[id(e)] = e, args.suite
    selected = set(filter(None, args.sets.split(","))) if args.sets is not None else baseline_sets(baseline) if baseline is not None else {"op-to", "op-bang"}
    found, rows = set(), []
    driver = Driver(ROOT / "target/debug/examples/reference_driver")
    try:
        for ref in root.findall(NS + "test-set"):
            name = ref.get("name")
            if name not in selected:
                continue
            found.add(name)
            path = args.suite / ref.get("file")
            test_set = parse_catalog(path)
            local = dict(registry)
            for e in test_set.findall(NS + "environment"):
                local[e.get("name")] = e, path.parent
                local[id(e)] = e, path.parent
            for case in test_set.findall(NS + "test-case"):
                try:
                    dependencies(test_set)
                    status, reason = run_case(case, path.parent, local, driver)
                except ProfileExcluded as e:
                    status, reason = "profile_excluded", str(e)
                except (HarnessError, OSError) as e:
                    status, reason = "harness_error", str(e)
                rows.append({"set": name, "case": case.get("name"), "status": status, "reason": reason})
            print(name, dict(Counter(c["status"] for c in rows if c["set"] == name)), flush=True)
    finally:
        driver.close()
    if selected != found or not rows:
        parser.error(f"missing QT3 sets/cases: {selected - found}")
    counts = dict(Counter(c["status"] for c in rows))
    args.report.parent.mkdir(parents=True, exist_ok=True)
    args.report.write_text(json.dumps({"suite_revision": revision, "counts": counts, "cases": rows}, indent=2) + "\n")
    print("Selected cases:", len(rows), counts, "report:", args.report)
    if args.baseline:
        if baseline["suite_revision"] != revision:
            parser.error("QT3 baseline revision differs")
        actual = {(c["set"], c["case"]): c["status"] for c in rows}
        missing = [c for c in baseline["passing_cases"] if actual.get(tuple(c)) != "pass"]
        print("QT3 baseline cases:", len(baseline["passing_cases"]), "regressions/missing:", missing)
        return int(bool(missing))
    return int(bool(counts.get("fail") or counts.get("harness_error")))

if __name__ == "__main__":
    sys.exit(main())
