#!/usr/bin/env python3
"""Development-only, partial W3C XSLT catalog runner. No implied conformance.

Missing setup/assertion support is a harness_error, never a passing/hidden case.
Engine gaps are failures. Profile exclusions are named and counted separately.
"""
import argparse
from collections import Counter
import json
from pathlib import Path
import re
import subprocess
import sys
import xml.etree.ElementTree as ET
from xml.dom import Node, minidom
from xml.sax.saxutils import quoteattr

NS = "{http://www.w3.org/2012/10/xslt-test-catalog}"
PIN = "fddf1cf920087e791f13315d68dfbe874d97dc56"
ROOT = Path(__file__).resolve().parents[2]
NAMESPACES = {}

class HarnessError(Exception):
    pass

class ProfileExcluded(Exception):
    pass

def parse_catalog(path):
    bindings, pending, stack = {}, [], []
    for event, value in ET.iterparse(path, events=("start-ns", "start", "end")):
        if event == "start-ns":
            pending.append(value)
        elif event == "start":
            stack.append(bindings)
            bindings = dict(bindings)
            bindings.update(pending)
            pending.clear()
            NAMESPACES[id(value)] = dict(bindings)
        else:
            bindings = stack.pop()
    return value


def assert_stylesheet(assertion):
    bindings = NAMESPACES.get(id(assertion), {})
    prefix = "_harness_xsl"
    while prefix in bindings:
        prefix += "_"
    declarations = [f'xmlns:{prefix}="http://www.w3.org/1999/XSL/Transform"']
    for p, uri in bindings.items():
        if p and p != "xml":
            declarations.append(f"xmlns:{p}={quoteattr(uri)}")
    expression = quoteattr("boolean((" + text(assertion) + "))")
    return (f'<{prefix}:stylesheet version="3.0" {" ".join(declarations)}>'
            f'<{prefix}:output method="text"/>'
            f'<{prefix}:variable name="result" select="/"/>'
            f'<{prefix}:template match="/"><{prefix}:value-of select={expression}/></{prefix}:template>'
            f'</{prefix}:stylesheet>')

def tag(node):
    return node.tag.removeprefix(NS)

def children(node, name):
    return node.findall(NS + name)

def child(node, name):
    return node.find(NS + name)

def text(node):
    return "".join(node.itertext())

def check_attrs(node, allowed):
    unknown = set(node.attrib) - set(allowed)
    if unknown:
        raise HarnessError(f"unsupported {tag(node)} attributes: {sorted(unknown)}")

def read_resource(node, base):
    if node.get("file"):
        path = (base / node.get("file")).resolve()
        # Test catalogs are local trusted development inputs. No network fetching.
        if "://" in node.get("file"):
            raise HarnessError("network catalog resource")
        return path.read_text(encoding="utf-8-sig")
    content = child(node, "content")
    if content is not None:
        return text(content)
    raise HarnessError(f"{tag(node)} requires file or content")

def component_name(node):
    """Resolve catalog QName attributes in their own namespace scope."""
    name = node.get("name", "").strip()
    ncname = r"[A-Za-z_][A-Za-z0-9_.-]*"
    if re.fullmatch(ncname, name):
        return name
    if match := re.fullmatch(r"Q\{([^{}]*)\}(" + ncname + r")", name):
        uri, local = match.groups()
        uri = " ".join(re.findall(r"[^ \t\r\n]+", uri))
        return f"Q{{{uri}}}{local}" if uri else local
    if match := re.fullmatch(f"({ncname}):({ncname})", name):
        prefix, local = match.groups()
        uri = NAMESPACES.get(id(node), {}).get(prefix)
        if uri is None:
            raise HarnessError(f"unbound catalog name prefix: {prefix}")
        return f"Q{{{uri}}}{local}" if uri else local
    raise HarnessError(f"invalid catalog component name: {name}")

def parameters(nodes, allow_tunnel=False, tunnel=False):
    result, seen = {}, set()
    for p in nodes:
        if tag(p) != "param":
            raise HarnessError(f"unsupported initial-entry child: {tag(p)}")
        check_attrs(p, ["name", "select", "static", "tunnel"])
        if p.get("static", "no") != "no":
            raise HarnessError("static catalog parameters")
        flag = p.get("tunnel", "no").strip()
        if flag not in {"yes", "true", "1", "no", "false", "0"}:
            raise HarnessError("invalid catalog tunnel flag")
        is_tunnel = flag in {"yes", "true", "1"}
        if is_tunnel and not allow_tunnel:
            raise HarnessError("global tunnel catalog parameters")
        s = p.get("select", "").strip()
        if re.fullmatch(r"'(?:[^']|'')*'|\"(?:[^\"]|\"\")*\"", s):
            value = s[1:-1].replace(s[0] * 2, s[0])
        elif re.fullmatch(r"[+-]?[0-9]+", s):
            value = {"type": "integer", "value": s}
        elif re.fullmatch(r"[+-]?(?:[0-9]+\.[0-9]*|\.[0-9]+)", s):
            value = {"type": "decimal", "value": s}
        elif re.fullmatch(r"[+-]?(?:[0-9]+(?:\.[0-9]*)?|\.[0-9]+)[eE][+-]?[0-9]+", s):
            value = {"type": "double", "value": s}
        elif s in {"true()", "false()"}:
            value = s == "true()"
        elif s == "()":
            value = {"type": "empty"}
        else:
            raise HarnessError("unsupported catalog parameter expression")
        name = component_name(p)
        if name in seen:
            raise HarnessError(f"duplicate catalog parameter: {name}")
        seen.add(name)
        if is_tunnel == tunnel:
            result[name] = value
    return result

def environment(node, registry, seen=()):
    result = {"source": "", "params": {}}
    base = registry.get(id(node), (None, Path()))[1]
    reference = node.get("ref")
    if reference:
        if reference in seen or reference not in registry:
            raise HarnessError(f"unresolved/cyclic environment {reference}")
        parent, _ = registry[reference]
        result = environment(parent, registry, (*seen, reference))
    for part in node:
        kind = tag(part)
        if kind == "source":
            check_attrs(part, ["role", "file", "uri", "validation"])
            if part.get("role") != "." or part.get("validation", "skip") != "skip":
                raise HarnessError("source variable bindings or validation")
            result["source"] = read_resource(part, base)
        elif kind == "stylesheet":
            check_attrs(part, ["file", "uri"])
            result["stylesheet"] = read_resource(part, base)
        elif kind == "param":
            result["params"].update(parameters([part]))
        else:
            raise HarnessError(f"unsupported environment component: {kind}")
    return result

def dependencies(node):
    dep = child(node, "dependencies")
    if dep is None:
        return
    for d in dep:
        kind, value = tag(d), d.get("value", "")
        positive = d.get("satisfied", "true") != "false"
        if kind == "spec":
            if "XSLT40" in value:
                raise ProfileExcluded("XSLT 4.0 profile")
            if not any(v in value for v in ["XSLT10", "XSLT20", "XSLT30"]):
                raise HarnessError(f"unknown spec dependency: {value}")
        elif kind == "feature" and value in {"streaming", "schema_aware", "schemaImport", "schemaValidation", "schemaAware", "static_typing", "staticTyping", "streaming-fallback"}:
            if positive:
                raise ProfileExcluded(f"optional processor profile: {value}")
        elif kind == "feature" and value in {"backwards_compatibility", "namespace_axis"}:
            if not positive:
                raise ProfileExcluded(f"requires processor without {value}")
        else:
            raise HarnessError(f"unsupported dependency: {kind}={value}, satisfied={positive}")

def strip_declaration(xml):
    xml = xml.removeprefix("\ufeff")
    if xml.startswith("<?xml") and len(xml) > 5 and xml[5].isspace():
        end = xml.find("?>")
        if end < 0:
            raise ValueError("unterminated declaration")
        xml = xml[end + 2:]
    return xml

def fragment(xml):
    return minidom.parseString("<_result>" + strip_declaration(xml) + "</_result>").documentElement

def canonical(node, ignore_prefixes=False):
    def name(n):
        return n.namespaceURI or "", n.localName or "", "" if ignore_prefixes else n.prefix or ""
    if node.nodeType == Node.ELEMENT_NODE:
        attrs = sorted((name(a), a.value) for a in node.attributes.values() if a.namespaceURI != "http://www.w3.org/2000/xmlns/")
        contents = []
        for c in node.childNodes:
            item = canonical(c, ignore_prefixes)
            if item[0] == "text" and contents and contents[-1][0] == "text":
                contents[-1] = ("text", contents[-1][1] + item[1])
            else:
                contents.append(item)
        return "element", name(node), attrs, contents
    if node.nodeType in (Node.TEXT_NODE, Node.CDATA_SECTION_NODE):
        return "text", node.data
    if node.nodeType == Node.COMMENT_NODE:
        return "comment", node.data
    if node.nodeType == Node.PROCESSING_INSTRUCTION_NODE:
        return "pi", node.target, node.data
    raise HarnessError(f"unsupported result node type {node.nodeType}")

def string_value(node):
    if node.nodeType in (Node.TEXT_NODE, Node.CDATA_SECTION_NODE):
        return node.data
    return "".join(string_value(c) for c in node.childNodes) if node.nodeType == Node.ELEMENT_NODE else ""

def preflight(assertion):
    kind = tag(assertion)
    if kind in {"all-of", "any-of", "not"}:
        if not len(assertion):
            raise HarnessError(f"empty {kind}")
        for a in assertion:
            preflight(a)
    elif kind not in {"error", "assert-xml", "assert-serialization", "assert-string-value", "assert"}:
        raise HarnessError(f"unsupported assertion: {kind}")
    else:
        allowed = {"error": ["code"], "assert-xml": ["file", "ignore-prefixes"], "assert-serialization": ["file", "ignore-prefixes"], "assert-string-value": ["normalize-space"], "assert": []}[kind]
        check_attrs(assertion, allowed)

def assertion(a, response, base, driver=None):
    kind = tag(a)
    if kind == "all-of":
        results = [assertion(c, response, base, driver) for c in a]
        return all(r[0] for r in results), "; ".join(r[1] for r in results if not r[0])
    if kind == "any-of":
        results = [assertion(c, response, base, driver) for c in a]
        return any(r[0] for r in results), "; ".join(r[1] for r in results)
    if kind == "not":
        if len(a) != 1:
            raise HarnessError("not requires one assertion")
        if response.get("diagnostics"):
            return False, "unexpected error in negated assertion"
        ok, reason = assertion(a[0], response, base, driver)
        return not ok, "negated assertion: " + reason
    errors = response.get("diagnostics", [])
    if kind == "error":
        codes = a.get("code", "").split()
        if not codes:
            raise HarnessError("missing expected error code")
        actual = [e["code"] for e in errors]
        ok = any(c in codes or "*" in codes and not c.startswith(("RUST_", "INTERNAL")) for c in actual)
        return ok, f"expected error {codes}; got {actual}"
    if errors:
        e = errors[0]
        return False, f"{e['code']}: {e['message']}"
    output = response.get("output", "")
    if kind == "assert":
        if driver is None or response.get("method") != "xml":
            raise HarnessError("XPath assertions currently require an XML result document and a driver")
        evaluated = driver.call({"stylesheet": assert_stylesheet(a), "source": output})
        if evaluated.get("diagnostics"):
            e = evaluated["diagnostics"][0]
            raise HarnessError(f"XPath assertion could not be evaluated: {e['code']}: {e['message']}")
        if evaluated.get("output") not in {"true", "false"}:
            raise HarnessError("XPath assertion returned no boolean")
        return evaluated["output"] == "true", "XPath assertion was false"

    if kind in {"assert-xml", "assert-serialization"}:
        expected = (base / a.get("file")).read_text(encoding="utf-8-sig") if a.get("file") else text(a)
        try:
            ignore = a.get("ignore-prefixes") == "true"
            equal = canonical(fragment(expected), ignore) == canonical(fragment(output), ignore)
        except Exception as e:
            return False, f"XML comparison failed: {e}"
        return equal, "XML tree mismatch" if not equal else ""
    expected = text(a)
    if response.get("method") == "text":
        actual = output
    else:
        try:
            actual = string_value(fragment(output))
        except Exception as e:
            return False, f"invalid XML result: {e}"
    if a.get("normalize-space") == "true":
        actual, expected = " ".join(re.findall(r"[^ \t\r\n]+", actual)), " ".join(re.findall(r"[^ \t\r\n]+", expected))
    return actual == expected, f"string mismatch: expected {expected!r}, got {actual!r}"

class Driver:
    def __init__(self, executable):
        self.process = subprocess.Popen([str(executable)], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
    def call(self, request):
        self.process.stdin.write(json.dumps(request) + "\n")
        self.process.stdin.flush()
        line = self.process.stdout.readline()
        if not line:
            raise HarnessError(f"engine driver exited: {self.process.poll()}")
        return json.loads(line)
    def close(self):
        self.process.stdin.close()
        if self.process.wait() != 0:
            raise HarnessError("engine driver failed")

def run_case(case, base, registry, driver):
    dependencies(case)
    result = child(case, "result")
    if result is None or not len(result):
        raise HarnessError("case has no result assertions")
    check_attrs(result, [])
    for a in result:
        preflight(a)
    request = {"source": "", "params": {}}
    for env in children(case, "environment"):
        registry[id(env)] = env, base
        resolved = environment(env, registry)
        request.update({k: v for k, v in resolved.items() if k != "params"})
        request["params"].update(resolved["params"])
    test = child(case, "test")
    if test is None:
        raise HarnessError("missing test")
    for part in test:
        kind = tag(part)
        if kind == "stylesheet":
            check_attrs(part, ["file", "uri"])
            request["stylesheet"] = read_resource(part, base)
        elif kind == "param":
            request["params"].update(parameters([part]))
        elif kind in {"initial-template", "initial-mode"}:
            check_attrs(part, ["name"])
            request["initialParams"] = parameters(part, allow_tunnel=True)
            request["initialTunnelParams"] = parameters(part, allow_tunnel=True, tunnel=True)
            request["initialTemplate" if kind == "initial-template" else "initialMode"] = component_name(part)
        elif kind == "output":
            check_attrs(part, ["serialize"])
        else:
            raise HarnessError(f"unsupported test entry component: {kind}")
    if "stylesheet" not in request:
        raise HarnessError("no stylesheet")
    response = driver.call(request)
    answers = [assertion(a, response, base, driver) for a in result]
    return ("pass", "") if all(r[0] for r in answers) else ("fail", "; ".join(r[1] for r in answers if not r[0]))

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("suite", type=Path)
    parser.add_argument("--sets", default="", help="comma-separated exact test-set names")
    parser.add_argument("--case", default="", help="test-case name substring")
    parser.add_argument("--report", type=Path, default=Path("conformance-results/xslt30.json"))
    parser.add_argument("--allow-other-revision", action="store_true")
    parser.add_argument("--baseline", type=Path, help="gate only the committed passing cases; all other statuses remain reported")
    args = parser.parse_args()
    catalog = args.suite / "catalog.xml"
    if not catalog.is_file():
        parser.error(f"missing catalog: {catalog}")
    revision = subprocess.check_output(["git", "-C", str(args.suite), "rev-parse", "HEAD"], text=True).strip()
    dirty = subprocess.check_output(["git", "-C", str(args.suite), "status", "--porcelain"], text=True).strip()
    if dirty or revision != PIN and not args.allow_other_revision:
        parser.error(f"suite must be clean and pinned to {PIN}; got {revision}, dirty={bool(dirty)}")
    subprocess.run(["cargo", "build", "--offline", "--example", "reference_driver"], cwd=ROOT, check=True)
    driver = Driver(ROOT / "target/debug/examples/reference_driver")
    root = parse_catalog(catalog)
    registry = {e.get("name"): (e, args.suite) for e in children(root, "environment")}
    for e in children(root, "environment"):
        registry[id(e)] = e, args.suite
    rows = []
    sets = set(filter(None, args.sets.split(",")))
    found = set()
    try:
        for ref in children(root, "test-set"):
            name = ref.get("name")
            if sets and name not in sets:
                continue
            found.add(name)
            path = args.suite / ref.get("file")
            test_set = parse_catalog(path)
            local = dict(registry)
            for e in children(test_set, "environment"):
                local[e.get("name")] = e, path.parent
                local[id(e)] = e, path.parent
            for case in children(test_set, "test-case"):
                if args.case not in case.get("name", ""):
                    continue
                try:
                    dependencies(test_set)
                    status, reason = run_case(case, path.parent, local, driver)
                except ProfileExcluded as e:
                    status, reason = "profile_excluded", str(e)
                except (HarnessError, OSError, ET.ParseError) as e:
                    status, reason = "harness_error", str(e)
                rows.append({"set": name, "case": case.get("name"), "status": status, "reason": reason})
            print(name, dict(Counter(r["status"] for r in rows if r["set"] == name)), flush=True)
    finally:
        driver.close()
    if sets - found or not rows:
        parser.error(f"no matching cases/sets: {sets - found}")
    counts = dict(Counter(r["status"] for r in rows))
    args.report.parent.mkdir(parents=True, exist_ok=True)
    args.report.write_text(json.dumps({"suite_revision": revision, "counts": counts, "cases": rows}, indent=2) + "\n")
    print("Selected cases:", len(rows), counts, "report:", args.report)
    if args.baseline:
        baseline = json.loads(args.baseline.read_text())
        if baseline["suite_revision"] != revision:
            parser.error("baseline suite revision differs")
        actual = {(r["set"], r["case"]): r["status"] for r in rows}
        regressions = [case for case in baseline["passing_cases"] if actual.get(tuple(case)) != "pass"]
        print("Baseline cases:", len(baseline["passing_cases"]), "regressions/missing:", regressions)
        return int(bool(regressions))
    return int(bool(counts.get("fail") or counts.get("harness_error")))

if __name__ == "__main__":
    sys.exit(main())
