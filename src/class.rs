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
    fn mermaid_str(self) -> &'static str {
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relation {
    pub from: String,
    pub to: String,
    pub kind: RelationKind,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassDiagram {
    pub classes: Vec<Class>,
    pub relations: Vec<Relation>,
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

    let mut i = 1;
    while i < raw.len() {
        let (line_num, text) = &raw[i];

        if let Some(rest) = text.strip_prefix("class ") {
            let rest = rest.trim();
            if let Some(brace) = rest.find('{') {
                let id = rest[..brace].trim().to_string();
                if id.is_empty() {
                    return Err(ParseError {
                        message: "empty class name".into(),
                        line: Some(*line_num),
                    });
                }
                ensure_class(&mut classes, &mut order, &id);
                let after = rest[brace + 1..].trim();
                if after.ends_with('}') {
                    let inner = after.trim_end_matches('}').trim();
                    if !inner.is_empty() {
                        for part in inner.split(';') {
                            let m = part.trim();
                            if !m.is_empty() {
                                classes.get_mut(&id).unwrap().members.push(ClassMember {
                                    text: m.to_string(),
                                });
                            }
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
                                classes.get_mut(&id).unwrap().members.push(ClassMember {
                                    text: m.to_string(),
                                });
                            }
                            i += 1;
                            break;
                        }
                        classes.get_mut(&id).unwrap().members.push(ClassMember {
                            text: body.clone(),
                        });
                        i += 1;
                    }
                }
                continue;
            } else {
                let id = rest.to_string();
                ensure_class(&mut classes, &mut order, &id);
                i += 1;
                continue;
            }
        }

        // `Animal : +String name` (not a relation label)
        if let Some((left, right)) = text.split_once(" : ") {
            let id = left.trim();
            let member = right.trim();
            if !id.is_empty()
                && !member.is_empty()
                && id
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
            {
                ensure_class(&mut classes, &mut order, id);
                classes.get_mut(id).unwrap().members.push(ClassMember {
                    text: member.to_string(),
                });
                i += 1;
                continue;
            }
        }

        if let Some(rel) = parse_relation(text) {
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

    Ok(ClassDiagram { classes, relations })
}

fn ensure_class(classes: &mut HashMap<String, Class>, order: &mut Vec<String>, id: &str) {
    if !classes.contains_key(id) {
        order.push(id.to_string());
        classes.insert(
            id.to_string(),
            Class {
                id: id.to_string(),
                members: Vec::new(),
            },
        );
    }
}

fn parse_relation(text: &str) -> Option<Relation> {
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
            let from = left.trim().to_string();
            let (to, label) = match right.split_once(':') {
                Some((to, lab)) => (to.trim().to_string(), lab.trim().to_string()),
                None => (right.trim().to_string(), String::new()),
            };
            if from.is_empty() || to.is_empty() {
                return None;
            }
            return Some(Relation {
                from,
                to,
                kind,
                label,
            });
        }
    }
    None
}

impl ClassDiagram {
    pub fn to_mermaid(&self) -> String {
        let mut out = String::from("classDiagram\n");
        for c in &self.classes {
            if c.members.is_empty() {
                out.push_str(&format!("    class {}\n", c.id));
            } else {
                out.push_str(&format!("    class {} {{\n", c.id));
                for m in &c.members {
                    out.push_str(&format!("        {}\n", m.text));
                }
                out.push_str("    }\n");
            }
        }
        for r in &self.relations {
            if r.label.is_empty() {
                out.push_str(&format!(
                    "    {} {} {}\n",
                    r.from,
                    r.kind.mermaid_str(),
                    r.to
                ));
            } else {
                out.push_str(&format!(
                    "    {} {} {} : {}\n",
                    r.from,
                    r.kind.mermaid_str(),
                    r.to,
                    r.label
                ));
            }
        }
        out
    }
}

struct LaidClass {
    id: String,
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
}

struct ClassLayout {
    width: f64,
    height: f64,
    classes: Vec<LaidClass>,
    relations: Vec<LaidRelation>,
}

const BOX_MIN_W: f64 = 120.0;
const CHAR_W: f64 = 7.5;
const LINE_H: f64 = 18.0;
const PAD: f64 = 10.0;
const H_GAP: f64 = 80.0;
const V_GAP: f64 = 70.0;
const MARGIN: f64 = 40.0;

fn class_size(c: &Class) -> (f64, f64) {
    let mut max_chars = c.id.len();
    for m in &c.members {
        max_chars = max_chars.max(m.text.len());
    }
    let w = (max_chars as f64 * CHAR_W + PAD * 2.0).max(BOX_MIN_W);
    // title + divider + members (at least empty compartment spacing)
    let lines = 1 + c.members.len().max(1);
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
        });
    }

    ClassLayout {
        width: max_w.max(MARGIN * 2.0),
        height: y - V_GAP + MARGIN,
        classes: laid_classes,
        relations: laid_relations,
    }
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
            t = esc(&c.id),
        ));
        let div_y = c.y + PAD + LINE_H + 2.0;
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
                t = esc(m),
            ));
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
}
