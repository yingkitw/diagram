//! PlantUML Compatibility adapter (sequence MVP → sequence IR).

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

/// Parse PlantUML source into a single-diagram Document (sequence only in MVP).
pub fn parse_to_document(source: &str) -> Result<Document, IrError> {
    if !is_plantuml(source) {
        return Err(IrError::from("PlantUML: expected @startuml header"));
    }
    if looks_like_class(source) {
        return Err(IrError::from(
            "PlantUML class diagrams are not supported yet; use sequence MVP",
        ));
    }
    if looks_like_activity(source) {
        return Err(IrError::from(
            "PlantUML activity diagrams are not supported yet; use sequence MVP",
        ));
    }
    Ok(Document::single(Diagram::Sequence(parse_sequence(source)?)))
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
    fn roundtrip_to_mermaid() {
        let src = "@startuml\nA -> B: ping\nB --> A: pong\n@enduml";
        let doc = parse_to_document(src).unwrap();
        let mmd = doc.to_mermaid().unwrap();
        assert!(mmd.contains("sequenceDiagram"));
        assert!(crate::sequence::parse(&mmd).is_ok());
    }

    #[test]
    fn rejects_class_diagram() {
        let src = "@startuml\nclass Foo {\n  +bar()\n}\n@enduml";
        assert!(parse_to_document(src).is_err());
    }
}
