use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum NodeShape {
    Rect,
    Diamond,
    Stadium,
}

impl NodeShape {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "rect" | "rectangle" => Some(Self::Rect),
            "diamond" => Some(Self::Diamond),
            "stadium" => Some(Self::Stadium),
            _ => None,
        }
    }
}

impl fmt::Display for NodeShape {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Rect => write!(f, "rect"),
            Self::Diamond => write!(f, "diamond"),
            Self::Stadium => write!(f, "stadium"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    pub text: String,
    pub shape: NodeShape,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub from: String,
    pub to: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagram {
    pub rankdir: String,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}

impl Diagram {
    pub fn new(rankdir: &str) -> Self {
        Self {
            rankdir: rankdir.to_string(),
            nodes: Vec::new(),
            edges: Vec::new(),
        }
    }

    pub fn add_node(&mut self, node: Node) -> Result<(), String> {
        if self.nodes.iter().any(|n| n.id == node.id) {
            return Err(format!("node '{}' already exists", node.id));
        }
        self.nodes.push(node);
        Ok(())
    }

    pub fn remove_node(&mut self, id: &str) {
        self.nodes.retain(|n| n.id != id);
        self.edges.retain(|e| e.from != id && e.to != id);
    }

    pub fn update_node(
        &mut self,
        id: &str,
        text: Option<&str>,
        shape: Option<NodeShape>,
    ) -> Result<(), String> {
        let node = self
            .nodes
            .iter_mut()
            .find(|n| n.id == id)
            .ok_or_else(|| format!("node '{}' not found", id))?;
        if let Some(t) = text {
            node.text = t.to_string();
        }
        if let Some(s) = shape {
            node.shape = s;
        }
        Ok(())
    }

    pub fn add_edge(&mut self, edge: Edge) -> Result<(), String> {
        if !self.nodes.iter().any(|n| n.id == edge.from) {
            return Err(format!("node '{}' not found", edge.from));
        }
        if !self.nodes.iter().any(|n| n.id == edge.to) {
            return Err(format!("node '{}' not found", edge.to));
        }
        self.edges.push(edge);
        Ok(())
    }

    pub fn remove_edge(&mut self, from: &str, to: &str) -> Result<(), String> {
        let before = self.edges.len();
        self.edges.retain(|e| e.from != from || e.to != to);
        if self.edges.len() == before {
            return Err(format!("edge '{} -> {}' not found", from, to));
        }
        Ok(())
    }

    pub fn get_node(&self, id: &str) -> Option<&Node> {
        self.nodes.iter().find(|n| n.id == id)
    }

    pub fn to_mermaid(&self) -> String {
        let mut out = format!("graph {}\n", self.rankdir);
        for n in &self.nodes {
            let t = &n.text;
            match n.shape {
                NodeShape::Rect => out.push_str(&format!("    {}[{}]\n", n.id, t)),
                NodeShape::Diamond => out.push_str(&format!("    {}{{{}}}\n", n.id, t)),
                NodeShape::Stadium => out.push_str(&format!("    {}({})\n", n.id, t)),
            }
        }
        for e in &self.edges {
            if e.label.is_empty() {
                out.push_str(&format!("    {} --> {}\n", e.from, e.to));
            } else {
                out.push_str(&format!("    {} -->|{}| {}\n", e.from, e.label, e.to));
            }
        }
        out.trim().to_string()
    }
}
