//! Mermaid state diagram parse, layout, and SVG render (MVP).

use crate::renderer::Theme;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt;

pub const START_END: &str = "[*]";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum StateNodeKind {
    #[default]
    Normal,
    StartEnd,
    Choice,
    Fork,
    Join,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateNode {
    pub id: String,
    pub label: String,
    pub kind: StateNodeKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transition {
    pub from: String,
    pub to: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateDiagram {
    pub states: Vec<StateNode>,
    pub transitions: Vec<Transition>,
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

pub fn is_state(source: &str) -> bool {
    source
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty() && !l.starts_with("%%"))
        .is_some_and(|l| l.starts_with("stateDiagram"))
}

pub fn parse(source: &str) -> Result<StateDiagram, ParseError> {
    let lines: Vec<(usize, &str)> = source
        .lines()
        .enumerate()
        .map(|(i, l)| (i + 1, l.trim()))
        .filter(|(_, l)| !l.is_empty() && !l.starts_with("%%"))
        .collect();

    if lines.is_empty()
        || !lines[0].1.starts_with("stateDiagram-v2")
            && !lines[0].1.starts_with("stateDiagram")
    {
        return Err(ParseError {
            message: "expected stateDiagram-v2 header".into(),
            line: lines.first().map(|(n, _)| *n),
        });
    }

    let mut labels: HashMap<String, String> = HashMap::new();
    let mut kinds: HashMap<String, StateNodeKind> = HashMap::new();
    let mut transitions: Vec<Transition> = Vec::new();
    let mut i = 1;
    while i < lines.len() {
        let (line_num, text) = lines[i];
        if text.starts_with("state ") {
            if text.contains('{') {
                i = skip_block(&lines, i)?;
                i += 1;
                continue;
            }
            parse_state_decl(text, &mut labels, &mut kinds).map_err(|m| ParseError {
                message: m,
                line: Some(line_num),
            })?;
            i += 1;
            continue;
        }
        if let Some((from, to, label)) = parse_transition_line(text) {
            ensure_start_end(&mut kinds, &from);
            ensure_start_end(&mut kinds, &to);
            transitions.push(Transition { from, to, label });
            i += 1;
            continue;
        }
        return Err(ParseError {
            message: format!("unrecognized state diagram line: {text}"),
            line: Some(line_num),
        });
    }

    let mut ids: HashSet<String> = labels.keys().cloned().collect();
    for t in &transitions {
        ids.insert(t.from.clone());
        ids.insert(t.to.clone());
    }

    let states: Vec<StateNode> = ids
        .into_iter()
        .map(|id| {
            let kind = kinds.get(&id).copied().unwrap_or_else(|| {
                if id == START_END {
                    StateNodeKind::StartEnd
                } else {
                    StateNodeKind::Normal
                }
            });
            let label = labels.get(&id).cloned().unwrap_or_else(|| {
                if id == START_END {
                    START_END.to_string()
                } else {
                    id.clone()
                }
            });
            StateNode { id, label, kind }
        })
        .collect();

    Ok(StateDiagram { states, transitions })
}

fn skip_block(lines: &[(usize, &str)], start: usize) -> Result<usize, ParseError> {
    let mut depth = 0usize;
    for (idx, (_, text)) in lines.iter().enumerate().skip(start) {
        for ch in text.chars() {
            if ch == '{' {
                depth += 1;
            } else if ch == '}' {
                if depth == 0 {
                    return Ok(idx);
                }
                depth = depth.saturating_sub(1);
            }
        }
        if depth == 0 && text.contains('}') {
            return Ok(idx);
        }
    }
    Err(ParseError {
        message: "unclosed composite state block".into(),
        line: lines.get(start).map(|(n, _)| *n),
    })
}

fn parse_state_decl(
    text: &str,
    labels: &mut HashMap<String, String>,
    kinds: &mut HashMap<String, StateNodeKind>,
) -> Result<(), String> {
    let rest = text.strip_prefix("state ").ok_or("expected state declaration")?;
    if let Some((quoted, id)) = parse_state_as(rest) {
        labels.insert(id.clone(), unquote_label(quoted));
        return Ok(());
    }
    if let Some((id, kind)) = parse_state_stereotype(rest) {
        kinds.insert(id.clone(), kind);
        labels.entry(id.clone()).or_insert(id);
        return Ok(());
    }
    Err(format!("invalid state declaration: {text}"))
}

fn parse_state_as(rest: &str) -> Option<(&str, String)> {
    let rest = rest.trim();
    let (quoted, after) = read_quoted_or_word(rest)?;
    let after = after.trim();
    let id = after.strip_prefix("as ")?.trim();
    if id.is_empty() {
        return None;
    }
    Some((quoted, id.to_string()))
}

fn parse_state_stereotype(rest: &str) -> Option<(String, StateNodeKind)> {
    let rest = rest.trim();
    let open = rest.rfind("<<")?;
    let close = rest[open..].find(">>")?;
    let id = rest[..open].trim();
    if id.is_empty() {
        return None;
    }
    let stereotype = rest[open + 2..open + close].trim().to_lowercase();
    let kind = match stereotype.as_str() {
        "choice" => StateNodeKind::Choice,
        "fork" => StateNodeKind::Fork,
        "join" => StateNodeKind::Join,
        _ => StateNodeKind::Normal,
    };
    Some((id.to_string(), kind))
}

fn read_quoted_or_word(s: &str) -> Option<(&str, &str)> {
    let s = s.trim_start();
    if let Some(rest) = s.strip_prefix('"') {
        let end = rest.find('"')?;
        let quoted = &rest[..end];
        Some((quoted, rest[end + 1..].trim_start()))
    } else {
        let end = s.find(char::is_whitespace).unwrap_or(s.len());
        Some((&s[..end], &s[end..]))
    }
}

fn unquote_label(s: &str) -> String {
    s.to_string()
}

fn parse_transition_line(line: &str) -> Option<(String, String, String)> {
    let (left, rest) = line.split_once("-->")?;
    let left = left.trim().to_string();
    let rest = rest.trim();
    let (right, label) = if let Some((r, l)) = rest.split_once(':') {
        (r.trim().to_string(), l.trim().to_string())
    } else {
        (rest.to_string(), String::new())
    };
    if left.is_empty() || right.is_empty() {
        return None;
    }
    Some((left, right, label))
}

fn ensure_start_end(kinds: &mut HashMap<String, StateNodeKind>, id: &str) {
    if id == START_END {
        kinds.insert(START_END.to_string(), StateNodeKind::StartEnd);
    }
}

impl StateDiagram {
    pub fn to_mermaid(&self) -> String {
        let mut out = String::from("stateDiagram-v2\n");
        for s in &self.states {
            if s.id == START_END {
                continue;
            }
            match s.kind {
                StateNodeKind::Normal if s.label != s.id => {
                    out.push_str(&format!(
                        "    state \"{}\" as {}\n",
                        esc_quote(&s.label),
                        s.id
                    ));
                }
                StateNodeKind::Choice => {
                    out.push_str(&format!("    state {} <<choice>>\n", s.id));
                }
                StateNodeKind::Fork => {
                    out.push_str(&format!("    state {} <<fork>>\n", s.id));
                }
                StateNodeKind::Join => {
                    out.push_str(&format!("    state {} <<join>>\n", s.id));
                }
                _ => {}
            }
        }
        for t in &self.transitions {
            if t.label.is_empty() {
                out.push_str(&format!("    {} --> {}\n", t.from, t.to));
            } else {
                out.push_str(&format!("    {} --> {}: {}\n", t.from, t.to, t.label));
            }
        }
        out
    }
}

fn esc_quote(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

struct LayoutNode {
    id: String,
    label: String,
    kind: StateNodeKind,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

struct LayoutEdge {
    from_x: f64,
    from_y: f64,
    to_x: f64,
    to_y: f64,
    label: String,
}

struct Layout {
    nodes: Vec<LayoutNode>,
    edges: Vec<LayoutEdge>,
    width: f64,
    height: f64,
}

const NODE_W: f64 = 110.0;
const NODE_H: f64 = 44.0;
const LAYER_GAP: f64 = 90.0;
const NODE_GAP: f64 = 40.0;
const PAD: f64 = 40.0;

fn layout(d: &StateDiagram) -> Layout {
    let positions = layered_positions(d);
    let mut nodes = Vec::new();
    let mut max_x = PAD;
    let mut max_y = PAD;

    for s in &d.states {
        let (x, y) = positions.get(&s.id).copied().unwrap_or((PAD, PAD));
        let (w, h) = node_size(s.kind);
        max_x = max_x.max(x + w / 2.0 + PAD);
        max_y = max_y.max(y + h / 2.0 + PAD);
        nodes.push(LayoutNode {
            id: s.id.clone(),
            label: s.label.clone(),
            kind: s.kind,
            x,
            y,
            w,
            h,
        });
    }

    let node_map: HashMap<&str, &LayoutNode> = nodes.iter().map(|n| (n.id.as_str(), n)).collect();
    let edges: Vec<LayoutEdge> = d
        .transitions
        .iter()
        .filter_map(|t| {
            let from = node_map.get(t.from.as_str())?;
            let to = node_map.get(t.to.as_str())?;
            Some(LayoutEdge {
                from_x: from.x,
                from_y: from.y + from.h / 2.0,
                to_x: to.x,
                to_y: to.y - to.h / 2.0,
                label: t.label.clone(),
            })
        })
        .collect();

    Layout {
        nodes,
        edges,
        width: max_x,
        height: max_y + PAD,
    }
}

fn node_size(kind: StateNodeKind) -> (f64, f64) {
    match kind {
        StateNodeKind::StartEnd => (24.0, 24.0),
        StateNodeKind::Choice => (56.0, 56.0),
        StateNodeKind::Normal | StateNodeKind::Fork | StateNodeKind::Join => (NODE_W, NODE_H),
    }
}

fn layered_positions(d: &StateDiagram) -> HashMap<String, (f64, f64)> {
    let ids: HashSet<&str> = d.states.iter().map(|s| s.id.as_str()).collect();
    let mut incoming: HashMap<&str, usize> = HashMap::new();
    let mut outgoing: HashMap<&str, Vec<&str>> = HashMap::new();
    for id in &ids {
        incoming.insert(id, 0);
        outgoing.insert(id, Vec::new());
    }
    for t in &d.transitions {
        if let Some(c) = incoming.get_mut(t.to.as_str()) {
            *c += 1;
        }
        if let Some(v) = outgoing.get_mut(t.from.as_str()) {
            v.push(t.to.as_str());
        }
    }

    let mut starts: Vec<&str> = ids
        .iter()
        .copied()
        .filter(|id| incoming.get(id).copied().unwrap_or(0) == 0)
        .collect();
    if starts.is_empty() {
        if ids.contains(START_END) {
            starts.push(START_END);
        } else if let Some(s) = d.states.first() {
            starts.push(s.id.as_str());
        }
    }

    let mut layer: HashMap<String, usize> = HashMap::new();
    let mut q: VecDeque<&str> = starts.into_iter().collect();
    let mut seen: HashSet<&str> = HashSet::new();
    while let Some(id) = q.pop_front() {
        if !seen.insert(id) {
            continue;
        }
        let l = *layer.get(id).unwrap_or(&0);
        if let Some(nexts) = outgoing.get(id) {
            for &to in nexts {
                let entry = layer.entry(to.to_string()).or_insert(l + 1);
                if l + 1 > *entry {
                    *entry = l + 1;
                }
                q.push_back(to);
            }
        }
    }

    for s in &d.states {
        layer.entry(s.id.clone()).or_insert(0);
    }

    let max_layer = layer.values().copied().max().unwrap_or(0);
    let mut by_layer: Vec<Vec<String>> = vec![Vec::new(); max_layer + 1];
    for s in &d.states {
        let l = layer.get(&s.id).copied().unwrap_or(0);
        by_layer[l].push(s.id.clone());
    }
    for layer_nodes in &mut by_layer {
        layer_nodes.sort();
    }

    let mut positions = HashMap::new();
    for (l, layer_nodes) in by_layer.iter().enumerate() {
        let count = layer_nodes.len().max(1) as f64;
        let span = (count - 1.0) * (NODE_W + NODE_GAP);
        let x0 = PAD + NODE_W / 2.0 + if count == 1.0 { 80.0 } else { 0.0 };
        for (i, id) in layer_nodes.iter().enumerate() {
            let x = x0 + i as f64 * (NODE_W + NODE_GAP) - span / 2.0;
            let y = PAD + NODE_H / 2.0 + l as f64 * LAYER_GAP;
            positions.insert(id.clone(), (x, y));
        }
    }
    positions
}

pub fn render_svg(d: &StateDiagram, theme: Theme) -> String {
    let laid = layout(d);
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
    let label_color = match theme {
        Theme::Dark => "#94a3b8",
        Theme::Light => "#475569",
    };

    let mut svg = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}">"##,
        w = laid.width.ceil() as i32,
        h = laid.height.ceil() as i32,
    );
    svg.push_str(&format!(r#"<rect width="100%" height="100%" fill="{bg}"/>"#));
    svg.push_str(&format!(
        r##"<defs><marker id="state-arrow" markerWidth="8" markerHeight="8" refX="6" refY="3" orient="auto"><path d="M0,0 L6,3 L0,6 Z" fill="{c}"/></marker></defs>"##,
        c = line_color,
    ));

    for e in &laid.edges {
        svg.push_str(&format!(
            r##"<line x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}" stroke="{c}" marker-end="url(#state-arrow)"/>"##,
            x1 = e.from_x,
            y1 = e.from_y,
            x2 = e.to_x,
            y2 = e.to_y,
            c = line_color,
        ));
        if !e.label.is_empty() {
            let mx = (e.from_x + e.to_x) / 2.0;
            let my = (e.from_y + e.to_y) / 2.0;
            svg.push_str(&format!(
                r##"<text x="{mx}" y="{my}" text-anchor="middle" fill="{label_color}" font-size="12" font-family="sans-serif">{t}</text>"##,
                t = esc_xml(&e.label),
            ));
        }
    }

    for n in &laid.nodes {
        match n.kind {
            StateNodeKind::StartEnd => {
                svg.push_str(&format!(
                    r##"<circle cx="{x}" cy="{y}" r="10" fill="{box_fill}" stroke="{box_stroke}" stroke-width="2"/>"##,
                    x = n.x,
                    y = n.y,
                    box_fill = box_fill,
                    box_stroke = box_stroke,
                ));
            }
            StateNodeKind::Choice => {
                let r = n.w / 2.0;
                svg.push_str(&format!(
                    r##"<polygon points="{x},{y1} {x1},{y} {x},{y2} {x2},{y}" fill="{box_fill}" stroke="{box_stroke}"/>"##,
                    x = n.x,
                    y = n.y,
                    x1 = n.x + r,
                    y1 = n.y - r,
                    x2 = n.x - r,
                    y2 = n.y + r,
                    box_fill = box_fill,
                    box_stroke = box_stroke,
                ));
                if n.label != n.id && n.label != START_END {
                    svg.push_str(&format!(
                        r##"<text x="{x}" y="{y}" text-anchor="middle" dominant-baseline="middle" fill="{text_color}" font-size="11" font-family="sans-serif">{label}</text>"##,
                        x = n.x,
                        y = n.y,
                        text_color = text_color,
                        label = esc_xml(&n.label),
                    ));
                }
            }
            _ => {
                let x = n.x - n.w / 2.0;
                let y = n.y - n.h / 2.0;
                svg.push_str(&format!(
                    r##"<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="14" fill="{box_fill}" stroke="{box_stroke}"/>"##,
                    w = n.w,
                    h = n.h,
                    box_fill = box_fill,
                    box_stroke = box_stroke,
                ));
                svg.push_str(&format!(
                    r##"<text x="{cx}" y="{cy}" text-anchor="middle" dominant-baseline="middle" fill="{text_color}" font-size="13" font-family="sans-serif">{label}</text>"##,
                    cx = n.x,
                    cy = n.y,
                    text_color = text_color,
                    label = esc_xml(&n.label),
                ));
            }
        }
    }

    svg.push_str("</svg>");
    svg
}

fn esc_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"stateDiagram-v2
    [*] --> Still
    Still --> [*]
    Still --> Moving
    Moving --> Still
    Moving --> Crash
    Crash --> [*]
"#;

    #[test]
    fn is_state_detects() {
        assert!(is_state(SAMPLE));
        assert!(!is_state("graph TD\n  A-->B\n"));
    }

    #[test]
    fn parse_basic() {
        let d = parse(SAMPLE).unwrap();
        assert!(d.states.iter().any(|s| s.id == "Still"));
        assert_eq!(d.transitions.len(), 6);
        assert!(d.transitions.iter().any(|t| t.from == "Still" && t.to == "Moving"));
    }

    #[test]
    fn parse_state_alias_and_choice() {
        let src = r#"stateDiagram-v2
    state "Waiting for user" as Wait
    state check <<choice>>
    [*] --> Wait
    Wait --> check
    check --> Done: ok
"#;
        let d = parse(src).unwrap();
        let wait = d.states.iter().find(|s| s.id == "Wait").unwrap();
        assert_eq!(wait.label, "Waiting for user");
        let check = d.states.iter().find(|s| s.id == "check").unwrap();
        assert_eq!(check.kind, StateNodeKind::Choice);
    }

    #[test]
    fn roundtrip() {
        let d = parse(SAMPLE).unwrap();
        let out = d.to_mermaid();
        let d2 = parse(&out).unwrap();
        assert_eq!(d2.transitions.len(), d.transitions.len());
    }

    #[test]
    fn render_contains_svg() {
        let d = parse(SAMPLE).unwrap();
        let svg = render_svg(&d, Theme::Dark);
        assert!(svg.contains("<svg"));
        assert!(svg.contains("Still"));
    }
}
