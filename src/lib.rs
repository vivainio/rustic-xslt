//! An early, deliberately limited XSLT engine. See README.md for supported syntax.
mod aggregate;
mod atomic_sequence;
mod backtrack;
mod binary;
mod collation;
mod compile;
mod deep_equal;
mod format_datetime;
mod format_number;
mod indent;
mod integer_type;
mod json;
mod maps;
mod mode;
mod node;
mod number;
mod numbering;
mod pattern;
mod qname;
mod regex_functions;
mod result;
mod runtime;
mod seqtype;
mod sort;
mod stream;
mod string_functions;
mod string_type;
mod temporal;
mod trees;
mod unicode_blocks;
mod uri_resolution;
mod whitespace;
mod xpath;
pub use stream::{StreamOptions, StreamStats};

use std::fmt;

/// A structured engine failure. Positions are one-based where available.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: String,
    pub message: String,
    pub phase: &'static str,
    pub line: u32,
    pub column: u32,
}
impl Diagnostic {
    pub(crate) fn new(code: &str, message: impl Into<String>, phase: &'static str) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            phase,
            line: 0,
            column: 0,
        }
    }
}
impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} ({}:{}): {}",
            self.code, self.line, self.column, self.message
        )
    }
}
impl std::error::Error for Diagnostic {}

/// Execution limits. Zero means no work/output is permitted, rather than unlimited.
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub max_steps: u64,
    pub max_depth: usize,
    pub max_output_bytes: usize,
    /// Aggregate serialized source bytes retained for temporary trees per run.
    pub max_temporary_bytes: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_steps: 20_000_000,
            max_depth: 128,
            max_output_bytes: 16 * 1024 * 1024,
            max_temporary_bytes: 64 * 1024 * 1024,
        }
    }
}

/// Atomic values accepted by the transformation parameter API.
/// Exact numeric values use lexical strings and the engine's numeric digit limit.
#[derive(Debug, Clone)]
pub enum ParameterValue {
    Empty,
    String(String),
    UntypedAtomic(String),
    Boolean(bool),
    Integer(String),
    Decimal(String),
    Double(f64),
    Float(f32),
}
impl From<String> for ParameterValue {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}
impl From<&str> for ParameterValue {
    fn from(value: &str) -> Self {
        Self::String(value.into())
    }
}
impl From<bool> for ParameterValue {
    fn from(value: bool) -> Self {
        Self::Boolean(value)
    }
}
impl From<i64> for ParameterValue {
    fn from(value: i64) -> Self {
        Self::Integer(value.to_string())
    }
}

/// Loads a resource (a text file, for `unparsed-text()` and `json-doc()`) by URI.
type LoadResource = dyn Fn(&str) -> Result<String, Diagnostic> + Send + Sync;
#[derive(Clone)]
pub struct ResourceResolver(pub std::sync::Arc<LoadResource>);
impl std::fmt::Debug for ResourceResolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ResourceResolver")
    }
}
impl ResourceResolver {
    pub fn new(
        resolver: impl Fn(&str) -> Result<String, Diagnostic> + Send + Sync + 'static,
    ) -> Self {
        Self(std::sync::Arc::new(resolver))
    }
}

/// Per-run string parameters, entry point and execution limits.
/// Component names are local names or `Q{uri}local`; lexical prefixes are invalid.
#[derive(Debug, Clone, Default)]
pub struct TransformOptions {
    /// String-valued global parameters, retained for compatibility.
    pub parameters: std::collections::BTreeMap<String, String>,
    /// Atomic global parameters. Names must not duplicate `parameters`.
    pub typed_parameters: std::collections::BTreeMap<String, ParameterValue>,
    /// Ordinary parameters for the initial template or initial template rules.
    /// These are independent of global parameters, even when names coincide.
    pub initial_parameters: std::collections::BTreeMap<String, ParameterValue>,
    /// Tunnel parameters for the initial template or initial template rules.
    pub initial_tunnel_parameters: std::collections::BTreeMap<String, ParameterValue>,
    pub initial_template: Option<String>,
    pub initial_mode: Option<String>,
    pub limits: Limits,
    /// Resource loader for `unparsed-text()`, `unparsed-text-lines()` and `json-doc()`.
    pub resolver: Option<ResourceResolver>,
}

/// A secondary result created by `xsl:result-document`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResultDocument {
    pub href: String,
    pub content: String,
}
/// The principal result and any secondary result documents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransformResult {
    pub principal: String,
    pub documents: Vec<ResultDocument>,
    /// Text of each `xsl:message` that did not terminate the transformation.
    pub messages: Vec<String>,
}
/// Immutable compiled instructions, reusable across threads and transformations.
#[derive(Debug)]
pub struct Stylesheet {
    templates: Vec<compile::Template>,
    omit_declaration: bool,
    standalone: Option<bool>,
    byte_order_mark: bool,
    indent: bool,
    markup: result::Markup,
    encoding: result::Encoding,
    cdata_elements: std::sync::Arc<Vec<String>>,
    character_maps: std::collections::BTreeMap<String, std::collections::BTreeMap<char, String>>,
    char_map: std::sync::Arc<std::collections::BTreeMap<char, String>>,
    doctype: Option<result::Doctype>,
    text_output: bool,
    backwards: bool,
    xpath_only: bool,
    version: u8,
    default_mode: String,
    modes: std::collections::BTreeMap<String, mode::Mode>,
    globals: Vec<compile::Binding>,
    functions: xpath::FunctionTable,
    keys: xpath::KeyTable,
    decimal_formats: xpath::DecimalFormats,
    document_uris: Vec<String>,
    outputs: std::collections::BTreeMap<String, result::Serialization>,
    attribute_sets: std::collections::BTreeMap<String, compile::AttributeSet>,
    space_rules: Vec<whitespace::Rule>,
}
impl Stylesheet {
    pub fn compile(source: &str) -> Result<Self, Diagnostic> {
        compile::compile(source, None)
    }
    /// Development bridge entry point: compile its stylesheet with the XPath function library.
    #[cfg(feature = "conformance")]
    #[doc(hidden)]
    pub fn compile_xpath_test_stylesheet(source: &str) -> Result<Self, Diagnostic> {
        xpath::with_xpath_profile(|| Self::compile(source))
    }
    #[cfg(feature = "conformance")]
    #[doc(hidden)]
    pub fn conformance_output_method(&self) -> &'static str {
        if self.text_output {
            "text"
        } else {
            match self.markup {
                result::Markup::Xml => "xml",
                result::Markup::Html => "html",
                result::Markup::Xhtml => "xhtml",
            }
        }
    }
    /// Compile a stylesheet whose `xsl:include` hrefs are loaded through `resolver`.
    pub fn compile_with_resolver(
        source: &str,
        resolver: &dyn Fn(&str) -> Result<String, Diagnostic>,
    ) -> Result<Self, Diagnostic> {
        compile::compile(source, Some(resolver))
    }
    pub fn transform(&self, source: &str) -> Result<String, Diagnostic> {
        self.transform_with_limits(source, Limits::default())
    }
    /// Transform a stream of fixed-depth XML records using a named template.
    /// Unsupported nonlocal expressions are rejected before input is consumed.
    /// Output is incremental and may be partial on failure.
    pub fn stream_records<R: std::io::BufRead, W: std::io::Write>(
        &self,
        input: R,
        output: &mut W,
        options: &StreamOptions,
    ) -> Result<StreamStats, Diagnostic> {
        stream::transform(self, input, output, options)
    }
    /// Transform and also return the documents created by `xsl:result-document`,
    /// each with the href it was written to.
    pub fn transform_with_documents(
        &self,
        source: &str,
        options: &TransformOptions,
    ) -> Result<TransformResult, Diagnostic> {
        let (principal, documents, messages) = runtime::transform_all(self, source, options)?;
        Ok(TransformResult {
            principal,
            messages,
            documents: documents
                .into_iter()
                .map(|(href, content)| ResultDocument { href, content })
                .collect(),
        })
    }
    pub fn transform_with_options(
        &self,
        source: &str,
        options: &TransformOptions,
    ) -> Result<String, Diagnostic> {
        runtime::transform(self, source, options)
    }
    pub fn transform_with_limits(
        &self,
        source: &str,
        limits: Limits,
    ) -> Result<String, Diagnostic> {
        runtime::transform(
            self,
            source,
            &TransformOptions {
                limits,
                ..TransformOptions::default()
            },
        )
    }
}

pub fn transform(stylesheet: &str, source: &str) -> Result<String, Diagnostic> {
    Stylesheet::compile(stylesheet)?.transform(source)
}

pub(crate) fn parse<'a>(
    source: &'a str,
    phase: &'static str,
) -> Result<roxmltree::Document<'a>, Diagnostic> {
    roxmltree::Document::parse_with_options(
        source,
        roxmltree::ParsingOptions {
            allow_dtd: false,
            nodes_limit: 1_000_000,
            entity_resolver: None,
        },
    )
    .map_err(|e| Diagnostic {
        code: if phase == "compile" {
            "XTSE0165"
        } else {
            "FODC0006"
        }
        .into(),
        message: e.to_string(),
        phase,
        line: e.pos().row,
        column: e.pos().col,
    })
}
