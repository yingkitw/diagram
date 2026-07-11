//! Mermaid Compatibility adapter: parse Mermaid source into canonical IR.

use crate::ir::{Diagram, Document, IrError};

/// Parse Mermaid source into a single typed diagram.
pub fn parse(source: &str) -> Result<Diagram, IrError> {
    if crate::sequence::is_sequence(source) {
        Ok(Diagram::Sequence(
            crate::sequence::parse(source).map_err(|e| e.to_string())?,
        ))
    } else if crate::class::is_class(source) {
        Ok(Diagram::Class(
            crate::class::parse(source).map_err(|e| e.to_string())?,
        ))
    } else if crate::gantt::is_gantt(source) {
        Ok(Diagram::Gantt(
            crate::gantt::parse(source).map_err(|e| e.to_string())?,
        ))
    } else {
        Ok(Diagram::Flowchart(
            crate::parser::parse(source).map_err(|e| e.to_string())?,
        ))
    }
}

/// Parse Mermaid source into a single-diagram Document.
pub fn parse_to_document(source: &str) -> Result<Document, IrError> {
    Ok(Document::single(parse(source)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::Kind;

    #[test]
    fn parse_flowchart_kind() {
        let d = parse("graph TD\n  A-->B\n").unwrap();
        assert_eq!(d.kind(), Kind::Flowchart);
    }

    #[test]
    fn parse_sequence_kind() {
        let d = parse("sequenceDiagram\n  A->>B: hi\n").unwrap();
        assert_eq!(d.kind(), Kind::Sequence);
    }
}
