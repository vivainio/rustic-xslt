//! Compile a Schematron schema with an external XSLT transpiler and reuse the
//! generated validator for one or more XML documents.
use rust_xslt::{Diagnostic, ResourceResolver, Stylesheet, TransformOptions};
use std::{
    error::Error,
    fs,
    path::{Path, PathBuf},
};

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

/// Only load local files within the schema's directory, following canonical paths.
fn local_resolver(root: PathBuf) -> ResourceResolver {
    ResourceResolver::new(move |uri| {
        let unavailable = || Diagnostic {
            code: "FODC0002".into(),
            message: "resource is unavailable or outside the schema directory".into(),
            phase: "run",
            line: 0,
            column: 0,
        };
        let path = uri
            .strip_prefix("file://")
            .filter(|path| path.starts_with('/') && !path.contains(['?', '#']))
            .ok_or_else(unavailable)?;
        let mut decoded = Vec::new();
        let mut bytes = path.bytes();
        while let Some(byte) = bytes.next() {
            if byte == b'%' {
                let first = bytes
                    .next()
                    .and_then(|b| (b as char).to_digit(16))
                    .ok_or_else(unavailable)?;
                let second = bytes
                    .next()
                    .and_then(|b| (b as char).to_digit(16))
                    .ok_or_else(unavailable)?;
                decoded.push((first * 16 + second) as u8);
            } else {
                decoded.push(byte);
            }
        }
        let decoded = String::from_utf8(decoded).map_err(|_| unavailable())?;
        #[cfg(windows)]
        let decoded = decoded.strip_prefix('/').unwrap_or(&decoded).to_owned();
        let path = Path::new(&decoded)
            .canonicalize()
            .map_err(|_| unavailable())?;
        if !path.starts_with(&root) {
            return Err(unavailable());
        }
        fs::read_to_string(path).map_err(|_| unavailable())
    })
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut arguments = std::env::args_os().skip(1);
    let transpiler_path = arguments
        .next()
        .ok_or("usage: schematron TRANSPILER.xsl SCHEMA.sch [--iso] [--iso-preprocess] [--phase NAME] [--max-steps N] DOCUMENT.xml [DOCUMENT.xml ...]")?;
    let schema_path = arguments.next().ok_or("missing Schematron schema")?;
    let mut phase = None;
    let mut max_steps = None;
    let mut iso = false;
    let mut iso_preprocess = false;
    let mut documents = Vec::new();
    while let Some(argument) = arguments.next() {
        if argument == "--iso-preprocess" {
            if iso_preprocess {
                return Err("--iso-preprocess may be supplied only once".into());
            }
            iso_preprocess = true;
        } else if argument == "--iso" {
            if iso {
                return Err("--iso may be supplied only once".into());
            }
            iso = true;
        } else if argument == "--phase" {
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
        } else if argument == "--max-steps" {
            if max_steps.is_some() {
                return Err("--max-steps may be supplied only once".into());
            }
            max_steps = Some(
                arguments
                    .next()
                    .ok_or("--max-steps requires a value")?
                    .into_string()
                    .map_err(|_| "max steps must be UTF-8")?
                    .parse::<u64>()?,
            );
        } else {
            documents.push(argument);
        }
    }
    if documents.is_empty() {
        return Err("provide at least one XML document".into());
    }
    if iso_preprocess && !iso {
        return Err("--iso-preprocess requires --iso".into());
    }
    let transpiler_path = Path::new(&transpiler_path);
    let schema_path = Path::new(&schema_path);
    let schema_uri = file_uri(schema_path)?;
    let schema_root = schema_path
        .canonicalize()?
        .parent()
        .ok_or("schema has no parent directory")?
        .to_path_buf();
    let resolver = local_resolver(schema_root);
    let compiler_root = transpiler_path
        .canonicalize()?
        .parent()
        .ok_or("transpiler has no parent directory")?
        .to_path_buf();
    let compiler_resolver = local_resolver(compiler_root.clone());
    let transpiler = Stylesheet::compile_with_base_uri_and_resolver(
        &fs::read_to_string(transpiler_path)?,
        &file_uri(transpiler_path)?,
        &*compiler_resolver.0,
    )?;
    let mut schema_options = TransformOptions {
        source_uri: Some(schema_uri.clone()),
        resolver: Some(resolver.clone()),
        ..Default::default()
    };
    if let Some(max_steps) = max_steps {
        schema_options.limits.max_steps = max_steps;
    }
    if iso {
        schema_options
            .parameters
            .insert("allow-foreign".into(), "true".into());
    }
    if let Some(phase) = phase {
        let name = if iso {
            "phase"
        } else {
            "Q{http://dmaus.name/ns/2023/schxslt}phase"
        };
        schema_options.parameters.insert(name.into(), phase);
    }
    let mut schema = fs::read_to_string(schema_path)?;
    if iso_preprocess {
        let mut preprocess_options = schema_options.clone();
        preprocess_options.parameters.clear();
        for name in ["iso_dsdl_include.xsl", "iso_abstract_expand.xsl"] {
            let path = compiler_root.join(name);
            let stage = Stylesheet::compile_with_base_uri_and_resolver(
                &fs::read_to_string(&path)?,
                &file_uri(&path)?,
                &*compiler_resolver.0,
            )?;
            schema = stage.transform_with_options(&schema, &preprocess_options)?;
        }
    }
    let generated = transpiler.transform_with_options(&schema, &schema_options)?;
    let validator =
        Stylesheet::compile_with_base_uri_and_resolver(&generated, &schema_uri, &*resolver.0)?;
    for path in documents {
        let path = Path::new(&path);
        let mut options = TransformOptions {
            source_uri: Some(file_uri(path)?),
            resolver: Some(resolver.clone()),
            ..Default::default()
        };
        if let Some(max_steps) = max_steps {
            options.limits.max_steps = max_steps;
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn resolver_only_reads_canonical_files_in_its_schema_directory() {
        let root = std::env::temp_dir().join(format!("rust-xslt-example-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let path = root.join("included file.sch");
        fs::write(&path, "<included/>").unwrap();
        let resolver = local_resolver(root.canonicalize().unwrap());
        assert_eq!(
            (resolver.0)(&file_uri(&path).unwrap()).unwrap(),
            "<included/>"
        );
        assert!((resolver.0)("https://example.test/file").is_err());
        assert!((resolver.0)("file:///etc/passwd").is_err());
        assert!((resolver.0)("file://host/file").is_err());
        assert!((resolver.0)("file:///invalid%zz").is_err());
        #[cfg(unix)]
        {
            let link = root.join("outside");
            std::os::unix::fs::symlink("/etc/passwd", &link).unwrap();
            let link_uri = format!("{}/outside", file_uri(&root).unwrap());
            assert!((resolver.0)(&link_uri).is_err());
        }
        fs::remove_dir_all(root).unwrap();
    }
}
