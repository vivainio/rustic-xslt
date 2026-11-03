//! Compile a Schematron schema with an external XSLT transpiler and reuse the
//! generated validator for one or more XML documents.
use rust_xslt::{Diagnostic, ResourceResolver, Stylesheet, TransformOptions};
use std::{
    error::Error,
    fs,
    path::{Path, PathBuf},
    process::ExitCode,
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

/// Only load local files within the selected directory, following canonical paths.
fn local_resolver(root: PathBuf) -> ResourceResolver {
    local_resolver_roots(vec![root])
}

/// Allow resources under any explicitly selected directory.
fn local_resolver_roots(roots: Vec<PathBuf>) -> ResourceResolver {
    ResourceResolver::new(move |uri| {
        let unavailable = || Diagnostic {
            code: "FODC0002".into(),
            message: "resource is unavailable or outside the allowed directories".into(),
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
        if !roots.iter().any(|root| path.starts_with(root)) {
            return Err(unavailable());
        }
        fs::read_to_string(path).map_err(|_| unavailable())
    })
}

fn transform_stage(
    sheet: &Stylesheet,
    source: &str,
    options: &TransformOptions,
    label: &str,
    reject_messages: bool,
) -> Result<String, Box<dyn Error>> {
    let result = sheet
        .transform_with_documents(source, options)
        .map_err(|error| format!("{label}: {error}"))?;
    for message in &result.messages {
        eprintln!("{label}: {message}");
    }
    if reject_messages && !result.messages.is_empty() {
        return Err(
            format!("{label} emitted messages; use --allow-compiler-messages to continue").into(),
        );
    }
    Ok(result.principal)
}

// The NEMSIS ISO skeleton computes its traversal expression in a global
// variable whose context is the document node. Its parent::node() guard then
// removes @* even when the schema has attribute-context rules.
fn has_attribute_rule_context(schema: &str) -> bool {
    let Ok(document) = roxmltree::Document::parse(schema) else {
        return false;
    };
    document.descendants().any(|node| {
        if !node.has_tag_name(("http://purl.oclc.org/dsdl/schematron", "rule")) {
            return false;
        }
        let Some(context) = node.attribute("context") else {
            return false;
        };
        let mut brackets = 0usize;
        let mut quote = None;
        for (index, ch) in context.char_indices() {
            if let Some(delimiter) = quote {
                if ch == delimiter {
                    quote = None;
                }
                continue;
            }
            match ch {
                '\'' | '"' => quote = Some(ch),
                '[' => brackets += 1,
                ']' => brackets = brackets.saturating_sub(1),
                '@' if brackets == 0 => return true,
                'a' if brackets == 0 && context[index..].starts_with("attribute::") => {
                    return true;
                }
                _ => {}
            }
        }
        false
    })
}

fn fix_iso_attribute_contexts(source: &str) -> Result<String, Box<dyn Error>> {
    let changes = [
        (
            "$attributes='true' and parent::node()",
            "$attributes='true'",
        ),
        (
            "<axsl:template match=\"*\" mode=\"schematron-select-full-path\">",
            "<axsl:template match=\"*|@*\" mode=\"schematron-select-full-path\">",
        ),
        (
            "<!-- XSLT1 syntax -->\n\t\t<axsl:text>/</axsl:text>",
            "<!-- XSLT1 syntax -->\n<axsl:apply-templates select=\"parent::*\" mode=\"schematron-get-full-path\"/>\n\t\t<axsl:text>/</axsl:text>",
        ),
    ];
    let mut result = source.to_owned();
    for (before, after) in changes {
        if result.matches(before).count() != 1 {
            return Err("ISO attribute fix does not match this compiler version".into());
        }
        result = result.replacen(before, after, 1);
    }
    Ok(result)
}

#[cfg(test)]
mod attribute_context_tests {
    use super::{fix_iso_attribute_contexts, has_attribute_rule_context};

    #[test]
    fn recognizes_attribute_contexts_outside_predicates() {
        for context in ["r/@id", "r | attribute::id", "@id"] {
            let schema = format!(
                r#"<schema xmlns="http://purl.oclc.org/dsdl/schematron"><rule context="{context}"/></schema>"#
            );
            assert!(has_attribute_rule_context(&schema), "{context}");
        }
        for context in ["r[@id]", "r[contains(@id, '@')]", "r[@id='@']"] {
            let schema = format!(
                r#"<schema xmlns="http://purl.oclc.org/dsdl/schematron"><rule context="{context}"/></schema>"#
            );
            assert!(!has_attribute_rule_context(&schema), "{context}");
        }
    }

    #[test]
    fn attribute_fix_requires_the_expected_unmodified_skeleton() {
        let source = "$attributes='true' and parent::node()\n<axsl:template match=\"*\" mode=\"schematron-select-full-path\">\n<!-- XSLT1 syntax -->\n\t\t<axsl:text>/</axsl:text>";
        let fixed = fix_iso_attribute_contexts(source).unwrap();
        assert!(!fixed.contains("and parent::node()"));
        assert!(fixed.contains("match=\"*|@*\""));
        assert!(fixed.contains("select=\"parent::*\" mode=\"schematron-get-full-path\""));
        assert!(fix_iso_attribute_contexts("unrecognized compiler").is_err());
        assert!(fix_iso_attribute_contexts(&format!("{source}{source}")).is_err());
    }
}

fn main() -> Result<ExitCode, Box<dyn Error>> {
    let mut arguments = std::env::args_os().skip(1);
    let transpiler_path = arguments
        .next()
        .ok_or("usage: schematron TRANSPILER.xsl SCHEMA.sch [--iso] [--iso-preprocess] [--iso-fix-attribute-contexts] [--allow-compiler-messages] [--fail-on-findings] [--phase NAME] [--max-steps N] [--max-depth N] DOCUMENT.xml [DOCUMENT.xml ...]")?;
    let schema_path = arguments.next().ok_or("missing Schematron schema")?;
    let mut phase = None;
    let mut max_steps = None;
    let mut max_depth = None;
    let mut iso = false;
    let mut iso_preprocess = false;
    let mut iso_fix_attributes = false;
    let mut allow_compiler_messages = false;
    let mut fail_on_findings = false;
    let mut documents = Vec::new();
    while let Some(argument) = arguments.next() {
        if argument == "--fail-on-findings" {
            if fail_on_findings {
                return Err("--fail-on-findings may be supplied only once".into());
            }
            fail_on_findings = true;
        } else if argument == "--allow-compiler-messages" {
            if allow_compiler_messages {
                return Err("--allow-compiler-messages may be supplied only once".into());
            }
            allow_compiler_messages = true;
        } else if argument == "--iso-fix-attribute-contexts" {
            if iso_fix_attributes {
                return Err("--iso-fix-attribute-contexts may be supplied only once".into());
            }
            iso_fix_attributes = true;
        } else if argument == "--iso-preprocess" {
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
        } else if argument == "--max-depth" {
            if max_depth.is_some() {
                return Err("--max-depth may be supplied only once".into());
            }
            max_depth = Some(
                arguments
                    .next()
                    .ok_or("--max-depth requires a value")?
                    .into_string()
                    .map_err(|_| "max depth must be UTF-8")?
                    .parse::<usize>()?,
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
    if iso_fix_attributes && !iso {
        return Err("--iso-fix-attribute-contexts requires --iso".into());
    }
    let transpiler_path = Path::new(&transpiler_path);
    let schema_path = Path::new(&schema_path);
    let schema_uri = file_uri(schema_path)?;
    let schema_root = schema_path
        .canonicalize()?
        .parent()
        .ok_or("schema has no parent directory")?
        .to_path_buf();
    let resolver = local_resolver(schema_root.clone());
    let compiler_root = transpiler_path
        .canonicalize()?
        .parent()
        .ok_or("transpiler has no parent directory")?
        .to_path_buf();
    let compiler_resolver = local_resolver(compiler_root.clone());
    let fix_applied = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let compiler_resolver = if iso_fix_attributes {
        let skeleton = compiler_root.join("iso_schematron_skeleton_for_saxon.xsl");
        let skeleton_uri = file_uri(&skeleton)?;
        let patched = fix_iso_attribute_contexts(&(compiler_resolver.0)(&skeleton_uri)?)?;
        let compiler_source = fs::read_to_string(transpiler_path)?;
        if !compiler_source.contains("iso_schematron_skeleton_for_saxon.xsl") {
            return Err(
                "ISO attribute fix requires a compiler importing the recognized skeleton".into(),
            );
        }
        let applied = fix_applied.clone();
        ResourceResolver::new(move |uri| {
            if uri == skeleton_uri {
                applied.store(true, std::sync::atomic::Ordering::Relaxed);
                Ok(patched.clone())
            } else {
                (compiler_resolver.0)(uri)
            }
        })
    } else {
        compiler_resolver
    };
    let transpiler = Stylesheet::compile_with_base_uri_and_resolver(
        &fs::read_to_string(transpiler_path)?,
        &file_uri(transpiler_path)?,
        &*compiler_resolver.0,
    )?;
    if iso_fix_attributes {
        if !fix_applied.load(std::sync::atomic::Ordering::Relaxed) {
            return Err("ISO attribute fix was not used by the compiler".into());
        }
        eprintln!("using ISO compiler attribute-context compatibility fix");
    }
    let mut schema_options = TransformOptions {
        source_uri: Some(schema_uri.clone()),
        resolver: Some(resolver.clone()),
        ..Default::default()
    };
    if let Some(max_steps) = max_steps {
        schema_options.limits.max_steps = max_steps;
    }
    if let Some(max_depth) = max_depth {
        schema_options.limits.max_depth = max_depth;
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
            schema = transform_stage(
                &stage,
                &schema,
                &preprocess_options,
                name,
                !allow_compiler_messages,
            )?;
        }
    }
    if iso
        && !iso_fix_attributes
        && has_attribute_rule_context(&schema)
        && fs::read_to_string(compiler_root.join("iso_schematron_skeleton_for_saxon.xsl"))
            .is_ok_and(|s| s.contains("$attributes='true' and parent::node()"))
    {
        eprintln!(
            "warning: this ISO compiler can skip attribute-context rules; use a different Schematron compiler or review its generated validator"
        );
    }
    let generated = transform_stage(
        &transpiler,
        &schema,
        &schema_options,
        "Schematron compiler",
        !allow_compiler_messages,
    )?;
    let validator =
        Stylesheet::compile_with_base_uri_and_resolver(&generated, &schema_uri, &*resolver.0)?;
    let mut findings = false;
    for path in documents {
        let path = Path::new(&path);
        let document_root = path
            .canonicalize()?
            .parent()
            .ok_or("document has no parent directory")?
            .to_path_buf();
        let mut options = TransformOptions {
            source_uri: Some(file_uri(path)?),
            resolver: Some(local_resolver_roots(vec![
                schema_root.clone(),
                document_root,
            ])),
            ..Default::default()
        };
        if let Some(max_steps) = max_steps {
            options.limits.max_steps = max_steps;
        }
        if let Some(max_depth) = max_depth {
            options.limits.max_depth = max_depth;
        }
        let report = transform_stage(
            &validator,
            &fs::read_to_string(path)?,
            &options,
            &path.display().to_string(),
            false,
        )?;
        let document = roxmltree::Document::parse(&report)?;
        let svrl = "http://purl.oclc.org/dsdl/svrl";
        if !document
            .root_element()
            .has_tag_name((svrl, "schematron-output"))
        {
            return Err(format!(
                "{}: validator did not produce an SVRL report",
                path.display()
            )
            .into());
        }
        if !document
            .descendants()
            .any(|n| n.has_tag_name((svrl, "fired-rule")))
        {
            eprintln!("{}: warning: no Schematron rules fired", path.display());
        }
        let failures = document
            .descendants()
            .filter(|n| n.has_tag_name((svrl, "failed-assert")))
            .count();
        let reports = document
            .descendants()
            .filter(|n| n.has_tag_name((svrl, "successful-report")))
            .count();
        findings |= failures > 0 || reports > 0;
        eprintln!(
            "{}: {failures} failed assertions, {reports} successful reports",
            path.display()
        );
        println!("{report}");
    }
    Ok(if fail_on_findings && findings {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validation_resolver_reads_only_schema_and_document_trees() {
        let root =
            std::env::temp_dir().join(format!("rust-xslt-validation-{}", std::process::id()));
        let schema = root.join("schema");
        let document = root.join("document");
        let outside = root.join("outside");
        for directory in [&schema, &document, &outside] {
            fs::create_dir_all(directory).unwrap();
            fs::write(
                directory.join("resource.xml"),
                directory.file_name().unwrap().to_string_lossy().as_bytes(),
            )
            .unwrap();
        }
        let resolver = local_resolver_roots(vec![
            schema.canonicalize().unwrap(),
            document.canonicalize().unwrap(),
        ]);
        assert_eq!(
            (resolver.0)(&file_uri(&schema.join("resource.xml")).unwrap()).unwrap(),
            "schema"
        );
        assert_eq!(
            (resolver.0)(&file_uri(&document.join("resource.xml")).unwrap()).unwrap(),
            "document"
        );
        assert!((resolver.0)(&file_uri(&outside.join("resource.xml")).unwrap()).is_err());
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(outside.join("resource.xml"), document.join("escape.xml"))
                .unwrap();
            let link = format!("{}/escape.xml", file_uri(&document).unwrap());
            assert!((resolver.0)(&link).is_err());
        }
        fs::remove_dir_all(root).unwrap();
    }

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
