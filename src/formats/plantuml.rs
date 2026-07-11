//! PlantUML Compatibility adapter (sequence, class, activity → IR).

use crate::class::{self, Class, ClassDiagram, ClassMember, Relation};
use crate::diagram::{Diagram as FcDiagram, Edge, Node, NodeShape};
use crate::ir::{Diagram, Document, IrError};
use crate::sequence::{Message, MessageArrow, Participant, SequenceDiagram};
use std::collections::HashMap;

/// True when source looks like PlantUML (`@startuml` … `@enduml`).
pub fn is_plantuml(source: &str) -> bool {
    source
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty() && !l.starts_with('\''))
        .is_some_and(|l| l.starts_with("@startuml"))
}

/// Parse PlantUML source into a single-diagram Document.
pub fn parse_to_document(source: &str) -> Result<Document, IrError> {
    if !is_plantuml(source) {
        return Err(IrError::from("PlantUML: expected @startuml header"));
    }
    if looks_like_activity(source) {
        return Ok(Document::single(Diagram::Flowchart(parse_activity(source)?)));
    }
    if looks_like_class(source) {
        return Ok(Document::single(Diagram::Class(parse_class(source)?)));
    }
    Ok(Document::single(Diagram::Sequence(parse_sequence(source)?)))
}

/// Export a Document to PlantUML (sequence and class diagrams only).
pub fn export_document(doc: &Document) -> Result<String, IrError> {
    let supported: Vec<(usize, &Diagram)> = doc
        .diagrams
        .iter()
        .enumerate()
        .filter_map(|(i, d)| match d {
            Diagram::Sequence(_) | Diagram::Class(_) => Some((i, d)),
            _ => None,
        })
        .collect();

    if supported.is_empty() {
        return Err(IrError::from(
            "PlantUML export supports sequence and class diagrams only",
        ));
    }

    let multi = doc.diagrams.len() > 1;
    let blocks: Vec<String> = supported
        .into_iter()
        .map(|(i, d)| {
            let mut block = String::new();
            if multi {
                block.push_str(&format!("' diagram {i}: {}\n", d.kind()));
            }
            block.push_str(&export_diagram(d));
            block
        })
        .collect();

    Ok(blocks.join("\n\n"))
}

fn export_diagram(d: &Diagram) -> String {
    match d {
        Diagram::Sequence(s) => export_sequence(s),
        Diagram::Class(c) => export_class(c),
        _ => String::new(),
    }
}

fn export_sequence(s: &SequenceDiagram) -> String {
    let mut out = String::from("@startuml\n");
    for p in &s.participants {
        if p.label != p.id {
            if p.label.contains(' ') {
                out.push_str(&format!(
                    "participant \"{}\" as {}\n",
                    escape_quote(&p.label),
                    p.id
                ));
            } else {
                out.push_str(&format!("participant {} as {}\n", p.id, p.label));
            }
        } else {
            out.push_str(&format!("participant {}\n", p.id));
        }
    }
    for m in &s.messages {
        let arrow = match m.arrow {
            MessageArrow::Solid => "->",
            MessageArrow::Dashed => "-->",
        };
        out.push_str(&format!("{} {} {}: {}\n", m.from, arrow, m.to, m.text));
    }
    out.push_str("@enduml\n");
    out
}

fn export_class(c: &ClassDiagram) -> String {
    let mut out = String::from("@startuml\n");
    for cls in &c.classes {
        if cls.members.is_empty() {
            out.push_str(&format!("class {}\n", cls.id));
        } else {
            out.push_str(&format!("class {} {{\n", cls.id));
            for m in &cls.members {
                out.push_str(&format!("  {}\n", m.text));
            }
            out.push_str("}\n");
        }
    }
    for r in &c.relations {
        let token = r.kind.mermaid_str();
        if r.label.is_empty() {
            out.push_str(&format!("{} {} {}\n", r.from, token, r.to));
        } else {
            out.push_str(&format!("{} {} {} : {}\n", r.from, token, r.to, r.label));
        }
    }
    out.push_str("@enduml\n");
    out
}

fn escape_quote(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

#[derive(Debug, Clone)]
enum ActivityStmt {
    Start,
    Stop,
    Action(String),
    If { condition: String, then_label: String },
    Else(String),
    EndIf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IfPhase {
    InThen,
    InElse,
}

#[derive(Debug, Clone)]
struct IfFrame {
    fork_id: String,
    then_label: String,
    else_label: String,
    yes_tail: Option<String>,
    no_tail: Option<String>,
    phase: IfPhase,
}

struct ActivityBuilder {
    diagram: FcDiagram,
    next_id: usize,
    tails: Vec<String>,
    if_stack: Vec<IfFrame>,
}

impl ActivityBuilder {
    fn new() -> Self {
        Self {
            diagram: FcDiagram::new("TD"),
            next_id: 0,
            tails: Vec::new(),
            if_stack: Vec::new(),
        }
    }

    fn finish(self) -> Result<FcDiagram, IrError> {
        if self.diagram.nodes.is_empty() {
            return Err(IrError::from("PlantUML activity: no steps"));
        }
        Ok(self.diagram)
    }

    fn add_node(&mut self, text: &str, shape: NodeShape) -> String {
        let id = format!("act_{}", self.next_id);
        self.next_id += 1;
        self.diagram
            .add_node(Node {
                id: id.clone(),
                text: text.to_string(),
                shape,
                href: None,
                tooltip: None,
            })
            .unwrap();
        id
    }

    fn connect(&mut self, from: &str, to: &str, label: &str) {
        self.diagram
            .add_edge(Edge {
                from: from.to_string(),
                to: to.to_string(),
                label: label.to_string(),
                style: Default::default(),
            })
            .unwrap();
    }

    fn connect_tails(&mut self, to: &str, label: &str) {
        let froms: Vec<String> = self.tails.drain(..).collect();
        for from in froms {
            self.connect(&from, to, label);
        }
    }

    fn set_single_tail(&mut self, id: &str) {
        self.tails = vec![id.to_string()];
    }

    fn apply(&mut self, stmt: ActivityStmt) -> Result<(), IrError> {
        match stmt {
            ActivityStmt::Start => {
                let id = self.add_node("start", NodeShape::Stadium);
                self.set_single_tail(&id);
            }
            ActivityStmt::Stop => {
                let id = self.add_node("stop", NodeShape::Stadium);
                self.connect_tails(&id, "");
                self.tails.clear();
            }
            ActivityStmt::Action(label) => {
                let id = self.add_node(&label, NodeShape::Rect);
                if let Some(frame) = self.if_stack.last() {
                    let fork_id = frame.fork_id.clone();
                    let then_label = frame.then_label.clone();
                    let else_label = frame.else_label.clone();
                    let phase = frame.phase;
                    match phase {
                        IfPhase::InThen => {
                            if self.tails.is_empty() {
                                self.connect(&fork_id, &id, &then_label);
                            } else {
                                self.connect_tails(&id, "");
                            }
                            self.if_stack.last_mut().unwrap().yes_tail = Some(id.clone());
                        }
                        IfPhase::InElse => {
                            if self.tails.is_empty() {
                                self.connect(&fork_id, &id, &else_label);
                            } else {
                                self.connect_tails(&id, "");
                            }
                            self.if_stack.last_mut().unwrap().no_tail = Some(id.clone());
                        }
                    }
                } else {
                    self.connect_tails(&id, "");
                }
                self.set_single_tail(&id);
            }
            ActivityStmt::If {
                condition,
                then_label,
            } => {
                let id = self.add_node(&condition, NodeShape::Diamond);
                self.connect_tails(&id, "");
                self.if_stack.push(IfFrame {
                    fork_id: id,
                    then_label,
                    else_label: String::new(),
                    yes_tail: None,
                    no_tail: None,
                    phase: IfPhase::InThen,
                });
                self.tails.clear();
            }
            ActivityStmt::Else(label) => {
                let frame = self
                    .if_stack
                    .last_mut()
                    .ok_or_else(|| IrError::from("PlantUML activity: else without if"))?;
                if frame.yes_tail.is_none() {
                    frame.yes_tail = self.tails.last().cloned();
                }
                frame.else_label = label;
                frame.phase = IfPhase::InElse;
                self.tails.clear();
            }
            ActivityStmt::EndIf => {
                let frame = self
                    .if_stack
                    .pop()
                    .ok_or_else(|| IrError::from("PlantUML activity: endif without if"))?;
                let mut merge = Vec::new();
                if let Some(y) = frame.yes_tail {
                    merge.push(y);
                }
                if let Some(n) = frame.no_tail {
                    if !merge.contains(&n) {
                        merge.push(n);
                    }
                }
                if merge.is_empty() {
                    merge.push(frame.fork_id);
                }
                self.tails = merge;
            }
        }
        Ok(())
    }
}

fn parse_activity(source: &str) -> Result<FcDiagram, IrError> {
    let mut builder = ActivityBuilder::new();
    for (line_num, text) in source.lines().enumerate() {
        let line = text.trim();
        if line.is_empty()
            || line.starts_with('@')
            || line.starts_with('\'')
            || line.starts_with("skinparam")
            || line.starts_with("title ")
            || line.starts_with("note ")
        {
            continue;
        }
        let stmt = parse_activity_line(line).ok_or_else(|| {
            IrError::from(format!(
                "PlantUML activity line {}: unrecognized: {line}",
                line_num + 1
            ))
        })?;
        builder.apply(stmt)?;
    }
    builder.finish()
}

fn parse_activity_line(line: &str) -> Option<ActivityStmt> {
    if line == "start" {
        return Some(ActivityStmt::Start);
    }
    if line == "stop" || line == "end" {
        return Some(ActivityStmt::Stop);
    }
    if line == "endif" {
        return Some(ActivityStmt::EndIf);
    }
    if let Some(label) = line.strip_prefix(':') {
        let label = label.trim_end_matches(';').trim();
        if !label.is_empty() {
            return Some(ActivityStmt::Action(label.to_string()));
        }
    }
    if let Some(rest) = line.strip_prefix("if ") {
        let (condition, rest) = parse_parens(rest)?;
        let rest = rest.trim();
        let rest = rest.strip_prefix("then")?.trim();
        let (then_label, _) = parse_parens(rest)?;
        return Some(ActivityStmt::If {
            condition,
            then_label,
        });
    }
    if let Some(rest) = line.strip_prefix("else") {
        let label = rest
            .trim()
            .strip_prefix('(')
            .and_then(|s| s.strip_suffix(')'))
            .unwrap_or("")
            .to_string();
        return Some(ActivityStmt::Else(label));
    }
    None
}

fn parse_parens(s: &str) -> Option<(String, &str)> {
    let s = s.trim_start();
    if !s.starts_with('(') {
        return None;
    }
    let mut depth = 0;
    let mut end = 0;
    for (i, c) in s.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    end = i;
                    break;
                }
            }
            _ => {}
        }
    }
    if depth != 0 {
        return None;
    }
    let inner = s[1..end].trim();
    Some((inner.to_string(), s[end + 1..].trim_start()))
}

fn looks_like_class(source: &str) -> bool {
    meaningful_lines(source).any(|l| {
        l.starts_with("class ")
            || l.starts_with("interface ")
            || l.starts_with("enum ")
            || l.contains(" <|-- ")
            || l.contains(" --|> ")
    })
}

fn looks_like_activity(source: &str) -> bool {
    meaningful_lines(source).any(|l| {
        l == "start" || l == "stop" || l == "end" || (l.starts_with(':') && !l.contains("->"))
    })
}

fn meaningful_lines(source: &str) -> impl Iterator<Item = &str> {
    source.lines().map(str::trim).filter(|l| {
        !l.is_empty()
            && !l.starts_with('@')
            && !l.starts_with('\'')
            && !l.starts_with("skinparam")
            && !l.starts_with("autonumber")
            && !l.starts_with("title ")
    })
}

fn parse_sequence(source: &str) -> Result<SequenceDiagram, IrError> {
    let mut order: Vec<String> = Vec::new();
    let mut labels: HashMap<String, String> = HashMap::new();
    let mut messages: Vec<Message> = Vec::new();

    for (line_num, text) in source.lines().enumerate() {
        let line = text.trim();
        if line.is_empty()
            || line.starts_with('@')
            || line.starts_with('\'')
            || line.starts_with("skinparam")
            || line.starts_with("autonumber")
            || line.starts_with("title ")
        {
            continue;
        }

        if let Some(rest) = line
            .strip_prefix("participant ")
            .or_else(|| line.strip_prefix("actor "))
        {
            let (id, label) = parse_participant(rest);
            if !order.iter().any(|p| p == &id) {
                order.push(id.clone());
            }
            labels.insert(id, label);
            continue;
        }

        if let Some(msg) = parse_message(line) {
            ensure_participant(&mut order, &mut labels, &msg.from);
            ensure_participant(&mut order, &mut labels, &msg.to);
            messages.push(msg);
            continue;
        }

        return Err(IrError::from(format!(
            "PlantUML sequence line {line_num}: unrecognized: {line}"
        )));
    }

    if messages.is_empty() && order.is_empty() {
        return Err(IrError::from("PlantUML sequence: no participants or messages"));
    }

    let participants = order
        .into_iter()
        .map(|id| {
            let label = labels.get(&id).cloned().unwrap_or_else(|| id.clone());
            Participant { id, label }
        })
        .collect();

    Ok(SequenceDiagram {
        participants,
        messages,
    })
}

fn ensure_participant(order: &mut Vec<String>, labels: &mut HashMap<String, String>, id: &str) {
    if !order.iter().any(|p| p == id) {
        order.push(id.to_string());
        labels.entry(id.to_string()).or_insert_with(|| id.to_string());
    }
}

fn parse_participant(rest: &str) -> (String, String) {
    let rest = rest.trim();
    if let Some(idx) = rest.find(" as ") {
        let (left, right) = rest.split_at(idx);
        let right = right.strip_prefix(" as ").unwrap_or("").trim();
        let left = left.trim();
        if left.starts_with('"') {
            let label = unquote(left);
            (right.to_string(), label)
        } else {
            (left.to_string(), right.to_string())
        }
    } else if rest.starts_with('"') {
        let label = unquote(rest);
        let id = label.replace(' ', "");
        (id.clone(), label)
    } else {
        let id = rest.to_string();
        (id.clone(), id)
    }
}

fn unquote(s: &str) -> String {
    let s = s.trim();
    if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
        s[1..s.len() - 1].replace("\\\"", "\"").replace("\\n", "\n")
    } else {
        s.to_string()
    }
}

fn parse_message(text: &str) -> Option<Message> {
    let normalized = text.replace(" : ", ":");
    for (arrow_str, arrow) in [
        ("-->>", MessageArrow::Dashed),
        ("->>", MessageArrow::Solid),
        ("-->", MessageArrow::Dashed),
        ("->", MessageArrow::Solid),
    ] {
        if let Some((left, right)) = normalized.split_once(arrow_str) {
            let from = left.trim().to_string();
            let (to, msg_text) = match right.split_once(':') {
                Some((to, t)) => (to.trim().to_string(), t.trim().to_string()),
                None => (right.trim().to_string(), String::new()),
            };
            if from.is_empty() || to.is_empty() {
                return None;
            }
            return Some(Message {
                from,
                to,
                text: msg_text,
                arrow,
            });
        }
    }
    None
}

fn parse_class(source: &str) -> Result<ClassDiagram, IrError> {
    let mut classes: HashMap<String, Class> = HashMap::new();
    let mut order: Vec<String> = Vec::new();
    let mut relations: Vec<Relation> = Vec::new();
    let mut in_together = false;

    let lines: Vec<(usize, &str)> = source
        .lines()
        .enumerate()
        .map(|(i, l)| (i + 1, l.trim()))
        .collect();
    let mut i = 0;
    while i < lines.len() {
        let (line_num, line) = lines[i];
        if line.is_empty()
            || line.starts_with('@')
            || line.starts_with('\'')
            || line.starts_with("skinparam")
            || line.starts_with("title ")
            || line.starts_with("hide ")
            || line.starts_with("note ")
            || line.starts_with("package ")
            || line.starts_with("namespace ")
            || line == "end"
            || line.starts_with("end ")
        {
            i += 1;
            continue;
        }
        if line == "together" || line.starts_with("together ") {
            in_together = true;
            i += 1;
            continue;
        }
        if in_together {
            if line == "}" {
                in_together = false;
            }
            i += 1;
            continue;
        }

        if let Some((id, body_start)) = parse_entity_decl(line) {
            class::ensure_class(&mut classes, &mut order, &id);
            if let Some(body) = body_start {
                if body.ends_with('}') {
                    let inner = body.trim_end_matches('}').trim();
                    push_members(&mut classes, &id, inner);
                    i += 1;
                    continue;
                }
                push_members(&mut classes, &id, body);
                i += 1;
                while i < lines.len() {
                    let (_, body_line) = lines[i];
                    if body_line == "}" {
                        i += 1;
                        break;
                    }
                    if let Some(stripped) = body_line.strip_suffix('}') {
                        push_members(&mut classes, &id, stripped);
                        i += 1;
                        break;
                    }
                    push_members(&mut classes, &id, body_line);
                    i += 1;
                }
                continue;
            }
            i += 1;
            continue;
        }

        if let Some((id, member)) = parse_colon_member(line) {
            class::ensure_class(&mut classes, &mut order, &id);
            classes.get_mut(&id).unwrap().members.push(ClassMember {
                text: member,
            });
            i += 1;
            continue;
        }

        if let Some(mut rel) = class::parse_relation_line(line) {
            rel.from = parse_relation_endpoint(&rel.from);
            rel.to = parse_relation_endpoint(&rel.to);
            class::ensure_class(&mut classes, &mut order, &rel.from);
            class::ensure_class(&mut classes, &mut order, &rel.to);
            relations.push(rel);
            i += 1;
            continue;
        }

        return Err(IrError::from(format!(
            "PlantUML class line {line_num}: unrecognized: {line}"
        )));
    }

    if classes.is_empty() && relations.is_empty() {
        return Err(IrError::from("PlantUML class: no classes or relations"));
    }

    let classes = order
        .into_iter()
        .filter_map(|id| classes.remove(&id))
        .collect();

    Ok(ClassDiagram { classes, relations })
}

fn parse_entity_decl(line: &str) -> Option<(String, Option<&str>)> {
    for prefix in ["abstract class ", "interface ", "enum ", "class "] {
        if let Some(rest) = line.strip_prefix(prefix) {
            let rest = rest.trim();
            if let Some(brace) = rest.find('{') {
                let id = strip_stereotype(&rest[..brace]);
                if id.is_empty() {
                    return None;
                }
                return Some((id, Some(rest[brace + 1..].trim())));
            }
            let id = strip_stereotype(rest);
            if id.is_empty() {
                return None;
            }
            return Some((id, None));
        }
    }
    None
}

fn strip_stereotype(s: &str) -> String {
    let s = s.trim();
    if let Some(idx) = s.find("<<") {
        s[..idx].trim().to_string()
    } else {
        s.to_string()
    }
}

fn parse_colon_member(line: &str) -> Option<(String, String)> {
    let (left, right) = line.split_once(':')?;
    let id = left.trim();
    let member = right.trim();
    if id.is_empty() || member.is_empty() {
        return None;
    }
    if id.contains('<') || id.contains('>') || id.contains('-') {
        return None;
    }
    Some((id.to_string(), member.to_string()))
}

fn parse_relation_endpoint(s: &str) -> String {
    s.split_whitespace()
        .filter(|t| !t.starts_with('"'))
        .last()
        .unwrap_or(s)
        .trim_matches('"')
        .to_string()
}

fn push_members(classes: &mut HashMap<String, Class>, id: &str, chunk: &str) {
    if chunk.is_empty() {
        return;
    }
    for part in chunk.split(';') {
        let m = part.trim();
        if !m.is_empty() {
            classes.get_mut(id).unwrap().members.push(ClassMember {
                text: m.to_string(),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{Diagram, Kind};

    #[test]
    fn is_plantuml_detects_header() {
        assert!(is_plantuml("@startuml\nAlice -> Bob: hi\n@enduml"));
        assert!(!is_plantuml("sequenceDiagram\n  A->>B: hi"));
    }

    #[test]
    fn parse_basic_sequence() {
        let src = r#"@startuml
participant Alice
participant "Bob Server" as Bob
Alice -> Bob: Hello
Bob --> Alice: Hi
@enduml"#;
        let doc = parse_to_document(src).unwrap();
        assert_eq!(doc.primary().unwrap().kind(), Kind::Sequence);
        let Diagram::Sequence(s) = doc.primary().unwrap() else {
            panic!("expected sequence");
        };
        assert_eq!(s.participants.len(), 2);
        assert_eq!(s.messages.len(), 2);
        assert_eq!(s.messages[0].arrow, MessageArrow::Solid);
        assert_eq!(s.messages[1].arrow, MessageArrow::Dashed);
    }

    #[test]
    fn export_sequence_roundtrip() {
        let src = "@startuml\nAlice -> Bob: hi\nBob --> Alice: pong\n@enduml";
        let doc = parse_to_document(src).unwrap();
        let out = export_document(&doc).unwrap();
        assert!(out.contains("@startuml"));
        assert!(out.contains("Alice -> Bob: hi"));
        let doc2 = parse_to_document(&out).unwrap();
        let Diagram::Sequence(s1) = doc.primary().unwrap() else {
            panic!("expected sequence");
        };
        let Diagram::Sequence(s2) = doc2.primary().unwrap() else {
            panic!("expected sequence");
        };
        assert_eq!(s1.messages.len(), s2.messages.len());
        assert_eq!(s1.messages[1].arrow, s2.messages[1].arrow);
    }

    #[test]
    fn export_class_roundtrip() {
        let src = "@startuml\nclass A { +x() }\nclass B\nA --> B : link\n@enduml";
        let doc = parse_to_document(src).unwrap();
        let out = export_document(&doc).unwrap();
        assert!(out.contains("class A"));
        assert!(out.contains("A --> B : link"));
        let doc2 = parse_to_document(&out).unwrap();
        let Diagram::Class(c1) = doc.primary().unwrap() else {
            panic!("expected class");
        };
        let Diagram::Class(c2) = doc2.primary().unwrap() else {
            panic!("expected class");
        };
        assert_eq!(c1.classes.len(), c2.classes.len());
        assert_eq!(c1.relations[0].label, c2.relations[0].label);
    }

    #[test]
    fn roundtrip_to_mermaid() {
        let src = "@startuml\nA -> B: ping\nB --> A: pong\n@enduml";
        let doc = parse_to_document(src).unwrap();
        let mmd = doc.to_mermaid().unwrap();
        assert!(mmd.contains("sequenceDiagram"));
        assert!(crate::sequence::parse(&mmd).is_ok());
    }

    #[test]
    fn parse_basic_class() {
        let src = r#"@startuml
class Animal {
  +name: String
  +eat()
}
class Duck {
  +swim()
}
Animal <|-- Duck
@enduml"#;
        let doc = parse_to_document(src).unwrap();
        assert_eq!(doc.primary().unwrap().kind(), Kind::Class);
        let Diagram::Class(c) = doc.primary().unwrap() else {
            panic!("expected class");
        };
        assert_eq!(c.classes.len(), 2);
        assert_eq!(c.relations.len(), 1);
        assert_eq!(c.relations[0].kind, class::RelationKind::Inheritance);
    }

    #[test]
    fn parse_class_interface_and_dependency() {
        let src = r#"@startuml
interface Repository
class Service
Service ..> Repository : uses
@enduml"#;
        let doc = parse_to_document(src).unwrap();
        let Diagram::Class(c) = doc.primary().unwrap() else {
            panic!("expected class");
        };
        assert_eq!(c.classes.len(), 2);
        assert_eq!(c.relations[0].kind, class::RelationKind::Dependency);
        assert_eq!(c.relations[0].label, "uses");
    }

    #[test]
    fn parse_basic_activity() {
        let src = r#"@startuml
start
:Hello;
stop
@enduml"#;
        let doc = parse_to_document(src).unwrap();
        assert_eq!(doc.primary().unwrap().kind(), Kind::Flowchart);
        let Diagram::Flowchart(fc) = doc.primary().unwrap() else {
            panic!("expected flowchart");
        };
        assert!(fc.nodes.len() >= 3);
        assert!(fc.edges.len() >= 2);
        assert!(fc.nodes.iter().any(|n| n.text == "Hello"));
    }

    #[test]
    fn parse_activity_if_else() {
        let src = r#"@startuml
start
:A;
if (ok?) then (yes)
  :B;
else (no)
  :C;
endif
:D;
stop
@enduml"#;
        let doc = parse_to_document(src).unwrap();
        let Diagram::Flowchart(fc) = doc.primary().unwrap() else {
            panic!("expected flowchart");
        };
        assert!(fc.nodes.iter().any(|n| n.shape == NodeShape::Diamond));
        assert!(fc.edges.iter().any(|e| e.label == "yes"));
        assert!(fc.edges.iter().any(|e| e.label == "no"));
        assert!(fc.edges.iter().any(|e| e.from.starts_with("act_") && e.to.starts_with("act_")));
    }

    #[test]
    fn activity_exports_to_mermaid() {
        let src = "@startuml\nstart\n:Work;\nstop\n@enduml";
        let doc = parse_to_document(src).unwrap();
        let mmd = doc.to_mermaid().unwrap();
        assert!(mmd.contains("graph TD"));
        assert!(crate::parser::parse(&mmd).is_ok());
    }
}
