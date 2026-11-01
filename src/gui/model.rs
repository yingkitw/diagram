//! Document model + editor state for the desktop GUI.
//!
//! This module is deliberately egui-free: it owns the serde-serializable
//! document, undo/redo history, editing operations, hit testing, and the
//! export paths (positioned SVG, Mermaid via the flowchart IR). Everything
//! here is deterministic and unit-testable headlessly.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::diagram::{self, EdgeStyle};

use super::shapes::{self, ShapeKind};

pub const HISTORY_LIMIT: usize = 50;
pub const GRID: f32 = 8.0;

fn default_edge_color() -> [u8; 3] {
    [87, 96, 106]
}

fn default_text_color() -> [u8; 3] {
    [31, 35, 40]
}

/// A placed shape on the canvas. `x`/`y` are the centre in world coordinates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuiNode {
    pub id: String,
    /// Catalog id (see [`shapes::CATALOG`]). Unknown ids degrade to a rect.
    pub shape: String,
    pub label: String,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub fill: [u8; 3],
    pub stroke: [u8; 3],
    #[serde(default = "default_text_color")]
    pub text: [u8; 3],
}

impl GuiNode {
    pub fn left(&self) -> f32 {
        self.x - self.w / 2.0
    }
    pub fn right(&self) -> f32 {
        self.x + self.w / 2.0
    }
    pub fn top(&self) -> f32 {
        self.y - self.h / 2.0
    }
    pub fn bottom(&self) -> f32 {
        self.y + self.h / 2.0
    }

    pub fn contains(&self, px: f32, py: f32) -> bool {
        px >= self.left() && px <= self.right() && py >= self.top() && py <= self.bottom()
    }

    pub fn kind(&self) -> ShapeKind {
        shapes::shape_def(&self.shape)
            .map(|d| d.kind)
            .unwrap_or(ShapeKind::Rect)
    }
}

/// A directed connection between two nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuiEdge {
    pub id: String,
    pub from: String,
    pub to: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub style: EdgeStyle,
    #[serde(default = "default_edge_color")]
    pub color: [u8; 3],
}

/// The serializable on-disk document (`.json`, versioned).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuiDocument {
    pub version: u32,
    pub name: String,
    pub nodes: Vec<GuiNode>,
    pub edges: Vec<GuiEdge>,
}

impl Default for GuiDocument {
    fn default() -> Self {
        Self {
            version: Self::CURRENT_VERSION,
            name: "Untitled".to_string(),
            nodes: Vec::new(),
            edges: Vec::new(),
        }
    }
}

impl GuiDocument {
    pub const CURRENT_VERSION: u32 = 1;

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn from_json(s: &str) -> Result<Self, serde_json::Error> {
        let mut doc: Self = serde_json::from_str(s)?;
        doc.sanitize();
        Ok(doc)
    }

    /// Drop dangling edges and duplicate ids (lossy but keeps the file loadable).
    pub fn sanitize(&mut self) {
        let mut seen = HashSet::new();
        self.nodes.retain(|n| seen.insert(n.id.clone()));
        let ids: HashSet<&str> = self.nodes.iter().map(|n| n.id.as_str()).collect();
        self.edges.retain(|e| ids.contains(e.from.as_str()) && ids.contains(e.to.as_str()));
        let mut seen_edges = HashSet::new();
        self.edges.retain(|e| seen_edges.insert(e.id.clone()));
    }

    pub fn node(&self, id: &str) -> Option<&GuiNode> {
        self.nodes.iter().find(|n| n.id == id)
    }

    pub fn node_mut(&mut self, id: &str) -> Option<&mut GuiNode> {
        self.nodes.iter_mut().find(|n| n.id == id)
    }

    pub fn edge(&self, id: &str) -> Option<&GuiEdge> {
        self.edges.iter().find(|e| e.id == id)
    }

    pub fn edge_mut(&mut self, id: &str) -> Option<&mut GuiEdge> {
        self.edges.iter_mut().find(|e| e.id == id)
    }

    /// Content bounding box over nodes (None for an empty canvas).
    pub fn bounds(&self) -> Option<(f32, f32, f32, f32)> {
        let mut it = self.nodes.iter();
        let first = it.next()?;
        let (mut minx, mut miny, mut maxx, mut maxy) =
            (first.left(), first.top(), first.right(), first.bottom());
        for n in it {
            minx = minx.min(n.left());
            miny = miny.min(n.top());
            maxx = maxx.max(n.right());
            maxy = maxy.max(n.bottom());
        }
        Some((minx, miny, maxx, maxy))
    }
}

/// Clipped endpoints of an edge (line stops at each node's bounding box).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EdgeGeom {
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
}

impl EdgeGeom {
    pub fn mid(&self) -> (f32, f32) {
        ((self.x1 + self.x2) / 2.0, (self.y1 + self.y2) / 2.0)
    }
}

/// Intersect the ray from (cx, cy) toward (tx, ty) with the w×h box centred
/// at (cx, cy); returns the boundary point.
fn clip_to_box(cx: f32, cy: f32, hw: f32, hh: f32, tx: f32, ty: f32) -> (f32, f32) {
    let dx = tx - cx;
    let dy = ty - cy;
    if dx == 0.0 && dy == 0.0 {
        return (cx, cy);
    }
    let sx = if dx != 0.0 { hw / dx.abs() } else { f32::INFINITY };
    let sy = if dy != 0.0 { hh / dy.abs() } else { f32::INFINITY };
    let s = sx.min(sy).min(1.0);
    (cx + dx * s, cy + dy * s)
}

/// Geometry of an edge between two nodes, clipped at both bounding boxes.
pub fn edge_geometry(doc: &GuiDocument, edge: &GuiEdge) -> Option<EdgeGeom> {
    let a = doc.node(&edge.from)?;
    let b = doc.node(&edge.to)?;
    let (x1, y1) = clip_to_box(a.x, a.y, a.w / 2.0, a.h / 2.0, b.x, b.y);
    let (x2, y2) = clip_to_box(b.x, b.y, b.w / 2.0, b.h / 2.0, a.x, a.y);
    Some(EdgeGeom { x1, y1, x2, y2 })
}

/// Distance from point to segment (used for edge hit testing).
pub fn point_segment_distance(px: f32, py: f32, x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
    let dx = x2 - x1;
    let dy = y2 - y1;
    let len_sq = dx * dx + dy * dy;
    if len_sq == 0.0 {
        return ((px - x1).powi(2) + (py - y1).powi(2)).sqrt();
    }
    let t = (((px - x1) * dx + (py - y1) * dy) / len_sq).clamp(0.0, 1.0);
    let cx = x1 + t * dx;
    let cy = y1 + t * dy;
    ((px - cx).powi(2) + (py - cy).powi(2)).sqrt()
}

/// Snap a world coordinate to the grid.
pub fn snap(v: f32) -> f32 {
    (v / GRID).round() * GRID
}

/// Z-order command (nodes are stored back-to-front; the last node draws on top).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZOrder {
    ToFront,
    Forward,
    Backward,
    ToBack,
}

/// Editor: document + selection + undo/redo history.
///
/// Mutations snapshot the document for undo; consecutive mutations with the
/// same operation tag coalesce into a single undo step (so a drag is one
/// undo, not fifty). Call [`Editor::end_gesture`] when a pointer gesture ends.
#[derive(Debug, Clone)]
pub struct Editor {
    pub doc: GuiDocument,
    pub selection: Vec<String>,
    pub edge_selection: Vec<String>,
    undo: Vec<GuiDocument>,
    redo: Vec<GuiDocument>,
    pub dirty: bool,
    next_node: u64,
    next_edge: u64,
    last_op: Option<&'static str>,
}

impl Default for Editor {
    fn default() -> Self {
        Self::new()
    }
}

impl Editor {
    pub fn new() -> Self {
        Self {
            doc: GuiDocument::default(),
            selection: Vec::new(),
            edge_selection: Vec::new(),
            undo: Vec::new(),
            redo: Vec::new(),
            dirty: false,
            next_node: 1,
            next_edge: 1,
            last_op: None,
        }
    }

    /// Build an editor around a loaded document, resuming id counters.
    pub fn from_document(doc: GuiDocument) -> Self {
        let mut ed = Self::new();
        ed.next_node = resume_counter(doc.nodes.iter().map(|n| n.id.as_str()), 'n');
        ed.next_edge = resume_counter(doc.edges.iter().map(|e| e.id.as_str()), 'e');
        ed.doc = doc;
        ed.dirty = false;
        ed
    }

    // -- history ----------------------------------------------------------

    fn snapshot(&mut self, op: &'static str) {
        if self.last_op == Some(op) {
            return; // coalesce a run of identical ops into one undo step
        }
        self.last_op = Some(op);
        self.undo.push(self.doc.clone());
        if self.undo.len() > HISTORY_LIMIT {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    /// Break undo coalescing (call at the end of a pointer gesture).
    pub fn end_gesture(&mut self) {
        self.last_op = None;
    }

    fn restore(&mut self, doc: GuiDocument) {
        self.doc = doc;
        let ids: HashSet<&str> = self.doc.nodes.iter().map(|n| n.id.as_str()).collect();
        self.selection.retain(|id| ids.contains(id.as_str()));
        let edge_ids: HashSet<&str> = self.doc.edges.iter().map(|e| e.id.as_str()).collect();
        self.edge_selection.retain(|id| edge_ids.contains(id.as_str()));
        self.dirty = true;
    }

    pub fn undo(&mut self) -> bool {
        self.end_gesture();
        let Some(prev) = self.undo.pop() else {
            return false;
        };
        self.redo.push(self.doc.clone());
        self.restore(prev);
        true
    }

    pub fn redo(&mut self) -> bool {
        self.end_gesture();
        let Some(next) = self.redo.pop() else {
            return false;
        };
        self.undo.push(self.doc.clone());
        self.restore(next);
        true
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    // -- node ops ---------------------------------------------------------

    pub fn add_node(&mut self, shape_id: &str, x: f32, y: f32) -> String {
        self.snapshot("add-node");
        let def = shapes::shape_def(shape_id);
        let id = format!("n{}", self.next_node);
        self.next_node += 1;
        let node = GuiNode {
            id: id.clone(),
            shape: shape_id.to_string(),
            label: def.map(|d| d.name).unwrap_or("Node").to_string(),
            x,
            y,
            w: def.map(|d| d.w).unwrap_or(120.0),
            h: def.map(|d| d.h).unwrap_or(60.0),
            fill: def.map(|d| d.fill).unwrap_or([255, 255, 255]),
            stroke: def.map(|d| d.stroke).unwrap_or([31, 35, 40]),
            text: default_text_color(),
        };
        self.doc.nodes.push(node);
        self.selection = vec![id.clone()];
        self.edge_selection.clear();
        self.dirty = true;
        id
    }

    pub fn move_selected_by(&mut self, dx: f32, dy: f32, snap_to_grid: bool) {
        if self.selection.is_empty() || (dx == 0.0 && dy == 0.0) {
            return;
        }
        self.snapshot("move");
        for id in self.selection.clone() {
            if let Some(n) = self.doc.node_mut(&id) {
                n.x += dx;
                n.y += dy;
                if snap_to_grid {
                    n.x = snap(n.x);
                    n.y = snap(n.y);
                }
            }
        }
        self.dirty = true;
    }

    pub fn set_pos(&mut self, id: &str, x: f32, y: f32) {
        self.snapshot("move");
        if let Some(n) = self.doc.node_mut(id) {
            n.x = x;
            n.y = y;
            self.dirty = true;
        }
    }

    /// Snap the selected nodes to the grid (end-of-drag; coalesces with the
    /// preceding drag moves into one undo step).
    pub fn snap_selected(&mut self) {
        if self.selection.is_empty() {
            return;
        }
        self.snapshot("move");
        for id in self.selection.clone() {
            if let Some(n) = self.doc.node_mut(&id) {
                n.x = snap(n.x);
                n.y = snap(n.y);
            }
        }
        self.dirty = true;
    }

    pub fn resize_node(&mut self, id: &str, w: f32, h: f32) {
        self.snapshot("resize");
        if let Some(n) = self.doc.node_mut(id) {
            n.w = w.max(24.0);
            n.h = h.max(24.0);
            self.dirty = true;
        }
    }

    pub fn set_label(&mut self, id: &str, label: &str) {
        if self.doc.node(id).map(|n| n.label.as_str()) == Some(label) {
            return;
        }
        self.snapshot("label");
        if let Some(n) = self.doc.node_mut(id) {
            n.label = label.to_string();
            self.dirty = true;
        }
    }

    /// Swap the shape, keeping label/position/size (graphine-style).
    pub fn set_shape(&mut self, id: &str, shape_id: &str) {
        if self.doc.node(id).map(|n| n.shape.as_str()) == Some(shape_id) {
            return;
        }
        self.snapshot("shape");
        if let Some(n) = self.doc.node_mut(id) {
            n.shape = shape_id.to_string();
            self.dirty = true;
        }
    }

    pub fn set_colors(
        &mut self,
        id: &str,
        fill: Option<[u8; 3]>,
        stroke: Option<[u8; 3]>,
        text: Option<[u8; 3]>,
    ) {
        self.snapshot("colors");
        if let Some(n) = self.doc.node_mut(id) {
            if let Some(f) = fill {
                n.fill = f;
            }
            if let Some(s) = stroke {
                n.stroke = s;
            }
            if let Some(t) = text {
                n.text = t;
            }
            self.dirty = true;
        }
    }

    pub fn reorder(&mut self, id: &str, cmd: ZOrder) {
        self.snapshot("z-order");
        let Some(i) = self.doc.nodes.iter().position(|n| n.id == id) else {
            return;
        };
        let last = self.doc.nodes.len() - 1;
        let target = match cmd {
            ZOrder::ToFront => last,
            ZOrder::Forward => (i + 1).min(last),
            ZOrder::Backward => i.saturating_sub(1),
            ZOrder::ToBack => 0,
        };
        if target != i {
            let node = self.doc.nodes.remove(i);
            self.doc.nodes.insert(target, node);
            self.dirty = true;
        }
    }

    // -- edge ops ---------------------------------------------------------

    /// Connect two nodes; returns the new edge id, or None for a self-loop /
    /// duplicate (same ordered pair already connected).
    pub fn connect(&mut self, from: &str, to: &str) -> Option<String> {
        if from == to {
            return None;
        }
        if self.doc.node(from).is_none() || self.doc.node(to).is_none() {
            return None;
        }
        if self.doc.edges.iter().any(|e| e.from == from && e.to == to) {
            return None;
        }
        self.snapshot("connect");
        let id = format!("e{}", self.next_edge);
        self.next_edge += 1;
        self.doc.edges.push(GuiEdge {
            id: id.clone(),
            from: from.to_string(),
            to: to.to_string(),
            label: String::new(),
            style: EdgeStyle::Arrow,
            color: default_edge_color(),
        });
        self.dirty = true;
        Some(id)
    }

    pub fn set_edge_label(&mut self, id: &str, label: &str) {
        if self.doc.edge(id).map(|e| e.label.as_str()) == Some(label) {
            return;
        }
        self.snapshot("edge-label");
        if let Some(e) = self.doc.edge_mut(id) {
            e.label = label.to_string();
            self.dirty = true;
        }
    }

    pub fn set_edge_style(&mut self, id: &str, style: EdgeStyle) {
        self.snapshot("edge-style");
        if let Some(e) = self.doc.edge_mut(id) {
            e.style = style;
            self.dirty = true;
        }
    }

    pub fn set_edge_color(&mut self, id: &str, color: [u8; 3]) {
        self.snapshot("edge-color");
        if let Some(e) = self.doc.edge_mut(id) {
            e.color = color;
            self.dirty = true;
        }
    }

    // -- selection ops ------------------------------------------------------

    pub fn clear_selection(&mut self) {
        self.selection.clear();
        self.edge_selection.clear();
    }

    pub fn select_all(&mut self) {
        self.selection = self.doc.nodes.iter().map(|n| n.id.clone()).collect();
        self.edge_selection.clear();
    }

    pub fn select_only(&mut self, id: &str) {
        self.selection = vec![id.to_string()];
        self.edge_selection.clear();
    }

    pub fn toggle_select(&mut self, id: &str) {
        if let Some(i) = self.selection.iter().position(|s| s == id) {
            self.selection.remove(i);
        } else {
            self.selection.push(id.to_string());
        }
    }

    pub fn select_only_edge(&mut self, id: &str) {
        self.edge_selection = vec![id.to_string()];
        self.selection.clear();
    }

    pub fn delete_selected(&mut self) {
        if self.selection.is_empty() && self.edge_selection.is_empty() {
            return;
        }
        self.snapshot("delete");
        let nodes: HashSet<&str> = self.selection.iter().map(String::as_str).collect();
        let edges: HashSet<&str> = self.edge_selection.iter().map(String::as_str).collect();
        self.doc.nodes.retain(|n| !nodes.contains(n.id.as_str()));
        self.doc.edges.retain(|e| {
            !edges.contains(e.id.as_str())
                && !nodes.contains(e.from.as_str())
                && !nodes.contains(e.to.as_str())
        });
        self.clear_selection();
        self.dirty = true;
    }

    /// Duplicate the selected nodes (offset by 24px) plus edges fully inside
    /// the selection. The copies become the new selection.
    pub fn duplicate_selected(&mut self) {
        if self.selection.is_empty() {
            return;
        }
        self.snapshot("duplicate");
        let selected: HashSet<String> = self.selection.iter().cloned().collect();
        let mut id_map: Vec<(String, String)> = Vec::new();
        let mut new_nodes = Vec::new();
        for n in &self.doc.nodes {
            if selected.contains(&n.id) {
                let new_id = format!("n{}", self.next_node);
                self.next_node += 1;
                let mut copy = n.clone();
                copy.id = new_id.clone();
                copy.x += 24.0;
                copy.y += 24.0;
                id_map.push((n.id.clone(), new_id));
                new_nodes.push(copy);
            }
        }
        let mut new_edges = Vec::new();
        for e in &self.doc.edges {
            let from_new = id_map.iter().find(|(old, _)| old == &e.from);
            let to_new = id_map.iter().find(|(old, _)| old == &e.to);
            if let (Some((_, f)), Some((_, t))) = (from_new, to_new) {
                let mut copy = e.clone();
                copy.id = format!("e{}", self.next_edge);
                self.next_edge += 1;
                copy.from = f.clone();
                copy.to = t.clone();
                new_edges.push(copy);
            }
        }
        self.doc.nodes.extend(new_nodes);
        self.doc.edges.extend(new_edges);
        self.selection = id_map.into_iter().map(|(_, new)| new).collect();
        self.edge_selection.clear();
        self.dirty = true;
    }

    // -- queries ----------------------------------------------------------

    /// Topmost node at a world point (nodes later in the vec draw on top).
    pub fn hit_node(&self, x: f32, y: f32) -> Option<&str> {
        self.doc
            .nodes
            .iter()
            .rev()
            .find(|n| n.contains(x, y))
            .map(|n| n.id.as_str())
    }

    /// Nearest edge within `tol` of a world point.
    pub fn hit_edge(&self, x: f32, y: f32, tol: f32) -> Option<&str> {
        let mut best: Option<(&str, f32)> = None;
        for e in &self.doc.edges {
            if let Some(g) = edge_geometry(&self.doc, e) {
                let d = point_segment_distance(x, y, g.x1, g.y1, g.x2, g.y2);
                if d <= tol && best.map(|(_, bd)| d < bd).unwrap_or(true) {
                    best = Some((e.id.as_str(), d));
                }
            }
        }
        best.map(|(id, _)| id)
    }

    /// Ids of nodes fully inside a world-space rect (marquee selection).
    pub fn nodes_in_rect(&self, minx: f32, miny: f32, maxx: f32, maxy: f32) -> Vec<String> {
        self.doc
            .nodes
            .iter()
            .filter(|n| {
                n.left() >= minx && n.right() <= maxx && n.top() >= miny && n.bottom() <= maxy
            })
            .map(|n| n.id.clone())
            .collect()
    }

    // -- export -----------------------------------------------------------

    /// Topology-only conversion to the flowchart IR (positions/colors are not
    /// representable there; shape kinds degrade to the IR subset).
    pub fn to_ir(&self) -> diagram::Diagram {
        let mut d = diagram::Diagram::new("LR");
        for n in &self.doc.nodes {
            d.nodes.push(diagram::Node {
                id: n.id.clone(),
                text: n.label.clone(),
                shape: n.kind().to_ir(),
                href: None,
                tooltip: None,
            });
        }
        for e in &self.doc.edges {
            d.edges.push(diagram::Edge {
                from: e.from.clone(),
                to: e.to.clone(),
                label: e.label.clone(),
                style: e.style,
            });
        }
        d
    }

    /// Mermaid flowchart export via the IR.
    pub fn to_mermaid(&self) -> String {
        self.to_ir().to_mermaid()
    }

    /// Positioned (WYSIWYG) SVG export — deterministic for a given document.
    pub fn to_svg(&self) -> String {
        let pad = 32.0_f32;
        let (minx, miny, w, h) = match self.doc.bounds() {
            Some((minx, miny, maxx, maxy)) => (
                minx - pad,
                miny - pad,
                (maxx - minx) + pad * 2.0,
                (maxy - miny) + pad * 2.0,
            ),
            None => (0.0, 0.0, 400.0, 300.0),
        };
        let mut out = String::new();
        out.push_str(&format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"{} {} {} {}\" font-family=\"sans-serif\">",
            shapes::num(minx),
            shapes::num(miny),
            shapes::num(w),
            shapes::num(h)
        ));
        // One arrowhead marker per distinct edge color.
        let mut colors: Vec<[u8; 3]> = Vec::new();
        for e in &self.doc.edges {
            if !colors.contains(&e.color) {
                colors.push(e.color);
            }
        }
        if !colors.is_empty() {
            out.push_str("<defs>");
            for c in &colors {
                let hex = shapes::hex(*c);
                out.push_str(&format!(
                    "<marker id=\"arrow-{}\" viewBox=\"0 0 10 10\" refX=\"9\" refY=\"5\" markerWidth=\"8\" markerHeight=\"8\" orient=\"auto-start-reverse\"><path d=\"M 0 0 L 10 5 L 0 10 z\" fill=\"{}\"/></marker>",
                    &hex[1..],
                    hex
                ));
            }
            out.push_str("</defs>");
        }
        // Edges first (they sit behind the nodes).
        for e in &self.doc.edges {
            let Some(g) = edge_geometry(&self.doc, e) else {
                continue;
            };
            let hex = shapes::hex(e.color);
            let width = if e.style == EdgeStyle::Thick { 3.0 } else { 1.5 };
            let dash = if e.style == EdgeStyle::Dashed {
                " stroke-dasharray=\"6 4\""
            } else {
                ""
            };
            out.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{} marker-end=\"url(#arrow-{})\"/>",
                shapes::num(g.x1),
                shapes::num(g.y1),
                shapes::num(g.x2),
                shapes::num(g.y2),
                hex,
                shapes::num(width),
                dash,
                &hex[1..]
            ));
            if !e.label.is_empty() {
                let (mx, my) = g.mid();
                out.push_str(&text_svg(mx, my - 6.0, &e.label, shapes::hex(e.color), 11.0));
            }
        }
        for n in &self.doc.nodes {
            let fill = shapes::hex(n.fill);
            let stroke = shapes::hex(n.stroke);
            shapes::shape_svg(
                &mut out,
                n.kind(),
                n.x,
                n.y,
                n.w,
                n.h,
                shapes::SvgStyle { fill: &fill, stroke: &stroke },
            );
            if let Some(badge) = shapes::shape_def(&n.shape).and_then(|d| d.badge) {
                out.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" font-size=\"8\" font-weight=\"bold\" fill=\"{}\" opacity=\"0.75\">{}</text>",
                    shapes::num(n.left() + 6.0),
                    shapes::num(n.top() + 12.0),
                    stroke,
                    badge
                ));
            }
            out.push_str(&text_svg(n.x, n.y, &n.label, shapes::hex(n.text), 13.0));
        }
        out.push_str("</svg>");
        out
    }
}

/// Multi-line centered SVG text block.
fn text_svg(x: f32, y: f32, text: &str, fill: String, size: f32) -> String {
    let lines: Vec<&str> = text.split('\n').collect();
    let line_h = size * 1.25;
    let total = line_h * (lines.len() as f32 - 1.0);
    let mut out = String::new();
    for (i, line) in lines.iter().enumerate() {
        let ly = y - total / 2.0 + line_h * i as f32 + size * 0.35;
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-size=\"{}\" fill=\"{}\" text-anchor=\"middle\">{}</text>",
            shapes::num(x),
            shapes::num(ly),
            shapes::num(size),
            fill,
            shapes::xml_escape(line)
        ));
    }
    out
}

/// Resume an id counter after loading: max numeric suffix + 1.
fn resume_counter<'a>(ids: impl Iterator<Item = &'a str>, prefix: char) -> u64 {
    ids.filter_map(|id| id.strip_prefix(prefix)?.parse::<u64>().ok())
        .max()
        .unwrap_or(0)
        + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    fn editor_with_two_nodes() -> Editor {
        let mut ed = Editor::new();
        ed.add_node("basic/rect", 100.0, 100.0);
        ed.end_gesture(); // the canvas calls this after placing each node
        ed.add_node("aws/lambda", 300.0, 100.0);
        ed.end_gesture();
        ed
    }

    #[test]
    fn json_roundtrip_preserves_document() {
        let mut ed = editor_with_two_nodes();
        ed.connect("n1", "n2");
        let json = ed.doc.to_json().unwrap();
        let loaded = GuiDocument::from_json(&json).unwrap();
        assert_eq!(loaded.nodes.len(), 2);
        assert_eq!(loaded.edges.len(), 1);
        assert_eq!(loaded.edges[0].from, "n1");
        assert_eq!(loaded.nodes[1].shape, "aws/lambda");
    }

    #[test]
    fn sanitize_drops_dangling_edges() {
        let json = r#"{
            "version": 1, "name": "t",
            "nodes": [{"id":"n1","shape":"basic/rect","label":"A","x":0,"y":0,"w":100,"h":50,"fill":[255,255,255],"stroke":[0,0,0],"text":[0,0,0]}],
            "edges": [{"id":"e1","from":"n1","to":"ghost","label":"","style":"Arrow","color":[0,0,0]}]
        }"#;
        let doc = GuiDocument::from_json(json).unwrap();
        assert!(doc.edges.is_empty());
    }

    #[test]
    fn undo_redo_roundtrip() {
        let mut ed = editor_with_two_nodes();
        assert_eq!(ed.doc.nodes.len(), 2);
        ed.undo();
        assert_eq!(ed.doc.nodes.len(), 1);
        ed.undo();
        assert_eq!(ed.doc.nodes.len(), 0);
        ed.redo();
        ed.redo();
        assert_eq!(ed.doc.nodes.len(), 2);
        assert!(!ed.can_redo());
    }

    #[test]
    fn connect_rejects_self_loop_and_duplicates() {
        let mut ed = editor_with_two_nodes();
        assert!(ed.connect("n1", "n1").is_none());
        assert!(ed.connect("n1", "n2").is_some());
        assert!(ed.connect("n1", "n2").is_none(), "duplicate edge");
        assert_eq!(ed.doc.edges.len(), 1);
    }

    #[test]
    fn delete_node_removes_incident_edges() {
        let mut ed = editor_with_two_nodes();
        ed.connect("n1", "n2");
        ed.select_only("n2");
        ed.delete_selected();
        assert_eq!(ed.doc.nodes.len(), 1);
        assert!(ed.doc.edges.is_empty());
    }

    #[test]
    fn duplicate_copies_nodes_and_internal_edges() {
        let mut ed = editor_with_two_nodes();
        ed.connect("n1", "n2");
        ed.select_all();
        ed.duplicate_selected();
        assert_eq!(ed.doc.nodes.len(), 4);
        assert_eq!(ed.doc.edges.len(), 2);
        assert_eq!(ed.selection.len(), 2);
        assert!(ed.selection.iter().all(|id| id.starts_with('n')));
        // The copied edge connects the copies, not the originals.
        let new_edge = &ed.doc.edges[1];
        assert_ne!(new_edge.from, "n1");
        assert!(ed.doc.node(&new_edge.from).is_some());
    }

    #[test]
    fn move_coalesces_into_single_undo_step() {
        let mut ed = editor_with_two_nodes();
        ed.select_only("n1");
        for _ in 0..10 {
            ed.move_selected_by(1.0, 0.0, false);
        }
        ed.end_gesture();
        let n1 = ed.doc.node("n1").unwrap();
        assert_eq!(n1.x, 110.0);
        assert!(ed.undo());
        let n1 = ed.doc.node("n1").unwrap();
        assert_eq!(n1.x, 100.0);
    }

    #[test]
    fn hit_test_returns_topmost() {
        let mut ed = Editor::new();
        ed.add_node("basic/rect", 100.0, 100.0);
        ed.add_node("basic/rect", 110.0, 100.0); // overlaps, on top
        assert_eq!(ed.hit_node(105.0, 100.0), Some("n2"));
        ed.reorder("n2", ZOrder::ToBack);
        assert_eq!(ed.hit_node(105.0, 100.0), Some("n1"));
    }

    #[test]
    fn edge_hit_test_uses_segment_distance() {
        let mut ed = editor_with_two_nodes();
        ed.connect("n1", "n2");
        // Midpoint of the n1→n2 line is roughly (200, 100).
        assert_eq!(ed.hit_edge(200.0, 103.0, 6.0), Some("e1"));
        assert_eq!(ed.hit_edge(200.0, 160.0, 6.0), None);
    }

    #[test]
    fn counters_resume_after_load() {
        let mut ed = editor_with_two_nodes();
        ed.connect("n1", "n2");
        let json = ed.doc.to_json().unwrap();
        let doc = GuiDocument::from_json(&json).unwrap();
        let mut ed2 = Editor::from_document(doc);
        let id = ed2.add_node("basic/rect", 0.0, 0.0);
        assert_eq!(id, "n3");
        assert!(ed2.connect("n3", "n1").is_some());
    }

    #[test]
    fn mermaid_export_contains_nodes_and_edges() {
        let mut ed = editor_with_two_nodes();
        ed.connect("n1", "n2");
        let mmd = ed.to_mermaid();
        assert!(mmd.contains("graph LR"), "{mmd}");
        assert!(mmd.contains("n1"));
        assert!(mmd.contains("n2"));
        assert!(mmd.contains("-->"));
    }

    #[test]
    fn svg_export_is_deterministic_and_positioned() {
        let mut ed = editor_with_two_nodes();
        ed.connect("n1", "n2");
        let svg1 = ed.to_svg();
        let svg2 = ed.to_svg();
        assert_eq!(svg1, svg2, "SVG export must be deterministic");
        assert!(svg1.starts_with("<svg"));
        assert!(svg1.contains("marker"), "arrowhead markers missing");
        assert!(svg1.contains("Lambda"), "node label missing");
        assert!(svg1.contains("AWS"), "provider badge missing");
        assert!(svg1.contains("#e67300"), "aws stroke color missing");
    }

    #[test]
    fn svg_escapes_label_text() {
        let mut ed = Editor::new();
        ed.add_node("basic/rect", 0.0, 0.0);
        ed.set_label("n1", "a <b> & \"c\"");
        let svg = ed.to_svg();
        assert!(svg.contains("a &lt;b&gt; &amp; &quot;c&quot;"));
        assert!(!svg.contains("<b>"));
    }

    #[test]
    fn edge_geometry_clips_at_node_bounds() {
        let mut ed = editor_with_two_nodes();
        ed.connect("n1", "n2");
        let g = edge_geometry(&ed.doc, &ed.doc.edges[0]).unwrap();
        let n1 = ed.doc.node("n1").unwrap();
        let n2 = ed.doc.node("n2").unwrap();
        assert!((g.x1 - n1.right()).abs() < 0.01, "x1={} right={}", g.x1, n1.right());
        assert!((g.x2 - n2.left()).abs() < 0.01);
        assert_eq!(g.y1, n1.y);
    }

    #[test]
    fn snap_selected_rounds_to_grid() {
        let mut ed = Editor::new();
        ed.add_node("basic/rect", 103.0, 97.0);
        ed.end_gesture();
        ed.snap_selected();
        let n = ed.doc.node("n1").unwrap();
        assert_eq!((n.x, n.y), (104.0, 96.0));
        // Coalesces with preceding moves: still a single undo back to pre-move.
        assert!(ed.undo());
        let n = ed.doc.node("n1").unwrap();
        assert_eq!((n.x, n.y), (103.0, 97.0));
    }

    #[test]
    fn z_order_forward_backward() {
        let mut ed = Editor::new();
        ed.add_node("basic/rect", 0.0, 0.0);
        ed.add_node("basic/rect", 0.0, 0.0);
        ed.add_node("basic/rect", 0.0, 0.0);
        ed.reorder("n1", ZOrder::ToFront);
        assert_eq!(ed.doc.nodes[2].id, "n1");
        ed.reorder("n1", ZOrder::Backward);
        assert_eq!(ed.doc.nodes[1].id, "n1");
        ed.reorder("n1", ZOrder::ToBack);
        assert_eq!(ed.doc.nodes[0].id, "n1");
    }
}
