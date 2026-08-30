//! Mermaid class diagram parse, layout, and SVG render (MVP).

use crate::renderer::Theme;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelationKind {
    /// `A <|-- B` — B inherits from A
    Inheritance,
    /// `A *-- B` — composition
    Composition,
    /// `A o-- B` — aggregation
    Aggregation,
    /// `A --> B` — association
    Association,
    /// `A -- B` — solid link
    Link,
    /// `A ..> B` — dependency
    Dependency,
    /// `A ..|> B` — realization
    Realization,
}

impl RelationKind {
    pub(crate) fn mermaid_str(self) -> &'static str {
        match self {
            Self::Inheritance => "<|--",
            Self::Composition => "*--",
            Self::Aggregation => "o--",
            Self::Association => "-->",
            Self::Link => "--",
            Self::Dependency => "..>",
            Self::Realization => "..|>",
        }
    }

    fn dashed(self) -> bool {
        matches!(self, Self::Dependency | Self::Realization)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassMember {
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Class {
    pub id: String,
    pub members: Vec<ClassMember>,
    /// Mermaid/PlantUML stereotype (e.g. `interface`, `enumeration`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stereotype: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relation {
    pub from: String,
    pub to: String,
    pub kind: RelationKind,
    pub label: String,
    /// Cardinality on the `from` side (e.g. `"1"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_card: Option<String>,
    /// Cardinality on the `to` side (e.g. `"*"` / `"1..*"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_card: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassNote {
    /// Class id this note is attached to.
    pub target: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassDiagram {
    pub classes: Vec<Class>,
    pub relations: Vec<Relation>,
    #[serde(default)]
    pub notes: Vec<ClassNote>,
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

pub fn is_class(source: &str) -> bool {
    source
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty() && !l.starts_with("%%"))
        .is_some_and(|l| l.starts_with("classDiagram"))
}

pub fn parse(source: &str) -> Result<ClassDiagram, ParseError> {
    let raw: Vec<(usize, String)> = source
        .lines()
        .enumerate()
        .map(|(i, l)| (i + 1, l.trim().to_string()))
        .filter(|(_, l)| !l.is_empty() && !l.starts_with("%%"))
        .collect();

    if raw.is_empty() || !raw[0].1.starts_with("classDiagram") {
        return Err(ParseError {
            message: "expected classDiagram header".into(),
            line: raw.first().map(|(n, _)| *n),
        });
    }

    let mut classes: HashMap<String, Class> = HashMap::new();
    let mut order: Vec<String> = Vec::new();
    let mut relations: Vec<Relation> = Vec::new();
    let mut notes: Vec<ClassNote> = Vec::new();

    let mut i = 1;
    while i < raw.len() {
        let (line_num, text) = &raw[i];

        if let Some(rest) = text.strip_prefix("class ") {
            let rest = rest.trim();
            if let Some(brace) = rest.find('{') {
                let (id, stereotype) = split_id_stereotype(rest[..brace].trim());
                if id.is_empty() {
                    return Err(ParseError {
                        message: "empty class name".into(),
                        line: Some(*line_num),
                    });
                }
                ensure_class(&mut classes, &mut order, &id);
                set_stereotype(&mut classes, &id, stereotype);
                let after = rest[brace + 1..].trim();
                if after.ends_with('}') {
                    let inner = after.trim_end_matches('}').trim();
                    if !inner.is_empty() {
                        for part in inner.split(';') {
                            let m = part.trim();
                            if m.is_empty() {
                                continue;
                            }
                            push_member_or_stereotype(&mut classes, &id, m);
                        }
                    }
                    i += 1;
                } else {
                    // Multi-line body until closing `}`
                    i += 1;
                    while i < raw.len() {
                        let (_, body) = &raw[i];
                        if body == "}" {
                            i += 1;
                            break;
                        }
                        if let Some(stripped) = body.strip_suffix('}') {
                            let m = stripped.trim();
                            if !m.is_empty() {
                                push_member_or_stereotype(&mut classes, &id, m);
                            }
                            i += 1;
                            break;
                        }
                        push_member_or_stereotype(&mut classes, &id, body);
                        i += 1;
                    }
                }
                continue;
            } else {
                let (id, stereotype) = split_id_stereotype(rest);
                ensure_class(&mut classes, &mut order, &id);
                set_stereotype(&mut classes, &id, stereotype);
                i += 1;
                continue;
            }
        }

        if let Some(note) = parse_class_note(text) {
            ensure_class(&mut classes, &mut order, &note.target);
            notes.push(note);
            i += 1;
            continue;
        }

        // `Animal : +String name` (not a relation label)
        if let Some((left, right)) = text.split_once(" : ") {
            let id = left.trim();
            let member = right.trim();
            if !id.is_empty()
                && !member.is_empty()
                && id
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == '~')
            {
                ensure_class(&mut classes, &mut order, id);
                classes.get_mut(id).unwrap().members.push(ClassMember {
                    text: member.to_string(),
                });
                i += 1;
                continue;
            }
        }

        if let Some(rel) = parse_relation_line(text) {
            ensure_class(&mut classes, &mut order, &rel.from);
            ensure_class(&mut classes, &mut order, &rel.to);
            relations.push(rel);
            i += 1;
            continue;
        }

        return Err(ParseError {
            message: format!("unrecognized class diagram line: {text}"),
            line: Some(*line_num),
        });
    }

    let classes = order
        .into_iter()
        .filter_map(|id| classes.remove(&id))
        .collect();

    Ok(ClassDiagram {
        classes,
        relations,
        notes,
    })
}

/// `note for Animal "text"`
fn parse_class_note(text: &str) -> Option<ClassNote> {
    let rest = text.strip_prefix("note for ")?.trim_start();
    let (target, after) = split_note_target(rest)?;
    let text = parse_quoted_note_text(after)?;
    if target.is_empty() || text.is_empty() {
        return None;
    }
    Some(ClassNote { target, text })
}

fn split_note_target(s: &str) -> Option<(String, &str)> {
    let s = s.trim_start();
    if s.starts_with('"') {
        return None;
    }
    // Target ends before the opening quote of the note body.
    let quote = s.find('"')?;
    let target = s[..quote].trim().to_string();
    Some((target, s[quote..].trim_start()))
}

fn parse_quoted_note_text(s: &str) -> Option<String> {
    let s = s.trim();
    if !s.starts_with('"') {
        return None;
    }
    let mut out = String::new();
    let mut chars = s[1..].chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                if let Some(n) = chars.next() {
                    out.push(n);
                }
            }
            '"' => return Some(out),
            other => out.push(other),
        }
    }
    None
}

pub(crate) fn ensure_class(classes: &mut HashMap<String, Class>, order: &mut Vec<String>, id: &str) {
    if !classes.contains_key(id) {
        order.push(id.to_string());
        classes.insert(
            id.to_string(),
            Class {
                id: id.to_string(),
                members: Vec::new(),
                stereotype: None,
            },
        );
    }
}

pub(crate) fn set_stereotype(
    classes: &mut HashMap<String, Class>,
    id: &str,
    stereotype: Option<String>,
) {
    let Some(stereo) = stereotype else {
        return;
    };
    if let Some(c) = classes.get_mut(id)
        && c.stereotype.is_none()
    {
        c.stereotype = Some(stereo);
    }
}

fn push_member_or_stereotype(classes: &mut HashMap<String, Class>, id: &str, text: &str) {
    let t = text.trim();
    if let Some(stereo) = stereotype_only(t) {
        set_stereotype(classes, id, Some(stereo));
        return;
    }
    classes.get_mut(id).unwrap().members.push(ClassMember {
        text: t.to_string(),
    });
}

/// `Foo <<interface>>` → (`Foo`, Some(`interface`))
pub(crate) fn split_id_stereotype(s: &str) -> (String, Option<String>) {
    let s = s.trim();
    let Some(open) = s.find("<<") else {
        return (s.to_string(), None);
    };
    let after = &s[open + 2..];
    let Some(close) = after.find(">>") else {
        return (s.to_string(), None);
    };
    let id = s[..open].trim().to_string();
    let stereo = after[..close].trim().to_string();
    if id.is_empty() || stereo.is_empty() {
        return (s.to_string(), None);
    }
    (id, Some(stereo))
}

fn stereotype_only(s: &str) -> Option<String> {
    let s = s.trim();
    if !s.starts_with("<<") || !s.ends_with(">>") || s.len() < 5 {
        return None;
    }
    let inner = s[2..s.len() - 2].trim();
    if inner.is_empty() {
        None
    } else {
        Some(inner.to_string())
    }
}

/// Render Mermaid `~T~` generics as `‹T›` (supports nesting like `List~List~int~~`).
pub(crate) fn display_generics(s: &str) -> String {
    replace_tilde_pairs(s, "‹", "›")
}

/// Mermaid `~T~` → PlantUML/angle `\<T\>`.
pub(crate) fn generics_to_angle(s: &str) -> String {
    replace_tilde_pairs(s, "<", ">")
}

/// PlantUML `\<T\>` → Mermaid `~T~` (does not touch `<<stereotypes>>`).
pub(crate) fn angle_generics_to_tilde(s: &str) -> String {
    let mut chars: Vec<char> = s.chars().collect();
    loop {
        // Prefer innermost (rightmost simple `\<...\>` pair). Skip `<<stereotypes>>`.
        let mut found: Option<(usize, usize)> = None;
        let mut i = 0;
        while i < chars.len() {
            if chars[i] == '<' {
                if i + 1 < chars.len() && chars[i + 1] == '<' {
                    // Skip stereotype `<<...>>`
                    if let Some(end) = chars[i + 2..]
                        .windows(2)
                        .position(|w| w == ['>', '>'])
                    {
                        i += 2 + end + 2;
                        continue;
                    }
                    i += 2;
                    continue;
                }
                if let Some(j) = (i + 1..chars.len()).find(|&j| chars[j] == '>') {
                    let inner = &chars[i + 1..j];
                    if !inner.is_empty() && !inner.contains(&'<') && !inner.contains(&'>') {
                        found = Some((i, j));
                    }
                }
            }
            i += 1;
        }
        let Some((i, j)) = found else {
            break;
        };
        let inner: String = chars[i + 1..j].iter().collect();
        let replacement: Vec<char> = format!("~{inner}~").chars().collect();
        chars.splice(i..=j, replacement);
    }
    chars.into_iter().collect()
}

fn replace_tilde_pairs(s: &str, open: &str, close: &str) -> String {
    let mut chars: Vec<char> = s.chars().collect();
    loop {
        // Prefer innermost non-empty pair (rightmost `~…~` with no nested `~`)
        // so `List~List~int~~` → `List‹List‹int››`.
        let mut found: Option<(usize, usize)> = None;
        let mut i = 0;
        while i < chars.len() {
            if chars[i] == '~'
                && let Some(j) = (i + 1..chars.len()).find(|&j| chars[j] == '~')
            {
                if j > i + 1 && !chars[i + 1..j].contains(&'~') {
                    found = Some((i, j));
                }
                i = j;
                continue;
            }
            i += 1;
        }
        let Some((i, j)) = found else {
            break;
        };
        let inner: String = chars[i + 1..j].iter().collect();
        let replacement: Vec<char> = format!("{open}{inner}{close}").chars().collect();
        chars.splice(i..=j, replacement);
    }
    chars.into_iter().collect()
}

pub(crate) fn parse_relation_line(text: &str) -> Option<Relation> {
    // Longest tokens first.
    let kinds = [
        ("..|>", RelationKind::Realization),
        ("<|--", RelationKind::Inheritance),
        ("*--", RelationKind::Composition),
        ("o--", RelationKind::Aggregation),
        ("-->", RelationKind::Association),
        ("..>", RelationKind::Dependency),
        ("--", RelationKind::Link),
    ];
    for (token, kind) in kinds {
        if let Some((left, right)) = text.split_once(token) {
            let (to_raw, label) = match right.split_once(':') {
                Some((to, lab)) => (to.trim(), lab.trim().to_string()),
                None => (right.trim(), String::new()),
            };
            let (from, from_card) = parse_left_endpoint(left.trim());
            let (to, to_card) = parse_right_endpoint(to_raw);
            if from.is_empty() || to.is_empty() {
                return None;
            }
            return Some(Relation {
                from,
                to,
                kind,
                label,
                from_card,
                to_card,
            });
        }
    }
    None
}

/// `Customer "1"` → (`Customer`, Some(`1`))
pub(crate) fn parse_left_endpoint(s: &str) -> (String, Option<String>) {
    let s = s.trim();
    if let Some((id, card)) = split_trailing_quoted(s) {
        return (id, Some(card));
    }
    (s.to_string(), None)
}

/// `"*" Ticket` or `Ticket` → id + optional card
pub(crate) fn parse_right_endpoint(s: &str) -> (String, Option<String>) {
    let s = s.trim();
    if let Some((card, id)) = split_leading_quoted(s) {
        return (id, Some(card));
    }
    (s.to_string(), None)
}

fn split_trailing_quoted(s: &str) -> Option<(String, String)> {
    let s = s.trim();
    if !s.ends_with('"') {
        return None;
    }
    let inner_end = s.len() - 1;
    let start = s[..inner_end].rfind('"')?;
    let id = s[..start].trim();
    let card = s[start + 1..inner_end].trim();
    if id.is_empty() {
        return None;
    }
    Some((id.to_string(), card.to_string()))
}

fn split_leading_quoted(s: &str) -> Option<(String, String)> {
    let s = s.trim();
    if !s.starts_with('"') {
        return None;
    }
    let end = s[1..].find('"')? + 1;
    let card = s[1..end].trim().to_string();
    let id = s[end + 1..].trim();
    if id.is_empty() {
        return None;
    }
    Some((card, id.to_string()))
}

fn format_card(card: &Option<String>) -> String {
    match card {
        Some(c) => format!(" \"{c}\""),
        None => String::new(),
    }
}

fn format_right_card(card: &Option<String>) -> String {
    match card {
        Some(c) => format!("\"{c}\" "),
        None => String::new(),
    }
}

impl ClassDiagram {
    pub fn to_mermaid(&self) -> String {
        let mut out = String::from("classDiagram\n");
        for c in &self.classes {
            let stereo = c
                .stereotype
                .as_ref()
                .map(|s| format!(" <<{s}>>"))
                .unwrap_or_default();
            if c.members.is_empty() {
                out.push_str(&format!("    class {}{stereo}\n", c.id));
            } else {
                out.push_str(&format!("    class {}{stereo} {{\n", c.id));
                for m in &c.members {
                    out.push_str(&format!("        {}\n", m.text));
                }
                out.push_str("    }\n");
            }
        }
        for r in &self.relations {
            let left = format!("{}{}", r.from, format_card(&r.from_card));
            let right = format!("{}{}", format_right_card(&r.to_card), r.to);
            if r.label.is_empty() {
                out.push_str(&format!(
                    "    {} {} {}\n",
                    left,
                    r.kind.mermaid_str(),
                    right
                ));
            } else {
                out.push_str(&format!(
                    "    {} {} {} : {}\n",
                    left,
                    r.kind.mermaid_str(),
                    right,
                    r.label
                ));
            }
        }
        for n in &self.notes {
            out.push_str(&format!(
                "    note for {} \"{}\"\n",
                n.target,
                n.text.replace('\\', "\\\\").replace('"', "\\\"")
            ));
        }
        out
    }
}

struct LaidClass {
    id: String,
    stereotype: Option<String>,
    members: Vec<String>,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

struct LaidRelation {
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    kind: RelationKind,
    label: String,
    from_card: Option<String>,
    to_card: Option<String>,
}

struct LaidNote {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    text: String,
    /// Attachment point on the target class (right-center).
    ax: f64,
    ay: f64,
}

struct ClassLayout {
    width: f64,
    height: f64,
    classes: Vec<LaidClass>,
    relations: Vec<LaidRelation>,
    notes: Vec<LaidNote>,
}

const BOX_MIN_W: f64 = 120.0;
const CHAR_W: f64 = 7.5;
const LINE_H: f64 = 18.0;
const PAD: f64 = 10.0;
const H_GAP: f64 = 80.0;
const V_GAP: f64 = 70.0;
const MARGIN: f64 = 40.0;

fn class_size(c: &Class) -> (f64, f64) {
    let title = display_generics(&c.id);
    let mut max_chars = title.chars().count();
    if let Some(s) = &c.stereotype {
        max_chars = max_chars.max(s.len() + 4); // «…»
    }
    for m in &c.members {
        max_chars = max_chars.max(display_generics(&m.text).chars().count());
    }
    let w = (max_chars as f64 * CHAR_W + PAD * 2.0).max(BOX_MIN_W);
    let header_lines = 1 + usize::from(c.stereotype.is_some());
    let lines = header_lines + c.members.len().max(1);
    let h = PAD * 2.0 + lines as f64 * LINE_H + 8.0;
    (w, h)
}

fn layout(diagram: &ClassDiagram) -> ClassLayout {
    // Layer by inheritance: parent above child for `<|--` where from=parent, to=child.
    let mut parents: HashMap<&str, Vec<&str>> = HashMap::new();
    let mut has_parent: HashSet<&str> = HashSet::new();
    for r in &diagram.relations {
        if r.kind == RelationKind::Inheritance || r.kind == RelationKind::Realization {
            parents.entry(r.from.as_str()).or_default().push(r.to.as_str());
            has_parent.insert(r.to.as_str());
        }
    }

    let ids: Vec<&str> = diagram.classes.iter().map(|c| c.id.as_str()).collect();
    let mut layer_of: HashMap<&str, usize> = HashMap::new();
    let mut queue: Vec<&str> = ids
        .iter()
        .copied()
        .filter(|id| !has_parent.contains(id))
        .collect();
    if queue.is_empty() {
        queue = ids.clone();
    }
    for id in &queue {
        layer_of.insert(id, 0);
    }
    let mut qi = 0;
    while qi < queue.len() {
        let id = queue[qi];
        qi += 1;
        let layer = *layer_of.get(id).unwrap_or(&0);
        if let Some(children) = parents.get(id) {
            for child in children {
                let next = layer + 1;
                if layer_of.get(child).copied().unwrap_or(0) < next {
                    layer_of.insert(child, next);
                }
                if !queue.contains(child) {
                    queue.push(child);
                }
            }
        }
    }
    for id in &ids {
        layer_of.entry(id).or_insert(0);
    }

    let mut by_layer: HashMap<usize, Vec<&Class>> = HashMap::new();
    for c in &diagram.classes {
        let layer = *layer_of.get(c.id.as_str()).unwrap_or(&0);
        by_layer.entry(layer).or_default().push(c);
    }
    let max_layer = by_layer.keys().copied().max().unwrap_or(0);

    let mut laid_classes = Vec::new();
    let mut positions: HashMap<String, (f64, f64, f64, f64)> = HashMap::new();
    let mut y = MARGIN;
    let mut max_w = MARGIN;

    for layer in 0..=max_layer {
        let Some(row) = by_layer.get(&layer) else {
            continue;
        };
        let sizes: Vec<(f64, f64)> = row.iter().map(|c| class_size(c)).collect();
        let row_h = sizes.iter().map(|(_, h)| *h).fold(0.0_f64, f64::max);
        let mut x = MARGIN;
        for (c, (w, h)) in row.iter().zip(sizes.iter()) {
            let cy = y + (row_h - h) / 2.0;
            laid_classes.push(LaidClass {
                id: c.id.clone(),
                stereotype: c.stereotype.clone(),
                members: c.members.iter().map(|m| m.text.clone()).collect(),
                x,
                y: cy,
                w: *w,
                h: *h,
            });
            positions.insert(c.id.clone(), (x, cy, *w, *h));
            x += w + H_GAP;
        }
        max_w = max_w.max(x - H_GAP + MARGIN);
        y += row_h + V_GAP;
    }

    let mut laid_relations = Vec::new();
    for r in &diagram.relations {
        let Some(&(x1, y1, w1, h1)) = positions.get(&r.from) else {
            continue;
        };
        let Some(&(x2, y2, w2, h2)) = positions.get(&r.to) else {
            continue;
        };
        // Connect bottom-center of parent to top-center of child for vertical,
        // else side centers.
        let (ax, ay, bx, by) = if (y2 - y1).abs() > (x2 - x1).abs() {
            if y2 > y1 {
                (x1 + w1 / 2.0, y1 + h1, x2 + w2 / 2.0, y2)
            } else {
                (x1 + w1 / 2.0, y1, x2 + w2 / 2.0, y2 + h2)
            }
        } else if x2 > x1 {
            (x1 + w1, y1 + h1 / 2.0, x2, y2 + h2 / 2.0)
        } else {
            (x1, y1 + h1 / 2.0, x2 + w2, y2 + h2 / 2.0)
        };
        laid_relations.push(LaidRelation {
            x1: ax,
            y1: ay,
            x2: bx,
            y2: by,
            kind: r.kind,
            label: r.label.clone(),
            from_card: r.from_card.clone(),
            to_card: r.to_card.clone(),
        });
    }

    let mut laid_notes = Vec::new();
    let mut note_bottom = y - V_GAP + MARGIN;
    let mut note_right = max_w;
    for (ni, note) in diagram.notes.iter().enumerate() {
        let Some(&(cx, cy, cw, ch)) = positions.get(&note.target) else {
            continue;
        };
        let lines = wrap_note_lines(&note.text, 28);
        let nw = (lines
            .iter()
            .map(|l| l.chars().count())
            .max()
            .unwrap_or(8) as f64
            * CHAR_W
            + PAD * 2.0)
            .max(80.0);
        let nh = PAD * 2.0 + lines.len().max(1) as f64 * LINE_H;
        let nx = cx + cw + 24.0;
        let ny = cy + (ni as f64) * 8.0;
        laid_notes.push(LaidNote {
            x: nx,
            y: ny,
            w: nw,
            h: nh,
            text: lines.join("\n"),
            ax: cx + cw,
            ay: cy + ch / 2.0,
        });
        note_right = note_right.max(nx + nw + MARGIN);
        note_bottom = note_bottom.max(ny + nh + MARGIN);
    }

    ClassLayout {
        width: max_w.max(note_right).max(MARGIN * 2.0),
        height: (y - V_GAP + MARGIN).max(note_bottom),
        classes: laid_classes,
        relations: laid_relations,
        notes: laid_notes,
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

pub fn render_svg(diagram: &ClassDiagram, theme: Theme) -> String {
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
    let muted = match theme {
        Theme::Dark => "#94a3b8",
        Theme::Light => "#64748b",
    };
    let line = match theme {
        Theme::Dark => "#64748b",
        Theme::Light => "#94a3b8",
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
        w = laid.width as i32,
        h = laid.height as i32,
    );
    svg.push_str(&format!(
        r#"<rect width="100%" height="100%" fill="{bg}"/>"#
    ));
    svg.push_str(&format!(
        r##"<defs>
  <marker id="class-arrow" markerWidth="10" markerHeight="7" refX="9" refY="3.5" orient="auto">
    <polygon points="0 0, 10 3.5, 0 7" fill="{c}"/>
  </marker>
  <marker id="class-triangle" markerWidth="12" markerHeight="10" refX="11" refY="5" orient="auto">
    <path d="M0,0 L12,5 L0,10 Z" fill="{bg}" stroke="{c}" stroke-width="1"/>
  </marker>
  <marker id="class-diamond" markerWidth="12" markerHeight="10" refX="11" refY="5" orient="auto">
    <path d="M0,5 L6,0 L12,5 L6,10 Z" fill="{c}" stroke="{c}"/>
  </marker>
  <marker id="class-diamond-empty" markerWidth="12" markerHeight="10" refX="11" refY="5" orient="auto">
    <path d="M0,5 L6,0 L12,5 L6,10 Z" fill="{bg}" stroke="{c}"/>
  </marker>
</defs>"##,
        c = line,
        bg = bg,
    ));

    for r in &laid.relations {
        let dash = if r.kind.dashed() {
            r#" stroke-dasharray="6,4""#
        } else {
            ""
        };
        let marker = match r.kind {
            RelationKind::Inheritance | RelationKind::Realization => "url(#class-triangle)",
            RelationKind::Composition => "url(#class-diamond)",
            RelationKind::Aggregation => "url(#class-diamond-empty)",
            RelationKind::Association | RelationKind::Dependency => "url(#class-arrow)",
            RelationKind::Link => "",
        };
        let marker_attr = if marker.is_empty() {
            String::new()
        } else {
            format!(r#" marker-end="{marker}""#)
        };
        svg.push_str(&format!(
            r##"<line x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}" stroke="{c}"{dash}{marker}/>"##,
            x1 = r.x1,
            y1 = r.y1,
            x2 = r.x2,
            y2 = r.y2,
            c = line,
            dash = dash,
            marker = marker_attr,
        ));
        if !r.label.is_empty() {
            svg.push_str(&format!(
                r##"<text x="{x}" y="{y}" text-anchor="middle" fill="{c}" font-size="11" font-family="sans-serif">{t}</text>"##,
                x = (r.x1 + r.x2) / 2.0,
                y = (r.y1 + r.y2) / 2.0 - 6.0,
                c = muted,
                t = esc(&r.label),
            ));
        }
        let dx = r.x2 - r.x1;
        let dy = r.y2 - r.y1;
        let len = (dx * dx + dy * dy).sqrt().max(1.0);
        let ox = -dy / len * 10.0;
        let oy = dx / len * 10.0;
        if let Some(card) = &r.from_card {
            svg.push_str(&format!(
                r##"<text x="{x}" y="{y}" text-anchor="middle" fill="{c}" font-size="11" font-family="sans-serif">{t}</text>"##,
                x = r.x1 + dx * 0.18 + ox,
                y = r.y1 + dy * 0.18 + oy,
                c = muted,
                t = esc(card),
            ));
        }
        if let Some(card) = &r.to_card {
            svg.push_str(&format!(
                r##"<text x="{x}" y="{y}" text-anchor="middle" fill="{c}" font-size="11" font-family="sans-serif">{t}</text>"##,
                x = r.x1 + dx * 0.82 + ox,
                y = r.y1 + dy * 0.82 + oy,
                c = muted,
                t = esc(card),
            ));
        }
    }

    for c in &laid.classes {
        svg.push_str(&format!(
            r##"<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="2" fill="{fill}" stroke="{stroke}"/>"##,
            x = c.x,
            y = c.y,
            w = c.w,
            h = c.h,
            fill = box_fill,
            stroke = box_stroke,
        ));
        let title_y = c.y + PAD + LINE_H * 0.75;
        svg.push_str(&format!(
            r##"<text x="{x}" y="{y}" text-anchor="middle" fill="{c}" font-size="13" font-weight="600" font-family="sans-serif">{t}</text>"##,
            x = c.x + c.w / 2.0,
            y = title_y,
            c = text_color,
            t = esc(&display_generics(&c.id)),
        ));
        let mut header_bottom = PAD + LINE_H;
        if let Some(stereo) = &c.stereotype {
            let sy = title_y + LINE_H;
            svg.push_str(&format!(
                r##"<text x="{x}" y="{y}" text-anchor="middle" fill="{c}" font-size="11" font-style="italic" font-family="sans-serif">«{t}»</text>"##,
                x = c.x + c.w / 2.0,
                y = sy,
                c = muted,
                t = esc(stereo),
            ));
            header_bottom += LINE_H;
        }
        let div_y = c.y + header_bottom + 2.0;
        svg.push_str(&format!(
            r##"<line x1="{x1}" y1="{y}" x2="{x2}" y2="{y}" stroke="{s}"/>"##,
            x1 = c.x,
            x2 = c.x + c.w,
            y = div_y,
            s = box_stroke,
        ));
        for (i, m) in c.members.iter().enumerate() {
            let my = div_y + LINE_H * (i as f64 + 0.85);
            svg.push_str(&format!(
                r##"<text x="{x}" y="{y}" fill="{c}" font-size="12" font-family="ui-monospace, monospace">{t}</text>"##,
                x = c.x + PAD,
                y = my,
                c = muted,
                t = esc(&display_generics(m)),
            ));
        }
    }

    for n in &laid.notes {
        svg.push_str(&format!(
            r##"<line x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}" stroke="{c}" stroke-dasharray="4,3"/>"##,
            x1 = n.ax,
            y1 = n.ay,
            x2 = n.x,
            y2 = n.y + n.h / 2.0,
            c = note_stroke,
        ));
        svg.push_str(&format!(
            r##"<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="4" fill="{fill}" stroke="{stroke}"/>"##,
            x = n.x,
            y = n.y,
            w = n.w,
            h = n.h,
            fill = note_fill,
            stroke = note_stroke,
        ));
        let mut ty = n.y + PAD + LINE_H * 0.75;
        for line in n.text.lines() {
            svg.push_str(&format!(
                r##"<text x="{x}" y="{y}" fill="{c}" font-size="12" font-family="sans-serif">{t}</text>"##,
                x = n.x + PAD,
                y = ty,
                c = text_color,
                t = esc(line),
            ));
            ty += LINE_H;
        }
    }

    svg.push_str("</svg>");
    svg
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"classDiagram
    class Animal {
        +String name
        +eat()
    }
    class Dog
    Animal <|-- Dog : inherits
    Animal --> Food
"#;

    #[test]
    fn parse_basic() {
        let d = parse(SAMPLE).unwrap();
        assert_eq!(d.classes.len(), 3);
        assert_eq!(d.classes[0].id, "Animal");
        assert_eq!(d.classes[0].members.len(), 2);
        assert_eq!(d.relations.len(), 2);
        assert_eq!(d.relations[0].kind, RelationKind::Inheritance);
        assert_eq!(d.relations[0].label, "inherits");
    }

    #[test]
    fn member_annotation() {
        let d = parse("classDiagram\n    Animal : +String name\n").unwrap();
        assert_eq!(d.classes[0].members[0].text, "+String name");
    }

    #[test]
    fn roundtrip() {
        let d = parse(SAMPLE).unwrap();
        let out = d.to_mermaid();
        let d2 = parse(&out).unwrap();
        assert_eq!(d2.classes.len(), d.classes.len());
        assert_eq!(d2.relations.len(), d.relations.len());
    }

    #[test]
    fn render_svg_ok() {
        let d = parse(SAMPLE).unwrap();
        let svg = render_svg(&d, Theme::Dark);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("Animal"));
        assert!(svg.contains("+String name"));
        assert!(svg.ends_with("</svg>"));
    }

    #[test]
    fn is_class_detects() {
        assert!(is_class(SAMPLE));
        assert!(!is_class("graph TD\n  A-->B\n"));
        assert!(!is_class("sequenceDiagram\n  A->>B: hi\n"));
    }

    #[test]
    fn parse_stereotype() {
        let src = r#"classDiagram
    class Shape <<interface>> {
        +draw()
    }
    class Circle {
        <<implementation>>
        +draw()
    }
    Circle ..|> Shape
"#;
        let d = parse(src).unwrap();
        let shape = d.classes.iter().find(|c| c.id == "Shape").unwrap();
        assert_eq!(shape.stereotype.as_deref(), Some("interface"));
        assert_eq!(shape.members.len(), 1);
        let circle = d.classes.iter().find(|c| c.id == "Circle").unwrap();
        assert_eq!(circle.stereotype.as_deref(), Some("implementation"));
        let out = d.to_mermaid();
        assert!(out.contains("<<interface>>"));
        let d2 = parse(&out).unwrap();
        assert_eq!(
            d2.classes
                .iter()
                .find(|c| c.id == "Shape")
                .unwrap()
                .stereotype
                .as_deref(),
            Some("interface")
        );
        let svg = render_svg(&d, Theme::Dark);
        assert!(svg.contains("«interface»"));
    }

    #[test]
    fn parse_cardinality() {
        let src = r#"classDiagram
    Customer "1" --> "*" Order : places
    Student "1" --> "1..*" Course
"#;
        let d = parse(src).unwrap();
        assert_eq!(d.relations.len(), 2);
        assert_eq!(d.relations[0].from, "Customer");
        assert_eq!(d.relations[0].to, "Order");
        assert_eq!(d.relations[0].from_card.as_deref(), Some("1"));
        assert_eq!(d.relations[0].to_card.as_deref(), Some("*"));
        assert_eq!(d.relations[0].label, "places");
        assert_eq!(d.relations[1].to_card.as_deref(), Some("1..*"));
        let out = d.to_mermaid();
        assert!(out.contains("\"1\""));
        assert!(out.contains("\"*\""));
        let d2 = parse(&out).unwrap();
        assert_eq!(d2.relations[0].from_card, d.relations[0].from_card);
        assert_eq!(d2.relations[0].to_card, d.relations[0].to_card);
        let svg = render_svg(&d, Theme::Light);
        assert!(svg.contains("1..*") || svg.contains("*"));
    }

    #[test]
    fn parse_generics() {
        let src = r#"classDiagram
    class Stack~T~ {
        +List~T~ items
        +push(item~T~) void
    }
    class List~List~int~~
    Stack~T~ --> List~T~ : stores
"#;
        let d = parse(src).unwrap();
        let stack = d.classes.iter().find(|c| c.id == "Stack~T~").unwrap();
        assert_eq!(stack.members[0].text, "+List~T~ items");
        assert!(d.classes.iter().any(|c| c.id == "List~List~int~~"));
        assert_eq!(d.relations[0].from, "Stack~T~");
        assert_eq!(display_generics("Stack~T~"), "Stack‹T›");
        assert_eq!(display_generics("List~List~int~~"), "List‹List‹int››");
        assert_eq!(generics_to_angle("Stack~T~"), "Stack<T>");
        assert_eq!(angle_generics_to_tilde("Stack<T>"), "Stack~T~");
        assert_eq!(
            angle_generics_to_tilde("List<List<int>>"),
            "List~List~int~~"
        );
        let out = d.to_mermaid();
        assert!(out.contains("Stack~T~"));
        let d2 = parse(&out).unwrap();
        assert!(d2.classes.iter().any(|c| c.id == "Stack~T~"));
        let svg = render_svg(&d, Theme::Dark);
        assert!(svg.contains("Stack‹T›") || svg.contains("‹T›"));
    }

    #[test]
    fn parse_notes() {
        let src = r#"classDiagram
    class Animal
    note for Animal "represents living creatures"
    Animal --> Food
"#;
        let d = parse(src).unwrap();
        assert_eq!(d.notes.len(), 1);
        assert_eq!(d.notes[0].target, "Animal");
        assert_eq!(d.notes[0].text, "represents living creatures");
        let out = d.to_mermaid();
        assert!(out.contains("note for Animal"));
        let d2 = parse(&out).unwrap();
        assert_eq!(d2.notes.len(), 1);
        assert_eq!(d2.notes[0].text, d.notes[0].text);
        let svg = render_svg(&d, Theme::Light);
        assert!(svg.contains("represents living creatures"));
    }
}
