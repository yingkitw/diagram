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
    /// Open fragment count when parsed (0 = top-level). Used for roundtrip at boundaries.
    #[serde(default)]
    pub depth: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FragmentKind {
    Loop,
    Alt,
    Opt,
}

impl FragmentKind {
    pub(crate) fn mermaid_keyword(self) -> &'static str {
        match self {
            Self::Loop => "loop",
            Self::Alt => "alt",
            Self::Opt => "opt",
        }
    }

    pub(crate) fn plantuml_keyword(self) -> &'static str {
        self.mermaid_keyword()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FragmentSection {
    pub label: String,
    /// Inclusive message index where this section begins.
    pub start_message: usize,
}

/// Combined fragment (`loop` / `alt` / `opt` … `end`) spanning a message range.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fragment {
    pub kind: FragmentKind,
    /// First section is the header; further sections are `else` branches (`alt` only).
    pub sections: Vec<FragmentSection>,
    /// Exclusive end message index.
    pub end_message: usize,
}

impl Fragment {
    pub fn start_message(&self) -> usize {
        self.sections
            .first()
            .map(|s| s.start_message)
            .unwrap_or(self.end_message)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SequenceDiagram {
    pub participants: Vec<Participant>,
    pub messages: Vec<Message>,
    #[serde(default)]
    pub notes: Vec<Note>,
    #[serde(default)]
    pub fragments: Vec<Fragment>,
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
    let mut fragments: Vec<Fragment> = Vec::new();
    let mut open: Vec<(FragmentKind, Vec<FragmentSection>)> = Vec::new();

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

        if let Some((kind, label)) = parse_fragment_start(text) {
            open.push((
                kind,
                vec![FragmentSection {
                    label,
                    start_message: messages.len(),
                }],
            ));
            continue;
        }

        if let Some(label) = text.strip_prefix("else").map(str::trim) {
            let Some((kind, sections)) = open.last_mut() else {
                return Err(ParseError {
                    message: "else without open fragment".into(),
                    line: Some(*line_num),
                });
            };
            if *kind != FragmentKind::Alt {
                return Err(ParseError {
                    message: "else is only valid inside alt".into(),
                    line: Some(*line_num),
                });
            }
            sections.push(FragmentSection {
                label: label.to_string(),
                start_message: messages.len(),
            });
            continue;
        }

        if *text == "end" {
            let Some((kind, sections)) = open.pop() else {
                return Err(ParseError {
                    message: "end without open fragment".into(),
                    line: Some(*line_num),
                });
            };
            fragments.push(Fragment {
                kind,
                sections,
                end_message: messages.len(),
            });
            continue;
        }

        if let Some(note) = parse_note(text, messages.len(), open.len()) {
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

    if !open.is_empty() {
        return Err(ParseError {
            message: "unclosed fragment (expected end)".into(),
            line: None,
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
        fragments,
    })
}

fn parse_fragment_start(text: &str) -> Option<(FragmentKind, String)> {
    for (prefix, kind) in [
        ("loop", FragmentKind::Loop),
        ("alt", FragmentKind::Alt),
        ("opt", FragmentKind::Opt),
    ] {
        if text == prefix {
            return Some((kind, String::new()));
        }
        if let Some(rest) = text.strip_prefix(prefix) {
            if rest.starts_with(' ') || rest.is_empty() {
                return Some((kind, rest.trim().to_string()));
            }
        }
    }
    None
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

fn parse_note(text: &str, before_message: usize, depth: usize) -> Option<Note> {
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
        depth,
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
        for event in self.timeline() {
            match event {
                TimelineEvent::FragStart { kind, label } => {
                    if label.is_empty() {
                        out.push_str(&format!("    {}\n", kind.mermaid_keyword()));
                    } else {
                        out.push_str(&format!("    {} {}\n", kind.mermaid_keyword(), label));
                    }
                }
                TimelineEvent::FragElse { label } => {
                    if label.is_empty() {
                        out.push_str("    else\n");
                    } else {
                        out.push_str(&format!("    else {label}\n"));
                    }
                }
                TimelineEvent::FragEnd => out.push_str("    end\n"),
                TimelineEvent::Note(n) => out.push_str(&format!("    {}\n", note_mermaid(n))),
                TimelineEvent::Message(m) => out.push_str(&format!(
                    "    {}{}{}: {}\n",
                    m.from,
                    m.arrow.mermaid_str(),
                    m.to,
                    m.text
                )),
            }
        }
        out
    }

    /// Ordered emit stream for Mermaid/PlantUML/layout (fragments + notes + messages).
    pub(crate) fn timeline(&self) -> Vec<TimelineEvent<'_>> {
        let mut events = Vec::new();
        let mut emitted = vec![false; self.notes.len()];
        let mut stack: Vec<&Fragment> = Vec::new();

        for i in 0..=self.messages.len() {
            // Close non-empty fragments that end here (trailing notes first).
            while stack
                .last()
                .is_some_and(|f| f.end_message == i && f.start_message() < i)
            {
                emit_notes_at(
                    &mut events,
                    &mut emitted,
                    &self.notes,
                    i,
                    stack.len(),
                );
                events.push(TimelineEvent::FragEnd);
                stack.pop();
            }

            // Top-level notes between fragments.
            emit_notes_at(&mut events, &mut emitted, &self.notes, i, stack.len());

            for f in self
                .fragments
                .iter()
                .filter(|f| f.start_message() == i)
            {
                let label = f
                    .sections
                    .first()
                    .map(|s| s.label.as_str())
                    .unwrap_or("");
                events.push(TimelineEvent::FragStart {
                    kind: f.kind,
                    label,
                });
                stack.push(f);
            }

            for f in stack.last().into_iter().flat_map(|f| f.sections.iter().skip(1)) {
                if f.start_message == i {
                    events.push(TimelineEvent::FragElse {
                        label: f.label.as_str(),
                    });
                }
            }

            emit_notes_at(
                &mut events,
                &mut emitted,
                &self.notes,
                i,
                stack.len(),
            );

            if let Some(m) = self.messages.get(i) {
                events.push(TimelineEvent::Message(m));
            }

            // Empty fragments opened and closed at this index.
            while stack.last().is_some_and(|f| f.end_message == i) {
                emit_notes_at(
                    &mut events,
                    &mut emitted,
                    &self.notes,
                    i,
                    stack.len(),
                );
                events.push(TimelineEvent::FragEnd);
                stack.pop();
            }
        }
        events
    }
}

fn emit_notes_at<'a>(
    events: &mut Vec<TimelineEvent<'a>>,
    emitted: &mut [bool],
    notes: &'a [Note],
    before_message: usize,
    depth: usize,
) {
    for (idx, n) in notes.iter().enumerate() {
        if emitted[idx] || n.before_message != before_message || n.depth != depth {
            continue;
        }
        emitted[idx] = true;
        events.push(TimelineEvent::Note(n));
    }
}

pub(crate) enum TimelineEvent<'a> {
    FragStart { kind: FragmentKind, label: &'a str },
    FragElse { label: &'a str },
    FragEnd,
    Note(&'a Note),
    Message(&'a Message),
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

struct LaidFragment {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    kind: FragmentKind,
    label: String,
    else_dividers: Vec<(f64, String)>,
    depth: usize,
}

struct SequenceLayout {
    width: f64,
    height: f64,
    participants: Vec<LaidParticipant>,
    messages: Vec<LaidMessage>,
    notes: Vec<LaidNote>,
    fragments: Vec<LaidFragment>,
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
const FRAG_HEADER: f64 = 22.0;
const FRAG_PAD: f64 = 10.0;

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
    let min_x = participants
        .iter()
        .map(|p| p.x)
        .fold(f64::INFINITY, f64::min);
    let max_x = participants
        .iter()
        .map(|p| p.x)
        .fold(f64::NEG_INFINITY, f64::max);
    let base_frag_x = if participants.is_empty() {
        MARGIN_X
    } else {
        min_x - BOX_W / 2.0 - FRAG_PAD
    };
    let base_frag_w = if participants.is_empty() {
        200.0
    } else {
        (max_x - min_x) + BOX_W + FRAG_PAD * 2.0
    };

    let header_bottom = TOP + BOX_H;
    let mut y = header_bottom + MSG_GAP;
    let mut messages = Vec::new();
    let mut notes = Vec::new();
    let mut fragments = Vec::new();

    struct OpenFrame {
        kind: FragmentKind,
        label: String,
        start_y: f64,
        depth: usize,
        else_dividers: Vec<(f64, String)>,
    }
    let mut open_frames: Vec<OpenFrame> = Vec::new();

    for event in diagram.timeline() {
        match event {
            TimelineEvent::FragStart { kind, label } => {
                let depth = open_frames.len();
                open_frames.push(OpenFrame {
                    kind,
                    label: label.to_string(),
                    start_y: y,
                    depth,
                    else_dividers: Vec::new(),
                });
                y += FRAG_HEADER + 6.0;
            }
            TimelineEvent::FragElse { label } => {
                if let Some(frame) = open_frames.last_mut() {
                    frame.else_dividers.push((y, label.to_string()));
                }
                y += FRAG_HEADER + 4.0;
            }
            TimelineEvent::FragEnd => {
                if let Some(frame) = open_frames.pop() {
                    let inset = frame.depth as f64 * 8.0;
                    let bottom = y + 4.0;
                    fragments.push(LaidFragment {
                        x: base_frag_x + inset,
                        y: frame.start_y,
                        w: (base_frag_w - inset * 2.0).max(80.0),
                        h: (bottom - frame.start_y).max(FRAG_HEADER + 8.0),
                        kind: frame.kind,
                        label: frame.label,
                        else_dividers: frame.else_dividers,
                        depth: frame.depth,
                    });
                    y = bottom + 4.0;
                }
            }
            TimelineEvent::Note(note) => {
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
            TimelineEvent::Message(m) => {
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
    }

    let footer_top = y + 10.0;
    let height = footer_top + BOX_H + TOP;
    let mut width = MARGIN_X * 2.0 + BOX_W + (n.saturating_sub(1) as f64) * COL_GAP;
    for note in &notes {
        width = width.max(note.x + note.w + MARGIN_X);
    }
    for f in &fragments {
        width = width.max(f.x + f.w + MARGIN_X);
    }

    SequenceLayout {
        width,
        height,
        participants,
        messages,
        notes,
        fragments,
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
    let frag_stroke = match theme {
        Theme::Dark => "#7dd3fc",
        Theme::Light => "#0284c7",
    };
    let frag_fill = match theme {
        Theme::Dark => "#0c4a6e55",
        Theme::Light => "#e0f2fe88",
    };
    let frag_label = match theme {
        Theme::Dark => "#bae6fd",
        Theme::Light => "#0369a1",
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

    // Outer fragments first so nested frames paint on top.
    let mut frags: Vec<&LaidFragment> = laid.fragments.iter().collect();
    frags.sort_by_key(|f| f.depth);
    for f in frags {
        svg.push_str(&format!(
            r##"<rect x="{x}" y="{y}" width="{w}" height="{h}" fill="{fill}" stroke="{stroke}" stroke-dasharray="4,3"/>"##,
            x = f.x,
            y = f.y,
            w = f.w,
            h = f.h,
            fill = frag_fill,
            stroke = frag_stroke,
        ));
        let chip = f.kind.mermaid_keyword();
        svg.push_str(&format!(
            r##"<rect x="{x}" y="{y}" width="{w}" height="{h}" fill="{stroke}"/>"##,
            x = f.x,
            y = f.y,
            w = (chip.len() as f64 * 7.5 + 12.0).min(f.w),
            h = FRAG_HEADER,
            stroke = frag_stroke,
        ));
        svg.push_str(&format!(
            r##"<text x="{x}" y="{y}" fill="{bg}" font-size="11" font-weight="600" font-family="sans-serif">{chip}</text>"##,
            x = f.x + 6.0,
            y = f.y + FRAG_HEADER * 0.72,
            bg = bg,
            chip = chip,
        ));
        if !f.label.is_empty() {
            svg.push_str(&format!(
                r##"<text x="{x}" y="{y}" fill="{c}" font-size="12" font-family="sans-serif">{t}</text>"##,
                x = f.x + chip.len() as f64 * 7.5 + 18.0,
                y = f.y + FRAG_HEADER * 0.72,
                c = frag_label,
                t = esc(&f.label),
            ));
        }
        for (ey, elabel) in &f.else_dividers {
            svg.push_str(&format!(
                r##"<line x1="{x1}" y1="{y}" x2="{x2}" y2="{y}" stroke="{c}" stroke-dasharray="4,3"/>"##,
                x1 = f.x,
                x2 = f.x + f.w,
                y = ey,
                c = frag_stroke,
            ));
            svg.push_str(&format!(
                r##"<text x="{x}" y="{y}" fill="{c}" font-size="11" font-family="sans-serif">else {t}</text>"##,
                x = f.x + 8.0,
                y = ey + FRAG_HEADER * 0.65,
                c = frag_label,
                t = esc(elabel),
            ));
        }
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

    #[test]
    fn parse_loop_alt_opt() {
        let src = r#"sequenceDiagram
    participant A
    participant B
    loop Heartbeat
        A->>B: ping
        B-->>A: pong
    end
    alt success
        A->>B: ok
    else failure
        A->>B: retry
    end
    opt Extra
        A->>B: bonus
    end
"#;
        let d = parse(src).unwrap();
        assert_eq!(d.messages.len(), 5);
        assert_eq!(d.fragments.len(), 3);
        assert_eq!(d.fragments[0].kind, FragmentKind::Loop);
        assert_eq!(d.fragments[0].sections[0].label, "Heartbeat");
        assert_eq!(d.fragments[0].start_message(), 0);
        assert_eq!(d.fragments[0].end_message, 2);
        assert_eq!(d.fragments[1].kind, FragmentKind::Alt);
        assert_eq!(d.fragments[1].sections.len(), 2);
        assert_eq!(d.fragments[1].sections[1].label, "failure");
        assert_eq!(d.fragments[1].sections[1].start_message, 3);
        assert_eq!(d.fragments[2].kind, FragmentKind::Opt);
        let out = d.to_mermaid();
        assert!(out.contains("loop Heartbeat"));
        assert!(out.contains("else failure"));
        assert!(out.contains("opt Extra"));
        let d2 = parse(&out).unwrap();
        assert_eq!(d2.fragments.len(), 3);
        assert_eq!(d2.messages.len(), 5);
        let svg = render_svg(&d, Theme::Dark);
        assert!(svg.contains("loop"));
        assert!(svg.contains("Heartbeat"));
        assert!(svg.contains("else failure"));
    }
}
