//! Code → IR: extract diagrams from source files via tree-sitter.
//!
//! Currently supports Rust. Languages plug in via [`Language`] variants +
//! per-language extractor modules. Each [`CodeKind`] produces a canonical
//! [`Document`] that flows through the existing render / export / MCP
//! pipeline.

mod rust_lang;
mod skeleton;
mod typescript_lang;

use crate::ir::{Document, IrError};
use std::fmt;

/// Source language understood by the code generator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Rust,
    TypeScript,
}

impl Language {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "rust" | "rs" => Some(Self::Rust),
            "typescript" | "ts" => Some(Self::TypeScript),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::TypeScript => "typescript",
        }
    }

    /// Infer language from a file extension. Returns `None` for unknown
    /// extensions so callers can fail with a clear message.
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "rs" => Some(Self::Rust),
            "ts" => Some(Self::TypeScript),
            _ => None,
        }
    }
}

impl fmt::Display for Language {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Diagram kind produced from a source file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodeKind {
    /// Class diagram from struct/enum/trait/impl items.
    Class,
    /// Module / file tree as a flowchart with subgraphs.
    Tree,
    /// Function call graph as a flowchart.
    Call,
}

impl CodeKind {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "class" | "classes" => Some(Self::Class),
            "tree" | "module-tree" | "modules" => Some(Self::Tree),
            "call" | "calls" | "callgraph" => Some(Self::Call),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Class => "class",
            Self::Tree => "tree",
            Self::Call => "call",
        }
    }
}

impl fmt::Display for CodeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Code-generation errors.
#[derive(Debug, Clone)]
pub struct CodeGenError {
    pub message: String,
}

impl fmt::Display for CodeGenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for CodeGenError {}

impl From<String> for CodeGenError {
    fn from(message: String) -> Self {
        Self { message }
    }
}

impl From<&str> for CodeGenError {
    fn from(message: &str) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl From<IrError> for CodeGenError {
    fn from(e: IrError) -> Self {
        Self {
            message: e.message,
        }
    }
}

/// Generate a [`Document`] from source text in the given language and kind.
pub fn from_source(
    source: &str,
    language: Language,
    kind: CodeKind,
) -> Result<Document, CodeGenError> {
    match language {
        Language::Rust => rust_lang::extract(source, kind),
        Language::TypeScript => typescript_lang::extract(source, kind),
    }
}

/// Generate a [`Document`] from a file path. Language is inferred from the
/// extension.
pub fn from_path(path: &std::path::Path, kind: CodeKind) -> Result<Document, CodeGenError> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .ok_or_else(|| CodeGenError::from("file has no extension; cannot infer language"))?;
    let language = Language::from_extension(ext).ok_or_else(|| {
        CodeGenError::from(format!(
            "unsupported source extension '.{ext}'; supported: rs (rust), ts (typescript)"
        ))
    })?;
    let source = std::fs::read_to_string(path)
        .map_err(|e| CodeGenError::from(format!("failed to read '{}': {}", path.display(), e)))?;
    from_source(&source, language, kind)
}

/// Resolve a source language for a CLI/MCP call: prefer the explicit
/// override, otherwise infer from the file extension.
pub fn resolve_language(
    path: &str,
    lang: Option<&str>,
) -> Result<Language, CodeGenError> {
    if let Some(s) = lang {
        return Language::parse(s).ok_or_else(|| {
            CodeGenError::from(format!(
                "unsupported language '{s}'; supported: rust, typescript"
            ))
        });
    }
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .ok_or_else(|| {
            CodeGenError::from(format!(
                "'{path}' has no extension; pass lang (rust|typescript)"
            ))
        })?;
    Language::from_extension(ext).ok_or_else(|| {
        CodeGenError::from(format!(
            "unsupported source extension '.{ext}'; pass lang (supported: rs, ts)"
        ))
    })
}

/// End-to-end code → IR → output file pipeline shared by CLI and MCP.
///
/// Reads the source file, resolves the language, runs the per-kind extractor,
/// and exports the resulting [`Document`] to `output` in the format chosen by
/// its extension (or `format` when supplied). Returns the document plus the
/// format it was written in so callers can format success messages.
pub fn write_to_path(
    path: &str,
    lang: Option<&str>,
    kind: CodeKind,
    output: &str,
    format: Option<crate::formats::Format>,
) -> Result<(Document, crate::formats::Format), CodeGenError> {
    let source = std::fs::read_to_string(path)
        .map_err(|e| CodeGenError::from(format!("failed to read '{path}': {e}")))?;
    let language = resolve_language(path, lang)?;
    let doc = from_source(&source, language, kind)?;
    let fmt = format.unwrap_or_else(|| crate::formats::Format::from_output_path(output));
    crate::formats::export_path(&doc, output, Some(fmt))?;
    Ok((doc, fmt))
}

/// Render a [`Document`] as a compilable source skeleton in `lang`.
///
/// This is the **UML → Code** direction: signatures come from the IR, bodies
/// are empty stubs, notes become comments. Deterministic and non-AI; see
/// ADR-0002.
pub fn skeleton(doc: &Document, lang: Language) -> String {
    skeleton::skeleton(doc, lang)
}

/// Load a diagram file and emit its skeleton. The diagram format is detected
/// from the file extension and content (Mermaid, JSON IR, DOT, D2, PlantUML).
pub fn skeleton_from_path(
    path: &str,
    lang: Language,
) -> Result<String, CodeGenError> {
    let doc = crate::ir::load_path(path).map_err(CodeGenError::from)?;
    Ok(skeleton::skeleton(&doc, lang))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn language_parse() {
        assert_eq!(Language::parse("rust"), Some(Language::Rust));
        assert_eq!(Language::parse("RS"), Some(Language::Rust));
        assert_eq!(Language::parse("python"), None);
    }

    #[test]
    fn code_kind_parse() {
        assert_eq!(CodeKind::parse("class"), Some(CodeKind::Class));
        assert_eq!(CodeKind::parse("tree"), Some(CodeKind::Tree));
        assert_eq!(CodeKind::parse("call"), Some(CodeKind::Call));
        assert_eq!(CodeKind::parse("nope"), None);
    }

    #[test]
    fn language_from_extension() {
        assert_eq!(Language::from_extension("rs"), Some(Language::Rust));
        assert_eq!(Language::from_extension("RS"), Some(Language::Rust));
        assert_eq!(Language::from_extension("py"), None);
    }

    #[test]
    fn from_path_reads_file_and_infers_language() {
        let dir = std::env::temp_dir();
        let pid = std::process::id();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = dir.join(format!("diagram_codegen_path_{pid}_{nanos}.rs"));
        {
            let mut f = std::fs::File::create(&path).unwrap();
            writeln!(
                f,
                "struct Foo {{ x: i32 }}\nenum Bar {{ A, B }}\nfn run() {{ Foo {{ x: 1 }} }}"
            )
            .unwrap();
        }
        let doc = from_path(&path, CodeKind::Class).unwrap();
        let primary = doc.primary().unwrap();
        let crate::ir::Diagram::Class(c) = primary else {
            panic!("expected class diagram");
        };
        let ids: Vec<&str> = c.classes.iter().map(|x| x.id.as_str()).collect();
        assert!(ids.contains(&"Foo"));
        assert!(ids.contains(&"Bar"));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn from_path_rejects_unknown_extension() {
        let dir = std::env::temp_dir();
        let pid = std::process::id();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = dir.join(format!("diagram_codegen_bad_{pid}_{nanos}.xyz"));
        std::fs::write(&path, "anything").unwrap();
        let err = from_path(&path, CodeKind::Class).unwrap_err();
        assert!(err.message.contains("unsupported source extension"));
        let _ = std::fs::remove_file(&path);
    }
}