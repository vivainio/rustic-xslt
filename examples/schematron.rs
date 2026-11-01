//! Compile a Schematron schema with an external SchXslt2 transpiler and reuse
//! the generated validator for one or more XML documents.
use rust_xslt::{Stylesheet, TransformOptions};
use std::{error::Error, fs, path::Path};

fn file_uri(path: &Path) -> Result<String, Box<dyn Error>> {
    let path = path.canonicalize()?;
    let text = path.to_string_lossy().replace('\\', "/");
    let mut uri = String::from("file://");
    if !text.starts_with('/') {
        uri.push('/');
    }
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'-' | b'_' | b'.' | b'~' | b':') {
            uri.push(char::from(byte));
        } else {
            uri.push_str(&format!("%{byte:02X}"));
        }
    }
    Ok(uri)
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut arguments = std::env::args_os().skip(1);
    let transpiler_path = arguments
        .next()
        .ok_or("usage: schematron TRANSPILER.xsl SCHEMA.sch DOCUMENT.xml [DOCUMENT.xml ...]")?;
    let schema_path = arguments.next().ok_or("missing Schematron schema")?;
    let mut phase = None;
    let mut documents = Vec::new();
    while let Some(argument) = arguments.next() {
        if argument == "--phase" {
            if phase.is_some() {
                return Err("--phase may be supplied only once".into());
            }
            phase = Some(
                arguments
                    .next()
                    .ok_or("--phase requires a value")?
                    .into_string()
                    .map_err(|_| "phase must be UTF-8")?,
            );
        } else {
            documents.push(argument);
        }
    }
    if documents.is_empty() {
        return Err("provide at least one XML document".into());
    }
    let transpiler_path = Path::new(&transpiler_path);
    let schema_path = Path::new(&schema_path);
    let schema_uri = file_uri(schema_path)?;
    let transpiler = Stylesheet::compile_with_base_uri(
        &fs::read_to_string(transpiler_path)?,
        &file_uri(transpiler_path)?,
    )?;
    let mut schema_options = TransformOptions {
        source_uri: Some(schema_uri.clone()),
        ..Default::default()
    };
    if let Some(phase) = phase {
        schema_options
            .parameters
            .insert("Q{http://dmaus.name/ns/2023/schxslt}phase".into(), phase);
    }
    let generated =
        transpiler.transform_with_options(&fs::read_to_string(schema_path)?, &schema_options)?;
    let validator = Stylesheet::compile_with_base_uri(&generated, &schema_uri)?;
    for path in documents {
        let path = Path::new(&path);
        let options = TransformOptions {
            source_uri: Some(file_uri(path)?),
            ..Default::default()
        };
        let report = validator.transform_with_options(&fs::read_to_string(path)?, &options)?;
        let document = roxmltree::Document::parse(&report)?;
        let svrl = "http://purl.oclc.org/dsdl/svrl";
        let failures = document
            .descendants()
            .filter(|n| n.has_tag_name((svrl, "failed-assert")))
            .count();
        let reports = document
            .descendants()
            .filter(|n| n.has_tag_name((svrl, "successful-report")))
            .count();
        eprintln!(
            "{}: {failures} failed assertions, {reports} successful reports",
            path.display()
        );
        println!("{report}");
    }
    Ok(())
}
