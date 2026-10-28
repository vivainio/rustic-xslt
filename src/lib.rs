//! An early, deliberately limited XSLT engine. See README.md for supported syntax.
mod compile;
mod node;
mod number;
mod pattern;
mod qname;
mod result;
mod runtime;
mod seqtype;
mod sort;
mod stream;
mod trees;
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
}

/// Immutable compiled instructions, reusable across threads and transformations.
#[derive(Debug)]
pub struct Stylesheet {
    templates: Vec<compile::Template>,
    omit_declaration: bool,
    text_output: bool,
    backwards: bool,
    version: u8,
    globals: Vec<compile::Binding>,
    space_rules: Vec<whitespace::Rule>,
}
impl Stylesheet {
    pub fn compile(source: &str) -> Result<Self, Diagnostic> {
        compile::compile(source)
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
