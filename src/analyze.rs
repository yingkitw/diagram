//! Structural analysis and metrics on canonical IR.

use crate::diagram as flowchart;
use crate::ir::{Diagram, Document};
use serde::Serialize;
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Debug, Clone, Serialize)]
pub struct DocumentMetrics {
    pub ir_version: u32,
    pub diagrams: usize,
    pub kind: String,
    #[serde(flatten)]
    pub detail: MetricsDetail,
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum MetricsDetail {
    Flowchart(FlowchartMetrics),
    Sequence(SequenceMetrics),
    Class(ClassMetrics),
    Gantt(GanttMetrics),
    Empty {},
}

#[derive(Debug, Clone, Serialize)]
pub struct FlowchartMetrics {
    pub nodes: usize,
    pub edges: usize,
    pub direction: String,
    pub sources: usize,
    pub sinks: usize,
    pub orphans: usize,
    pub orphan_rate: f64,
    pub max_depth: usize,
    pub cycles: Vec<String>,
    pub validation_issues: Vec<String>,
    pub shapes: ShapeCounts,
}

#[derive(Debug, Clone, Serialize)]
pub struct ShapeCounts {
    pub rect: usize,
    pub diamond: usize,
    pub stadium: usize,
    pub hexagon: usize,
    pub cylinder: usize,
    pub circle: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct SequenceMetrics {
    pub participants: usize,
    pub messages: usize,
    pub solid_messages: usize,
    pub dashed_messages: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClassMetrics {
    pub classes: usize,
    pub relations: usize,
    pub members: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct GanttMetrics {
    pub title: String,
    pub tasks: usize,
    pub sections: usize,
    pub span_days: i64,
    pub critical_tasks: usize,
    pub done_tasks: usize,
    pub active_tasks: usize,
}

pub fn metrics(doc: &Document) -> DocumentMetrics {
    let Some(d) = doc.primary() else {
        return DocumentMetrics {
            ir_version: doc.version,
            diagrams: doc.diagrams.len(),
            kind: "none".into(),
            detail: MetricsDetail::Empty {},
        };
    };
    DocumentMetrics {
        ir_version: doc.version,
        diagrams: doc.diagrams.len(),
        kind: d.kind().to_string(),
        detail: match d {
            Diagram::Flowchart(fc) => MetricsDetail::Flowchart(flowchart_metrics(fc)),
            Diagram::Sequence(s) => MetricsDetail::Sequence(sequence_metrics(s)),
            Diagram::Class(c) => MetricsDetail::Class(class_metrics(c)),
            Diagram::Gantt(g) => MetricsDetail::Gantt(gantt_metrics(g)),
        },
    }
}

fn flowchart_metrics(d: &flowchart::Diagram) -> FlowchartMetrics {
    let issues = d.validate();
    let cycles: Vec<String> = issues
        .iter()
        .filter(|i| i.starts_with("cycle detected:"))
        .cloned()
        .collect();

    let node_ids: HashSet<&str> = d.nodes.iter().map(|n| n.id.as_str()).collect();
    let mut has_edge: HashSet<&str> = HashSet::new();
    let mut incoming: HashMap<&str, usize> = HashMap::new();
    let mut outgoing: HashMap<&str, usize> = HashMap::new();

    for n in &d.nodes {
        incoming.insert(n.id.as_str(), 0);
        outgoing.insert(n.id.as_str(), 0);
    }
    for e in &d.edges {
        has_edge.insert(e.from.as_str());
        has_edge.insert(e.to.as_str());
        *incoming.entry(e.to.as_str()).or_default() += 1;
        *outgoing.entry(e.from.as_str()).or_default() += 1;
    }

    let orphans = d
        .nodes
        .iter()
        .filter(|n| !has_edge.contains(n.id.as_str()))
        .count();
    let orphan_rate = if d.nodes.is_empty() {
        0.0
    } else {
        orphans as f64 / d.nodes.len() as f64
    };

    let sources = node_ids
        .iter()
        .filter(|id| incoming.get(**id).copied().unwrap_or(0) == 0)
        .count();
    let sinks = node_ids
        .iter()
        .filter(|id| outgoing.get(**id).copied().unwrap_or(0) == 0)
        .count();

    let max_depth = flowchart_max_depth(d);

    let mut shapes = [0usize; 6];
    for n in &d.nodes {
        shapes[match n.shape {
            flowchart::NodeShape::Rect => 0,
            flowchart::NodeShape::Diamond => 1,
            flowchart::NodeShape::Stadium => 2,
            flowchart::NodeShape::Hexagon => 3,
            flowchart::NodeShape::Cylinder => 4,
            flowchart::NodeShape::Circle => 5,
        }] += 1;
    }

    FlowchartMetrics {
        nodes: d.nodes.len(),
        edges: d.edges.len(),
        direction: d.rankdir.clone(),
        sources,
        sinks,
        orphans,
        orphan_rate,
        max_depth,
        cycles,
        validation_issues: issues,
        shapes: ShapeCounts {
            rect: shapes[0],
            diamond: shapes[1],
            stadium: shapes[2],
            hexagon: shapes[3],
            cylinder: shapes[4],
            circle: shapes[5],
        },
    }
}

fn flowchart_max_depth(d: &flowchart::Diagram) -> usize {
    let mut incoming: HashMap<&str, Vec<&str>> = HashMap::new();
    let mut outgoing: HashMap<&str, Vec<&str>> = HashMap::new();
    let mut all_ids: HashSet<&str> = HashSet::new();

    for n in &d.nodes {
        all_ids.insert(n.id.as_str());
        incoming.entry(n.id.as_str()).or_default();
        outgoing.entry(n.id.as_str()).or_default();
    }
    for e in &d.edges {
        incoming.entry(e.to.as_str()).or_default().push(e.from.as_str());
        outgoing.entry(e.from.as_str()).or_default().push(e.to.as_str());
    }

    let mut layers: HashMap<&str, usize> = HashMap::new();
    let mut queue: VecDeque<&str> = VecDeque::new();

    for &id in &all_ids {
        if incoming.get(id).is_none_or(|i| i.is_empty()) {
            queue.push_back(id);
            layers.insert(id, 0);
        }
    }
    if queue.is_empty() && !all_ids.is_empty() {
        let first = *all_ids.iter().next().unwrap();
        queue.push_back(first);
        layers.insert(first, 0);
    }

    while let Some(id) = queue.pop_front() {
        let layer = *layers.get(id).unwrap_or(&0);
        if let Some(children) = outgoing.get(id) {
            for child in children {
                let next = layer + 1;
                if layers.get(child).copied().unwrap_or(0) < next {
                    layers.insert(child, next);
                    queue.push_back(child);
                }
            }
        }
    }

    layers.values().copied().max().unwrap_or(0)
}

fn sequence_metrics(s: &crate::sequence::SequenceDiagram) -> SequenceMetrics {
    let solid = s
        .messages
        .iter()
        .filter(|m| m.arrow == crate::sequence::MessageArrow::Solid)
        .count();
    SequenceMetrics {
        participants: s.participants.len(),
        messages: s.messages.len(),
        solid_messages: solid,
        dashed_messages: s.messages.len() - solid,
    }
}

fn class_metrics(c: &crate::class::ClassDiagram) -> ClassMetrics {
    ClassMetrics {
        classes: c.classes.len(),
        relations: c.relations.len(),
        members: c.classes.iter().map(|cl| cl.members.len()).sum(),
    }
}

fn gantt_metrics(g: &crate::gantt::GanttDiagram) -> GanttMetrics {
    let sections: HashSet<&str> = g.tasks.iter().map(|t| t.section.as_str()).collect();
    let min = g.tasks.iter().map(|t| t.start).min();
    let max = g.tasks.iter().map(|t| t.end).max();
    let span_days = match (min, max) {
        (Some(a), Some(b)) => (b - a).max(0),
        _ => 0,
    };
    GanttMetrics {
        title: g.title.clone(),
        tasks: g.tasks.len(),
        sections: sections.len(),
        span_days,
        critical_tasks: g.tasks.iter().filter(|t| t.crit).count(),
        done_tasks: g.tasks.iter().filter(|t| t.done).count(),
        active_tasks: g.tasks.iter().filter(|t| t.active).count(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir;

    #[test]
    fn flowchart_metrics_orphans_and_cycles() {
        let doc = ir::from_mermaid("graph TD\n  A-->B\n  B-->A\n  C[alone]\n").unwrap();
        let m = metrics(&doc);
        assert_eq!(m.kind, "flowchart");
        let MetricsDetail::Flowchart(fc) = m.detail else {
            panic!("expected flowchart metrics");
        };
        assert_eq!(fc.nodes, 3);
        assert_eq!(fc.edges, 2);
        assert_eq!(fc.orphans, 1);
        assert!((fc.orphan_rate - 1.0 / 3.0).abs() < 0.001);
        assert!(!fc.cycles.is_empty());
    }

    #[test]
    fn flowchart_max_depth_on_dag() {
        let doc = ir::from_mermaid("graph TD\n  A-->B\n  B-->C\n").unwrap();
        let MetricsDetail::Flowchart(fc) = metrics(&doc).detail else {
            panic!("expected flowchart metrics");
        };
        assert_eq!(fc.max_depth, 2);
    }

    #[test]
    fn sequence_metrics_counts() {
        let doc = ir::from_mermaid(
            "sequenceDiagram\n  A->>B: hi\n  B-->>A: bye\n",
        )
        .unwrap();
        let MetricsDetail::Sequence(s) = metrics(&doc).detail else {
            panic!("expected sequence");
        };
        assert_eq!(s.participants, 2);
        assert_eq!(s.messages, 2);
        assert_eq!(s.solid_messages, 1);
        assert_eq!(s.dashed_messages, 1);
    }

    #[test]
    fn metrics_json_serializes() {
        let doc = ir::from_mermaid("graph TD\n  A-->B\n").unwrap();
        let json = serde_json::to_string(&metrics(&doc)).unwrap();
        assert!(json.contains("\"kind\":\"flowchart\""));
        assert!(json.contains("\"nodes\":2"));
    }
}
