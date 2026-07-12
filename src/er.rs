//! Mermaid ER diagram parse, layout, and SVG render (MVP).

use crate::renderer::Theme;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Cardinality {
    /// Exactly one `||` or `|`
    One,
    /// Zero or one `|o` / `o|`
    ZeroOrOne,
    /// Zero or more `}o` / `o{`
    ZeroOrMore,
    /// One or more `}|` / `|{`
    OneOrMore,
}

impl Cardinality {
    fn mermaid_left(self) -> &'static str {
        match self {
            Self::One => "||",
            Self::ZeroOrOne => "|o",
            Self::ZeroOrMore => "}o",
            Self::OneOrMore => "}|",
        }
    }

    fn mermaid_right(self) -> &'static str {
        match self {
            Self::One => "||",
            Self::ZeroOrOne => "o|",
            Self::ZeroOrMore => "o{",
            Self::OneOrMore => "|{",
        }
    }

    fn symbol(self) -> &'static str {
        match self {
            Self::One => "1",
            Self::ZeroOrOne => "0..1",
            Self::ZeroOrMore => "0..*",
            Self::OneOrMore => "1..*",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attribute {
    pub name: String,
    pub type_name: String,
    pub pk: bool,
    pub fk: bool,
    pub uk: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: String,
    pub attributes: Vec<Attribute>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    pub from: String,
    pub to: String,
    pub from_card: Cardinality,
    pub to_card: Cardinality,
    pub identifying: bool,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErDiagram {
    pub entities: Vec<Entity>,
    pub relationships: Vec<Relationship>,
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

pub fn is_er(source: &str) -> bool {
    source
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty() && !l.starts_with("%%"))
        .is_some_and(|l| l.starts_with("erDiagram"))
}

pub fn parse(source: &str) -> Result<ErDiagram, ParseError> {
    let lines: Vec<(usize, &str)> = source
        .lines()
        .enumerate()
        .map(|(i, l)| (i + 1, l.trim()))
        .filter(|(_, l)| !l.is_empty() && !l.starts_with("%%"))
        .collect();

    if lines.is_empty() || !lines[0].1.starts_with("erDiagram") {
        return Err(ParseError {
            message: "expected erDiagram header".into(),
            line: lines.first().map(|(n, _)| *n),
        });
    }

    let mut entities: HashMap<String, Entity> = HashMap::new();
    let mut relationships: Vec<Relationship> = Vec::new();
    let mut i = 1;
    while i < lines.len() {
        let (line_num, text) = lines[i];
        if is_entity_block_start(text) {
            let (entity, next) = parse_entity_block(&lines, i)?;
            entities.insert(entity.id.clone(), entity);
            i = next;
            continue;
        }
        if let Some(rel) = parse_relationship(text) {
            entities
                .entry(rel.from.clone())
                .or_insert_with(|| Entity {
                    id: rel.from.clone(),
                    attributes: Vec::new(),
                });
            entities.entry(rel.to.clone()).or_insert_with(|| Entity {
                id: rel.to.clone(),
                attributes: Vec::new(),
            });
            relationships.push(rel);
            i += 1;
            continue;
        }
        // Bare entity name
        if text
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        {
            entities.entry(text.to_string()).or_insert_with(|| Entity {
                id: text.to_string(),
                attributes: Vec::new(),
            });
            i += 1;
            continue;
        }
        return Err(ParseError {
            message: format!("unrecognized ER line: {text}"),
            line: Some(line_num),
        });
    }

    let mut entities: Vec<Entity> = entities.into_values().collect();
    entities.sort_by(|a, b| a.id.cmp(&b.id));

    Ok(ErDiagram {
        entities,
        relationships,
    })
}

fn is_entity_block_start(text: &str) -> bool {
    let trimmed = text.trim();
    if !trimmed.ends_with('{') {
        return false;
    }
    // Relationship cardinalities include `{` (e.g. `o{`); those also contain `--` or `..`.
    if trimmed.contains("--") || trimmed.contains("..") {
        return false;
    }
    let name = trimmed.trim_end_matches('{').trim();
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
}

fn parse_entity_block(
    lines: &[(usize, &str)],
    start: usize,
) -> Result<(Entity, usize), ParseError> {
    let (line_num, text) = lines[start];
    let name = text
        .split('{')
        .next()
        .unwrap_or("")
        .trim()
        .to_string();
    if name.is_empty() {
        return Err(ParseError {
            message: "entity block missing name".into(),
            line: Some(line_num),
        });
    }

    let mut attributes = Vec::new();
    let mut i = start + 1;
    if text.contains('}') {
        // empty block on same line
        return Ok((
            Entity {
                id: name,
                attributes,
            },
            start + 1,
        ));
    }

    while i < lines.len() {
        let (ln, t) = lines[i];
        if t == "}" || t.starts_with('}') {
            return Ok((
                Entity {
                    id: name,
                    attributes,
                },
                i + 1,
            ));
        }
        attributes.push(parse_attribute(t).map_err(|m| ParseError {
            message: m,
            line: Some(ln),
        })?);
        i += 1;
    }
    Err(ParseError {
        message: "unclosed entity block".into(),
        line: Some(line_num),
    })
}

fn parse_attribute(text: &str) -> Result<Attribute, String> {
    let parts: Vec<&str> = text.split_whitespace().collect();
    if parts.len() < 2 {
        return Err(format!("attribute needs type and name: {text}"));
    }
    let type_name = parts[0].to_string();
    let name = parts[1].to_string();
    let mut pk = false;
    let mut fk = false;
    let mut uk = false;
    for flag in parts.iter().skip(2) {
        match flag.to_uppercase().as_str() {
            "PK" => pk = true,
            "FK" => fk = true,
            "UK" => uk = true,
            _ => {}
        }
    }
    Ok(Attribute {
        name,
        type_name,
        pk,
        fk,
        uk,
    })
}

fn parse_relationship(text: &str) -> Option<Relationship> {
    // CUSTOMER ||--o{ ORDER : places
    let (left_part, rest) = split_rel_op(text)?;
    let from = left_part.trim().to_string();
    if from.is_empty() {
        return None;
    }

    let (identifying, left_card, right_card, after) = parse_rel_op(rest)?;
    let after = after.trim();
    let (to, label) = if let Some((t, l)) = after.split_once(':') {
        (t.trim().to_string(), l.trim().to_string())
    } else {
        let to = after
            .split_whitespace()
            .next()
            .unwrap_or("")
            .to_string();
        (to, String::new())
    };
    if to.is_empty() {
        return None;
    }

    Some(Relationship {
        from,
        to,
        from_card: left_card,
        to_card: right_card,
        identifying,
        label,
    })
}

fn split_rel_op(text: &str) -> Option<(&str, &str)> {
    for op in ["||--", "|o--", "}o--", "}|--", "||..", "|o..", "}o..", "}|.."] {
        if let Some(idx) = text.find(op) {
            return Some((&text[..idx], &text[idx..]));
        }
    }
    None
}

fn parse_rel_op(rest: &str) -> Option<(bool, Cardinality, Cardinality, &str)> {
    let identifying = rest.contains("--");
    let sep = if identifying { "--" } else { ".." };
    let sep_idx = rest.find(sep)?;
    let left = &rest[..sep_idx];
    let right_and_after = &rest[sep_idx + sep.len()..];

    let left_card = parse_left_card(left)?;
    // right card is first 2 chars typically
    let (right_card, after) = parse_right_card(right_and_after)?;
    Some((identifying, left_card, right_card, after))
}

fn parse_left_card(s: &str) -> Option<Cardinality> {
    match s {
        "||" => Some(Cardinality::One),
        "|o" => Some(Cardinality::ZeroOrOne),
        "}o" => Some(Cardinality::ZeroOrMore),
        "}|" => Some(Cardinality::OneOrMore),
        _ => None,
    }
}

fn parse_right_card(s: &str) -> Option<(Cardinality, &str)> {
    if let Some(rest) = s.strip_prefix("||") {
        Some((Cardinality::One, rest))
    } else if let Some(rest) = s.strip_prefix("o|") {
        Some((Cardinality::ZeroOrOne, rest))
    } else if let Some(rest) = s.strip_prefix("o{") {
        Some((Cardinality::ZeroOrMore, rest))
    } else if let Some(rest) = s.strip_prefix("|{") {
        Some((Cardinality::OneOrMore, rest))
    } else {
        None
    }
}

impl ErDiagram {
    pub fn to_mermaid(&self) -> String {
        let mut out = String::from("erDiagram\n");
        for e in &self.entities {
            if e.attributes.is_empty() {
                continue;
            }
            out.push_str(&format!("    {} {{\n", e.id));
            for a in &e.attributes {
                out.push_str(&format!("        {} {}", a.type_name, a.name));
                if a.pk {
                    out.push_str(" PK");
                }
                if a.fk {
                    out.push_str(" FK");
                }
                if a.uk {
                    out.push_str(" UK");
                }
                out.push('\n');
            }
            out.push_str("    }\n");
        }
        for r in &self.relationships {
            let sep = if r.identifying { "--" } else { ".." };
            let op = format!(
                "{}{}{}",
                r.from_card.mermaid_left(),
                sep,
                r.to_card.mermaid_right()
            );
            if r.label.is_empty() {
                out.push_str(&format!("    {} {} {}\n", r.from, op, r.to));
            } else {
                out.push_str(&format!("    {} {} {} : {}\n", r.from, op, r.to, r.label));
            }
        }
        // Emit bare entities that only appear in relationships without attributes
        // (already covered by relationships). Entities with no attrs and no rels:
        let related: HashSet<&str> = self
            .relationships
            .iter()
            .flat_map(|r| [r.from.as_str(), r.to.as_str()])
            .collect();
        for e in &self.entities {
            if e.attributes.is_empty() && !related.contains(e.id.as_str()) {
                out.push_str(&format!("    {}\n", e.id));
            }
        }
        out
    }
}

struct LaidEntity {
    id: String,
    attributes: Vec<String>,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

struct LaidRel {
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    from_card: Cardinality,
    to_card: Cardinality,
    label: String,
}

struct ErLayout {
    width: f64,
    height: f64,
    entities: Vec<LaidEntity>,
    relationships: Vec<LaidRel>,
}

const BOX_MIN_W: f64 = 140.0;
const CHAR_W: f64 = 7.5;
const LINE_H: f64 = 18.0;
const PAD: f64 = 10.0;
const H_GAP: f64 = 100.0;
const V_GAP: f64 = 80.0;
const MARGIN: f64 = 40.0;

fn entity_size(e: &Entity) -> (f64, f64) {
    let mut max_chars = e.id.len();
    for a in &e.attributes {
        let mut len = a.type_name.len() + 1 + a.name.len();
        if a.pk {
            len += 3;
        }
        if a.fk {
            len += 3;
        }
        if a.uk {
            len += 3;
        }
        max_chars = max_chars.max(len);
    }
    let w = (max_chars as f64 * CHAR_W + PAD * 2.0).max(BOX_MIN_W);
    let lines = 1 + e.attributes.len().max(1);
    let h = PAD * 2.0 + lines as f64 * LINE_H + 8.0;
    (w, h)
}

fn attr_display(a: &Attribute) -> String {
    let mut s = format!("{} {}", a.type_name, a.name);
    if a.pk {
        s.push_str(" PK");
    }
    if a.fk {
        s.push_str(" FK");
    }
    if a.uk {
        s.push_str(" UK");
    }
    s
}

fn layout(diagram: &ErDiagram) -> ErLayout {
    // Simple left-to-right: entities with outgoing first, then layered by relationship order.
    let mut incoming: HashMap<&str, usize> = HashMap::new();
    let mut outgoing: HashMap<&str, Vec<&str>> = HashMap::new();
    for e in &diagram.entities {
        incoming.insert(e.id.as_str(), 0);
        outgoing.insert(e.id.as_str(), Vec::new());
    }
    for r in &diagram.relationships {
        *incoming.entry(r.to.as_str()).or_default() += 1;
        if let Some(v) = outgoing.get_mut(r.from.as_str()) {
            v.push(r.to.as_str());
        }
    }

    let mut layer_of: HashMap<&str, usize> = HashMap::new();
    let mut queue: Vec<&str> = diagram
        .entities
        .iter()
        .map(|e| e.id.as_str())
        .filter(|id| incoming.get(id).copied().unwrap_or(0) == 0)
        .collect();
    if queue.is_empty() {
        queue = diagram.entities.iter().map(|e| e.id.as_str()).collect();
    }
    for id in &queue {
        layer_of.insert(id, 0);
    }
    let mut qi = 0;
    while qi < queue.len() {
        let id = queue[qi];
        qi += 1;
        let layer = *layer_of.get(id).unwrap_or(&0);
        if let Some(nexts) = outgoing.get(id) {
            for &to in nexts {
                let next = layer + 1;
                if layer_of.get(to).copied().unwrap_or(0) < next {
                    layer_of.insert(to, next);
                }
                if !queue.contains(&to) {
                    queue.push(to);
                }
            }
        }
    }
    for e in &diagram.entities {
        layer_of.entry(e.id.as_str()).or_insert(0);
    }

    let mut by_layer: HashMap<usize, Vec<&Entity>> = HashMap::new();
    for e in &diagram.entities {
        let layer = *layer_of.get(e.id.as_str()).unwrap_or(&0);
        by_layer.entry(layer).or_default().push(e);
    }
    let max_layer = by_layer.keys().copied().max().unwrap_or(0);

    let mut laid = Vec::new();
    let mut positions: HashMap<String, (f64, f64, f64, f64)> = HashMap::new();
    let mut y = MARGIN;
    let mut max_w = MARGIN;

    for layer in 0..=max_layer {
        let Some(row) = by_layer.get(&layer) else {
            continue;
        };
        let sizes: Vec<(f64, f64)> = row.iter().map(|e| entity_size(e)).collect();
        let row_h = sizes.iter().map(|(_, h)| *h).fold(0.0_f64, f64::max);
        let mut x = MARGIN;
        for (e, (w, h)) in row.iter().zip(sizes.iter()) {
            let cy = y + (row_h - h) / 2.0;
            laid.push(LaidEntity {
                id: e.id.clone(),
                attributes: e.attributes.iter().map(attr_display).collect(),
                x,
                y: cy,
                w: *w,
                h: *h,
            });
            positions.insert(e.id.clone(), (x, cy, *w, *h));
            x += w + H_GAP;
        }
        max_w = max_w.max(x - H_GAP + MARGIN);
        y += row_h + V_GAP;
    }

    let mut laid_rels = Vec::new();
    for r in &diagram.relationships {
        let Some(&(x1, y1, w1, h1)) = positions.get(&r.from) else {
            continue;
        };
        let Some(&(x2, y2, _w2, h2)) = positions.get(&r.to) else {
            continue;
        };
        laid_rels.push(LaidRel {
            x1: x1 + w1,
            y1: y1 + h1 / 2.0,
            x2: x2,
            y2: y2 + h2 / 2.0,
            from_card: r.from_card,
            to_card: r.to_card,
            label: r.label.clone(),
        });
    }

    ErLayout {
        width: max_w.max(MARGIN * 2.0),
        height: y - V_GAP + MARGIN,
        entities: laid,
        relationships: laid_rels,
    }
}

fn esc_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub fn render_svg(d: &ErDiagram, theme: Theme) -> String {
    let laid = layout(d);
    let bg = match theme {
        Theme::Dark => "#1a1a2e",
        Theme::Light => "#ffffff",
    };
    let box_fill = match theme {
        Theme::Dark => "#334155",
        Theme::Light => "#f1f5f9",
    };
    let header_fill = match theme {
        Theme::Dark => "#1e293b",
        Theme::Light => "#e2e8f0",
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
        r##"<defs><marker id="er-arrow" markerWidth="8" markerHeight="8" refX="6" refY="3" orient="auto"><path d="M0,0 L6,3 L0,6 Z" fill="{c}"/></marker></defs>"##,
        c = line_color,
    ));

    for r in &laid.relationships {
        svg.push_str(&format!(
            r##"<line x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}" stroke="{c}" marker-end="url(#er-arrow)"/>"##,
            x1 = r.x1,
            y1 = r.y1,
            x2 = r.x2,
            y2 = r.y2,
            c = line_color,
        ));
        let mx = (r.x1 + r.x2) / 2.0;
        let my = (r.y1 + r.y2) / 2.0 - 10.0;
        let card = format!(
            "{} — {}",
            r.from_card.symbol(),
            r.to_card.symbol()
        );
        svg.push_str(&format!(
            r##"<text x="{mx}" y="{my}" text-anchor="middle" fill="{label_color}" font-size="11" font-family="sans-serif">{card}</text>"##,
            card = esc_xml(&card),
        ));
        if !r.label.is_empty() {
            svg.push_str(&format!(
                r##"<text x="{mx}" y="{y}" text-anchor="middle" fill="{label_color}" font-size="12" font-family="sans-serif">{t}</text>"##,
                y = my + 14.0,
                t = esc_xml(&r.label),
            ));
        }
    }

    for e in &laid.entities {
        let header_h = PAD + LINE_H + 4.0;
        svg.push_str(&format!(
            r##"<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="4" fill="{box_fill}" stroke="{box_stroke}"/>"##,
            x = e.x,
            y = e.y,
            w = e.w,
            h = e.h,
            box_fill = box_fill,
            box_stroke = box_stroke,
        ));
        svg.push_str(&format!(
            r##"<rect x="{x}" y="{y}" width="{w}" height="{hh}" rx="4" fill="{header_fill}" stroke="{box_stroke}"/>"##,
            x = e.x,
            y = e.y,
            w = e.w,
            hh = header_h,
            header_fill = header_fill,
            box_stroke = box_stroke,
        ));
        // Fix bottom corners of header
        svg.push_str(&format!(
            r##"<rect x="{x}" y="{y}" width="{w}" height="{hh}" fill="{header_fill}"/>"##,
            x = e.x,
            y = e.y + 4.0,
            w = e.w,
            hh = header_h - 4.0,
            header_fill = header_fill,
        ));
        svg.push_str(&format!(
            r##"<text x="{cx}" y="{cy}" text-anchor="middle" dominant-baseline="middle" fill="{text_color}" font-size="13" font-weight="bold" font-family="sans-serif">{id}</text>"##,
            cx = e.x + e.w / 2.0,
            cy = e.y + header_h / 2.0,
            text_color = text_color,
            id = esc_xml(&e.id),
        ));
        let mut ty = e.y + header_h + PAD / 2.0 + LINE_H / 2.0;
        if e.attributes.is_empty() {
            // empty body ok
        } else {
            for a in &e.attributes {
                svg.push_str(&format!(
                    r##"<text x="{x}" y="{ty}" fill="{text_color}" font-size="12" font-family="sans-serif">{a}</text>"##,
                    x = e.x + PAD,
                    text_color = text_color,
                    a = esc_xml(a),
                ));
                ty += LINE_H;
            }
        }
    }

    svg.push_str("</svg>");
    svg
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"erDiagram
    CUSTOMER ||--o{ ORDER : places
    ORDER ||--|{ LINE-ITEM : contains
    CUSTOMER {
        string name
        string custNumber PK
        string sector
    }
    ORDER {
        int orderNumber PK
        string deliveryAddress
    }
    LINE-ITEM {
        string productCode
        int quantity
        float pricePerUnit
    }
"#;

    #[test]
    fn is_er_detects() {
        assert!(is_er(SAMPLE));
        assert!(!is_er("classDiagram\n  class A\n"));
    }

    #[test]
    fn parse_basic() {
        let d = parse(SAMPLE).unwrap();
        assert_eq!(d.entities.len(), 3);
        assert_eq!(d.relationships.len(), 2);
        let customer = d.entities.iter().find(|e| e.id == "CUSTOMER").unwrap();
        assert_eq!(customer.attributes.len(), 3);
        assert!(customer.attributes.iter().any(|a| a.pk && a.name == "custNumber"));
        assert_eq!(d.relationships[0].from_card, Cardinality::One);
        assert_eq!(d.relationships[0].to_card, Cardinality::ZeroOrMore);
        assert_eq!(d.relationships[0].label, "places");
    }

    #[test]
    fn roundtrip() {
        let d = parse(SAMPLE).unwrap();
        let out = d.to_mermaid();
        let d2 = parse(&out).unwrap();
        assert_eq!(d2.entities.len(), d.entities.len());
        assert_eq!(d2.relationships.len(), d.relationships.len());
    }

    #[test]
    fn render_contains_svg() {
        let d = parse(SAMPLE).unwrap();
        let svg = render_svg(&d, Theme::Dark);
        assert!(svg.contains("<svg"));
        assert!(svg.contains("CUSTOMER"));
        assert!(svg.contains("ORDER"));
    }
}
