use crate::diagram::{Diagram, NodeShape};
use std::collections::{HashMap, HashSet, VecDeque};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct LayoutNode {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub label: String,
    pub shape: NodeShape,
}

#[derive(Debug, Clone, Serialize)]
pub struct LayoutEdge {
    pub from: String,
    pub to: String,
    pub label: String,
    pub points: Vec<(f64, f64)>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Layout {
    pub nodes: Vec<LayoutNode>,
    pub edges: Vec<LayoutEdge>,
    pub width: f64,
    pub height: f64,
}

const NODE_WIDTH: f64 = 130.0;
const NODE_HEIGHT: f64 = 46.0;
const LAYER_GAP: f64 = 80.0;
const NODE_GAP: f64 = 40.0;
const PADDING: f64 = 50.0;

pub fn layout(diagram: &Diagram) -> Layout {
    let mut incoming: HashMap<&str, Vec<&str>> = HashMap::new();
    let mut outgoing: HashMap<&str, Vec<&str>> = HashMap::new();
    let mut all_ids: HashSet<&str> = HashSet::new();

    for n in &diagram.nodes {
        all_ids.insert(n.id.as_str());
        incoming.entry(n.id.as_str()).or_default();
        outgoing.entry(n.id.as_str()).or_default();
    }

    for e in &diagram.edges {
        incoming.entry(e.to.as_str()).or_default().push(e.from.as_str());
        outgoing.entry(e.from.as_str()).or_default().push(e.to.as_str());
    }

    let mut layers: HashMap<&str, usize> = HashMap::new();
    let mut queue: VecDeque<&str> = VecDeque::new();

    for &id in &all_ids {
        if incoming.get(id).map_or(true, |i| i.is_empty()) {
            queue.push_back(id);
            layers.insert(id, 0);
        }
    }

    if queue.is_empty() {
        if let Some(first) = all_ids.iter().next() {
            queue.push_back(first);
            layers.insert(first, 0);
        }
    }

    while let Some(node) = queue.pop_front() {
        let layer = layers[node];
        if let Some(children) = outgoing.get(node) {
            for child in children.clone() {
                let new_layer = layer + 1;
                let updated = match layers.get(child) {
                    Some(&existing) if existing >= new_layer => false,
                    _ => true,
                };
                if updated {
                    layers.insert(child, new_layer);
                    queue.push_back(child);
                }
            }
        }
    }

    let max_layer = layers.values().copied().max().unwrap_or(0);

    let is_horizontal = diagram.rankdir == "LR" || diagram.rankdir == "RL";
    let reverse = diagram.rankdir == "RL" || diagram.rankdir == "BT";

    let mut layer_nodes: HashMap<usize, Vec<&str>> = HashMap::new();
    for (&id, &layer) in &layers {
        layer_nodes.entry(layer).or_default().push(id);
    }

    for layer in layer_nodes.values_mut() {
        layer.sort_by(|a, b| {
            let a_in = incoming.get(a).map(|v| v.len()).unwrap_or(0);
            let b_in = incoming.get(b).map(|v| v.len()).unwrap_or(0);
            b_in.cmp(&a_in)
        });
    }

    let mut layout_nodes: Vec<LayoutNode> = Vec::new();
    let mut pos_map: HashMap<&str, (f64, f64)> = HashMap::new();

    for layer in 0..=max_layer {
        let nodes_in_layer = layer_nodes.get(&layer).map(|v| v.as_slice()).unwrap_or(&[]);
        let count = nodes_in_layer.len() as f64;
        let total_width = (count - 1.0) * NODE_GAP + count * NODE_WIDTH;
        let start_x = (total_width / -2.0) + NODE_WIDTH / 2.0;

        for (i, &id) in nodes_in_layer.iter().enumerate() {
            let node = diagram.get_node(id).unwrap();
            let layer_actual = if reverse { max_layer - layer } else { layer };
            let x = if is_horizontal {
                layer_actual as f64 * (NODE_WIDTH + LAYER_GAP) + NODE_WIDTH / 2.0 + PADDING
            } else {
                start_x + i as f64 * (NODE_WIDTH + NODE_GAP) + PADDING
            };
            let y = if is_horizontal {
                start_x + i as f64 * (NODE_HEIGHT + NODE_GAP) + PADDING
            } else {
                layer_actual as f64 * (NODE_HEIGHT + LAYER_GAP) + NODE_HEIGHT / 2.0 + PADDING
            };

            pos_map.insert(id, (x, y));
            layout_nodes.push(LayoutNode {
                id: id.to_string(),
                x,
                y,
                width: NODE_WIDTH,
                height: NODE_HEIGHT,
                label: node.text.clone(),
                shape: node.shape,
            });
        }
    }

    let mut layout_edges: Vec<LayoutEdge> = Vec::new();
    for e in &diagram.edges {
        let from_pos = pos_map.get(e.from.as_str());
        let to_pos = pos_map.get(e.to.as_str());
        if let (Some(&(fx, fy)), Some(&(tx, ty))) = (from_pos, to_pos) {
            let start = point_on_rect(tx, ty, fx - NODE_WIDTH / 2.0, fy - NODE_HEIGHT / 2.0, NODE_WIDTH, NODE_HEIGHT);
            let end = point_on_rect(fx, fy, tx - NODE_WIDTH / 2.0, ty - NODE_HEIGHT / 2.0, NODE_WIDTH, NODE_HEIGHT);
            let mid_x = (fx + tx) / 2.0;
            let mid_y = (fy + ty) / 2.0;

            layout_edges.push(LayoutEdge {
                from: e.from.clone(),
                to: e.to.clone(),
                label: e.label.clone(),
                points: vec![start, (mid_x, mid_y), end],
            });
        }
    }

    let mut min_x = f64::MAX;
    let mut min_y = f64::MAX;
    let mut max_x = f64::MIN;
    let mut max_y = f64::MIN;

    for n in &layout_nodes {
        let hw = n.width / 2.0;
        let hh = n.height / 2.0;
        min_x = min_x.min(n.x - hw);
        min_y = min_y.min(n.y - hh);
        max_x = max_x.max(n.x + hw);
        max_y = max_y.max(n.y + hh);
    }

    let width = (max_x - min_x + PADDING * 2.0).max(300.0);
    let height = (max_y - min_y + PADDING * 2.0).max(200.0);
    let ox = PADDING - min_x;
    let oy = PADDING - min_y;

    for n in &mut layout_nodes {
        n.x += ox;
        n.y += oy;
    }
    for e in &mut layout_edges {
        for p in &mut e.points {
            p.0 += ox;
            p.1 += oy;
        }
    }

    Layout {
        nodes: layout_nodes,
        edges: layout_edges,
        width,
        height,
    }
}

fn point_on_rect(
    px: f64,
    py: f64,
    rx: f64,
    ry: f64,
    rw: f64,
    rh: f64,
) -> (f64, f64) {
    let cx = rx + rw / 2.0;
    let cy = ry + rh / 2.0;
    let dx = px - cx;
    let dy = py - cy;
    let ax = dx.abs();
    let ay = dy.abs();
    let hw = rw / 2.0;
    let hh = rh / 2.0;

    if ax * hh > ay * hw {
        (cx + dx.signum() * hw, cy + dy * hw / ax.max(1e-10))
    } else if ay > 0.0 {
        (cx + dx * hh / ay, cy + dy.signum() * hh)
    } else {
        (cx, cy - hh)
    }
}
