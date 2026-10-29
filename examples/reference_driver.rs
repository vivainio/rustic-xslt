//! Development-only JSON-lines protocol used by the W3C/differential harness.
use rust_xslt::{Diagnostic, ParameterValue, Stylesheet, TransformOptions};
use serde_json::{Value, json};
use std::io::{self, BufRead, Write};
fn diagnostic(e: Diagnostic) -> Value {
    json!({ "code": e.code, "message": e.message, "phase": e.phase, "line": e.line, "col": e.column })
}
fn parameter(value: &Value) -> Result<ParameterValue, Diagnostic> {
    let invalid = || Diagnostic {
        code: "RUST_XSLT_INVALID_PARAMETER".into(),
        message: "invalid driver parameter value".into(),
        phase: "run",
        line: 0,
        column: 0,
    };
    if let Some(s) = value.as_str() {
        return Ok(ParameterValue::String(s.into()));
    }
    if let Some(b) = value.as_bool() {
        return Ok(ParameterValue::Boolean(b));
    }
    let text = value.get("value").and_then(Value::as_str);
    Ok(match value.get("type").and_then(Value::as_str) {
        Some("empty") => ParameterValue::Empty,
        Some("integer") => ParameterValue::Integer(text.ok_or_else(invalid)?.into()),
        Some("decimal") => ParameterValue::Decimal(text.ok_or_else(invalid)?.into()),
        Some("untypedAtomic") => ParameterValue::UntypedAtomic(text.ok_or_else(invalid)?.into()),
        Some("double" | "float") => {
            let s = text.ok_or_else(invalid)?;
            let n = match s {
                "INF" => f64::INFINITY,
                "-INF" => f64::NEG_INFINITY,
                "NaN" => f64::NAN,
                _ => s.parse().map_err(|_| invalid())?,
            };
            if value["type"] == "float" {
                ParameterValue::Float(n as f32)
            } else {
                ParameterValue::Double(n)
            }
        }
        _ => return Err(invalid()),
    })
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
    let resources: std::collections::BTreeMap<String, String> = req
        .get("resources")
        .and_then(Value::as_object)
        .map(|m| {
            m.iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_owned())))
                .collect()
        })
        .unwrap_or_default();
    let mut options = TransformOptions {
        resolver: Some(rust_xslt::ResourceResolver::new(move |href| {
            resources.get(href).cloned().ok_or_else(|| Diagnostic {
                code: "FODC0002".into(),
                message: format!("resource not found: {href}"),
                phase: "run",
                line: 0,
                column: 0,
            })
        })),
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
    for (key, kind) in [
        ("params", 0),
        ("initialParams", 1),
        ("initialTunnelParams", 2),
    ] {
        if let Some(params) = req.get(key).and_then(Value::as_object) {
            for (k, v) in params {
                let value = match parameter(v) {
                    Ok(value) => value,
                    Err(e) => return json!({"diagnostics":[diagnostic(e)]}),
                };
                match kind {
                    1 => {
                        options.initial_parameters.insert(k.clone(), value);
                    }
                    2 => {
                        options.initial_tunnel_parameters.insert(k.clone(), value);
                    }
                    _ => {
                        options.typed_parameters.insert(k.clone(), value);
                    }
                }
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
