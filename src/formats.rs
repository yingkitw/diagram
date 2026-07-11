//! Format detection and import/export adapters around the canonical IR.

use crate::ir::{self, Document, IrError};

/// Concrete serialization format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Mermaid,
    JsonIr,
}

impl Format {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "mermaid" | "mmd" => Some(Self::Mermaid),
            "json" | "ir" | "json-ir" | "json_ir" => Some(Self::JsonIr),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Mermaid => "mermaid",
            Self::JsonIr => "json",
        }
    }
}

/// Detect format from content and optional path extension.
pub fn detect(source: &str, path: Option<&str>) -> Format {
    let trimmed = source.trim_start();
    if trimmed.starts_with('{') {
        return Format::JsonIr;
    }
    if let Some(p) = path {
        let lower = p.to_lowercase();
        if lower.ends_with(".json") {
            return Format::JsonIr;
        }
        if lower.ends_with(".mmd") || lower.ends_with(".mermaid") {
            return Format::Mermaid;
        }
    }
    Format::Mermaid
}

/// Import source text in the given format into a Document.
pub fn import_str(source: &str, format: Format) -> Result<Document, IrError> {
    match format {
        Format::Mermaid => ir::from_mermaid(source),
        Format::JsonIr => Document::from_json(source)
            .map_err(|e| IrError::from(format!("invalid JSON IR: {e}"))),
    }
}

/// Export a Document to the given format.
pub fn export_str(doc: &Document, format: Format) -> Result<String, IrError> {
    match format {
        Format::Mermaid => doc.to_mermaid().map_err(IrError::from),
        Format::JsonIr => doc
            .to_json()
            .map_err(|e| IrError::from(format!("JSON serialize failed: {e}"))),
    }
}

/// Import a file into a Document (`from` overrides detection when set).
pub fn import_path(path: &str, from: Option<Format>) -> Result<(Document, Format), IrError> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read '{path}': {e}"))?;
    let format = from.unwrap_or_else(|| detect(&content, Some(path)));
    Ok((import_str(&content, format)?, format))
}

/// Export a Document to a path (format from `to` or destination extension).
pub fn export_path(doc: &Document, path: &str, to: Option<Format>) -> Result<Format, IrError> {
    let format = to.unwrap_or_else(|| {
        let lower = path.to_lowercase();
        if lower.ends_with(".json") {
            Format::JsonIr
        } else {
            Format::Mermaid
        }
    });
    let body = export_str(doc, format)?;
    std::fs::write(path, body).map_err(|e| format!("Failed to write '{path}': {e}"))?;
    Ok(format)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_json_object() {
        assert_eq!(detect("  {\"version\":1}", None), Format::JsonIr);
    }

    #[test]
    fn detect_extension() {
        assert_eq!(detect("graph TD\n", Some("x.json")), Format::JsonIr);
        assert_eq!(detect("graph TD\n", Some("x.mmd")), Format::Mermaid);
    }

    #[test]
    fn mermaid_to_json_to_mermaid() {
        let src = "graph TD\n  A[Start] --> B[End]\n";
        let doc = import_str(src, Format::Mermaid).unwrap();
        let json = export_str(&doc, Format::JsonIr).unwrap();
        let doc2 = import_str(&json, Format::JsonIr).unwrap();
        let out = export_str(&doc2, Format::Mermaid).unwrap();
        assert!(out.contains("A"));
        assert!(out.contains("B"));
    }
}
