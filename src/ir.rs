//! Canonical diagram IR: Document / Diagram / Kind.

use crate::class::{self, ClassDiagram};
use crate::diagram as flowchart;
use crate::gantt::{self, GanttDiagram};
use crate::layout;
use crate::renderer::{self, Theme};
use crate::sequence::{self, SequenceDiagram};
use crate::state::{self, StateDiagram};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Diagram kind (semantics), independent of Format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Flowchart,
    Sequence,
    Class,
    Gantt,
    State,
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Flowchart => write!(f, "flowchart"),
            Self::Sequence => write!(f, "sequence"),
            Self::Class => write!(f, "class"),
            Self::Gantt => write!(f, "gantt"),
            Self::State => write!(f, "state"),
        }
    }
}

/// A single typed diagram in the canonical IR.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data", rename_all = "lowercase")]
pub enum Diagram {
    Flowchart(flowchart::Diagram),
    Sequence(SequenceDiagram),
    Class(ClassDiagram),
    Gantt(GanttDiagram),
    State(StateDiagram),
}

impl Diagram {
    pub fn kind(&self) -> Kind {
        match self {
            Self::Flowchart(_) => Kind::Flowchart,
            Self::Sequence(_) => Kind::Sequence,
            Self::Class(_) => Kind::Class,
            Self::Gantt(_) => Kind::Gantt,
            Self::State(_) => Kind::State,
        }
    }

    pub fn to_mermaid(&self) -> String {
        match self {
            Self::Flowchart(d) => d.to_mermaid(),
            Self::Sequence(d) => d.to_mermaid(),
            Self::Class(d) => d.to_mermaid(),
            Self::Gantt(d) => d.to_mermaid(),
            Self::State(d) => d.to_mermaid(),
        }
    }

    pub fn render_svg(&self, theme: Theme) -> String {
        match self {
            Self::Flowchart(d) => {
                let laid = layout::layout(d);
                renderer::render_svg_with_theme(&laid, theme)
            }
            Self::Sequence(d) => sequence::render_svg(d, theme),
            Self::Class(d) => class::render_svg(d, theme),
            Self::Gantt(d) => gantt::render_svg(d, theme),
            Self::State(d) => state::render_svg(d, theme),
        }
    }
}

/// Top-level IR unit: one or more diagrams plus schema version.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub version: u32,
    pub diagrams: Vec<Diagram>,
}

impl Document {
    pub const CURRENT_VERSION: u32 = 1;

    pub fn single(diagram: Diagram) -> Self {
        Self {
            version: Self::CURRENT_VERSION,
            diagrams: vec![diagram],
        }
    }

    pub fn primary(&self) -> Option<&Diagram> {
        self.diagrams.first()
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn from_json(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }

    pub fn to_mermaid(&self) -> Result<String, String> {
        if self.diagrams.is_empty() {
            return Err("document has no diagrams".into());
        }
        if self.diagrams.len() == 1 {
            return Ok(self.diagrams[0].to_mermaid());
        }
        Ok(self
            .diagrams
            .iter()
            .enumerate()
            .map(|(i, d)| format!("%% diagram {i}: {}\n{}", d.kind(), d.to_mermaid()))
            .collect::<Vec<_>>()
            .join("\n\n"))
    }

    pub fn render_svg(&self, theme: Theme) -> Result<String, String> {
        if self.diagrams.is_empty() {
            return Err("document has no diagrams".into());
        }
        if self.diagrams.len() == 1 {
            return Ok(self.diagrams[0].render_svg(theme));
        }
        let svgs: Vec<String> = self
            .diagrams
            .iter()
            .map(|d| d.render_svg(theme))
            .collect();
        crate::composite::combine_svgs(&svgs)
    }

    pub fn render_diagram_at(&self, index: usize, theme: Theme) -> Result<String, String> {
        self.diagrams
            .get(index)
            .map(|d| d.render_svg(theme))
            .ok_or_else(|| format!("diagram index {index} out of range (0..{})", self.diagrams.len()))
    }
}

/// Human-readable summary lines for CLI `info`.
pub fn info_lines(path: &str, doc: &Document) -> Vec<String> {
    let mut lines = vec![
        format!("File: {path}"),
        format!("IR version: {}", doc.version),
        format!("Diagrams: {}", doc.diagrams.len()),
    ];
    if doc.diagrams.is_empty() {
        return lines;
    }
    if doc.diagrams.len() > 1 {
        for (i, d) in doc.diagrams.iter().enumerate() {
            lines.push(format!("Diagram {i}: {}", d.kind()));
            lines.extend(diagram_summary_lines(d));
        }
        return lines;
    }
    let d = &doc.diagrams[0];
    lines.push(format!("Kind: {}", d.kind()));
    lines.extend(diagram_summary_lines(d));
    lines
}

fn diagram_summary_lines(d: &Diagram) -> Vec<String> {
    match d {
        Diagram::Flowchart(fc) => {
            let mut shapes = [0usize; 6];
            for n in &fc.nodes {
                shapes[match n.shape {
                    flowchart::NodeShape::Rect => 0,
                    flowchart::NodeShape::Diamond => 1,
                    flowchart::NodeShape::Stadium => 2,
                    flowchart::NodeShape::Hexagon => 3,
                    flowchart::NodeShape::Cylinder => 4,
                    flowchart::NodeShape::Circle => 5,
                }] += 1;
            }
            vec![
                format!("Direction: {}", fc.rankdir),
                format!("Nodes: {}", fc.nodes.len()),
                format!("  rect:     {}", shapes[0]),
                format!("  diamond:  {}", shapes[1]),
                format!("  stadium:  {}", shapes[2]),
                format!("  hexagon:  {}", shapes[3]),
                format!("  cylinder: {}", shapes[4]),
                format!("  circle:   {}", shapes[5]),
                format!("Edges: {}", fc.edges.len()),
            ]
        }
        Diagram::Sequence(s) => vec![
            format!("Participants: {}", s.participants.len()),
            format!("Messages: {}", s.messages.len()),
        ],
        Diagram::Class(c) => vec![
            format!("Classes: {}", c.classes.len()),
            format!("Relations: {}", c.relations.len()),
        ],
        Diagram::Gantt(g) => {
            let mut out = vec![format!("Tasks: {}", g.tasks.len())];
            if !g.title.is_empty() {
                out.insert(0, format!("Title: {}", g.title));
            }
            out
        }
        Diagram::State(s) => vec![
            format!("States: {}", s.states.len()),
            format!("Transitions: {}", s.transitions.len()),
        ],
    }
}

/// JSON summary for MCP `get_info`.
pub fn info_json(path: &str, doc: &Document) -> serde_json::Value {
    if doc.diagrams.is_empty() {
        return serde_json::json!({
            "path": path,
            "ir_version": doc.version,
            "diagrams": 0,
        });
    }
    if doc.diagrams.len() > 1 {
        return serde_json::json!({
            "path": path,
            "ir_version": doc.version,
            "diagrams": doc.diagrams.len(),
            "kind": "multi",
            "entries": doc.diagrams.iter().enumerate().map(|(i, d)| {
                let mut entry = diagram_info_json(d);
                if let serde_json::Value::Object(ref mut map) = entry {
                    map.insert("index".into(), serde_json::json!(i));
                }
                entry
            }).collect::<Vec<_>>(),
        });
    }
    let mut value = diagram_info_json(&doc.diagrams[0]);
    if let serde_json::Value::Object(ref mut map) = value {
        map.insert("path".into(), serde_json::json!(path));
        map.insert("ir_version".into(), serde_json::json!(doc.version));
        map.insert("diagrams".into(), serde_json::json!(1));
    }
    value
}

fn diagram_info_json(d: &Diagram) -> serde_json::Value {
    match d {
        Diagram::Flowchart(fc) => {
            let mut shapes = [0usize; 6];
            for n in &fc.nodes {
                shapes[match n.shape {
                    flowchart::NodeShape::Rect => 0,
                    flowchart::NodeShape::Diamond => 1,
                    flowchart::NodeShape::Stadium => 2,
                    flowchart::NodeShape::Hexagon => 3,
                    flowchart::NodeShape::Cylinder => 4,
                    flowchart::NodeShape::Circle => 5,
                }] += 1;
            }
            serde_json::json!({
                "kind": "flowchart",
                "direction": fc.rankdir,
                "nodes": fc.nodes.len(),
                "edges": fc.edges.len(),
                "shapes": {
                    "rect": shapes[0],
                    "diamond": shapes[1],
                    "stadium": shapes[2],
                    "hexagon": shapes[3],
                    "cylinder": shapes[4],
                    "circle": shapes[5],
                },
            })
        }
        Diagram::Sequence(s) => serde_json::json!({
            "kind": "sequence",
            "participants": s.participants.len(),
            "messages": s.messages.len(),
        }),
        Diagram::Class(c) => serde_json::json!({
            "kind": "class",
            "classes": c.classes.len(),
            "relations": c.relations.len(),
        }),
        Diagram::Gantt(g) => serde_json::json!({
            "kind": "gantt",
            "title": g.title,
            "tasks": g.tasks.len(),
        }),
        Diagram::State(s) => serde_json::json!({
            "kind": "state",
            "states": s.states.len(),
            "transitions": s.transitions.len(),
        }),
    }
}

#[derive(Debug, Clone)]
pub struct IrError {
    pub message: String,
}

impl fmt::Display for IrError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for IrError {}

impl From<String> for IrError {
    fn from(message: String) -> Self {
        Self { message }
    }
}

impl From<&str> for IrError {
    fn from(message: &str) -> Self {
        Self {
            message: message.into(),
        }
    }
}

/// Parse Mermaid Compatibility source into a single-diagram Document.
pub fn from_mermaid(source: &str) -> Result<Document, IrError> {
    crate::formats::mermaid::parse_to_document(source)
}

/// Load a Document from a file path (Mermaid or JSON IR, by detection).
pub fn load_path(path: &str) -> Result<Document, IrError> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read '{path}': {e}"))?;
    crate::formats::import_str(&content, crate::formats::detect(&content, Some(path)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flowchart_roundtrip_json() {
        let src = "graph TD\n  A-->B\n";
        let doc = from_mermaid(src).unwrap();
        assert_eq!(doc.primary().unwrap().kind(), Kind::Flowchart);
        let json = doc.to_json().unwrap();
        let doc2 = Document::from_json(&json).unwrap();
        assert_eq!(doc2.version, 1);
        assert_eq!(doc2.primary().unwrap().kind(), Kind::Flowchart);
        assert!(doc2.to_mermaid().unwrap().contains("A"));
    }

    #[test]
    fn multi_document_render_and_mermaid() {
        let doc = Document {
            version: 1,
            diagrams: vec![
                Diagram::Flowchart(
                    crate::parser::parse("graph TD\n  A-->B\n").unwrap(),
                ),
                Diagram::Sequence(
                    crate::sequence::parse("sequenceDiagram\n  A->>B: hi\n").unwrap(),
                ),
            ],
        };
        let mmd = doc.to_mermaid().unwrap();
        assert!(mmd.contains("%% diagram 0:"));
        assert!(mmd.contains("%% diagram 1:"));
        let svg = doc.render_svg(Theme::Dark).unwrap();
        assert!(svg.contains("<svg"));
        let roundtrip = crate::formats::mermaid::parse_to_document(&mmd).unwrap();
        assert_eq!(roundtrip.diagrams.len(), 2);
    }
}
