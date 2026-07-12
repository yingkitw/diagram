//! Structural analysis and metrics on canonical IR.

use crate::diagram as flowchart;
use crate::ir::{Diagram, Document};
use serde::Serialize;
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Debug, Clone, Serialize)]
pub struct DiagramMetricsEntry {
    pub index: usize,
    pub kind: String,
    #[serde(flatten)]
    pub detail: MetricsDetail,
}

#[derive(Debug, Clone, Serialize)]
pub struct DocumentMetrics {
    pub ir_version: u32,
    pub diagrams: usize,
    pub kind: String,
    #[serde(flatten)]
    pub detail: MetricsDetail,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entries: Option<Vec<DiagramMetricsEntry>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum MetricsDetail {
    Flowchart(FlowchartMetrics),
    Sequence(SequenceMetrics),
    Class(ClassMetrics),
    Gantt(GanttMetrics),
    State(StateMetrics),
    Er(ErMetrics),
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
    pub notes: usize,
    pub self_messages: usize,
    pub fragments: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClassMetrics {
    pub classes: usize,
    pub relations: usize,
    pub members: usize,
    pub notes: usize,
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

#[derive(Debug, Clone, Serialize)]
pub struct StateMetrics {
    pub states: usize,
    pub transitions: usize,
    pub start_end_nodes: usize,
    pub choice_nodes: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ErMetrics {
    pub entities: usize,
    pub relationships: usize,
    pub attributes: usize,
}

pub fn metrics(doc: &Document) -> DocumentMetrics {
    if doc.diagrams.is_empty() {
        return DocumentMetrics {
            ir_version: doc.version,
            diagrams: 0,
            kind: "none".into(),
            detail: MetricsDetail::Empty {},
            entries: None,
        };
    }
    if doc.diagrams.len() > 1 {
        let entries = doc
            .diagrams
            .iter()
            .enumerate()
            .map(|(i, d)| DiagramMetricsEntry {
                index: i,
                kind: d.kind().to_string(),
                detail: metrics_for_diagram(d),
            })
            .collect();
        return DocumentMetrics {
            ir_version: doc.version,
            diagrams: doc.diagrams.len(),
            kind: "multi".into(),
            detail: MetricsDetail::Empty {},
            entries: Some(entries),
        };
    }
    let d = &doc.diagrams[0];
    DocumentMetrics {
        ir_version: doc.version,
        diagrams: 1,
        kind: d.kind().to_string(),
        detail: metrics_for_diagram(d),
        entries: None,
    }
}

fn metrics_for_diagram(d: &Diagram) -> MetricsDetail {
    match d {
        Diagram::Flowchart(fc) => MetricsDetail::Flowchart(flowchart_metrics(fc)),
        Diagram::Sequence(s) => MetricsDetail::Sequence(sequence_metrics(s)),
        Diagram::Class(c) => MetricsDetail::Class(class_metrics(c)),
        Diagram::Gantt(g) => MetricsDetail::Gantt(gantt_metrics(g)),
        Diagram::State(s) => MetricsDetail::State(state_metrics(s)),
        Diagram::Er(e) => MetricsDetail::Er(er_metrics(e)),
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
    let self_messages = s.messages.iter().filter(|m| m.from == m.to).count();
    SequenceMetrics {
        participants: s.participants.len(),
        messages: s.messages.len(),
        solid_messages: solid,
        dashed_messages: s.messages.len() - solid,
        notes: s.notes.len(),
        self_messages,
        fragments: s.fragments.len(),
    }
}

fn class_metrics(c: &crate::class::ClassDiagram) -> ClassMetrics {
    ClassMetrics {
        classes: c.classes.len(),
        relations: c.relations.len(),
        members: c.classes.iter().map(|cl| cl.members.len()).sum(),
        notes: c.notes.len(),
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

fn state_metrics(s: &crate::state::StateDiagram) -> StateMetrics {
    StateMetrics {
        states: s.states.len(),
        transitions: s.transitions.len(),
        start_end_nodes: s
            .states
            .iter()
            .filter(|n| n.kind == crate::state::StateNodeKind::StartEnd)
            .count(),
        choice_nodes: s
            .states
            .iter()
            .filter(|n| n.kind == crate::state::StateNodeKind::Choice)
            .count(),
    }
}

fn er_metrics(e: &crate::er::ErDiagram) -> ErMetrics {
    ErMetrics {
        entities: e.entities.len(),
        relationships: e.relationships.len(),
        attributes: e.entities.iter().map(|ent| ent.attributes.len()).sum(),
    }
}

// --- Semantic diff (IR-level) ---

#[derive(Debug, Clone, Serialize)]
pub struct DocumentDiff {
    pub left_diagrams: usize,
    pub right_diagrams: usize,
    pub diagram_count_changed: bool,
    pub unchanged: bool,
    pub summary: Vec<String>,
    pub entries: Vec<DiagramDiffEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiagramDiffEntry {
    pub index: usize,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub left_kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub right_kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<DiffDetail>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum DiffDetail {
    Flowchart(flowchart::DiagramDiff),
    Sequence(SequenceDiff),
    Class(ClassDiff),
    Gantt(GanttDiff),
    State(StateDiff),
    Er(ErDiff),
}

#[derive(Debug, Clone, Serialize)]
pub struct SequenceDiff {
    pub added_participants: Vec<String>,
    pub removed_participants: Vec<String>,
    pub added_messages: Vec<crate::sequence::Message>,
    pub removed_messages: Vec<crate::sequence::Message>,
    pub modified_messages: Vec<(crate::sequence::Message, crate::sequence::Message)>,
    pub added_notes: usize,
    pub removed_notes: usize,
    pub added_fragments: usize,
    pub removed_fragments: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClassDiff {
    pub added_classes: Vec<String>,
    pub removed_classes: Vec<String>,
    pub added_relations: Vec<crate::class::Relation>,
    pub removed_relations: Vec<crate::class::Relation>,
    pub added_notes: usize,
    pub removed_notes: usize,
    pub changed_stereotypes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GanttDiff {
    pub title_changed: bool,
    pub added_tasks: Vec<String>,
    pub removed_tasks: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StateDiff {
    pub added_states: Vec<String>,
    pub removed_states: Vec<String>,
    pub added_transitions: Vec<crate::state::Transition>,
    pub removed_transitions: Vec<crate::state::Transition>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ErDiff {
    pub added_entities: Vec<String>,
    pub removed_entities: Vec<String>,
    pub added_relationships: Vec<crate::er::Relationship>,
    pub removed_relationships: Vec<crate::er::Relationship>,
}

/// Structural diff of two Documents (any supported kind, by diagram index).
pub fn diff_documents(left: &Document, right: &Document) -> DocumentDiff {
    let left_diagrams = left.diagrams.len();
    let right_diagrams = right.diagrams.len();
    let diagram_count_changed = left_diagrams != right_diagrams;
    let max = left_diagrams.max(right_diagrams);
    let mut entries = Vec::new();
    let mut summary = Vec::new();

    if diagram_count_changed {
        summary.push(format!(
            "diagram count changed: {left_diagrams} → {right_diagrams}"
        ));
    }

    for i in 0..max {
        let entry = match (left.diagrams.get(i), right.diagrams.get(i)) {
            (None, Some(r)) => {
                summary.push(format!("diagram {i}: added ({})", r.kind()));
                DiagramDiffEntry {
                    index: i,
                    status: "added".into(),
                    left_kind: None,
                    right_kind: Some(r.kind().to_string()),
                    detail: None,
                }
            }
            (Some(l), None) => {
                summary.push(format!("diagram {i}: removed ({})", l.kind()));
                DiagramDiffEntry {
                    index: i,
                    status: "removed".into(),
                    left_kind: Some(l.kind().to_string()),
                    right_kind: None,
                    detail: None,
                }
            }
            (Some(l), Some(r)) => diff_diagram_pair(i, l, r, &mut summary),
            (None, None) => unreachable!(),
        };
        entries.push(entry);
    }

    let unchanged = !diagram_count_changed && entries.iter().all(|e| e.status == "unchanged");

    DocumentDiff {
        left_diagrams,
        right_diagrams,
        diagram_count_changed,
        unchanged,
        summary,
        entries,
    }
}

fn diff_diagram_pair(
    index: usize,
    left: &Diagram,
    right: &Diagram,
    summary: &mut Vec<String>,
) -> DiagramDiffEntry {
    if left.kind() != right.kind() {
        summary.push(format!(
            "diagram {index}: kind changed {} → {}",
            left.kind(),
            right.kind()
        ));
        return DiagramDiffEntry {
            index,
            status: "kind_changed".into(),
            left_kind: Some(left.kind().to_string()),
            right_kind: Some(right.kind().to_string()),
            detail: None,
        };
    }

    let (detail, unchanged) = match (left, right) {
        (Diagram::Flowchart(l), Diagram::Flowchart(r)) => {
            let d = l.diff(r);
            let unchanged = flowchart_unchanged(&d);
            (Some(DiffDetail::Flowchart(d)), unchanged)
        }
        (Diagram::Sequence(l), Diagram::Sequence(r)) => {
            let d = diff_sequence(l, r);
            let unchanged = sequence_unchanged(&d);
            (Some(DiffDetail::Sequence(d)), unchanged)
        }
        (Diagram::Class(l), Diagram::Class(r)) => {
            let d = diff_class(l, r);
            let unchanged = class_unchanged(&d);
            (Some(DiffDetail::Class(d)), unchanged)
        }
        (Diagram::Gantt(l), Diagram::Gantt(r)) => {
            let d = diff_gantt(l, r);
            let unchanged = gantt_unchanged(&d);
            (Some(DiffDetail::Gantt(d)), unchanged)
        }
        (Diagram::State(l), Diagram::State(r)) => {
            let d = diff_state(l, r);
            let unchanged = state_unchanged(&d);
            (Some(DiffDetail::State(d)), unchanged)
        }
        (Diagram::Er(l), Diagram::Er(r)) => {
            let d = diff_er(l, r);
            let unchanged = er_unchanged(&d);
            (Some(DiffDetail::Er(d)), unchanged)
        }
        _ => unreachable!("kinds matched above"),
    };

    if !unchanged {
        summary.push(format!("diagram {index}: changed ({})", left.kind()));
    }

    DiagramDiffEntry {
        index,
        status: if unchanged { "unchanged" } else { "changed" }.into(),
        left_kind: Some(left.kind().to_string()),
        right_kind: Some(right.kind().to_string()),
        detail,
    }
}

fn flowchart_unchanged(d: &flowchart::DiagramDiff) -> bool {
    d.added_nodes.is_empty()
        && d.removed_nodes.is_empty()
        && d.modified_nodes.is_empty()
        && d.added_edges.is_empty()
        && d.removed_edges.is_empty()
        && d.modified_edges.is_empty()
        && !d.rankdir_changed
}

fn diff_sequence(
    left: &crate::sequence::SequenceDiagram,
    right: &crate::sequence::SequenceDiagram,
) -> SequenceDiff {
    let left_ids: HashSet<&str> = left.participants.iter().map(|p| p.id.as_str()).collect();
    let right_ids: HashSet<&str> = right.participants.iter().map(|p| p.id.as_str()).collect();

    let added_participants: Vec<String> = right
        .participants
        .iter()
        .filter(|p| !left_ids.contains(p.id.as_str()))
        .map(|p| p.id.clone())
        .collect();
    let removed_participants: Vec<String> = left
        .participants
        .iter()
        .filter(|p| !right_ids.contains(p.id.as_str()))
        .map(|p| p.id.clone())
        .collect();

    let mut added_messages = Vec::new();
    let mut removed_messages = Vec::new();
    let mut modified_messages = Vec::new();

    let max_len = left.messages.len().max(right.messages.len());
    for i in 0..max_len {
        match (left.messages.get(i), right.messages.get(i)) {
            (Some(l), Some(r)) if messages_equal(l, r) => {}
            (Some(l), Some(r)) => modified_messages.push((l.clone(), r.clone())),
            (Some(l), None) => removed_messages.push(l.clone()),
            (None, Some(r)) => added_messages.push(r.clone()),
            (None, None) => {}
        }
    }

    SequenceDiff {
        added_participants,
        removed_participants,
        added_messages,
        removed_messages,
        modified_messages,
        added_notes: right.notes.len().saturating_sub(left.notes.len()),
        removed_notes: left.notes.len().saturating_sub(right.notes.len()),
        added_fragments: right.fragments.len().saturating_sub(left.fragments.len()),
        removed_fragments: left.fragments.len().saturating_sub(right.fragments.len()),
    }
}

fn messages_equal(a: &crate::sequence::Message, b: &crate::sequence::Message) -> bool {
    a.from == b.from && a.to == b.to && a.text == b.text && a.arrow == b.arrow
}

fn sequence_unchanged(d: &SequenceDiff) -> bool {
    d.added_participants.is_empty()
        && d.removed_participants.is_empty()
        && d.added_messages.is_empty()
        && d.removed_messages.is_empty()
        && d.modified_messages.is_empty()
        && d.added_notes == 0
        && d.removed_notes == 0
        && d.added_fragments == 0
        && d.removed_fragments == 0
}

fn diff_class(
    left: &crate::class::ClassDiagram,
    right: &crate::class::ClassDiagram,
) -> ClassDiff {
    let left_ids: HashSet<&str> = left.classes.iter().map(|c| c.id.as_str()).collect();
    let right_ids: HashSet<&str> = right.classes.iter().map(|c| c.id.as_str()).collect();

    let added_classes: Vec<String> = right
        .classes
        .iter()
        .filter(|c| !left_ids.contains(c.id.as_str()))
        .map(|c| c.id.clone())
        .collect();
    let removed_classes: Vec<String> = left
        .classes
        .iter()
        .filter(|c| !right_ids.contains(c.id.as_str()))
        .map(|c| c.id.clone())
        .collect();

    let relation_key = |r: &crate::class::Relation| {
        format!(
            "{}|{}|{:?}|{}|{:?}|{:?}",
            r.from,
            r.to,
            r.kind,
            r.label,
            r.from_card,
            r.to_card
        )
    };

    let left_rel: HashSet<String> = left.relations.iter().map(relation_key).collect();
    let right_rel: HashSet<String> = right.relations.iter().map(relation_key).collect();

    let added_relations: Vec<crate::class::Relation> = right
        .relations
        .iter()
        .filter(|r| !left_rel.contains(&relation_key(r)))
        .cloned()
        .collect();
    let removed_relations: Vec<crate::class::Relation> = left
        .relations
        .iter()
        .filter(|r| !right_rel.contains(&relation_key(r)))
        .cloned()
        .collect();

    let left_stereo: HashMap<&str, Option<&str>> = left
        .classes
        .iter()
        .map(|c| (c.id.as_str(), c.stereotype.as_deref()))
        .collect();
    let changed_stereotypes: Vec<String> = right
        .classes
        .iter()
        .filter(|c| {
            left_stereo
                .get(c.id.as_str())
                .is_some_and(|s| *s != c.stereotype.as_deref())
        })
        .map(|c| c.id.clone())
        .collect();

    ClassDiff {
        added_classes,
        removed_classes,
        added_relations,
        removed_relations,
        added_notes: right.notes.len().saturating_sub(left.notes.len()),
        removed_notes: left.notes.len().saturating_sub(right.notes.len()),
        changed_stereotypes,
    }
}

fn class_unchanged(d: &ClassDiff) -> bool {
    d.added_classes.is_empty()
        && d.removed_classes.is_empty()
        && d.added_relations.is_empty()
        && d.removed_relations.is_empty()
        && d.added_notes == 0
        && d.removed_notes == 0
        && d.changed_stereotypes.is_empty()
}

fn diff_gantt(
    left: &crate::gantt::GanttDiagram,
    right: &crate::gantt::GanttDiagram,
) -> GanttDiff {
    let title_changed = left.title != right.title;

    let task_key = |t: &crate::gantt::GanttTask| {
        format!(
            "{}|{}|{}",
            t.section,
            t.id.as_deref().unwrap_or(t.name.as_str()),
            if t.milestone { "milestone" } else { "task" }
        )
    };

    let left_keys: HashSet<String> = left.tasks.iter().map(task_key).collect();
    let right_keys: HashSet<String> = right.tasks.iter().map(task_key).collect();

    let added_tasks: Vec<String> = right
        .tasks
        .iter()
        .filter(|t| !left_keys.contains(&task_key(t)))
        .map(|t| t.id.clone().unwrap_or_else(|| t.name.clone()))
        .collect();
    let removed_tasks: Vec<String> = left
        .tasks
        .iter()
        .filter(|t| !right_keys.contains(&task_key(t)))
        .map(|t| t.id.clone().unwrap_or_else(|| t.name.clone()))
        .collect();

    GanttDiff {
        title_changed,
        added_tasks,
        removed_tasks,
    }
}

fn gantt_unchanged(d: &GanttDiff) -> bool {
    !d.title_changed && d.added_tasks.is_empty() && d.removed_tasks.is_empty()
}

fn diff_state(
    left: &crate::state::StateDiagram,
    right: &crate::state::StateDiagram,
) -> StateDiff {
    let left_ids: HashSet<&str> = left.states.iter().map(|s| s.id.as_str()).collect();
    let right_ids: HashSet<&str> = right.states.iter().map(|s| s.id.as_str()).collect();

    let added_states: Vec<String> = right
        .states
        .iter()
        .filter(|s| !left_ids.contains(s.id.as_str()))
        .map(|s| s.id.clone())
        .collect();
    let removed_states: Vec<String> = left
        .states
        .iter()
        .filter(|s| !right_ids.contains(s.id.as_str()))
        .map(|s| s.id.clone())
        .collect();

    let transition_key = |t: &crate::state::Transition| {
        format!("{}|{}|{}", t.from, t.to, t.label)
    };
    let left_t: HashSet<String> = left.transitions.iter().map(transition_key).collect();
    let right_t: HashSet<String> = right.transitions.iter().map(transition_key).collect();

    let added_transitions: Vec<crate::state::Transition> = right
        .transitions
        .iter()
        .filter(|t| !left_t.contains(&transition_key(t)))
        .cloned()
        .collect();
    let removed_transitions: Vec<crate::state::Transition> = left
        .transitions
        .iter()
        .filter(|t| !right_t.contains(&transition_key(t)))
        .cloned()
        .collect();

    StateDiff {
        added_states,
        removed_states,
        added_transitions,
        removed_transitions,
    }
}

fn state_unchanged(d: &StateDiff) -> bool {
    d.added_states.is_empty()
        && d.removed_states.is_empty()
        && d.added_transitions.is_empty()
        && d.removed_transitions.is_empty()
}

fn diff_er(left: &crate::er::ErDiagram, right: &crate::er::ErDiagram) -> ErDiff {
    let left_ids: HashSet<&str> = left.entities.iter().map(|e| e.id.as_str()).collect();
    let right_ids: HashSet<&str> = right.entities.iter().map(|e| e.id.as_str()).collect();

    let added_entities: Vec<String> = right
        .entities
        .iter()
        .filter(|e| !left_ids.contains(e.id.as_str()))
        .map(|e| e.id.clone())
        .collect();
    let removed_entities: Vec<String> = left
        .entities
        .iter()
        .filter(|e| !right_ids.contains(e.id.as_str()))
        .map(|e| e.id.clone())
        .collect();

    let rel_key = |r: &crate::er::Relationship| {
        format!(
            "{}|{}|{:?}|{:?}|{}|{}",
            r.from, r.to, r.from_card, r.to_card, r.identifying, r.label
        )
    };
    let left_r: HashSet<String> = left.relationships.iter().map(rel_key).collect();
    let right_r: HashSet<String> = right.relationships.iter().map(rel_key).collect();

    let added_relationships: Vec<crate::er::Relationship> = right
        .relationships
        .iter()
        .filter(|r| !left_r.contains(&rel_key(r)))
        .cloned()
        .collect();
    let removed_relationships: Vec<crate::er::Relationship> = left
        .relationships
        .iter()
        .filter(|r| !right_r.contains(&rel_key(r)))
        .cloned()
        .collect();

    ErDiff {
        added_entities,
        removed_entities,
        added_relationships,
        removed_relationships,
    }
}

fn er_unchanged(d: &ErDiff) -> bool {
    d.added_entities.is_empty()
        && d.removed_entities.is_empty()
        && d.added_relationships.is_empty()
        && d.removed_relationships.is_empty()
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

    #[test]
    fn multi_document_metrics_entries() {
        let doc = ir::from_mermaid("graph TD\n  A-->B\n").unwrap();
        let doc2 = Document {
            version: 1,
            diagrams: vec![
                doc.primary().unwrap().clone(),
                ir::from_mermaid("sequenceDiagram\n  A->>B: hi\n")
                    .unwrap()
                    .primary()
                    .unwrap()
                    .clone(),
            ],
        };
        let m = metrics(&doc2);
        assert_eq!(m.kind, "multi");
        assert_eq!(m.entries.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn diff_flowchart_detects_node_and_edge_changes() {
        let left = ir::from_mermaid("graph TD\n  A-->B\n").unwrap();
        let right = ir::from_mermaid("graph TD\n  A-->B\n  A-->C\n").unwrap();
        let d = diff_documents(&left, &right);
        assert!(!d.unchanged);
        assert_eq!(d.entries.len(), 1);
        assert_eq!(d.entries[0].status, "changed");
        let Some(DiffDetail::Flowchart(fc)) = &d.entries[0].detail else {
            panic!("expected flowchart diff");
        };
        assert_eq!(fc.added_edges.len(), 1);
        assert_eq!(fc.added_edges[0].to, "C");
    }

    #[test]
    fn diff_identical_documents_unchanged() {
        let doc = ir::from_mermaid("graph TD\n  A-->B\n").unwrap();
        let d = diff_documents(&doc, &doc);
        assert!(d.unchanged);
        assert_eq!(d.entries[0].status, "unchanged");
    }

    #[test]
    fn diff_sequence_participant_change() {
        let left = ir::from_mermaid("sequenceDiagram\n  A->>B: hi\n").unwrap();
        let right = ir::from_mermaid("sequenceDiagram\n  participant C\n  A->>B: hi\n").unwrap();
        let d = diff_documents(&left, &right);
        let Some(DiffDetail::Sequence(s)) = &d.entries[0].detail else {
            panic!("expected sequence diff");
        };
        assert!(s.added_participants.contains(&"C".to_string()));
    }

    #[test]
    fn state_metrics_counts() {
        let doc = ir::from_mermaid(
            "stateDiagram-v2\n  [*] --> A\n  A --> B\n  state check <<choice>>\n",
        )
        .unwrap();
        let MetricsDetail::State(s) = metrics(&doc).detail else {
            panic!("expected state metrics");
        };
        assert!(s.states >= 2);
        assert_eq!(s.transitions, 2);
        assert!(s.start_end_nodes >= 1);
    }

    #[test]
    fn er_metrics_counts() {
        let doc = ir::from_mermaid(
            "erDiagram\n  CUSTOMER ||--o{ ORDER : places\n  CUSTOMER {\n    string name PK\n  }\n",
        )
        .unwrap();
        let MetricsDetail::Er(e) = metrics(&doc).detail else {
            panic!("expected er metrics");
        };
        assert_eq!(e.entities, 2);
        assert_eq!(e.relationships, 1);
        assert_eq!(e.attributes, 1);
    }

    #[test]
    fn diff_document_count_change() {
        let one = ir::from_mermaid("graph TD\n  A-->B\n").unwrap();
        let two = Document {
            version: 1,
            diagrams: vec![
                one.primary().unwrap().clone(),
                ir::from_mermaid("sequenceDiagram\n  A->>B: hi\n")
                    .unwrap()
                    .primary()
                    .unwrap()
                    .clone(),
            ],
        };
        let d = diff_documents(&one, &two);
        assert!(d.diagram_count_changed);
        assert_eq!(d.entries.len(), 2);
        assert_eq!(d.entries[1].status, "added");
    }
}
