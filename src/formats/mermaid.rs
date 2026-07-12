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
    } else if crate::state::is_state(source) {
        Ok(Diagram::State(
            crate::state::parse(source).map_err(|e| e.to_string())?,
        ))
    } else if crate::er::is_er(source) {
        Ok(Diagram::Er(
            crate::er::parse(source).map_err(|e| e.to_string())?,
        ))
    } else {
        Ok(Diagram::Flowchart(
            crate::parser::parse(source).map_err(|e| e.to_string())?,
        ))
    }
}

/// Parse Mermaid source into a Document (supports `%% diagram N:` multi-chunk files).
pub fn parse_to_document(source: &str) -> Result<Document, IrError> {
    let chunks = split_mermaid_chunks(source);
    if chunks.len() == 1 {
        return Ok(Document::single(parse(&chunks[0])?));
    }
    let diagrams: Result<Vec<Diagram>, IrError> = chunks.iter().map(|c| parse(c)).collect();
    Ok(Document {
        version: Document::CURRENT_VERSION,
        diagrams: diagrams?,
    })
}

/// Split a Mermaid file on `%% diagram N:` markers.
pub fn split_mermaid_chunks(source: &str) -> Vec<String> {
    if !source.contains("%% diagram ") {
        return vec![source.to_string()];
    }
    let mut chunks: Vec<String> = Vec::new();
    let mut current = String::new();
    for line in source.lines() {
        if line.trim().starts_with("%% diagram ") {
            if !current.trim().is_empty() {
                chunks.push(current.trim().to_string());
                current.clear();
            }
            continue;
        }
        current.push_str(line);
        current.push('\n');
    }
    if !current.trim().is_empty() {
        chunks.push(current.trim().to_string());
    }
    if chunks.is_empty() {
        vec![source.to_string()]
    } else {
        chunks
    }
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
    fn parse_multi_chunk_document() {
        let src = "%% diagram 0: flowchart\ngraph TD\n  A-->B\n\n%% diagram 1: sequence\nsequenceDiagram\n  A->>B: hi\n";
        let doc = parse_to_document(src).unwrap();
        assert_eq!(doc.diagrams.len(), 2);
        assert_eq!(doc.diagrams[0].kind(), Kind::Flowchart);
        assert_eq!(doc.diagrams[1].kind(), Kind::Sequence);
    }

    #[test]
    fn parse_state_kind() {
        let d = parse("stateDiagram-v2\n  [*] --> A\n").unwrap();
        assert_eq!(d.kind(), Kind::State);
    }

    #[test]
    fn parse_er_kind() {
        let d = parse("erDiagram\n  A ||--o{ B : has\n").unwrap();
        assert_eq!(d.kind(), Kind::Er);
    }
}
