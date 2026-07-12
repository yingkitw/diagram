//! Mermaid sequence diagram parse, layout, and SVG render (MVP).

use crate::renderer::Theme;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum MessageArrow {
    /// Solid arrow `->>`
    #[default]
    Solid,
    /// Dashed arrow `-->>`
    Dashed,
}

impl MessageArrow {
    fn mermaid_str(self) -> &'static str {
        match self {
            Self::Solid => "->>",
            Self::Dashed => "-->>",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Participant {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub from: String,
    pub to: String,
    pub text: String,
    pub arrow: MessageArrow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotePlacement {
    LeftOf,
    RightOf,
    Over,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub placement: NotePlacement,
    /// One actor for left/right; one or two for over.
    pub actors: Vec<String>,
    pub text: String,
    /// Emit this note before `messages[before_message]` (or after all if == messages.len()).
    pub before_message: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SequenceDiagram {
    pub participants: Vec<Participant>,
    pub messages: Vec<Message>,
    #[serde(default)]
    pub notes: Vec<Note>,
}

#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub line: Option<usize>,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.line {
            Some(l) => write!(f, "line {}: {}", l, self.message),
            None => write!(f, "{}", self.message),
        }
    }
}

impl std::error::Error for ParseError {}

/// True when source looks like a sequence diagram.
pub fn is_sequence(source: &str) -> bool {
    source
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty() && !l.starts_with("%%"))
        .is_some_and(|l| l.starts_with("sequenceDiagram"))
}

pub fn parse(source: &str) -> Result<SequenceDiagram, ParseError> {
    let lines: Vec<(usize, &str)> = source
        .lines()
        .enumerate()
        .map(|(i, l)| (i + 1, l.trim()))
        .filter(|(_, l)| !l.is_empty() && !l.starts_with("%%"))
        .collect();

    if lines.is_empty() || !lines[0].1.starts_with("sequenceDiagram") {
        return Err(ParseError {
            message: "expected sequenceDiagram header".into(),
            line: lines.first().map(|(n, _)| *n),
        });
    }

    let mut order: Vec<String> = Vec::new();
    let mut labels: HashMap<String, String> = HashMap::new();
    let mut messages: Vec<Message> = Vec::new();
    let mut notes: Vec<Note> = Vec::new();

    for (line_num, text) in lines.iter().skip(1) {
        if let Some(rest) = text
            .strip_prefix("participant ")
            .or_else(|| text.strip_prefix("actor "))
        {
            let (id, label) = parse_participant(rest);
            if !order.iter().any(|p| p == &id) {
                order.push(id.clone());
            }
            labels.insert(id, label);
            continue;
        }

        if let Some(note) = parse_note(text, messages.len()) {
            for a in &note.actors {
                ensure_participant(&mut order, &mut labels, a);
            }
            notes.push(note);
            continue;
        }

        if let Some(msg) = parse_message(text) {
            ensure_participant(&mut order, &mut labels, &msg.from);
            ensure_participant(&mut order, &mut labels, &msg.to);
            messages.push(msg);
            continue;
        }

        return Err(ParseError {
            message: format!("unrecognized sequence line: {text}"),
            line: Some(*line_num),
        });
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
        notes,
    })
}

fn ensure_participant(order: &mut Vec<String>, labels: &mut HashMap<String, String>, id: &str) {
    if !order.iter().any(|p| p == id) {
        order.push(id.to_string());
        labels.entry(id.to_string()).or_insert_with(|| id.to_string());
    }
}

fn parse_participant(rest: &str) -> (String, String) {
    if let Some((id, alias)) = rest.split_once(" as ") {
        (id.trim().to_string(), alias.trim().to_string())
    } else {
        let id = rest.trim().to_string();
        (id.clone(), id)
    }
}

fn parse_message(text: &str) -> Option<Message> {
    // Longer arrows first so `-->>` is not mistaken for `->>`.
    for (arrow_str, arrow) in [("-->>", MessageArrow::Dashed), ("->>", MessageArrow::Solid)] {
        if let Some((left, right)) = text.split_once(arrow_str) {
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

fn parse_note(text: &str, before_message: usize) -> Option<Note> {
    let rest = text.strip_prefix("Note ")?.trim_start();
    let (placement, rest) = if let Some(r) = rest.strip_prefix("left of ") {
        (NotePlacement::LeftOf, r)
    } else if let Some(r) = rest.strip_prefix("right of ") {
        (NotePlacement::RightOf, r)
    } else if let Some(r) = rest.strip_prefix("over ") {
        (NotePlacement::Over, r)
    } else {
        return None;
    };
    let (actors_part, note_text) = rest.split_once(':')?;
    let actors: Vec<String> = actors_part
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if actors.is_empty() {
        return None;
    }
    if matches!(placement, NotePlacement::LeftOf | NotePlacement::RightOf) && actors.len() != 1 {
        return None;
    }
    Some(Note {
        placement,
        actors,
        text: note_text.trim().to_string(),
        before_message,
    })
}

impl SequenceDiagram {
    pub fn to_mermaid(&self) -> String {
        let mut out = String::from("sequenceDiagram\n");
        for p in &self.participants {
            if p.label != p.id {
                out.push_str(&format!("    participant {} as {}\n", p.id, p.label));
            } else {
                out.push_str(&format!("    participant {}\n", p.id));
            }
        }
        for i in 0..=self.messages.len() {
            for n in self.notes.iter().filter(|n| n.before_message == i) {
                out.push_str(&format!("    {}\n", note_mermaid(n)));
            }
            if let Some(m) = self.messages.get(i) {
                out.push_str(&format!(
                    "    {}{}{}: {}\n",
                    m.from,
                    m.arrow.mermaid_str(),
                    m.to,
                    m.text
                ));
            }
        }
        out
    }
}

fn note_mermaid(n: &Note) -> String {
    let actors = n.actors.join(",");
    let place = match n.placement {
        NotePlacement::LeftOf => format!("Note left of {actors}"),
        NotePlacement::RightOf => format!("Note right of {actors}"),
        NotePlacement::Over => format!("Note over {actors}"),
    };
    format!("{place}: {}", n.text)
}

struct LaidParticipant {
    id: String,
    label: String,
    x: f64,
}

struct LaidMessage {
    from_x: f64,
    to_x: f64,
    y: f64,
    text: String,
    arrow: MessageArrow,
    self_msg: bool,
}

struct LaidNote {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    text: String,
}

struct SequenceLayout {
    width: f64,
    height: f64,
    participants: Vec<LaidParticipant>,
    messages: Vec<LaidMessage>,
    notes: Vec<LaidNote>,
    header_bottom: f64,
    footer_top: f64,
}

const COL_GAP: f64 = 160.0;
const MARGIN_X: f64 = 40.0;
const BOX_W: f64 = 100.0;
const BOX_H: f64 = 40.0;
const MSG_GAP: f64 = 48.0;
const NOTE_GAP: f64 = 56.0;
const TOP: f64 = 30.0;
const NOTE_PAD: f64 = 8.0;
const NOTE_CHAR_W: f64 = 7.0;
const NOTE_LINE_H: f64 = 16.0;

fn layout(diagram: &SequenceDiagram) -> SequenceLayout {
    let n = diagram.participants.len().max(1);
    let participants: Vec<LaidParticipant> = diagram
        .participants
        .iter()
        .enumerate()
        .map(|(i, p)| LaidParticipant {
            id: p.id.clone(),
            label: p.label.clone(),
            x: MARGIN_X + BOX_W / 2.0 + i as f64 * COL_GAP,
        })
        .collect();

    let x_of: HashMap<&str, f64> = participants.iter().map(|p| (p.id.as_str(), p.x)).collect();

    let header_bottom = TOP + BOX_H;
    let mut y = header_bottom + MSG_GAP;
    let mut messages = Vec::new();
    let mut notes = Vec::new();

    for i in 0..=diagram.messages.len() {
        for note in diagram.notes.iter().filter(|n| n.before_message == i) {
            let (nx, nw) = note_x_w(note, &x_of);
            let lines = wrap_note_lines(&note.text, 28);
            let nh = NOTE_PAD * 2.0 + lines.len().max(1) as f64 * NOTE_LINE_H;
            notes.push(LaidNote {
                x: nx,
                y,
                w: nw,
                h: nh,
                text: lines.join("\n"),
            });
            y += nh + 12.0;
        }
        if let Some(m) = diagram.messages.get(i) {
            let from_x = *x_of.get(m.from.as_str()).unwrap_or(&MARGIN_X);
            let to_x = *x_of.get(m.to.as_str()).unwrap_or(&MARGIN_X);
            let self_msg = m.from == m.to;
            messages.push(LaidMessage {
                from_x,
                to_x,
                y,
                text: m.text.clone(),
                arrow: m.arrow,
                self_msg,
            });
            y += if self_msg { NOTE_GAP } else { MSG_GAP };
        }
    }

    let footer_top = y + 10.0;
    let height = footer_top + BOX_H + TOP;
    let mut width = MARGIN_X * 2.0 + BOX_W + (n.saturating_sub(1) as f64) * COL_GAP;
    for note in &notes {
        width = width.max(note.x + note.w + MARGIN_X);
    }

    SequenceLayout {
        width,
        height,
        participants,
        messages,
        notes,
        header_bottom,
        footer_top,
    }
}

fn note_x_w(note: &Note, x_of: &HashMap<&str, f64>) -> (f64, f64) {
    let text_w = (note.text.len().min(28) as f64 * NOTE_CHAR_W + NOTE_PAD * 2.0).max(80.0);
    match note.placement {
        NotePlacement::LeftOf => {
            let ax = *x_of.get(note.actors[0].as_str()).unwrap_or(&MARGIN_X);
            (ax - BOX_W / 2.0 - text_w - 8.0, text_w)
        }
        NotePlacement::RightOf => {
            let ax = *x_of.get(note.actors[0].as_str()).unwrap_or(&MARGIN_X);
            (ax + BOX_W / 2.0 + 8.0, text_w)
        }
        NotePlacement::Over => {
            let xs: Vec<f64> = note
                .actors
                .iter()
                .map(|a| *x_of.get(a.as_str()).unwrap_or(&MARGIN_X))
                .collect();
            let min_x = xs.iter().cloned().fold(f64::INFINITY, f64::min);
            let max_x = xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let left = min_x - BOX_W / 4.0;
            let right = max_x + BOX_W / 4.0;
            let w = (right - left).max(text_w);
            (left, w)
        }
    }
}

fn wrap_note_lines(text: &str, width: usize) -> Vec<String> {
    if text.len() <= width {
        return vec![text.to_string()];
    }
    let mut lines = Vec::new();
    let mut rest = text;
    while rest.len() > width {
        let split = rest[..width]
            .rfind(' ')
            .filter(|&i| i > width / 3)
            .unwrap_or(width);
        lines.push(rest[..split].trim().to_string());
        rest = rest[split..].trim_start();
    }
    if !rest.is_empty() {
        lines.push(rest.to_string());
    }
    lines
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub fn render_svg(diagram: &SequenceDiagram, theme: Theme) -> String {
    let laid = layout(diagram);
    let bg = match theme {
        Theme::Dark => "#1a1a2e",
        Theme::Light => "#ffffff",
    };
    let box_fill = match theme {
        Theme::Dark => "#334155",
        Theme::Light => "#f1f5f9",
    };
    let box_stroke = match theme {
        Theme::Dark => "#475569",
        Theme::Light => "#cbd5e1",
    };
    let text_color = match theme {
        Theme::Dark => "#f1f5f9",
        Theme::Light => "#1e293b",
    };
    let line_color = match theme {
        Theme::Dark => "#64748b",
        Theme::Light => "#94a3b8",
    };
    let msg_color = match theme {
        Theme::Dark => "#94a3b8",
        Theme::Light => "#475569",
    };
    let note_fill = match theme {
        Theme::Dark => "#3f3f1f",
        Theme::Light => "#fef9c3",
    };
    let note_stroke = match theme {
        Theme::Dark => "#a3a34a",
        Theme::Light => "#ca8a04",
    };

    let mut svg = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}">"##,
        w = laid.width.ceil() as i32,
        h = laid.height.ceil() as i32,
    );
    svg.push_str(&format!(
        r#"<rect width="100%" height="100%" fill="{bg}"/>"#
    ));
    svg.push_str(&format!(
        r##"<defs>
  <marker id="seq-arrow" markerWidth="8" markerHeight="8" refX="6" refY="3" orient="auto">
    <path d="M0,0 L6,3 L0,6 Z" fill="{c}"/>
  </marker>
</defs>"##,
        c = line_color,
    ));

    for p in &laid.participants {
        svg.push_str(&format!(
            r##"<line x1="{x}" y1="{y1}" x2="{x}" y2="{y2}" stroke="{c}" stroke-dasharray="4,4"/>"##,
            x = p.x,
            y1 = laid.header_bottom,
            y2 = laid.footer_top,
            c = line_color,
        ));
        draw_box(
            &mut svg,
            p.x,
            TOP + BOX_H / 2.0,
            &p.label,
            box_fill,
            box_stroke,
            text_color,
        );
        draw_box(
            &mut svg,
            p.x,
            laid.footer_top + BOX_H / 2.0,
            &p.label,
            box_fill,
            box_stroke,
            text_color,
        );
    }

    for n in &laid.notes {
        svg.push_str(&format!(
            r##"<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="4" fill="{fill}" stroke="{stroke}"/>"##,
            x = n.x,
            y = n.y,
            w = n.w,
            h = n.h,
            fill = note_fill,
            stroke = note_stroke,
        ));
        let mut ty = n.y + NOTE_PAD + NOTE_LINE_H * 0.75;
        for line in n.text.lines() {
            svg.push_str(&format!(
                r##"<text x="{x}" y="{ty}" fill="{c}" font-size="12" font-family="sans-serif">{t}</text>"##,
                x = n.x + NOTE_PAD,
                c = text_color,
                t = esc(line),
            ));
            ty += NOTE_LINE_H;
        }
    }

    for m in &laid.messages {
        let dash = match m.arrow {
            MessageArrow::Solid => "",
            MessageArrow::Dashed => r#" stroke-dasharray="6,4""#,
        };
        if m.self_msg {
            let x = m.from_x;
            let y1 = m.y - 10.0;
            let y2 = m.y + 10.0;
            let loop_x = x + 40.0;
            svg.push_str(&format!(
                r##"<path d="M{x},{y1} L{loop_x},{y1} L{loop_x},{y2} L{x},{y2}" fill="none" stroke="{c}"{dash} marker-end="url(#seq-arrow)"/>"##,
                c = line_color,
                dash = dash,
            ));
            if !m.text.is_empty() {
                svg.push_str(&format!(
                    r##"<text x="{x}" y="{y}" text-anchor="start" fill="{c}" font-size="12" font-family="sans-serif">{t}</text>"##,
                    x = loop_x + 6.0,
                    y = m.y,
                    c = msg_color,
                    t = esc(&m.text),
                ));
            }
        } else {
            svg.push_str(&format!(
                r##"<line x1="{x1}" y1="{y}" x2="{x2}" y2="{y}" stroke="{c}"{dash} marker-end="url(#seq-arrow)"/>"##,
                x1 = m.from_x,
                x2 = m.to_x,
                y = m.y,
                c = line_color,
                dash = dash,
            ));
            if !m.text.is_empty() {
                let mid = (m.from_x + m.to_x) / 2.0;
                svg.push_str(&format!(
                    r##"<text x="{x}" y="{y}" text-anchor="middle" fill="{c}" font-size="12" font-family="sans-serif">{t}</text>"##,
                    x = mid,
                    y = m.y - 8.0,
                    c = msg_color,
                    t = esc(&m.text),
                ));
            }
        }
    }

    svg.push_str("</svg>");
    svg
}

fn draw_box(
    svg: &mut String,
    cx: f64,
    cy: f64,
    label: &str,
    fill: &str,
    stroke: &str,
    text: &str,
) {
    let x = cx - BOX_W / 2.0;
    let y = cy - BOX_H / 2.0;
    svg.push_str(&format!(
        r##"<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="4" fill="{fill}" stroke="{stroke}"/>"##,
        w = BOX_W,
        h = BOX_H,
    ));
    svg.push_str(&format!(
        r##"<text x="{cx}" y="{ty}" text-anchor="middle" dominant-baseline="middle" fill="{text}" font-size="13" font-family="sans-serif">{label}</text>"##,
        ty = cy,
        label = esc(label),
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"sequenceDiagram
    participant Alice
    participant Bob
    Alice->>Bob: Hello Bob
    Bob-->>Alice: Hi Alice
"#;

    #[test]
    fn parse_basic() {
        let d = parse(SAMPLE).unwrap();
        assert_eq!(d.participants.len(), 2);
        assert_eq!(d.messages.len(), 2);
        assert_eq!(d.messages[0].arrow, MessageArrow::Solid);
        assert_eq!(d.messages[1].arrow, MessageArrow::Dashed);
        assert_eq!(d.messages[0].text, "Hello Bob");
    }

    #[test]
    fn implicit_participants() {
        let d = parse(
            "sequenceDiagram\n    A->>B: hi\n",
        )
        .unwrap();
        assert_eq!(d.participants.len(), 2);
        assert_eq!(d.participants[0].id, "A");
        assert_eq!(d.participants[1].id, "B");
    }

    #[test]
    fn participant_alias() {
        let d = parse(
            "sequenceDiagram\n    participant A as Alice\n    A->>B: hi\n",
        )
        .unwrap();
        assert_eq!(d.participants[0].label, "Alice");
    }

    #[test]
    fn roundtrip() {
        let d = parse(SAMPLE).unwrap();
        let out = d.to_mermaid();
        let d2 = parse(&out).unwrap();
        assert_eq!(d2.participants.len(), d.participants.len());
        assert_eq!(d2.messages.len(), d.messages.len());
    }

    #[test]
    fn render_contains_svg() {
        let d = parse(SAMPLE).unwrap();
        let svg = render_svg(&d, Theme::Dark);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("Alice"));
        assert!(svg.contains("Hello Bob"));
        assert!(svg.ends_with("</svg>"));
    }

    #[test]
    fn parse_notes_and_self_message() {
        let src = r#"sequenceDiagram
    participant A
    participant B
    Note left of A: thinking
    A->>B: hi
    Note over A,B: shared
    B->>B: self check
    Note right of B: done
"#;
        let d = parse(src).unwrap();
        assert_eq!(d.notes.len(), 3);
        assert_eq!(d.notes[0].placement, NotePlacement::LeftOf);
        assert_eq!(d.notes[0].before_message, 0);
        assert_eq!(d.notes[1].placement, NotePlacement::Over);
        assert_eq!(d.notes[1].before_message, 1);
        assert_eq!(d.messages[1].from, "B");
        assert_eq!(d.messages[1].to, "B");
        let out = d.to_mermaid();
        let d2 = parse(&out).unwrap();
        assert_eq!(d2.notes.len(), 3);
        assert_eq!(d2.messages.len(), 2);
        let svg = render_svg(&d, Theme::Dark);
        assert!(svg.contains("thinking"));
        assert!(svg.contains("self check"));
    }

    #[test]
    fn is_sequence_detects() {
        assert!(is_sequence(SAMPLE));
        assert!(!is_sequence("graph TD\n  A-->B\n"));
    }
}
