//! Development-only JSON-lines protocol used by the W3C/differential harness.
use rust_xslt::{Diagnostic, Stylesheet, TransformOptions};
use serde_json::{Value, json};
use std::io::{self, BufRead, Write};
fn diagnostic(e: Diagnostic) -> Value {
    json!({ "code": e.code, "message": e.message, "phase": e.phase, "line": e.line, "col": e.column })
}
fn run(req: Value) -> Value {
    let string = |key| req.get(key).and_then(Value::as_str).unwrap_or("");
    let sheet = match Stylesheet::compile(string("stylesheet")) {
        Ok(s) => s,
        Err(e) => return json!({"diagnostics":[diagnostic(e)]}),
    };
    if string("operation") == "compile" {
        return json!({"diagnostics":[]});
    }
    let mut options = TransformOptions {
        initial_template: req
            .get("initialTemplate")
            .and_then(Value::as_str)
            .map(str::to_owned),
        initial_mode: req
            .get("initialMode")
            .and_then(Value::as_str)
            .map(str::to_owned),
        limits: rust_xslt::Limits {
            max_steps: 1_000_000,
            ..Default::default()
        },
        ..Default::default()
    };
    if let Some(params) = req.get("params").and_then(Value::as_object) {
        for (k, v) in params {
            if let Some(s) = v.as_str() {
                options.parameters.insert(k.clone(), s.into());
            }
        }
    }
    let method = roxmltree::Document::parse(string("stylesheet"))
        .ok()
        .and_then(|d| {
            d.root_element()
                .children()
                .find(|n| {
                    n.is_element()
                        && n.tag_name().namespace() == Some("http://www.w3.org/1999/XSL/Transform")
                        && n.tag_name().name() == "output"
                })
                .and_then(|n| n.attribute("method"))
                .map(str::to_owned)
        })
        .unwrap_or("xml".into());
    match sheet.transform_with_options(string("source"), &options) {
        Ok(output) => json!({"output":output,"method":method,"diagnostics":[]}),
        Err(e) => json!({"diagnostics":[diagnostic(e)]}),
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut stdout = io::BufWriter::new(io::stdout().lock());
    for line in io::stdin().lock().lines() {
        let req: Value = serde_json::from_str(&line?)?;
        serde_json::to_writer(&mut stdout, &run(req))?;
        stdout.write_all(b"\n")?;
        stdout.flush()?;
    }
    Ok(())
}
