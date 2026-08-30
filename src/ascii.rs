//! ASCII art render backend: IR → monospace box-and-arrow text.
//!
//! Deterministic, zero-token, no fonts or rasterizer — useful for embedding
//! diagrams in plain-text READMEs, terminal output, and diffs. Inspired by
//! graphine's "ASCII art export" brainstorm item, done as an IR walk instead
//! of a canvas rasterization. Flowcharts render as connected boxes; other
//! kinds render as a compact text outline (their structure is not planar, so
//! a box layout would be lossy and noisy).

use crate::diagram as flowchart;
use crate::ir::{Diagram, Document};
use std::collections::{HashMap, HashSet, VecDeque};

/// Render a Document to ASCII art text.
///
/// Flowchart diagrams use a layered box-and-arrow layout (BFS layering, the
/// same approach as `layout.rs`). Other kinds fall back to a compact text
/// outline so the output is still useful in a terminal/README without
/// misleading geometry.
pub fn render_document(doc: &Document) -> String {
    if doc.diagrams.is_empty() {
        return "(empty document)\n".into();
    }
    let mut out = String::new();
    let multi = doc.diagrams.len() > 1;
    for (i, d) in doc.diagrams.iter().enumerate() {
        if multi {
            if i > 0 {
                out.push('\n');
            }
            out.push_str(&format!("// diagram {i}: {}\n", d.kind()));
        }
        out.push_str(&render_diagram(d));
    }
    out
}

fn render_diagram(d: &Diagram) -> String {
    match d {
        Diagram::Flowchart(fc) => render_flowchart(fc),
        Diagram::Sequence(s) => render_sequence_outline(s),
        Diagram::Class(c) => render_class_outline(c),
        Diagram::Gantt(g) => render_gantt_outline(g),
        Diagram::State(s) => render_state_outline(s),
        Diagram::Er(e) => render_er_outline(e),
    }
}

/// BFS topological layers (longest-path from sources), matching `layout.rs`.
fn bfs_layers(fc: &flowchart::Diagram) -> Vec<Vec<String>> {
    let mut incoming: HashMap<&str, Vec<&str>> = HashMap::new();
    let mut outgoing: HashMap<&str, Vec<&str>> = HashMap::new();
    let mut all_ids: HashSet<&str> = HashSet::new();
    for n in &fc.nodes {
        all_ids.insert(n.id.as_str());
        incoming.entry(n.id.as_str()).or_default();
        outgoing.entry(n.id.as_str()).or_default();
    }
    for e in &fc.edges {
        incoming.entry(e.to.as_str()).or_default().push(e.from.as_str());
        outgoing.entry(e.from.as_str()).or_default().push(e.to.as_str());
    }

    let mut layer_of: HashMap<&str, usize> = HashMap::new();
    let mut queue: VecDeque<&str> = VecDeque::new();
    for &id in &all_ids {
        if incoming.get(id).is_none_or(|i| i.is_empty()) {
            queue.push_back(id);
            layer_of.insert(id, 0);
        }
    }
    if queue.is_empty()
        && let Some(&first) = all_ids.iter().next()
    {
        queue.push_back(first);
        layer_of.insert(first, 0);
    }
    let bound = all_ids.len();
    while let Some(node) = queue.pop_front() {
        let layer = layer_of[node];
        if let Some(children) = outgoing.get(node) {
            for &child in children {
                let new_layer = layer + 1;
                if new_layer >= bound {
                    continue;
                }
                if layer_of.get(child).is_none_or(|&l| l < new_layer) {
                    layer_of.insert(child, new_layer);
                    queue.push_back(child);
                }
            }
        }
    }

    // Any nodes unreachable from sources (cycles) land on layer 0.
    for &id in &all_ids {
        layer_of.entry(id).or_insert(0);
    }

    let max_layer = layer_of.values().copied().max().unwrap_or(0);
    let mut layers: Vec<Vec<String>> = vec![Vec::new(); max_layer + 1];
    for (&id, &layer) in &layer_of {
        layers[layer].push(id.to_string());
    }
    // Stable ordering: more incoming edges first (matches layout.rs heuristic).
    for layer in layers.iter_mut() {
        layer.sort_by(|a, b| {
            let a_in = incoming.get(a.as_str()).map(|v| v.len()).unwrap_or(0);
            let b_in = incoming.get(b.as_str()).map(|v| v.len()).unwrap_or(0);
            b_in.cmp(&a_in).then(a.cmp(b))
        });
    }
    layers
}

/// Box-and-arrow layout for flowcharts.
///
/// Each node is rendered as a bordered box sized to its label. Layers are
/// stacked vertically (top-down) regardless of `rankdir`, since ASCII columns
/// are narrow; the arrow direction is preserved in the connector labels.
fn render_flowchart(fc: &flowchart::Diagram) -> String {
    if fc.nodes.is_empty() {
        return "(empty flowchart)\n".into();
    }
    let layers = bfs_layers(fc);
    if layers.is_empty() {
        return "(no layers)\n".into();
    }

    let label_of = |id: &str| -> String {
        fc.nodes
            .iter()
            .find(|n| n.id == id)
            .map(|n| n.text.clone())
            .unwrap_or_else(|| id.to_string())
    };

    // Build per-layer rows with column widths sized to the widest box in the row.
    let mut boxes: Vec<Vec<(String, String)>> = Vec::with_capacity(layers.len());
    let mut col_widths: Vec<usize> = Vec::with_capacity(layers.len());
    for layer in &layers {
        let mut row = Vec::with_capacity(layer.len());
        let mut max_w = 0usize;
        for id in layer {
            let label = label_of(id);
            let w = (label.chars().count() + 2).max(6);
            max_w = max_w.max(w);
            row.push((id.clone(), label));
        }
        col_widths.push(max_w);
        boxes.push(row);
    }

    let box_height = 3;
    let mut rendered_layers: Vec<Vec<String>> = Vec::with_capacity(boxes.len());
    for (li, row) in boxes.iter().enumerate() {
        let w = col_widths[li];
        let mut lines = vec![String::new(); box_height];
        for (_id, label) in row {
            lines[0].push_str(&format!("+{}+  ", "-".repeat(w)));
            let pad = w.saturating_sub(label.chars().count());
            lines[1].push_str(&format!(
                "|{}{}{}|  ",
                " ".repeat(pad / 2),
                label,
                " ".repeat(pad - pad / 2)
            ));
            lines[2].push_str(&format!("+{}+  ", "-".repeat(w)));
        }
        for l in lines.iter_mut() {
            while l.ends_with(' ') {
                l.pop();
            }
        }
        rendered_layers.push(lines);
    }

    let edge_label = |from: &str, to: &str| -> String {
        fc.edges
            .iter()
            .find(|e| e.from == from && e.to == to)
            .map(|e| e.label.clone())
            .unwrap_or_default()
    };

    let mut out = String::new();
    for (li, lines) in rendered_layers.iter().enumerate() {
        for l in lines {
            out.push_str(l);
            out.push('\n');
        }
        if li + 1 < rendered_layers.len() {
            let from_layer = &boxes[li];
            let to_layer = &boxes[li + 1];
            let mut connectors: Vec<String> = Vec::with_capacity(from_layer.len());
            for (fid, _flabel) in from_layer {
                let targets: Vec<&String> = to_layer
                    .iter()
                    .filter(|(tid, _)| fc.edges.iter().any(|e| e.from == *fid && e.to == *tid))
                    .map(|(tid, _)| tid)
                    .collect();
                if targets.is_empty() {
                    connectors.push("   ".into());
                } else if targets.len() == 1 {
                    let lbl = edge_label(fid, targets[0]);
                    if lbl.is_empty() {
                        connectors.push(" | ".into());
                    } else {
                        connectors.push(format!(" |{}|", truncate(&lbl, 10)));
                    }
                } else {
                    connectors.push(format!(" |×{} ", targets.len()));
                }
            }
            let mut row = connectors.join("  ");
            while row.ends_with(' ') {
                row.pop();
            }
            out.push_str(&row);
            out.push('\n');
        }
    }
    out
}

fn truncate(s: &str, max: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max {
        s.to_string()
    } else {
        format!("{}…", chars[..max.saturating_sub(1)].iter().collect::<String>())
    }
}

fn render_sequence_outline(s: &crate::sequence::SequenceDiagram) -> String {
    use crate::sequence::MessageArrow;
    let mut out = String::from("sequenceDiagram\n");
    for p in &s.participants {
        let name = if p.label.is_empty() { &p.id } else { &p.label };
        out.push_str(&format!("  participant {}\n", name));
    }
    for m in &s.messages {
        let arrow = match m.arrow {
            MessageArrow::Solid => "->>",
            MessageArrow::Dashed => "-->>",
        };
        out.push_str(&format!("  {} {} {}: {}\n", m.from, arrow, m.to, m.text));
    }
    out
}

fn render_class_outline(c: &crate::class::ClassDiagram) -> String {
    let mut out = String::from("classDiagram\n");
    for cls in &c.classes {
        let stereo = cls
            .stereotype
            .as_deref()
            .map(|s| format!(" <<{s}>>"))
            .unwrap_or_default();
        out.push_str(&format!("  class {}{}\n", cls.id, stereo));
        for m in &cls.members {
            out.push_str(&format!("    {}\n", m.text));
        }
    }
    for r in &c.relations {
        out.push_str(&format!("  {} {} {}\n", r.from, r.kind.mermaid_str(), r.to));
    }
    out
}

fn render_gantt_outline(g: &crate::gantt::GanttDiagram) -> String {
    let mut out = String::new();
    if !g.title.is_empty() {
        out.push_str(&format!("gantt: {}\n", g.title));
    }
    for t in &g.tasks {
        out.push_str(&format!("  {} [{}]\n", t.name, t.section));
    }
    out
}

fn render_state_outline(s: &crate::state::StateDiagram) -> String {
    let mut out = String::from("stateDiagram-v2\n");
    for t in &s.transitions {
        if t.label.is_empty() {
            out.push_str(&format!("  {} --> {}\n", t.from, t.to));
        } else {
            out.push_str(&format!("  {} --> {} : {}\n", t.from, t.to, t.label));
        }
    }
    out
}

fn render_er_outline(e: &crate::er::ErDiagram) -> String {
    let mut out = String::from("erDiagram\n");
    for ent in &e.entities {
        out.push_str(&format!("  {} {{\n", ent.id));
        for a in &ent.attributes {
            out.push_str(&format!("    {} {}\n", a.type_name, a.name));
        }
        out.push_str("  }\n");
    }
    for r in &e.relationships {
        out.push_str(&format!(
            "  {} {}--{} {} : {}\n",
            r.from,
            r.from_card.symbol(),
            r.to_card.symbol(),
            r.to,
            r.label
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flowchart_ascii_has_boxes_and_arrows() {
        let doc = crate::ir::from_mermaid("graph TD\n    A[Start] --> B[End]\n").unwrap();
        let out = render_document(&doc);
        assert!(out.contains('+'));
        assert!(out.contains('|'));
        assert!(out.contains("Start"));
        assert!(out.contains("End"));
    }

    #[test]
    fn flowchart_ascii_multilayer() {
        let doc = crate::ir::from_mermaid(
            "graph TD\n    A[Alpha] --> B[Beta]\n    B --> C[Gamma]\n",
        )
        .unwrap();
        let out = render_document(&doc);
        // Three layers stacked vertically.
        let plus_count = out.matches('+').count();
        assert!(plus_count >= 6, "expected >=6 box corners, got {plus_count}");
        assert!(out.contains("Alpha"));
        assert!(out.contains("Gamma"));
    }

    #[test]
    fn sequence_outline() {
        let doc = crate::ir::from_mermaid("sequenceDiagram\n    A->>B: hi\n").unwrap();
        let out = render_document(&doc);
        assert!(out.contains("sequenceDiagram"));
        assert!(out.contains("A ->> B: hi"));
    }

    #[test]
    fn empty_document() {
        let doc = Document { version: 1, diagrams: vec![] };
        let out = render_document(&doc);
        assert!(out.contains("empty"));
    }

    #[test]
    fn multi_diagram_markers() {
        let doc = Document {
            version: 1,
            diagrams: vec![
                crate::ir::from_mermaid("graph TD\n    A-->B\n").unwrap().primary().unwrap().clone(),
                crate::ir::from_mermaid("sequenceDiagram\n    A->>B: x\n").unwrap().primary().unwrap().clone(),
            ],
        };
        let out = render_document(&doc);
        assert!(out.contains("// diagram 0: flowchart"));
        assert!(out.contains("// diagram 1: sequence"));
    }
}
